use std::f32::consts::{FRAC_PI_2, PI, TAU};

use bevy::{
    input::mouse::{MouseMotion, MouseScrollUnit, MouseWheel},
    prelude::*,
    window::{CursorGrabMode, PrimaryWindow},
};

// Bundle to spawn our custom camera easily
#[derive(Bundle, Default)]
pub struct PanOrbitCameraBundle {
    pub camera: Camera3dBundle,
    pub state: PanOrbitState,
    pub settings: PanOrbitSettings,
}

// The internal state of the pan-orbit controller
#[derive(Component)]
pub struct PanOrbitState {
    pub center: Vec3,
    pub radius: f32,
    pub upside_down: bool,
    pub pitch: f32,
    pub yaw: f32,
    pub min_radius: f32,
    pub max_radius: f32,
}

/// The configuration of the pan-orbit controller
#[derive(Component)]
pub struct PanOrbitSettings {
    /// World units per pixel of mouse motion
    pub pan_sensitivity: f32,
    /// Radians per pixel of mouse motion
    pub orbit_sensitivity: f32,
    /// Exponent per pixel of mouse motion
    pub zoom_sensitivity: f32,
    /// Key to hold for panning
    pub pan_key: Option<MouseButton>,
    /// Key to hold for orbiting
    pub orbit_key: Option<MouseButton>,
    /// What action is bound to the scroll wheel?
    pub scroll_action: Option<PanOrbitAction>,
    /// For devices with a notched scroll wheel, like desktop mice
    pub scroll_line_sensitivity: f32,
    /// For devices with smooth scrolling, like touchpads
    pub scroll_pixel_sensitivity: f32,
}

impl Default for PanOrbitState {
    fn default() -> Self {
        PanOrbitState {
            max_radius: 1000.0,
            min_radius: 0.1,
            center: Vec3::ZERO,
            radius: 20.0,
            upside_down: false,
            pitch: -0.55196005,
            yaw: -0.4406954,
        }
    }
}

