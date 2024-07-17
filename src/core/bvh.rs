use bevy::{
    math::{
        bounding::{Aabb3d, RayCast3d},
        Ray3d,
    },
    reflect::Reflect,
};
use lox::{
    core::{half_edge::PolyConfig, HalfEdgeMesh},
    FaceHandle,
};

pub struct Bvh {
    root: Option<Node>,
}

impl Default for Bvh {
    fn default() -> Self {
        Self::new()
    }
}

struct Bucket {
    primitive_count: u32,
    aabb: Aabb3d,
}

enum Node {
    NonLeaf {
        left: Option<Box<Node>>,
        right: Option<Box<Node>>,
        aabb: Aabb3d,
    },
    Leaf {
        aabb: Aabb3d,
        primitive_list: Vec<FaceHandle>,
    },
}

impl Bvh {
    pub fn new() -> Self {
        Self { root: None }
    }

    pub fn intersect(&self, ray: &RayCast3d) {}
}

impl From<HalfEdgeMesh> for Bvh {
    fn from(mesh: HalfEdgeMesh) -> Self {
        let minimum_primitive_per_leaf = 16u32;
        let bucket_count = 16u32;

        let a = lox::mesh! {
            type: HalfEdgeMesh<PolyConfig>,
            vertices: [
                v0: [0.0, 0.0, 0.0],
                v1: [0.0, 1.0, 0.0],
                v2: [1.0, 0.0, 0.0],
                v3: [1.0, 1.0, 0.5],
            ],
            faces: [
                [v0, v2, v1],
                [v3, v1, v2],
            ],
        };

        return Bvh::new();
    }
}
