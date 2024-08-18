use bevy::{
    ecs::query::QuerySingleError, prelude::*, window::RequestRedraw, winit::EventLoopProxy,
};
use events::EventPlugin;
use std::{
    ptr,
    sync::{RwLock, RwLockReadGuard, RwLockWriteGuard},
};
use unsafe_world::UnsafeWorld;
use wasm_bindgen::prelude::*;

use crate::core::{
    editable_mesh::EditableMeshBundle,
    editor::{ActiveTool, Focused, Tools, UserSpace, ViewportMaterial},
    grid::Grid3d,
    highlight::Highlight,
    interaction::InteractionMode,
    tools::ToolType,
};

use super::core::editor::EditorPlugin;
pub mod events;
pub mod transport;

mod unsafe_world;

static WORLD: RwLock<UnsafeWorld> = RwLock::new(UnsafeWorld::empty());

fn set_world(world: &World) {
    let raw_world = ptr::from_ref(world).cast_mut();
    let mut lock = WORLD.write().unwrap();
    *lock = UnsafeWorld(raw_world);
}

fn world<'w>() -> Option<RwLockReadGuard<'w, UnsafeWorld>> {
    if let Ok(lock) = WORLD.read() {
        if lock.0.is_null() {
            return None;
        } else {
            return Some(lock);
        }
    };
    None
}

fn world_mut<'w>() -> Option<RwLockWriteGuard<'w, UnsafeWorld>> {
    if let Ok(lock) = WORLD.write() {
        if lock.0.is_null() {
            return None;
        } else {
            return Some(lock);
        }
    }
    None
}

#[wasm_bindgen]
pub fn trigger_update() {
    let Some(world) = world() else {
        return;
    };
    wakeup_world(&world);
}

#[inline]
pub fn wakeup_world(world: &World) {
    let event_loop_proxy = world.get_non_send_resource::<EventLoopProxy>().unwrap();

    event_loop_proxy.send_event(RequestRedraw).unwrap();
}

#[wasm_bindgen]
pub fn init_app_with_canvas_selector(canvas_selector: String, width: f32, height: f32) {
    App::new()
        .add_systems(PreStartup, set_world)
        .add_plugins((
            EditorPlugin {
                main_window_canvas_selector: canvas_selector,
                width,
                height,
            },
            EventPlugin,
        ))
        .run();
}

#[wasm_bindgen]
pub fn spawn_uvsphere(options: transport::UvSphereOptions) {
    let Some(mut world) = world_mut() else {
        return;
    };

    let mesh = Sphere::new(options.radius)
        .mesh()
        .uv(options.longitudes as usize, options.latitudes as usize)
        .with_duplicated_vertices()
        .with_computed_flat_normals();

    let material = world
        .get_resource_mut::<ViewportMaterial>()
        .unwrap()
        .0
        .clone();

    let mut meshes = world.get_resource_mut::<Assets<Mesh>>().unwrap();

    let editable_mesh = EditableMeshBundle {
        material,
        ..EditableMeshBundle::from_mesh(mesh, &mut meshes)
    };
    world.spawn((editable_mesh, Name::from("Uv Sphere"), UserSpace));

    wakeup_world(&world);
}

#[wasm_bindgen]
pub fn spawn_cube(option: transport::CubeOptions) {
    let Some(mut world) = world_mut() else {
        return;
    };

    let mesh = Cuboid::from_size(Vec3::splat(option.size)).mesh();

    let material = world
        .get_resource_mut::<ViewportMaterial>()
        .unwrap()
        .0
        .clone();

    let mut meshes = world.get_resource_mut::<Assets<Mesh>>().unwrap();

    let editable_mesh = EditableMeshBundle {
        material,
        ..EditableMeshBundle::from_mesh(mesh, &mut meshes)
    };
    world.spawn((editable_mesh, Name::from("Cube"), UserSpace));

    wakeup_world(&world);
}

