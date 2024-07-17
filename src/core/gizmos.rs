use std::f32::consts;

use bevy::{
    prelude::*,
    window::{PrimaryWindow, WindowResized},
};

use super::pan_orbit_camera_plugin::PanOrbitCameraUpdate;

#[derive(Component)]
pub struct ScaleGizmo;

#[derive(Component)]
pub struct TranslationGizmo;

#[derive(Reflect, Default, GizmoConfigGroup)]
struct CursorGizmo {
    position: Vec3,
    orientation: Quat,
}

#[derive(Resource, Default)]
pub struct GizmoScaleToViewportRatio(pub f32);

#[derive(Resource)]
pub struct GizmoPlaneDistance(pub f32);

impl Default for GizmoPlaneDistance {
    fn default() -> Self {
        Self(5.0)
    }
}

#[derive(Resource, Default)]
struct GizmoDataHandles {
    cube: Handle<Mesh>,
    cylinder: Handle<Mesh>,
}

#[derive(SystemSet, Hash, PartialEq, Eq, Clone, Debug)]
struct GizmoConfigInitSet;

pub struct CustomGizmoPlugin;

impl Plugin for CustomGizmoPlugin {
    fn build(&self, app: &mut App) {
        app.init_gizmo_group::<CursorGizmo>()
            .insert_resource(GizmoDataHandles::default())
            .insert_resource(GizmoPlaneDistance::default())
            .insert_resource(GizmoScaleToViewportRatio::default())
            .add_systems(
                Startup,
                (
                    Self::setup_gizmo_config.in_set(GizmoConfigInitSet),
                    Self::setup_scale_gizmo.after(GizmoConfigInitSet),
                ),
            )
            .add_systems(
                Update,
                (
                    Self::sync_gizmo_with_camera.after(PanOrbitCameraUpdate),
                    Self::handle_window_resize_event,
                ),
            );
    }
}

impl CustomGizmoPlugin {
    fn sync_gizmo_with_camera(
        mut gizmos: Query<&mut Transform, (With<ScaleGizmo>, Without<Camera3d>)>,
        q_camera: Query<&Transform, With<Camera3d>>,
        gizmo_plane_distance: Res<GizmoPlaneDistance>,
    ) {
        let camera_transform = q_camera.single();
        for mut transform in gizmos.iter_mut() {
            let b = (camera_transform.translation - transform.translation).length();
            transform.translation = Self::project_to_plane(
                camera_transform.translation,
                camera_transform.forward().into(),
                Vec3::ZERO,
                gizmo_plane_distance.0,
            );

            let a = (camera_transform.translation - transform.translation).length();
            info!("{b} {a}");
        }
    }

    fn setup_gizmo_config(
        mut gizmo_config: ResMut<GizmoConfigStore>,
        gizmo_plane_distance: Res<GizmoPlaneDistance>,
        mut gizmo_scale_to_viewport: ResMut<GizmoScaleToViewportRatio>,
        q_window: Query<&Window, With<PrimaryWindow>>,
    ) {
        let (config, _) = gizmo_config.config_mut::<CursorGizmo>();
        config.depth_bias = -1.;

        let window = q_window.single();
        let height_gizmo = gizmo_plane_distance.0 * f32::tan(consts::FRAC_PI_4 / 2.0) * 2.0;
        let height_viewport = window.height();
        gizmo_scale_to_viewport.0 = height_gizmo / height_viewport;
    }

    fn handle_window_resize_event(
        mut evr_window_resized: EventReader<WindowResized>,
        mut gizmo_scale_to_viewport: ResMut<GizmoScaleToViewportRatio>,
        gizmo_data_handles: ResMut<GizmoDataHandles>,
        mut meshes: ResMut<Assets<Mesh>>,
        gizmo_plane_distance: Res<GizmoPlaneDistance>,
    ) {
        let Some(window_resized) = evr_window_resized.read().last() else {
            return;
        };
        let height_gizmo = gizmo_plane_distance.0 * f32::tan(consts::FRAC_PI_4 / 2.0) * 2.0;
        let height_viewport = window_resized.height;
        let new_gizmo_viewport_scale = height_gizmo / height_viewport;

        if (new_gizmo_viewport_scale - gizmo_scale_to_viewport.0).abs() > 0.0001 {
            gizmo_scale_to_viewport.0 = new_gizmo_viewport_scale;

            let new_cube_mesh = Mesh::from(Cuboid::from_size(Vec3::splat(
                10. * gizmo_scale_to_viewport.0,
            )))
            .translated_by(Vec3::new(80. * gizmo_scale_to_viewport.0, 0.0, 0.0));

            let new_cylinder_mesh = Mesh::from(Cylinder::new(
                1. * gizmo_scale_to_viewport.0,
                55. * gizmo_scale_to_viewport.0,
            ))
            .rotated_by(Quat::from_rotation_z(std::f32::consts::FRAC_PI_2))
            .translated_by(Vec3::new(
                (30. + 20.) * gizmo_scale_to_viewport.0,
                0.0,
                0.0,
            ));
            let cube = meshes.get_mut(&gizmo_data_handles.cube).unwrap();
            *cube = new_cube_mesh;

            let cylinder = meshes.get_mut(&gizmo_data_handles.cylinder).unwrap();
            *cylinder = new_cylinder_mesh;
        }
    }

