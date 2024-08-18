use bevy::prelude::*;
use wasm_bindgen::prelude::*;

use crate::core::editor::UserSpace;

use super::transport::EntityEvent;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = __wasm_callback_handles__)]
    fn dispatch_entity_event(event: EntityEvent, entity_index: u32);
}

pub struct EventPlugin;

impl Plugin for EventPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, Self::change_detection);
    }
}

impl EventPlugin {
    fn change_detection(
        changed_transforms: Query<Entity, (With<UserSpace>, Changed<Transform>)>,
        changed_visibility: Query<Entity, (With<UserSpace>, Changed<Visibility>)>,
        changed_children: Query<Entity, (With<UserSpace>, Changed<Children>)>,
        changed_parent: Query<Entity, (With<UserSpace>, Changed<Parent>)>,
    ) {
        for entity in changed_transforms.iter() {
            dispatch_entity_event(EntityEvent::TransformChanged, entity.index());
        }

        for entity in changed_visibility.iter() {
            dispatch_entity_event(EntityEvent::VisibilityChanged, entity.index());
        }

        for entity in changed_children.iter() {
            dispatch_entity_event(EntityEvent::ChildrenChanged, entity.index());
        }
        for entity in changed_parent.iter() {
            dispatch_entity_event(EntityEvent::ParentChanged, entity.index());
        }
    }
}