#[wasm_bindgen]
pub fn spawn_cylinder(option: transport::CylinderOptions) {
    let Some(mut world) = world_mut() else {
        return;
    };

    let mesh = Cylinder::new(option.radius, option.height * 0.5)
        .mesh()
        .resolution(option.segments)
        .build()
        .with_duplicated_vertices()
        .with_computed_flat_normals();

    let material = world
        .get_resource_mut::<ViewportMaterial>()
        .unwrap()
        .0
        .clone();

    let mut meshes = world.get_resource_mut::<Assets<Mesh>>().unwrap();

    let editable_mesh = EditableMeshBundle {
        material,
        ..EditableMeshBundle::from_mesh(mesh, &mut meshes)
    };
    world.spawn((editable_mesh, Name::from("Cylinder"), UserSpace));

    wakeup_world(&world);
}

#[wasm_bindgen]
pub fn get_entity_transform(entity_index: u32) -> transport::Transform {
    let Some(world) = world() else {
        return default();
    };

    let Some(entity) = world.get_entity(Entity::from_raw(entity_index)) else {
        return default();
    };

    let Some(transform) = entity.get::<Transform>() else {
        return default();
    };

    return (*transform).into();
}

#[wasm_bindgen]
pub fn quat_to_euler(quat: transport::Quat, mode: String) -> transport::Vec3 {
    let quat: Quat = quat.into();

    match mode.as_str() {
        "XYZ" => {
            let (x, y, z) = quat.to_euler(EulerRot::XYZ);
            transport::Vec3 { x, y, z }
        }
        "XZY" => {
            let (x, z, y) = quat.to_euler(EulerRot::XZY);
            transport::Vec3 { x, y, z }
        }
        "YXZ" => {
            let (y, x, z) = quat.to_euler(EulerRot::YXZ);
            transport::Vec3 { x, y, z }
        }
        "YZX" => {
            let (y, z, x) = quat.to_euler(EulerRot::YZX);
            transport::Vec3 { x, y, z }
        }
        "ZXY" => {
            let (z, x, y) = quat.to_euler(EulerRot::ZXY);
            transport::Vec3 { x, y, z }
        }
        "ZYX" => {
            let (z, y, x) = quat.to_euler(EulerRot::ZYX);
            transport::Vec3 { x, y, z }
        }
        _ => transport::Vec3::default(),
    }
}

#[wasm_bindgen]
pub fn get_entity_name(entity_index: u32) -> String {
    let Some(world) = world() else {
        return "".to_string();
    };

    let Some(entity) = world.get_entity(Entity::from_raw(entity_index)) else {
        return "".to_string();
    };

    let Some(name) = entity.get::<Name>() else {
        return "".to_string();
    };

    return name.into();
}

#[wasm_bindgen]
pub fn fill_entity_transform(entity_index: u32, transport_transform: &mut transport::Transform) {
    let Some(world) = world() else {
        return;
    };

    let Some(entity) = world.get_entity(Entity::from_raw(entity_index)) else {
        return default();
    };

    let Some(transform) = entity.get::<Transform>() else {
        return default();
    };

    *transport_transform = (*transform).into();
}

#[wasm_bindgen]
pub fn toggle_grid(enable: bool) {
    let Some(mut world) = world_mut() else {
        return;
    };

    let mut query = world.query_filtered::<&mut Visibility, With<Grid3d>>();

    let Ok(mut visibility) = query.get_single_mut(&mut *world) else {
        return;
    };

    if enable {
        *visibility = Visibility::Inherited;
    } else {
        *visibility = Visibility::Hidden;
    }

    wakeup_world(&world);
}

#[wasm_bindgen]
pub fn is_grid_active() -> bool {
    let Some(mut world) = world_mut() else {
        return false;
    };

    let mut query = world.query_filtered::<&Visibility, With<Grid3d>>();

    let Ok(visibility) = query.get_single(&mut *world) else {
        return false;
    };

    if *visibility == Visibility::Hidden {
        return false;
    } else {
        return true;
    }
}

