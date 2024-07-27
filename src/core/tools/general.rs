use core::f32;
use std::f32::consts;

use bevy::{
    math::bounding::{Aabb3d, IntersectsVolume, RayCast3d},
    prelude::*,
    window::{CursorGrabMode, PrimaryWindow},
};

use crate::{
    core::{
        editor_plugin::Focused,
        gizmos::{
            CustomGizmo, GizmoColors, GizmoPlaneDistance, GizmoScaleToViewportRatio, RotationGizmo,
            ScaleGizmo, TranslationGizmo,
        },
        pan_orbit_camera_plugin::{PanOrbitSettings, PanOrbitState, PrimaryCamera},
    },
    utils,
};
pub struct Translation;

#[derive(Debug, Copy, Clone)]
enum TranslateAction {
    X,
    Y,
    Z,
    XY,
    XZ,
    YZ,
    XYZ,
}
#[derive(Default, Copy, Clone)]
pub struct TranslateToolState {
    active_action: Option<TranslateAction>,
    prev_cursor_position: Option<Vec2>,
    start_position: Option<Vec3>,
}

impl Translation {
    pub fn cleanup_system(mut translation_gizmo: Query<&mut Visibility, With<TranslationGizmo>>) {
        for mut visibility in translation_gizmo.iter_mut() {
            *visibility = Visibility::Hidden;
        }
    }

