use bevy::{ecs::system::SystemId, prelude::SystemSet};
pub mod general;

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
