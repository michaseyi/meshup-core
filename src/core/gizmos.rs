use std::f32::consts;

use bevy::{
    prelude::*,
    render::view::RenderLayers,
    window::{PrimaryWindow, WindowResized},
};

use super::{
    dim3::Cone,
    pan_orbit_camera::{PanOrbitCameraUpdate, PrimaryCamera},
};

#[derive(Component)]
pub struct ScaleGizmo;

#[derive(Component)]
pub struct TranslationGizmo;

#[derive(Component)]
pub struct GizmoRoot;
#[derive(Reflect, Default, GizmoConfigGroup)]
pub struct CustomGizmo;

#[derive(Resource, Default)]
pub struct GizmoScaleToViewportRatio(pub f32);

#[derive(Resource)]
pub struct GizmoPlaneDistance(pub f32);

impl Default for GizmoPlaneDistance {
    fn default() -> Self {
        Self(100.0)
    }
}

#[derive(Resource, Default)]
pub struct GizmoDataHandles {
    cube_mesh: Handle<Mesh>,
    cylinder_mesh: Handle<Mesh>,
    plane_mesh: Handle<Mesh>,
    cone_mesh: Handle<Mesh>,
    red_material: Handle<StandardMaterial>,
    green_material: Handle<StandardMaterial>,
    blue_material: Handle<StandardMaterial>,
}

#[derive(Component)]
pub struct GizmoCamera;
#[derive(SystemSet, Hash, PartialEq, Eq, Clone, Debug)]
pub struct GizmoConfigInitSet;

pub struct CustomGizmoPlugin;

#[derive(GizmoConfigGroup, Default, Reflect)]
pub struct RotationGizmo;

#[derive(Resource)]
pub struct GizmoColors {
    pub red: Color,
    pub blue: Color,
    pub green: Color,
    pub dark_red: Color,
    pub dark_green: Color,
    pub dark_blue: Color,
}

impl Default for GizmoColors {
    fn default() -> Self {
        Self {
            red: Color::rgb_u8(255, 107, 107),
            green: Color::rgb_u8(120, 255, 120),
            blue: Color::rgb_u8(107, 170, 255),
            dark_red: Color::rgb_u8(255, 30, 30),
            dark_green: Color::rgb_u8(60, 255, 60),
            dark_blue: Color::rgb_u8(60, 60, 255),
        }
    }
}

impl Plugin for CustomGizmoPlugin {
    fn build(&self, app: &mut App) {
        app.world.spawn((
            Camera3dBundle {
                camera: Camera {
                    order: 1,
                    ..default()
                },
                ..default()
            },
            GizmoCamera,
            RenderLayers::layer(1),
        ));
        app.init_gizmo_group::<CustomGizmo>()
            .init_gizmo_group::<RotationGizmo>()
            .insert_resource(GizmoDataHandles::default())
            .insert_resource(GizmoPlaneDistance::default())
            .insert_resource(GizmoScaleToViewportRatio::default())
            .insert_resource(GizmoColors::default())
            .add_systems(
                Startup,
                (
                    Self::setup_gizmo_config.in_set(GizmoConfigInitSet),
                    Self::setup_scale_gizmo.after(GizmoConfigInitSet),
                    Self::setup_translation_gizmo.after(GizmoConfigInitSet),
                ),
            )
            .add_systems(
                Update,
                (
                    Self::sync_gizmo_cam_with_main_cam.after(PanOrbitCameraUpdate),
                    Self::handle_window_resize_event,
                ),
            );
    }
}

impl CustomGizmoPlugin {
    fn sync_gizmo_cam_with_main_cam(
        q_main_camera: Query<&Transform, With<PrimaryCamera>>,
        mut q_gizmo_camera: Query<&mut Transform, (With<GizmoCamera>, Without<PrimaryCamera>)>,
    ) {
        let main_camera_transform = q_main_camera.single();
        let mut gizmo_camera_transform = q_gizmo_camera.single_mut();

        gizmo_camera_transform.translation = main_camera_transform.translation;
        gizmo_camera_transform.scale = main_camera_transform.scale;
        gizmo_camera_transform.rotation = main_camera_transform.rotation;
    }

