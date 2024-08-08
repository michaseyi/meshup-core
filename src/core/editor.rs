use std::borrow::BorrowMut;

use bevy::{
    ecs::system::SystemId,
    math::bounding::{
        Aabb3d, AabbCast3d, BoundingCircle, BoundingCircleCast, BoundingVolume, IntersectsVolume,
        RayCast3d,
    },
    pbr::wireframe::{Wireframe, WireframePlugin},
    prelude::*,
    render::{primitives::Aabb, view::RenderLayers},
    utils::{HashMap, HashSet},
    window::{CompositeAlphaMode, PrimaryWindow, WindowResolution},
    winit::WinitSettings,
};
use lox::{algo::bounding::BoundingBox, core::Mesh as LoxMesh, Handle as LoxHandle};

use crate::binding::transport::ToolType;

use super::{
    editable_mesh::{
        bvh::{bvh_debug_system, BoundingVolumeHierarchy},
        ActiveEdges, EditableMesh, EditableMeshBundle,
    },
    fps::FpsPlugin,
    gizmos::CustomGizmoPlugin,
    grid::GridPlugin,
    pan_orbit_camera::{PanOrbitCameraPlugin, PanOrbitCameraUpdate, PrimaryCamera},
    tools::{self, ToolSet},
};

pub struct EditorPlugin {
    pub main_window_canvas_selector: String,
    pub width: f32,
    pub height: f32,
}

#[derive(Component)]
pub struct UserSpace;

#[derive(Component)]
struct SyncWithCamera;

#[derive(Clone)]
pub struct Tool {
    pub startup_system: Option<SystemId>,
    pub update_system: Option<SystemId>,
    pub cleanup_system: Option<SystemId>,
}
#[derive(Resource, Default)]
pub struct Tools {
    pub map: HashMap<ToolType, Tool>,
}

#[derive(Resource, Default)]
pub struct ViewportMaterial(pub Handle<StandardMaterial>);

#[derive(Resource, Default)]
pub struct ActiveTool(pub Option<Tool>);

#[derive(Component)]
pub struct Focused;

#[derive(Resource, Default)]
pub struct Cursor3d {
    pub position: Vec3,
    pub orientation: Quat,
}

impl Plugin for EditorPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            DefaultPlugins.set(WindowPlugin {
                primary_window: Some(Window {
                    resolution: WindowResolution::new(self.width, self.height),
                    canvas: Some(self.main_window_canvas_selector.clone()),
                    ..default()
                }),
                ..default()
            }),
            FpsPlugin,
            PanOrbitCameraPlugin,
            CustomGizmoPlugin,
            GridPlugin,
        ))
        .insert_resource(WinitSettings::desktop_app())
        .insert_resource(ActiveTool::default())
        .insert_resource(Tools::default())
        .insert_resource(Cursor3d::default())
        .insert_resource(ClearColor(Color::rgb_u8(63, 63, 63)))
        .add_systems(Startup, Self::populate_scene)
        .add_systems(
            Update,
            (
                Self::handle_interaction,
                // Self::wireframe_focused,
                // bvh_debug_system,
            )
                .after(ToolSet::Update),
        )
        .add_systems(
            Update,
            (
                Self::sync_light_with_camera,
                Self::run_tool.in_set(ToolSet::Update),
            )
                .after(PanOrbitCameraUpdate),
        );
        Self::init_data(&mut app.world);
        Self::register_tools(&mut app.world);
    }
}

impl EditorPlugin {
    fn init_data(world: &mut World) {
        let mut materials = world
            .get_resource_mut::<Assets<StandardMaterial>>()
            .unwrap();

        let viewport_material = materials.add(StandardMaterial {
            cull_mode: None,
            double_sided: true,
            base_color: Color::rgb_linear(0.35, 0.35, 0.35),
            ..default()
        });

        world.insert_resource(ViewportMaterial(viewport_material));
    }

    fn run_tool(active_tool: Res<ActiveTool>, mut commands: Commands) {
        let Some(tool) = &active_tool.0 else {
            return;
        };
        let Some(update_system) = tool.update_system else {
            return;
        };
        commands.run_system(update_system);
    }

    fn register_tools(world: &mut World) {
        // General catergory tools
        // Translation
        let translation_tool_update =
            world.register_system(tools::general::Translation::update_system);
        let translation_tool_cleanup =
            world.register_system(tools::general::Translation::cleanup_system);

        // Scale
        let scale_tool_update = world.register_system(tools::general::Scale::update_system);
        let scale_tool_cleanup = world.register_system(tools::general::Scale::cleanup_system);

        // Rotation
        let rotation_tool_update = world.register_system(tools::general::Rotation::update_system);

        let mut tool_registry = world.get_resource_mut::<Tools>().unwrap();

        tool_registry.map.insert(
            ToolType::Move,
            Tool {
                startup_system: None,
                update_system: Some(translation_tool_update),
                cleanup_system: Some(translation_tool_cleanup),
            },
        );

        tool_registry.map.insert(
            ToolType::Scale,
            Tool {
                startup_system: None,
                update_system: Some(scale_tool_update),
                cleanup_system: Some(scale_tool_cleanup),
            },
        );

        tool_registry.map.insert(
            ToolType::Rotate,
            Tool {
                startup_system: None,
                update_system: Some(rotation_tool_update),
                cleanup_system: None,
            },
        );
    }