impl Default for PanOrbitSettings {
    fn default() -> Self {
        PanOrbitSettings {
            pan_sensitivity: 0.001,                 // 1000 pixels per world unit
            orbit_sensitivity: 0.1f32.to_radians(), // 0.1 degree per pixel
            zoom_sensitivity: 0.01,
            pan_key: Some(MouseButton::Right),
            orbit_key: Some(MouseButton::Left),
            scroll_action: Some(PanOrbitAction::Zoom),
            scroll_line_sensitivity: 1.0, // 1 "line" == 16 "pixels of motion"
            scroll_pixel_sensitivity: 1.0 / 16.0,
        }
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PanOrbitAction {
    Orbit,
    Zoom,
}

#[derive(Component)]
pub struct PrimaryCamera;

pub struct PanOrbitCameraPlugin;

impl PanOrbitCameraPlugin {
    fn pan_orbit_camera_controller(
        mut allow_orbit: Local<bool>,
        mouse: Res<ButtonInput<MouseButton>>,
        mut evr_motion: EventReader<MouseMotion>,
        mut evr_scroll: EventReader<MouseWheel>,
        mut window: Query<&mut Window, With<PrimaryWindow>>,
        mut q_camera: Query<
            (&PanOrbitSettings, &mut PanOrbitState, &mut Transform),
            With<Camera3d>,
        >,
    ) {
        // First, accumulate the total amount of
        let mut total_motion: Vec2 = evr_motion.read().map(|ev| ev.delta).sum();

        // Reverse Y (Bevy's Worldspace coordinate system is Y-Up,
        // but events are in window/ui coordinates, which are Y-Down)
        total_motion.y = -total_motion.y;

        let mut total_scroll_lines = Vec2::ZERO;
        let mut total_scroll_pixels = Vec2::ZERO;

        for ev in evr_scroll.read() {
            match ev.unit {
                MouseScrollUnit::Line => {
                    total_scroll_lines.x += ev.x;
                    total_scroll_lines.y -= ev.y;
                }
                MouseScrollUnit::Pixel => {
                    total_scroll_pixels.x += ev.x;
                    total_scroll_pixels.y -= ev.y;
                }
            }
        }

        let mut window = window.single_mut();

        if mouse.just_pressed(MouseButton::Left) {
            if let Some(cursor_position) = window.cursor_position() {
                let threshold = 200.0;
                let (x, y) = (cursor_position.x, cursor_position.y);
                if x > window.width() - threshold && y < threshold {
                    *allow_orbit = true;
                    window.cursor.grab_mode = CursorGrabMode::Confined;
                }
            }
        }

        for (settings, mut state, mut transform) in &mut q_camera {
            // Check how much of each thing we need to apply.
            // Accumulate values from motion and scroll,
            // based on our configuration settings.
            let mut total_pan = Vec2::ZERO;
            if settings
                .pan_key
                .map(|key| mouse.pressed(key))
                .unwrap_or(false)
            {
                total_pan -= total_motion * settings.pan_sensitivity;
            }

            let mut total_orbit = Vec2::ZERO;

            if settings
                .orbit_key
                .map(|key| mouse.pressed(key))
                .unwrap_or(false)
                && *allow_orbit
            {
                total_orbit -= (total_motion * Vec2::new(1.0, -1.0)) * settings.orbit_sensitivity;
            } else {
                *allow_orbit = false;
                window.cursor.grab_mode = CursorGrabMode::None;
            }

            if settings.scroll_action == Some(PanOrbitAction::Orbit) {
                total_orbit -= total_scroll_lines
                    * settings.scroll_line_sensitivity
                    * settings.orbit_sensitivity;
                total_orbit -= total_scroll_pixels
                    * settings.scroll_pixel_sensitivity
                    * settings.orbit_sensitivity;
            }

            let mut total_zoom = Vec2::ZERO;
            if settings.scroll_action == Some(PanOrbitAction::Zoom) {
                total_zoom -= total_scroll_lines
                    * settings.scroll_line_sensitivity
                    * settings.zoom_sensitivity;
                total_zoom -= total_scroll_pixels
                    * settings.scroll_pixel_sensitivity
                    * settings.zoom_sensitivity;
            }

            // Upon starting a new orbit maneuver (key is just pressed),
            // check if we are starting it upside-down
            if settings
                .orbit_key
                .map(|key| mouse.just_pressed(key))
                .unwrap_or(false)
            {
                state.upside_down = state.pitch < -FRAC_PI_2 || state.pitch > FRAC_PI_2;
            }

            // If we are upside down, reverse the X orbiting
            if state.upside_down {
                total_orbit.x = -total_orbit.x;
            }

            // Now we can actually do the things!

            let mut any = false;

            // To ZOOM, we need to multiply our radius.
            if total_zoom != Vec2::ZERO {
                any = true;
                // in order for zoom to feel intuitive,
                // everything needs to be exponential
                // (done via multiplication)
                // not linear
                // (done via addition)

                // so we compute the exponential of our
                // accumulated value and multiply by that
                state.radius *= (-total_zoom.y).exp();
                state.radius = state.radius.min(state.max_radius).max(state.min_radius);
            }

            // To ORBIT, we change our pitch and yaw values
            if total_orbit != Vec2::ZERO {
                any = true;
                state.yaw += total_orbit.x;
                state.pitch += total_orbit.y;
                // wrap around, to stay between +- 180 degrees
                if state.yaw > PI {
                    state.yaw -= TAU; // 2 * PI
                }
                if state.yaw < -PI {
                    state.yaw += TAU; // 2 * PI
                }
                if state.pitch > PI {
                    state.pitch -= TAU; // 2 * PI
                }
                if state.pitch < -PI {
                    state.pitch += TAU; // 2 * PI
                }
            }

            // To PAN, we can get the UP and RIGHT direction
            // vectors from the camera's transform, and use
            // them to move the center point. Multiply by the
            // radius to make the pan adapt to the current zoom.
            if total_pan != Vec2::ZERO {
                any = true;
                let radius = state.radius;
                state.center += transform.right() * total_pan.x * radius;
                state.center += transform.up() * total_pan.y * radius;
            }

            // Finally, compute the new camera transform.
            // (if we changed anything, or if the pan-orbit
            // controller was just added and thus we are running
            // for the first time and need to initialize)
            if any || state.is_added() {
                // YXZ Euler Rotation performs yaw/pitch/roll.
                transform.rotation = Quat::from_euler(EulerRot::YXZ, state.yaw, state.pitch, 0.0);
                // To position the camera, get the backward direction vector
                // and place the camera at the desired radius from the center.
                transform.translation = state.center + transform.back() * state.radius;
            }
        }
    }
}

#[derive(SystemSet, Hash, Debug, Eq, Clone, PartialEq)]
pub struct PanOrbitCameraUpdate;

impl Plugin for PanOrbitCameraPlugin {
    fn build(&self, app: &mut App) {
        app.world
            .spawn((PanOrbitCameraBundle::default(), PrimaryCamera));

        app.add_systems(
            Update,
            Self::pan_orbit_camera_controller
                .run_if(any_with_component::<PanOrbitState>)
                .in_set(PanOrbitCameraUpdate),
        );
    }
}
