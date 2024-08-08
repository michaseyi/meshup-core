pub mod bvh;

pub mod algo;

use bevy::{
    asset::Handle,
    math::Vec3,
    prelude::*,
    render::mesh::{Indices, PrimitiveTopology, VertexAttributeValues},
    utils::HashSet,
};
use bvh::BoundingVolumeHierarchy;
use lox::{
    core::{half_edge::PolyConfig, HalfEdgeMesh, MeshMut},
    leer::Empty,
    map::{DenseMap, PropStoreMut},
    FaceHandle, Handle as LoxHandle, VertexHandle,
};

#[derive(Component, Deref, DerefMut, Default)]
pub struct ActiveVertices(pub HashSet<u32>);

#[derive(Component, Deref, DerefMut, Default)]
pub struct ActiveEdges(pub HashSet<u32>);

#[derive(Component, DerefMut, Deref, Default)]
pub struct ActiveFaces(pub HashSet<u32>);

#[derive(Bundle, Default)]
pub struct EditableMeshBundle {
    pub mesh: Handle<Mesh>,
    pub editable_mesh: EditableMesh,
    pub material: Handle<StandardMaterial>,
    pub active_edges: ActiveEdges,
    pub active_vertices: ActiveVertices,
    pub active_faces: ActiveFaces,
    pub visibility: Visibility,
    pub inherited_visibility: InheritedVisibility,
    pub view_visibility: ViewVisibility,
    pub transform: Transform,
    pub global_transform: GlobalTransform,
    pub bvh: BoundingVolumeHierarchy,
}

#[derive(Component)]
pub struct EditableMesh {
    pub structure: HalfEdgeMesh<PolyConfig>,
    pub vertex_positions: DenseMap<VertexHandle, Vec3>,
    pub vertex_normals: DenseMap<VertexHandle, Vec3>,
    pub face_normals: DenseMap<FaceHandle, Vec3>,
}

impl Default for EditableMesh {
    fn default() -> Self {
        Self {
            structure: HalfEdgeMesh::empty(),
            vertex_positions: DenseMap::new(),
            vertex_normals: DenseMap::new(),
            face_normals: DenseMap::new(),
        }
    }
}

impl EditableMeshBundle {
    pub fn from_mesh(raw_mesh: Mesh, meshes: &mut Assets<Mesh>) -> Self {
        let editable_mesh = EditableMesh::from(&raw_mesh);
        let bvh = BoundingVolumeHierarchy::from(&editable_mesh);
        let mesh = meshes.add(raw_mesh);
        Self {
            editable_mesh,
            bvh,
            mesh,
            ..default()
        }
    }
}

impl From<&Mesh> for EditableMesh {
    fn from(mesh: &Mesh) -> Self {
        let mut editable_mesh = EditableMesh::default();

        if mesh.primitive_topology() != PrimitiveTopology::TriangleList {
            unimplemented!("Only triangle list topologies are supported");
        }

        let position_attribute = match mesh
            .attribute(Mesh::ATTRIBUTE_POSITION)
            .expect("EditableMesh requires the position attribute")
        {
            VertexAttributeValues::Float32x3(v) => v,
            _ => unimplemented!("Non-f32x3 positions are not supported yet"),
        };

        let normal_attribute = match mesh
            .attribute(Mesh::ATTRIBUTE_NORMAL)
            .expect("EditableMesh requires the normal attribute")
        {
            VertexAttributeValues::Float32x3(v) => v,
            _ => unimplemented!("Non-f32x3 normal are not supported yet"),
        };

        let Some(indices) = mesh.indices() else {
            position_attribute
                .chunks(3)
                .into_iter()
                .zip(normal_attribute.chunks(3).into_iter())
                .for_each(|(positions, normals)| {
                    let mut handles = [VertexHandle::new(0); 3];

                    for handle in &mut handles {
                        *handle = editable_mesh.structure.add_vertex();
                    }

                    handles
                        .clone()
                        .into_iter()
                        .zip(positions.into_iter().zip(normals.into_iter()))
                        .for_each(|(vertex, (position, normal))| {
                            editable_mesh
                                .vertex_positions
                                .insert(vertex, Vec3::from_array(position.clone()));

                            editable_mesh
                                .vertex_normals
                                .insert(vertex, Vec3::from_array(normal.clone()));
                        });

                    editable_mesh.structure.add_face(&handles);
                });

            return editable_mesh;
        };

        let vertices: Vec<VertexHandle> = position_attribute
            .into_iter()
            .zip(normal_attribute.into_iter())
            .map(|(position, normal)| {
                let vertex = editable_mesh.structure.add_vertex();
                editable_mesh
                    .vertex_positions
                    .insert(vertex, Vec3::from_array(position.clone()));

                editable_mesh
                    .vertex_normals
                    .insert(vertex, Vec3::from_array(normal.clone()));

                vertex
            })
            .collect();

        match indices {
            Indices::U16(indices) => {
                for chunk in indices.chunks(3) {
                    editable_mesh.structure.add_face(&[
                        vertices[chunk[0] as usize],
                        vertices[chunk[1] as usize],
                        vertices[chunk[2] as usize],
                    ]);
                }
            }
            Indices::U32(indices) => {
                for chunk in indices.chunks(3) {
                    editable_mesh.structure.add_face(&[
                        vertices[chunk[0] as usize],
                        vertices[chunk[1] as usize],
                        vertices[chunk[2] as usize],
                    ]);
                }
            }
        }

        editable_mesh
    }
}

#[cfg(test)]
mod test {
    use bevy::prelude::*;
    use lox::core::Mesh as LoxMesh;

    use super::EditableMesh;

    #[test]
    fn test_editable_mesh_from_cuboid_mesh() {
        let mesh: Mesh = Cuboid::from_size(Vec3::splat(1.0)).mesh();

        let editable_mesh: EditableMesh = (&mesh).into();

        assert_eq!(
            mesh.count_vertices(),
            editable_mesh.structure.num_vertices() as usize
        );
    }
}