    pub fn update_system(
        mut state: Local<TranslateToolState>,
        mut focused_entity: Query<&mut Transform, With<Focused>>,
        mut translate_gizmo: Query<
            (&mut Visibility, &mut Transform),
            (With<TranslationGizmo>, Without<Focused>),
        >,
        q_main_camera: Query<
            (&Camera, &Transform),
            (
                With<PrimaryCamera>,
                Without<TranslationGizmo>,
                Without<Focused>,
            ),
        >,
        gizmo_plane_distance: Res<GizmoPlaneDistance>,
        pixel_scale: Res<GizmoScaleToViewportRatio>,
        mouse: Res<ButtonInput<MouseButton>>,
        window: Query<&Window, With<PrimaryWindow>>,
        mut gizmo: Gizmos<CustomGizmo>,
        colors: Res<GizmoColors>,
    ) {
        let (mut gizmo_visiblity, mut gizmo_transform) = translate_gizmo.single_mut();
        let (camera, camera_transform) = q_main_camera.single();

        let Some(mut entity_transform) = focused_entity.iter_mut().nth(0) else {
            *gizmo_visiblity = Visibility::Hidden;
            return;
        };
        *gizmo_visiblity = Visibility::Visible;

        let gizmo_origin = utils::projection::project_to_plane(
            camera_transform.translation,
            camera_transform.forward().into(),
            entity_transform.translation,
            gizmo_plane_distance.0,
        );

        if !mouse.pressed(MouseButton::Left) {
            gizmo_transform.translation = gizmo_origin.clone();
            state.active_action = None;
            state.prev_cursor_position = None;
            return;
        }

        let window = window.single();

        let Some(cursor_position) = window.cursor_position() else {
            gizmo_transform.translation = gizmo_origin.clone();
            return;
        };

        let mut curr_action: Option<TranslateAction> = None;

        if let Some(prev_action) = &state.active_action {
            curr_action = Some(prev_action.clone());
        } else {
            let ray = match camera.viewport_to_world(
                &(GlobalTransform::IDENTITY.mul_transform(*camera_transform)),
                cursor_position,
            ) {
                Some(ray) => RayCast3d::from_ray(ray, 100.),
                None => {
                    gizmo_transform.translation = gizmo_origin.clone();
                    return;
                }
            };

            let half_width = 15. * pixel_scale.0 * 0.5;
            let half_height = 90. * pixel_scale.0 * 0.5;
            let plane_center = 25. * pixel_scale.0;
            let half_plane_thickness = 0.05;
            let half_plane_size = 15. * pixel_scale.0 * 0.5;

            let x_aabb = Aabb3d::new(
                Vec3::new(half_height, 0.0, 0.) + gizmo_origin,
                Vec3::new(half_height, half_width, half_width),
            );
            let y_aabb = Aabb3d::new(
                Vec3::new(0.0, half_height, 0.) + gizmo_origin,
                Vec3::new(half_width, half_height, half_width),
            );
            let z_aabb = Aabb3d::new(
                Vec3::new(0.0, 0.0, half_height) + gizmo_origin,
                Vec3::new(half_width, half_width, half_height),
            );

            let xz_aabb = Aabb3d::new(
                Vec3::new(plane_center, 0., plane_center) + gizmo_origin,
                Vec3::new(half_plane_size, half_plane_thickness, half_plane_size),
            );

            let xy_aabb = Aabb3d::new(
                Vec3::new(plane_center, plane_center, 0.) + gizmo_origin,
                Vec3::new(half_plane_size, half_plane_size, half_plane_thickness),
            );

            let yz_aabb = Aabb3d::new(
                Vec3::new(0., plane_center, plane_center) + gizmo_origin,
                Vec3::new(half_plane_thickness, half_plane_size, half_plane_size),
            );

            let mut closest_t_action = None;
            let mut closest_t = f32::MAX;

            for (aabb, action) in [
                (x_aabb, TranslateAction::X),
                (y_aabb, TranslateAction::Y),
                (z_aabb, TranslateAction::Z),
                (xy_aabb, TranslateAction::XY),
                (xz_aabb, TranslateAction::XZ),
                (yz_aabb, TranslateAction::YZ),
            ] {
                let Some(t) = ray.aabb_intersection_at(&aabb) else {
                    continue;
                };

                if t < closest_t {
                    closest_t = t;
                    closest_t_action = Some(action);
                }
            }

            let Some(action) = closest_t_action else {
                gizmo_transform.translation = gizmo_origin.clone();
                return;
            };

            state.active_action = Some(action);
            curr_action = Some(action);
            state.start_position = Some(gizmo_origin);
        }

        if state.prev_cursor_position.is_none() {
            state.prev_cursor_position = Some(cursor_position);
            return;
        }

        let prev_cursor_position = state.prev_cursor_position.unwrap();

        let mut delta = cursor_position - prev_cursor_position;
        delta.y = -delta.y;

        let movement: Vec3 = camera_transform.right() * delta.x + camera_transform.up() * delta.y;

        let moves = Vec3::new(
            movement.dot(Vec3::X),
            movement.dot(Vec3::Y),
            movement.dot(Vec3::Z),
        ) * 0.03;

        state.prev_cursor_position = Some(cursor_position);

        let action = curr_action.unwrap();
        let translation = match action {
            TranslateAction::X => Vec3::new(moves.x, 0.0, 0.0),
            TranslateAction::Y => Vec3::new(0.0, moves.y, 0.0),
            TranslateAction::Z => Vec3::new(0.0, 0.0, moves.z),
            TranslateAction::XY => Vec3::new(moves.x, moves.y, 0.0),
            TranslateAction::XZ => {
                let change = Vec3::new(moves.x, 0.0, moves.z);

                let origin = state.start_position.unwrap();

                gizmo.line(
                    origin + Direction3d::X * -50.,
                    origin + Direction3d::X * 50.,
                    colors.red,
                );
                gizmo.line(
                    origin + Direction3d::Z * -50.,
                    origin + Direction3d::Z * 50.,
                    colors.blue,
                );

                change
            }
            TranslateAction::YZ => {
                let change = Vec3::new(0.0, moves.y, moves.z);

                let origin = state.start_position.unwrap();

                gizmo.line(
                    origin + Direction3d::Y * -50.,
                    origin + Direction3d::Y * 50.,
                    colors.green,
                );
                gizmo.line(
                    origin + Direction3d::Z * -50.,
                    origin + Direction3d::Z * 50.,
                    colors.blue,
                );

                change
            }
            TranslateAction::XYZ => moves,
        };
        entity_transform.translation += translation;

        gizmo_transform.translation = utils::projection::project_to_plane(
            camera_transform.translation,
            camera_transform.forward().into(),
            entity_transform.translation,
            gizmo_plane_distance.0,
        );
    }
}

pub struct Scale;

#[derive(Default, Copy, Clone)]
pub struct ScaleToolState {
    active_action: Option<ScaleAction>,
    prev_cursor_position: Option<Vec2>,
}

#[derive(Debug, Copy, Clone)]
enum ScaleAction {
    X,
    Y,
    Z,
    XY,
    XZ,
    YZ,
    XYZ,
}

impl Scale {
    pub fn cleanup_system(mut translation_gizmo: Query<&mut Visibility, With<ScaleGizmo>>) {
        for mut visibility in translation_gizmo.iter_mut() {
            *visibility = Visibility::Hidden;
        }
    }

