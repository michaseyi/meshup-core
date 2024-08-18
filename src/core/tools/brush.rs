use bevy::{math::Vec3, prelude::Transform};
use lox::FaceHandle;

use crate::core::editable_mesh::EditableMesh;

pub trait Brush {
    fn brush(
        &self,
        mesh: &mut EditableMesh,
        transform: &Transform,
        intersection: Option<(FaceHandle, Vec3)>,
    );
}



