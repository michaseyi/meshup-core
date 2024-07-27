use bevy::ecs::system::SystemId;
pub mod general;

pub struct Tool {
    pub startup_system: SystemId,
    pub update_system: SystemId,
    pub shutdown_system: SystemId,
}