    pub fn update_system(
        mut state: Local<ScaleToolState>,
        mut focused_entity: Query<&mut Transform, With<Focused>>,
        mut scale_gizmo: Query<
            (&mut Visibility, &mut Transform),
            (With<ScaleGizmo>, Without<Focused>),
        >,
        q_main_camera: Query<
            (&Camera, &Transform),
            (With<PrimaryCamera>, Without<ScaleGizmo>, Without<Focused>),
        >,
        gizmo_plane_distance: Res<GizmoPlaneDistance>,
        pixel_scale: Res<GizmoScaleToViewportRatio>,
        mouse: Res<ButtonInput<MouseButton>>,
        window: Query<&Window, With<PrimaryWindow>>,
        mut custom_gizmo: Gizmos<CustomGizmo>,
    ) {
        let (mut gizmo_visiblity, mut gizmo_transform) = scale_gizmo.single_mut();
        let (camera, camera_transform) = q_main_camera.single();

        let Some(mut entity_transform) = focused_entity.iter_mut().nth(0) else {
            *gizmo_visiblity = Visibility::Hidden;
            return;
        };
        *gizmo_visiblity = Visibility::Visible;

        // Since scaling does not change the position of the entity, we can this value can be cached
        let gizmo_origin = utils::projection::project_to_plane(
            camera_transform.translation,
            camera_transform.forward().into(),
            entity_transform.translation,
            gizmo_plane_distance.0,
        );

        custom_gizmo.circle(
            gizmo_origin,
            camera_transform.forward(),
            90. * pixel_scale.0,
            Color::WHITE,
        );

        custom_gizmo.circle(
            gizmo_origin,
            camera_transform.forward(),
            20. * pixel_scale.0,
            Color::WHITE,
        );

        if !mouse.pressed(MouseButton::Left) {
            gizmo_transform.translation = gizmo_origin.clone();
            state.active_action = None;
            state.prev_cursor_position = None;
            return;
        }

        let Some(cursor_position) = window.single().cursor_position() else {
            gizmo_transform.translation = gizmo_origin.clone();
            return;
        };

        let mut curr_action: Option<ScaleAction> = None;

        if let Some(prev_action) = &state.active_action {
            curr_action = Some(prev_action.clone());
        } else {
            let ray = match camera.viewport_to_world(
                &(GlobalTransform::IDENTITY.mul_transform(*camera_transform)),
                cursor_position,
            ) {
                Some(ray) => RayCast3d::from_ray(ray, 100.),
                None => {
                    gizmo_transform.translation = gizmo_origin.clone();
                    return;
                }
            };

            let half_width = 15. * pixel_scale.0 * 0.5;
            let half_height = 85. * pixel_scale.0 * 0.5;
            let plane_center = 25. * pixel_scale.0;
            let half_plane_thickness = 0.05;
            let half_plane_size = 15. * pixel_scale.0 * 0.5;

            let x_scale_aabb = Aabb3d::new(
                Vec3::new(half_height, 0.0, 0.) + gizmo_origin,
                Vec3::new(half_height, half_width, half_width),
            );
            let y_scale_aabb = Aabb3d::new(
                Vec3::new(0.0, half_height, 0.) + gizmo_origin,
                Vec3::new(half_width, half_height, half_width),
            );
            let z_scale_aabb = Aabb3d::new(
                Vec3::new(0.0, 0.0, half_height) + gizmo_origin,
                Vec3::new(half_width, half_width, half_height),
            );

            let xz_scale_aabb = Aabb3d::new(
                Vec3::new(plane_center, 0., plane_center) + gizmo_origin,
                Vec3::new(half_plane_size, half_plane_thickness, half_plane_size),
            );

            let xy_scale_aabb = Aabb3d::new(
                Vec3::new(plane_center, plane_center, 0.) + gizmo_origin,
                Vec3::new(half_plane_size, half_plane_size, half_plane_thickness),
            );

            let yz_scale_aabb = Aabb3d::new(
                Vec3::new(0., plane_center, plane_center) + gizmo_origin,
                Vec3::new(half_plane_thickness, half_plane_size, half_plane_size),
            );

            let mut closest_t_action = None;
            let mut closest_t = f32::MAX;

            for (aabb, action) in [
                (x_scale_aabb, ScaleAction::X),
                (y_scale_aabb, ScaleAction::Y),
                (z_scale_aabb, ScaleAction::Z),
                (xy_scale_aabb, ScaleAction::XY),
                (xz_scale_aabb, ScaleAction::XZ),
                (yz_scale_aabb, ScaleAction::YZ),
            ] {
                let Some(t) = ray.aabb_intersection_at(&aabb) else {
                    continue;
                };

                if t < closest_t {
                    closest_t = t;
                    closest_t_action = Some(action);
                }
            }

            let Some(action) = closest_t_action else {
                gizmo_transform.translation = gizmo_origin.clone();
                return;
            };

            state.active_action = Some(action);
            curr_action = Some(action);
        }

        if state.prev_cursor_position.is_none() {
            state.prev_cursor_position = Some(cursor_position);
            return;
        }

        let prev_cursor_position = state.prev_cursor_position.unwrap();

        let mut delta = cursor_position - prev_cursor_position;
        delta.y = -delta.y;

        let movement: Vec3 = camera_transform.right() * delta.x + camera_transform.up() * delta.y;

        let scales = Vec3::new(
            movement.dot(Vec3::X),
            movement.dot(Vec3::Y),
            movement.dot(Vec3::Z),
        ) * 0.02;

        state.prev_cursor_position = Some(cursor_position);

        let action = curr_action.unwrap();
        let scale = match action {
            ScaleAction::X => Vec3::new(scales.x, 0.0, 0.0),
            ScaleAction::Y => Vec3::new(0.0, scales.y, 0.0),
            ScaleAction::Z => Vec3::new(0.0, 0.0, scales.z),
            ScaleAction::XY => {
                let scale = scales.x + scales.y;
                Vec3::new(scale, scale, 0.0)
            }
            ScaleAction::XZ => {
                let scale = scales.x + scales.z;
                Vec3::new(scale, 0.0, scale)
            }
            ScaleAction::YZ => {
                let scale = scales.y + scales.z;
                Vec3::new(0.0, scale, scale)
            }
            ScaleAction::XYZ => {
                let scale = scales.x + scales.y + scales.z;
                Vec3::splat(scale)
            }
        };
        entity_transform.scale += scale;

        gizmo_transform.translation = gizmo_origin.clone();
    }
}