#[wasm_bindgen]
pub fn set_entity_transform(entity_index: u32, transport_transform: &transport::Transform) {
    let Some(mut world) = world_mut() else {
        return;
    };

    let Some(mut entity) = world.get_entity_mut(Entity::from_raw(entity_index)) else {
        return;
    };

    *entity.get_mut::<Transform>().unwrap() = (*transport_transform).into();

    wakeup_world(&world);
}

#[wasm_bindgen]
pub fn set_active_tool(tool_type: ToolType) {
    let Some(mut world) = world_mut() else {
        return;
    };

    let tool = match world.get_resource::<Tools>().unwrap().map.get(&tool_type) {
        Some(tool) => tool.clone(),
        None => return,
    };

    let mut active_tool = world.get_resource_mut::<ActiveTool>().unwrap();

    let mut last_active_tool_cleanup = None;
    let new_active_tool_startup = tool.startup_system;

    if let Some(active_tool) = &active_tool.0 {
        last_active_tool_cleanup = active_tool.cleanup_system.clone();
    }

    active_tool.0 = Some(tool);

    if let Some(last_active_tool_cleanup) = last_active_tool_cleanup {
        world.run_system(last_active_tool_cleanup).unwrap();
    }

    if let Some(new_active_tool_startup) = new_active_tool_startup {
        world.run_system(new_active_tool_startup).unwrap();
    }

    wakeup_world(&world);
}

#[wasm_bindgen]
pub fn unset_active_tool() {
    let Some(mut world) = world_mut() else {
        return;
    };

    let mut active_tool = world.get_resource_mut::<ActiveTool>().unwrap();

    let mut last_active_tool_cleanup = None;

    if let Some(active_tool) = &active_tool.0 {
        if let Some(cleanup_system) = active_tool.cleanup_system {
            last_active_tool_cleanup = Some(cleanup_system);
        }
    }

    active_tool.0 = None;

    if let Some(cleanup_system) = last_active_tool_cleanup {
        world.run_system(cleanup_system).unwrap();
    }

    wakeup_world(&world);
}

#[wasm_bindgen]
pub fn get_root_entities() -> Vec<u32> {
    let Some(mut world) = world_mut() else {
        return vec![];
    };

    let mut root_entities = world.query_filtered::<Entity, (With<UserSpace>, Without<Parent>)>();

    let ids: Vec<u32> = root_entities
        .iter(&mut world)
        .map(|entity| entity.index())
        .collect();
    return ids;
}

#[wasm_bindgen]
pub fn get_entity_children(entity_index: u32) -> Vec<u32> {
    let Some(world) = world() else {
        return vec![];
    };

    let Some(entity) = world.get_entity(Entity::from_raw(entity_index)) else {
        return vec![];
    };
    match entity.get::<Children>() {
        Some(children) => children.iter().map(|entity| entity.index()).collect(),
        None => vec![],
    }
}

#[wasm_bindgen]
pub fn toggle_entity_visibility(entity_index: u32, visible: bool) {
    let Some(mut world) = world_mut() else {
        return;
    };

    let Some(mut entity) = world.get_entity_mut(Entity::from_raw(entity_index)) else {
        return;
    };

    let mut visibility = entity.get_mut::<Visibility>().unwrap();

    if visible {
        *visibility = Visibility::Inherited;
    } else {
        *visibility = Visibility::Hidden;
    }

    wakeup_world(&world);
}

#[wasm_bindgen]
pub fn get_entity_visibility(entity_index: u32) -> bool {
    let Some(world) = world() else {
        return false;
    };

    let Some(entity) = world.get_entity(Entity::from_raw(entity_index)) else {
        return false;
    };

    let visibility = entity.get::<Visibility>().unwrap();

    if *visibility == Visibility::Hidden {
        return false;
    } else {
        return true;
    }
}

