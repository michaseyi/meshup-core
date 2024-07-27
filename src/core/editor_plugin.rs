use bevy::{
    ecs::system::SystemId, prelude::*, render::view::RenderLayers, utils::HashMap,
    window::WindowResolution, winit::WinitSettings,
};

use crate::binding::transport::ToolType;

use super::{
    fps::FpsPlugin,
    gizmos::CustomGizmoPlugin,
    pan_orbit_camera_plugin::{PanOrbitCameraPlugin, PanOrbitCameraUpdate, PrimaryCamera},
    tools,
};

pub struct EditorPlugin {
    pub main_window_canvas_selector: String,
    pub width: f32,
    pub height: f32,
}

#[derive(Component)]
pub struct External;

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
pub struct ActiveTool(pub Option<Tool>);

#[derive(Component)]
pub struct Focused;

impl Plugin for EditorPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            DefaultPlugins.set(WindowPlugin {
                primary_window: Some(Window {
                    resolution: WindowResolution::new(self.width, self.height),
                    transparent: true,
                    // composite_alpha_mode: bevy::window::CompositeAlphaMode::PreMultiplied,
                    canvas: Some(self.main_window_canvas_selector.clone()),
                    ..default()
                }),
                ..default()
            }),
            FpsPlugin,
            PanOrbitCameraPlugin,
            CustomGizmoPlugin,
        ))
        .insert_resource(WinitSettings::game())
        .insert_resource(ActiveTool::default())
        .insert_resource(Tools::default())
        .insert_resource(ClearColor(Color::NONE))
        .add_systems(Startup, Self::populate_scene)
        .add_systems(
            Update,
            (Self::sync_light_with_camera, Self::run_tool).after(PanOrbitCameraUpdate),
        );

        Self::register_tools(&mut app.world);
    }
}

impl EditorPlugin {
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
        mut materials: ResMut<Assets<StandardMaterial>>,
    ) {
        ambient_light.brightness = 250.0;

        commands.spawn((
            PbrBundle {
                mesh: meshes.add(Cuboid::new(2.0, 2.0, 2.0)),
                material: materials.add(StandardMaterial {
                    cull_mode: None,
                    double_sided: true,
                    base_color: Color::rgb_linear(0.35, 0.35, 0.35),
                    ..default()
                }),
                ..default()
            },
            Focused,
            External,
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
}
