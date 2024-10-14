use bevy::{math::Vec3, prelude::Transform};
use lox::FaceHandle;

use crate::core::editable_mesh::EditableMesh;

pub struct BrushContext<'a> {
    pub intersection: Option<(FaceHandle, Vec3)>,
    pub radius: f32,
    pub strength: f32,
    pub mesh: &'a mut EditableMesh,
    pub transform: &'a Transform,
}

pub trait Brush {
    fn brush(&self, context: BrushContext);
}