    fn populate_scene(
        mut ambient_light: ResMut<AmbientLight>,
        mut commands: Commands,
        mut meshes: ResMut<Assets<Mesh>>,
        viewport_material: Res<ViewportMaterial>,
        mut gizmo_config: ResMut<GizmoConfigStore>,
    ) {
        let (config, _) = gizmo_config.config_mut::<DefaultGizmoConfigGroup>();
        config.depth_bias = -0.001;
        config.line_width = 1.2;

        ambient_light.brightness = 250.0;

        commands.spawn((
            EditableMeshBundle {
                material: viewport_material.0.clone(),
                ..EditableMeshBundle::from_mesh(
                    Sphere::new(1.0)
                        .mesh()
                        .ico(5)
                        .unwrap()
                        .with_duplicated_vertices()
                        .with_computed_flat_normals(),
                    &mut meshes,
                )
            },
            Name::from("Ico Sphere"),
            UserSpace,
            Focused,
        ));

        commands.spawn((
            DirectionalLightBundle {
                directional_light: DirectionalLight {
                    illuminance: light_consts::lux::OVERCAST_DAY * 2.,
                    ..default()
                },

                ..default()
            },
            SyncWithCamera,
            RenderLayers::all(),
        ));
    }

    fn sync_light_with_camera(
        mut query: Query<&mut Transform, (With<SyncWithCamera>, Without<PrimaryCamera>)>,
        camera_query: Query<&Transform, With<PrimaryCamera>>,
    ) {
        let Ok(camera_transform) = camera_query.get_single() else {
            return;
        };

        for mut light_transform in query.iter_mut() {
            light_transform.translation = camera_transform.translation;
            light_transform.rotation = camera_transform.rotation;
            light_transform.scale = camera_transform.scale;
        }
    }

    fn wireframe_focused(
        mut gizmo: Gizmos,
        query: Query<(&Transform, &EditableMesh, &ActiveEdges), With<Focused>>,
    ) {
        let Ok((transform, mesh, active_edges)) = query.get_single() else {
            return;
        };

        let transform = transform.compute_matrix();

        for edge in mesh.structure.edges() {
            let [start, end] = edge
                .endpoints()
                .map(|v| (transform * mesh.vertex_positions[v.handle()].extend(1.0)).truncate());

            gizmo.line(
                start,
                end,
                if active_edges.contains(&edge.handle().idx()) {
                    Color::WHITE
                } else {
                    Color::BLACK
                },
            );
        }
    }
    fn handle_interaction(
        mut commands: Commands,
        focused: Query<
            (
                &BoundingVolumeHierarchy,
                &Transform,
                &Name,
                &EditableMesh,
                Entity,
            ),
            With<Focused>,
        >,
        query: Query<
            (
                &BoundingVolumeHierarchy,
                &Transform,
                &Name,
                &EditableMesh,
                Entity,
            ),
            Without<Focused>,
        >,
        mouse: Res<ButtonInput<MouseButton>>,
        window: Query<&Window, With<PrimaryWindow>>,
        camera: Query<(&Camera, &GlobalTransform), With<PrimaryCamera>>,
    ) {
        if !mouse.just_pressed(MouseButton::Left) {
            return;
        }

        let window = window.single();

        let Some(cursor_position) = window.cursor_position() else {
            return;
        };

        let (camera, global_transform) = camera.single();

        let Some(ray) = camera.viewport_to_world(global_transform, cursor_position) else {
            return;
        };

        let ray_cast = RayCast3d::from_ray(ray, 1000.0);

        let focused_entity = focused.get_single();

        let (mut closest_t, mut closest_entity) = (f32::MAX, None);

        for (bvh, transform, name, mesh, entity) in query.iter().chain(focused.iter()) {
            if let Some((face_handle, t)) = bvh.intersects_ray_at(&ray_cast, transform, mesh) {
                info!(
                    "Intersection hit {:?} {:?}, at face {:?} at point {:?}",
                    name,
                    entity,
                    face_handle,
                    ray_cast.ray.get_point(t)
                );

                if t < closest_t {
                    closest_t = t;
                    closest_entity = Some(entity);
                }
            }
        }

        if let Some(entity) = closest_entity {
            commands.entity(entity).insert(Focused);
        }

        // if let (Ok((_, _, _, entity)), None) = (&focused_entity, &closest_entity) {
        //     commands.entity(entity.clone()).remove::<Focused>();
        // }

        if let (Ok((_, _, _, _, entity)), Some(closest_entity)) = (&focused_entity, &closest_entity)
        {
            if entity != closest_entity {
                commands.entity(entity.clone()).remove::<Focused>();
            }
        }
    }
}
