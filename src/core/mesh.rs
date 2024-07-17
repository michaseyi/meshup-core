use bevy::{
    asset::{Asset, Handle},
    math::Vec3,
    pbr::PbrBundle,
    prelude::*,
};
use lox::{
    core::{half_edge::PolyConfig, HalfEdgeMesh},
    leer::Empty,
    map::DenseMap,
    VertexHandle,
};

#[derive(Default, Bundle)]
pub struct MeshBundle {
    pub pbr_bundle: PbrBundle,
    pub mesh: Handle<HalfEdgeMeshAsset>,
}

#[derive(Asset, Reflect)]
pub struct HalfEdgeMeshAsset {
    // #[reflect(ignore)]
    // pub mesh: HalfEdgeMesh<PolyConfig>,
}

impl Default for PolyMesh {
    fn default() -> Self {
        PolyMesh {
            poly_mesh: HalfEdgeMesh::empty(),
            vertex_positions: DenseMap::empty(),
            vertex_normals: DenseMap::empty(),
            face_normals: DenseMap::empty(),
        }
    }
}

#[derive(Asset, Reflect)]
pub struct PolyMesh {
    #[reflect(ignore)]
    #[reflect(default = "HalfEdgeMesh::empty")]
    poly_mesh: HalfEdgeMesh<PolyConfig>,
    #[reflect(ignore)]
    #[reflect(default = "DenseMap::empty")]
    vertex_positions: DenseMap<VertexHandle, Vec3>,
    #[reflect(ignore)]
    #[reflect(default = "DenseMap::empty")]
    vertex_normals: DenseMap<VertexHandle, Vec3>,
    #[reflect(ignore)]
    #[reflect(default = "DenseMap::empty")]
    face_normals: DenseMap<VertexHandle, Vec3>,
}
