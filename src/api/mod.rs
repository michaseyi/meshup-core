use bevy::{prelude::*};
use std::{
    ptr,
    sync::{RwLock, RwLockReadGuard, RwLockWriteGuard},
};
use unsafe_world::UnsafeWorld;
use wasm_bindgen::prelude::*;

use super::core::editor_plugin::EditorPlugin;
mod transport;
mod unsafe_world;

static WORLD: RwLock<UnsafeWorld> = RwLock::new(UnsafeWorld::empty());

#[wasm_bindgen]
pub struct Api;

impl Api {
    pub fn set_world(world: &World) {
        let raw_world = ptr::from_ref(world).cast_mut();
        let mut lock = WORLD.write().unwrap();
        *lock = UnsafeWorld(raw_world);
    }

    pub fn world<'w>() -> Option<RwLockReadGuard<'w, UnsafeWorld>> {
        if let Ok(lock) = WORLD.read() {
            if lock.0.is_null() {
                return None;
            } else {
                return Some(lock);
            }
        };
        None
    }

    pub fn world_mut<'w>() -> Option<RwLockWriteGuard<'w, UnsafeWorld>> {
        if let Ok(lock) = WORLD.write() {
            if lock.0.is_null() {
                return None;
            } else {
                return Some(lock);
            }
        }
        None
    }
}

#[wasm_bindgen]
impl Api {
    #[wasm_bindgen(static_method_of=Api)]
    pub fn init_app_with_canvas_selector(canvas_selector: String, width: f32, height: f32) {
        App::new()
            .add_systems(Startup, Api::set_world)
            .add_plugins(EditorPlugin {
                main_window_canvas_selector: canvas_selector,
                width,
                height,
            })
            .run();
    }

    #[wasm_bindgen(static_method_of=Api)]
    pub fn spawn_uvsphere(options: transport::UvSphereOptions) {
        if let Some(mut world) = Api::world_mut() {
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

    #[wasm_bindgen(static_method_of=Api)]
    pub fn get_entity_transform(entity_index: u32) -> transport::Transform {
        let Some(world) = Api::world_mut() else {
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

    #[wasm_bindgen(static_method_of=Api)]
    pub fn destroy(transform: &transport::Transform) {
        info!("{transform:?}");
    }
}