#[wasm_bindgen]
pub fn get_active_entity() -> i32 {
    let Some(mut world) = world_mut() else {
        return 0;
    };

    let mut active_entity = world.query_filtered::<Entity, With<Focused>>();

    match active_entity.get_single(&world) {
        Ok(entity) => entity.index() as i32,
        Err(_) => -1,
    }
}

#[wasm_bindgen]
pub fn is_entity_focused(entity_index: u32) -> bool {
    let Some(world) = world() else {
        return false;
    };

    let Some(entity) = world.get_entity(Entity::from_raw(entity_index)) else {
        return false;
    };

    entity.get::<Focused>().is_some()
}

#[wasm_bindgen]
pub fn focus_entity(entity_index: u32) {
    let Some(mut world) = world_mut() else {
        return;
    };

    let mut query = world.query_filtered::<Entity, With<Focused>>();

    match query.get_single(&world) {
        Ok(entity) => {
            let Some(mut entity_ref) = world.get_entity_mut(entity) else {
                return;
            };

            entity_ref.remove::<Focused>();
        }
        _ => {}
    }

    let Some(mut entity) = world.get_entity_mut(Entity::from_raw(entity_index)) else {
        return;
    };

    entity.insert(Focused);

    wakeup_world(&world);
}

#[wasm_bindgen]
pub fn unfocus() {
    let Some(mut world) = world_mut() else {
        return;
    };

    let mut query = world.query_filtered::<Entity, With<Focused>>();

    match query.get_single(&world) {
        Ok(entity) => {
            let Some(mut entity_ref) = world.get_entity_mut(entity) else {
                return;
            };

            entity_ref.remove::<Focused>();
        }
        _ => {}
    }

    wakeup_world(&world);
}

#[wasm_bindgen]
pub fn set_interaction_mode(mode: InteractionMode) {
    let Some(mut world) = world_mut() else {
        return;
    };

    // For interaction modes asides InteractionMode::Object, there must be a focused enitity for the mode to be valid.
    let mut focused_entity = world.query_filtered::<(), With<Focused>>();

    match &mode {
        InteractionMode::Object => {}
        _ => {
            if let Err(_) = focused_entity.get_single(&world) {
                return;
            }
            // focused_entity.get_single(&world)
            // .expect("Setting interaction mode to any other than InteractionMode::Object requires a focused entity, as interaction operations will be done relative to that entity");
        }
    };

    let mut interaction_mode = world.get_resource_mut::<InteractionMode>().unwrap();

    *interaction_mode = mode;

    wakeup_world(&world);
}

#[wasm_bindgen]
pub fn get_interaction_mode() -> InteractionMode {
    let world = world().unwrap();

    let interaction_mode = world.get_resource::<InteractionMode>().unwrap();

    interaction_mode.clone()
}

#[wasm_bindgen]
pub fn highlight_entity(entity_index: u32) {
    let Some(mut world) = world_mut() else {
        return;
    };

    let Some(mut entity) = world.get_entity_mut(Entity::from_raw(entity_index)) else {
        return;
    };

    entity.insert(Highlight);

    wakeup_world(&world);
}

#[wasm_bindgen]
pub fn unhighlight_entity(entity_index: u32) {
    let Some(mut world) = world_mut() else {
        return;
    };

    let Some(mut entity) = world.get_entity_mut(Entity::from_raw(entity_index)) else {
        return;
    };

    entity.remove::<Highlight>();

    wakeup_world(&world);
}

#[wasm_bindgen]
pub fn is_highlighted(entity_index: u32) -> bool {
    let Some(world) = world() else {
        return false;
    };

    let Some(entity) = world.get_entity(Entity::from_raw(entity_index)) else {
        return false;
    };

    entity.get::<Highlight>().is_some()
}
