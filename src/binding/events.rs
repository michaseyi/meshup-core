use bevy::prelude::*;
use wasm_bindgen::prelude::*;

use crate::core::editor_plugin::External;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = __wasm_callback_handles__)]
    fn entity_transform_update(entity_index: u32);
}

pub struct EventPlugin;

impl Plugin for EventPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, Self::transform_change_detection);
    }
}

impl EventPlugin {
    fn transform_change_detection(query: Query<Entity, (With<External>, Changed<Transform>)>) {
        for c in query.iter() {
            entity_transform_update(c.index());
        }
    }
}
