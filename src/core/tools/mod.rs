use bevy::{ecs::system::SystemId, prelude::SystemSet};
use wasm_bindgen::prelude::*;
pub mod general;
pub mod brush;

pub struct Tool {
    pub startup_system: SystemId,
    pub update_system: SystemId,
    pub shutdown_system: SystemId,
}

#[derive(SystemSet, Hash, PartialEq, Eq, Clone, Copy, Debug)]
pub enum ToolSet {
    Startup,
    Update,
    Shutdown,
}

#[wasm_bindgen]
#[derive(Hash, Eq, PartialEq, Debug)]
pub enum ToolType {
    Move,
    Rotate,
    Scale,
    Cursor,
}
