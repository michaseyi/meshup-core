use bevy::{prelude::*, window::WindowResolution, winit::WinitSettings};
use std::{borrow::{Borrow, BorrowMut}, cell::UnsafeCell, ptr};

use super::{
    fps::FpsPlugin,
    gizmos::CustomGizmoPlugin,
    pan_orbit_camera_plugin::{PanOrbitCameraPlugin, PanOrbitCameraUpdate},
};

pub struct EditorPlugin {
    pub main_window_canvas_selector: String,
    pub width: f32,
    pub height: f32,
}

#[derive(Component)]
struct SyncWithCamera;

impl Plugin for EditorPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            DefaultPlugins.set(WindowPlugin {
                primary_window: Some(Window {
                    resolution: WindowResolution::new(self.width, self.height),
                    transparent: true,
                    composite_alpha_mode: bevy::window::CompositeAlphaMode::PreMultiplied,
                    canvas: Some(self.main_window_canvas_selector.clone()),
                    ..default()
                }),
                ..default()
            }),
            FpsPlugin,
            PanOrbitCameraPlugin,
            CustomGizmoPlugin,
        ))
        .insert_resource(WinitSettings::desktop_app())
        .insert_resource(ClearColor(Color::rgb(0., 0., 0.).with_a(0.)))
        .add_systems(Startup, populate_scene)
        .add_systems(Update, sync_light_with_camera.after(PanOrbitCameraUpdate));
    }
}

fn populate_scene(
    mut ambient_light: ResMut<AmbientLight>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    ambient_light.brightness = 250.0;

    commands.spawn(PbrBundle {
        mesh: meshes.add(Cuboid::new(2.0, 2.0, 2.0)),
        material: materials.add(StandardMaterial {
            cull_mode: None,
            double_sided: true,
            base_color: Color::rgb_linear(0.35, 0.35, 0.35),
            ..default()
        }),
        ..default()
    });

    commands.spawn((
        DirectionalLightBundle {
            directional_light: DirectionalLight {
                illuminance: light_consts::lux::OVERCAST_DAY * 2.,
                ..default()
            },

            ..default()
        },
        SyncWithCamera,
    ));
}

fn sync_light_with_camera(
    mut query: Query<&mut Transform, (With<SyncWithCamera>, Without<Camera3d>)>,
    camera_query: Query<&Transform, With<Camera3d>>,
) {
    for mut light_transform in query.iter_mut() {
        if let Ok(camera_transform) = camera_query.get_single() {
            light_transform.translation = camera_transform.translation;
            light_transform.rotation = camera_transform.rotation;
            light_transform.scale = camera_transform.scale;
        }
    }
}

