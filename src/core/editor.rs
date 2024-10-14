use bevy::{
    ecs::system::SystemId, prelude::*, render::view::RenderLayers, utils::HashMap,
    window::WindowResolution, winit::WinitSettings,
};

use bevy_obj::ObjPlugin;
use lox::{core::Mesh as LoxMesh, Handle as LoxHandle};

use crate::utils;

use super::{
    editable_mesh::{
        bvh::bvh_debug_system, ActiveEdges, EditableMesh, EditableMeshBundle, EditableMeshPlugin,
    },
    fps::FpsPlugin,
    gizmos::{CustomGizmoPlugin, GizmoPlaneDistance, GizmoScaleToViewportRatio},
    grid::GridPlugin,
    highlight::HighlightPlugin,
    interaction::{InteractionPlugin, InteractionSet},
    pan_orbit_camera::{PanOrbitCameraPlugin, PanOrbitCameraUpdate, PrimaryCamera},
    tools::{self, ToolSet, ToolType},
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
            EditableMeshPlugin,
            InteractionPlugin,
            // HighlightPlugin::<StandardMaterial>::default(),
            ObjPlugin,
        ))
        .insert_resource(WinitSettings::desktop_app())
        .insert_resource(ActiveTool::default())
        .insert_resource(Tools::default())
        .insert_resource(Cursor3d::default())
        .insert_resource(ClearColor(Color::rgb_u8(63, 63, 63)))
        .add_systems(Startup, Self::init_default_scene)
        .add_systems(
            Update,
            (
                // Self::wireframe_focused,
                 bvh_debug_system).after(ToolSet::Update),
        )
        .add_systems(
            Update,
            (Self::sync_light_with_camera, Self::draw_cursor_3d).after(PanOrbitCameraUpdate),
        )
        .add_systems(
            Update,
            Self::run_active_tool
                .in_set(ToolSet::Update)
                .after(InteractionSet::ActivesUpdate),
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

    fn run_active_tool(active_tool: Res<ActiveTool>, mut commands: Commands) {
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
    fn draw_cursor_3d(
        cursor: Res<Cursor3d>,
        mut gizmo: Gizmos,
        camera: Query<&Transform, With<PrimaryCamera>>,
        pixel_scale: Res<GizmoScaleToViewportRatio>,
        plane_distance: Res<GizmoPlaneDistance>,
    ) {
        let camera = camera.single();

        let forward: Vec3 = camera.forward().into();
        let cursor_position = utils::projection::project_to_plane(
            camera.translation,
            forward,
            cursor.position,
            plane_distance.0,
        );

        let ring_radius = 8.0 * pixel_scale.0;
        let axis_height = 15.0 * pixel_scale.0;
        let offset = 5.0 * pixel_scale.0;

        gizmo.circle(
            cursor_position,
            forward.try_into().unwrap(),
            ring_radius,
            Color::WHITE,
        );

        for axis in [Direction3d::X, Direction3d::Y, Direction3d::Z] {
            gizmo.line(
                cursor_position + axis * offset,
                cursor_position + axis * (axis_height + offset),
                Color::BLACK,
            );
            gizmo.line(
                cursor_position - axis * offset,
                cursor_position - axis * (axis_height + offset),
                Color::BLACK,
            );
        }
    }
    fn init_default_scene(
        mut ambient_light: ResMut<AmbientLight>,
        mut commands: Commands,
        mut meshes: ResMut<Assets<Mesh>>,
        viewport_material: Res<ViewportMaterial>,
        mut gizmo_config: ResMut<GizmoConfigStore>,
    ) {
        let (config, _) = gizmo_config.config_mut::<DefaultGizmoConfigGroup>();
        // config.depth_bias = -0.001;
        config.line_width = 1.5;
        // config.depth_bias = 0.2;
        // config.line_width = 3.0;

        ambient_light.brightness = 250.0;

        commands.spawn((
            EditableMeshBundle {
                material: viewport_material.0.clone(),
                ..EditableMeshBundle::from_mesh(
                    bevy_obj::load_obj_from_bytes(include_bytes!("../assets/mesh/cat.obj"))
                        .unwrap()
                        .transformed_by(Transform::from_scale(Vec3::splat(0.1)))
                        .with_duplicated_vertices()
                        .with_computed_flat_normals(),
                    &mut meshes,
                )
            },
            Name::from("Cat"),
            UserSpace,
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
                    Color::ORANGE_RED
                },
            );
        }
    }
}