pub struct Rotation;

impl Rotation {
    pub fn update_system(
        pixel_scale: Res<GizmoScaleToViewportRatio>,
        mut rotation_gizmzo: Gizmos<RotationGizmo>,
        mut custom_gizmo: Gizmos<CustomGizmo>,
        q_main_camera: Query<(&Camera, &Transform), (With<PrimaryCamera>, Without<Focused>)>,
        gizmo_plane_distance: Res<GizmoPlaneDistance>,
        window: Query<&Window, With<PrimaryWindow>>,
        mut focused_entity: Query<&mut Transform, With<Focused>>,
        colors: Res<GizmoColors>,
    ) {
        let Some(mut entity_transform) = focused_entity.iter_mut().nth(0) else {
            return;
        };

        let (camera, camera_transform) = q_main_camera.single();

        let (pitch, yaw, roll) =
            utils::projection::compute_orientation_angles(-Vec3::from(camera_transform.forward()));

        let origin = utils::projection::project_to_plane(
            camera_transform.translation,
            camera_transform.forward().into(),
            entity_transform.translation,
            gizmo_plane_distance.0,
        );

        rotation_gizmzo.arc_3d(
            f32::consts::PI,
            80. * pixel_scale.0,
            origin,
            Quat::from_rotation_y(f32::consts::PI).mul_quat(Quat::from_rotation_y(yaw)),
            colors.green,
        );
        rotation_gizmzo.arc_3d(
            f32::consts::PI,
            80. * pixel_scale.0,
            origin,
            Quat::from_rotation_y(f32::consts::PI)
                .mul_quat(Quat::from_rotation_z(f32::consts::FRAC_PI_2))
                .mul_quat(Quat::from_rotation_z(roll)),
            colors.red,
        );
        rotation_gizmzo.arc_3d(
            f32::consts::PI,
            80. * pixel_scale.0,
            origin,
            Quat::from_rotation_y(f32::consts::PI)
                .mul_quat(Quat::from_rotation_x(-f32::consts::FRAC_PI_2))
                .mul_quat(Quat::from_rotation_x(pitch)),
            colors.blue,
        );

        custom_gizmo.circle(
            origin,
            camera_transform.forward(),
            90. * pixel_scale.0,
            Color::WHITE,
        );

    }
}