    fn project_to_plane(
        camera_position: Vec3,
        camera_forward: Vec3,
        object_position: Vec3,
        plane_distance: f32,
    ) -> Vec3 {
        let object_to_camera_vector = camera_position - object_position;
        let object_to_camera_direction = object_to_camera_vector.normalize();

        let camera_alignment = camera_forward.dot(-object_to_camera_vector) - plane_distance;
        let projection_distance =
            object_to_camera_direction.dot(-camera_forward * camera_alignment);
        object_position + object_to_camera_direction * projection_distance
    }

    fn setup_scale_gizmo(
        mut commands: Commands,
        mut meshes: ResMut<Assets<Mesh>>,
        mut gizmo_data_handles: ResMut<GizmoDataHandles>,
        mut materials: ResMut<Assets<StandardMaterial>>,
        gizmo_scale_to_viewport: Res<GizmoScaleToViewportRatio>,
    ) {
        let cube_mesh = meshes.add(
            Mesh::from(Cuboid::from_size(Vec3::splat(
                10. * gizmo_scale_to_viewport.0,
            )))
            .translated_by(Vec3::new(80. * gizmo_scale_to_viewport.0, 0.0, 0.0)),
        );

        let cylinder_mesh = meshes.add(
            Cylinder::new(
                1. * gizmo_scale_to_viewport.0,
                55. * gizmo_scale_to_viewport.0,
            )
            .mesh()
            .resolution(6)
            .build()
            .rotated_by(Quat::from_rotation_z(std::f32::consts::FRAC_PI_2))
            .translated_by(Vec3::new(
                (30. + 20.) * gizmo_scale_to_viewport.0,
                0.0,
                0.0,
            )),
        );

        gizmo_data_handles.cube = cube_mesh.clone();
        gizmo_data_handles.cylinder = cylinder_mesh.clone();

        let depth_bias = f32::MAX;
        let unlit = true;
        let cull_mode = None;

        let red_material = materials.add(StandardMaterial {
            base_color: Color::rgb_u8(255, 107, 107),
            unlit,
            depth_bias,
            cull_mode,
            ..default()
        });

        let green_material = materials.add(StandardMaterial {
            base_color: Color::rgb_u8(107, 255, 107),
            unlit,
            depth_bias,
            cull_mode,
            ..default()
        });

        let blue_material = materials.add(StandardMaterial {
            base_color: Color::rgb_u8(107, 170, 255),
            unlit,
            cull_mode,
            depth_bias,
            ..default()
        });

        let scale_gizmo_root = commands.spawn((SpatialBundle::default(), ScaleGizmo)).id();

        for (orientation, material) in [
            (Quat::default(), &red_material),
            (
                Quat::from_rotation_z(std::f32::consts::FRAC_PI_2),
                &green_material,
            ),
            (
                Quat::from_rotation_y(-std::f32::consts::FRAC_PI_2),
                &blue_material,
            ),
        ] {
            let axis = commands
                .spawn(SpatialBundle {
                    transform: Transform::from_rotation(orientation.clone()),
                    ..default()
                })
                .set_parent(scale_gizmo_root)
                .id();
            commands
                .spawn(PbrBundle {
                    material: material.clone(),
                    mesh: cube_mesh.clone(),
                    ..default()
                })
                .set_parent(axis);

            commands
                .spawn(PbrBundle {
                    material: material.clone(),
                    mesh: cylinder_mesh.clone(),
                    ..default()
                })
                .set_parent(axis);
        }

        let plane_mesh = meshes.add(
            Plane3d {
                normal: Direction3d::Y,
            }
            .mesh()
            .size(
                20. * gizmo_scale_to_viewport.0,
                20. * gizmo_scale_to_viewport.0,
            )
            .build()
            .translated_by(Vec3::new(
                (10. + 15.) * gizmo_scale_to_viewport.0,
                0.0,
                (10. + 15.) * gizmo_scale_to_viewport.0,
            )),
        );

        for (orientation, material) in [
            (
                Quat::from_rotation_z(std::f32::consts::FRAC_PI_2),
                &red_material,
            ),
            (Quat::default(), &green_material),
            (
                Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2),
                &blue_material,
            ),
        ] {
            commands
                .spawn(PbrBundle {
                    material: material.clone(),
                    transform: Transform::from_rotation(orientation.clone()),
                    mesh: plane_mesh.clone(),
                    ..default()
                })
                .set_parent(scale_gizmo_root);
        }

        let translation_gizmo_root = commands
            .spawn((SpatialBundle::default(), TranslationGizmo))
            .id();
    }
}