    fn setup_gizmo_config(
        mut gizmo_config: ResMut<GizmoConfigStore>,
        gizmo_plane_distance: Res<GizmoPlaneDistance>,
        mut gizmo_scale_to_viewport: ResMut<GizmoScaleToViewportRatio>,
        q_window: Query<&Window, With<PrimaryWindow>>,
        mut meshes: ResMut<Assets<Mesh>>,
        mut materials: ResMut<Assets<StandardMaterial>>,
        mut gizmo_data_handles: ResMut<GizmoDataHandles>,
        gizmo_colors: Res<GizmoColors>,
    ) {
        let (config, _) = gizmo_config.config_mut::<CustomGizmo>();
        config.render_layers = RenderLayers::layer(1);
        config.line_width = 1.5;

        let (config, _) = gizmo_config.config_mut::<RotationGizmo>();
        config.render_layers = RenderLayers::layer(1);
        config.line_width = 3.;

        let window = q_window.single();
        let height_gizmo = gizmo_plane_distance.0 * f32::tan(consts::FRAC_PI_4 / 2.0) * 2.0;
        let height_viewport = window.height();
        gizmo_scale_to_viewport.0 = height_gizmo / height_viewport;

        let (cube_mesh, cylinder_mesh, plane_mesh, cone_mesh) =
            Self::create_gizmo_objects(gizmo_scale_to_viewport.0);

        gizmo_data_handles.cube_mesh = meshes.add(cube_mesh);
        gizmo_data_handles.cylinder_mesh = meshes.add(cylinder_mesh);
        gizmo_data_handles.plane_mesh = meshes.add(plane_mesh);
        gizmo_data_handles.cone_mesh = meshes.add(cone_mesh);

        gizmo_data_handles.red_material = materials.add(StandardMaterial {
            base_color: gizmo_colors.red,
            double_sided: true,
            unlit: true,
            cull_mode: None,
            ..default()
        });

        gizmo_data_handles.green_material = materials.add(StandardMaterial {
            base_color: gizmo_colors.green,
            double_sided: true,
            unlit: true,
            cull_mode: None,
            ..default()
        });

        gizmo_data_handles.blue_material = materials.add(StandardMaterial {
            base_color: gizmo_colors.blue,
            cull_mode: None,
            unlit: true,
            double_sided: true,
            ..default()
        });
    }

    fn handle_window_resize_event(
        mut evr_window_resized: EventReader<WindowResized>,
        mut gizmo_scale_to_viewport: ResMut<GizmoScaleToViewportRatio>,
        gizmo_data_handles: Res<GizmoDataHandles>,
        mut meshes: ResMut<Assets<Mesh>>,
        gizmo_plane_distance: Res<GizmoPlaneDistance>,
    ) {
        let Some(window_resized) = evr_window_resized.read().last() else {
            return;
        };
        let height_gizmo = gizmo_plane_distance.0 * f32::tan(consts::FRAC_PI_4 / 2.0) * 2.0;
        let height_viewport = window_resized.height;
        let new_gizmo_viewport_scale = height_gizmo / height_viewport;

        if (new_gizmo_viewport_scale - gizmo_scale_to_viewport.0).abs() > 0.00001 {
            gizmo_scale_to_viewport.0 = new_gizmo_viewport_scale;

            let (cube_mesh, cylinder_mesh, plane_mesh, cone_mesh) =
                Self::create_gizmo_objects(gizmo_scale_to_viewport.0);

            *meshes.get_mut(&gizmo_data_handles.cube_mesh).unwrap() = cube_mesh;
            *meshes.get_mut(&gizmo_data_handles.cylinder_mesh).unwrap() = cylinder_mesh;
            *meshes.get_mut(&gizmo_data_handles.plane_mesh).unwrap() = plane_mesh;
            *meshes.get_mut(&gizmo_data_handles.cone_mesh).unwrap() = cone_mesh;
        }
    }

    /// returns gizmo meshes in order (cube, cylinder, plane, cone)
    fn create_gizmo_objects(pixel_scale: f32) -> (Mesh, Mesh, Mesh, Mesh) {
        let cube_mesh = Mesh::from(Cuboid::from_size(Vec3::splat(10. * pixel_scale)))
            .translated_by(Vec3::new(80. * pixel_scale, 0.0, 0.0));

        let cylinder_mesh = Cylinder::new(1. * pixel_scale, 55. * pixel_scale)
            .mesh()
            .resolution(3)
            .build()
            .rotated_by(Quat::from_rotation_z(std::f32::consts::FRAC_PI_2))
            .translated_by(Vec3::new((30. + 20.) * pixel_scale, 0.0, 0.0));

        let plane_mesh = Plane3d {
            normal: Direction3d::Y,
        }
        .mesh()
        .size(15. * pixel_scale, 15. * pixel_scale)
        .build()
        .translated_by(Vec3::new(
            (10. + 15.) * pixel_scale,
            0.0,
            (10. + 15.) * pixel_scale,
        ));
        let cone_mesh = Cone::new(5. * pixel_scale, 15. * pixel_scale)
            .mesh()
            .resolution(16)
            .build()
            .rotated_by(Quat::from_rotation_z(-std::f32::consts::FRAC_PI_2))
            .translated_by(Vec3::new(75. * pixel_scale, 0.0, 0.0));

        (cube_mesh, cylinder_mesh, plane_mesh, cone_mesh)
    }

