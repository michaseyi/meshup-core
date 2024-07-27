use bevy::prelude::*;
use events::EventPlugin;
use std::{
    ptr,
    sync::{RwLock, RwLockReadGuard, RwLockWriteGuard},
};
use unsafe_world::UnsafeWorld;
use wasm_bindgen::prelude::*;

use crate::core::editor_plugin::{ActiveTool, External, Tools};

use super::core::editor_plugin::EditorPlugin;
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
    if let Some(mut world) = world_mut() {
        let mesh = world.get_resource_mut::<Assets<Mesh>>().unwrap().add(
            Sphere::new(options.radius)
                .mesh()
                .uv(options.longitudes as usize, options.latitudes as usize),
        );

        let material = world
            .get_resource_mut::<Assets<StandardMaterial>>()
            .unwrap()
            .add(Color::rgb(0.8, 0.8, 0.8));

        world.spawn((
            PbrBundle {
                mesh,
                material,
                transform: Transform::from_xyz(0.0, 5.0, 0.0),
                ..default()
            },
            Name::from("UVSphere"),
        ));
    }
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
pub fn set_entity_transform(entity_index: u32, transport_transform: &transport::Transform) {
    let Some(mut world) = world_mut() else {
        return;
    };

    let Some(mut entity) = world.get_entity_mut(Entity::from_raw(entity_index)) else {
        return;
    };

    *entity.get_mut::<Transform>().unwrap() = (*transport_transform).into();
}

#[wasm_bindgen]
pub fn set_active_tool(tool_type: transport::ToolType) {
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
}

#[wasm_bindgen]
pub fn get_root_entities() -> Vec<u32> {
    let Some(mut world) = world_mut() else {
        return vec![];
    };

    let mut root_entities = world.query_filtered::<Entity, (With<External>, Without<Parent>)>();

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