    fn setup_scale_gizmo(mut commands: Commands, gizmo_data_handles: Res<GizmoDataHandles>) {
        let scale_gizmo_root = commands
            .spawn((
                SpatialBundle {
                    visibility: Visibility::Hidden,
                    ..default()
                },
                ScaleGizmo,
                GizmoRoot,
            ))
            .id();

        for (orientation, material) in [
            (Quat::default(), &gizmo_data_handles.red_material),
            (
                Quat::from_rotation_z(std::f32::consts::FRAC_PI_2),
                &gizmo_data_handles.green_material,
            ),
            (
                Quat::from_rotation_y(-std::f32::consts::FRAC_PI_2),
                &gizmo_data_handles.blue_material,
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
                .spawn((
                    PbrBundle {
                        material: material.clone(),
                        mesh: gizmo_data_handles.cube_mesh.clone(),
                        ..default()
                    },
                    RenderLayers::layer(1),
                ))
                .set_parent(axis);

            commands
                .spawn((
                    PbrBundle {
                        material: material.clone(),
                        mesh: gizmo_data_handles.cylinder_mesh.clone(),
                        ..default()
                    },
                    RenderLayers::layer(1),
                ))
                .set_parent(axis);
        }

        for (orientation, material) in [
            (
                Quat::from_rotation_z(std::f32::consts::FRAC_PI_2),
                &gizmo_data_handles.red_material,
            ),
            (Quat::default(), &gizmo_data_handles.green_material),
            (
                Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2),
                &gizmo_data_handles.blue_material,
            ),
        ] {
            commands
                .spawn((
                    PbrBundle {
                        material: material.clone(),
                        transform: Transform::from_rotation(orientation.clone()),
                        mesh: gizmo_data_handles.plane_mesh.clone(),
                        ..default()
                    },
                    RenderLayers::layer(1),
                ))
                .set_parent(scale_gizmo_root);
        }
    }

    fn setup_translation_gizmo(mut commands: Commands, gizmo_data_handles: Res<GizmoDataHandles>) {
        let scale_gizmo_root = commands
            .spawn((
                SpatialBundle {
                    visibility: Visibility::Hidden,
                    ..default()
                },
                TranslationGizmo,
                GizmoRoot,
            ))
            .id();

        for (orientation, material) in [
            (Quat::default(), &gizmo_data_handles.red_material),
            (
                Quat::from_rotation_z(std::f32::consts::FRAC_PI_2),
                &gizmo_data_handles.green_material,
            ),
            (
                Quat::from_rotation_y(-std::f32::consts::FRAC_PI_2),
                &gizmo_data_handles.blue_material,
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
                .spawn((
                    PbrBundle {
                        material: material.clone(),
                        mesh: gizmo_data_handles.cone_mesh.clone(),
                        ..default()
                    },
                    RenderLayers::layer(1),
                ))
                .set_parent(axis);

            commands
                .spawn((
                    PbrBundle {
                        material: material.clone(),
                        mesh: gizmo_data_handles.cylinder_mesh.clone(),
                        ..default()
                    },
                    RenderLayers::layer(1),
                ))
                .set_parent(axis);
        }

        for (orientation, material) in [
            (
                Quat::from_rotation_z(std::f32::consts::FRAC_PI_2),
                &gizmo_data_handles.red_material,
            ),
            (Quat::default(), &gizmo_data_handles.green_material),
            (
                Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2),
                &gizmo_data_handles.blue_material,
            ),
        ] {
            commands
                .spawn((
                    PbrBundle {
                        material: material.clone(),
                        transform: Transform::from_rotation(orientation.clone()),
                        mesh: gizmo_data_handles.plane_mesh.clone(),
                        ..default()
                    },
                    RenderLayers::layer(1),
                ))
                .set_parent(scale_gizmo_root);
        }
    }
}
