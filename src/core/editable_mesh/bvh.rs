use core::panic;
use std::collections::VecDeque;

use bevy::{
    log::{info, warn},
    math::{
        bounding::{Aabb3d, BoundingVolume, IntersectsVolume, RayCast3d},
        Mat4, Quat, Vec3,
    },
    prelude::{Color, Component, Gizmos, Query, Transform, With},
    render::primitives::Aabb,
};
use lox::{
    core::{Mesh as LoxMesh, Orientable},
    map::{DenseMap, PropStore, PropStoreMut},
    FaceHandle, Handle,
};

use crate::core::editor::Focused;

use crate::core::dim3::ray_intersects_convex_plane_at;

use super::EditableMesh;

#[derive(Component)]
pub struct BoundingVolumeHierarchy {
    pub nodes: Vec<Node>,
}

impl Default for BoundingVolumeHierarchy {
    fn default() -> Self {
        Self { nodes: Vec::new() }
    }
}

#[derive(Clone, Copy, Default, Debug)]
struct Bucket {
    primitive_count: u32,
    aabb: Option<Aabb3d>,
}

pub enum Node {
    NonLeaf {
        left: u32,
        right: u32,
        aabb: Aabb3d,
    },
    Leaf {
        aabb: Aabb3d,
        primitive_list: Vec<FaceHandle>,
    },
}

impl Node {
    pub fn intersects_ray(&self, ray: &RayCast3d, transform: &Transform) -> Option<f32> {
        let aabb = match self {
            Node::Leaf { aabb, .. } => aabb,
            Node::NonLeaf { aabb, .. } => aabb,
        };

        let aabb = Aabb3d::new(
            aabb.center() * transform.scale + transform.translation,
            aabb.half_size() * transform.scale,
        );

        ray.aabb_intersection_at(&aabb)
    }

    pub fn is_leaf(&self) -> bool {
        match self {
            Node::Leaf { .. } => true,
            Node::NonLeaf { .. } => false,
        }
    }

    pub fn primitives(&self) -> Option<&Vec<FaceHandle>> {
        match self {
            Node::Leaf { primitive_list, .. } => Some(primitive_list),
            Node::NonLeaf { .. } => None,
        }
    }

    pub fn children(&self) -> Option<[u32; 2]> {
        match self {
            Node::Leaf { .. } => None,
            Node::NonLeaf { left, right, .. } => Some([left.clone(), right.clone()]),
        }
    }

    pub fn aabb(&self) -> Aabb3d {
        match self {
            Node::Leaf { aabb, .. } => aabb.clone(),
            Node::NonLeaf { aabb, .. } => aabb.clone(),
        }
    }
}

pub fn bvh_debug_system(
    query: Query<(&BoundingVolumeHierarchy, &Transform), With<Focused>>,
    mut gizmo: Gizmos,
) {
    for (bvh, transform) in query.iter() {
        for node in bvh.nodes.iter() {
            if !node.is_leaf() {
                continue;
            }
            let aabb: Aabb3d = node.aabb();

            let transform = transform.compute_matrix()
                * Mat4::from_scale_rotation_translation(
                    aabb.half_size() * 2.0,
                    Quat::IDENTITY,
                    aabb.center(),
                );
            gizmo.cuboid(transform, Color::BLACK);
        }
    }
}

impl BoundingVolumeHierarchy {
    const BUCKET_COUNT: usize = 16;

    pub fn new() -> Self {
        Self::default()
    }

    /// Does fast ray intersection test. Does not consider the primitives in the leaf node.
    pub fn intersects_ray_at_fast(&self, ray: &RayCast3d, transform: &Transform) -> Option<f32> {
        let transformed_ray = RayCast3d::new(
            transform.rotation.mul_vec3(ray.ray.origin.into()),
            transform
                .rotation
                .mul_vec3(ray.ray.direction.into())
                .try_into()
                .unwrap(),
            ray.max,
        );

        let mut stack = VecDeque::<u32>::new();

        stack.push_back(0);

        let mut closest_t = None;

        while !stack.is_empty() {
            let top = stack.pop_back().unwrap();
            let node = &self.nodes[top as usize];

            match node.intersects_ray(&transformed_ray, transform) {
                Some(t) => {
                    if let Some(value) = closest_t {
                        if t > value {
                            continue;
                        }
                    }
                    if node.is_leaf() {
                        closest_t = Some(closest_t.unwrap_or(f32::MAX).min(t));
                    } else {
                        let [left, right] = node.children().unwrap();

                        let left_node = &self.nodes[left as usize];
                        let right_node = &self.nodes[right as usize];

                        let left_t = left_node.intersects_ray(&transformed_ray, transform);
                        let right_t = right_node.intersects_ray(&transformed_ray, transform);

                        match (left_t, right_t) {
                            (Some(left_t), Some(right_t)) => {
                                if left_t < right_t {
                                    stack.push_back(right);
                                    stack.push_back(left);
                                } else {
                                    stack.push_back(left);
                                    stack.push_back(right);
                                }
                            }
                            _ => {
                                stack.push_back(left);
                                stack.push_back(right);
                            }
                        }
                    }
                }
                None => {}
            }
        }

        closest_t
    }

    pub fn intersects_ray_at(
        &self,
        ray: &RayCast3d,
        transform: &Transform,
        mesh: &EditableMesh,
    ) -> Option<(FaceHandle, f32)> {
        let transformed_ray = RayCast3d::new(
            transform.rotation.mul_vec3(ray.ray.origin.into()),
            transform
                .rotation
                .mul_vec3(ray.ray.direction.into())
                .try_into()
                .unwrap(),
            ray.max,
        );

        let mut stack = VecDeque::<u32>::new();

        stack.push_back(0);

        let mut closest = Option::<(FaceHandle, f32)>::None;

        while !stack.is_empty() {
            let top = stack.pop_back().unwrap();
            let node = &self.nodes[top as usize];

            match node.intersects_ray(&transformed_ray, transform) {
                Some(t) => {
                    if let Some((_, value)) = closest {
                        if t > value {
                            continue;
                        }
                    }
                    if node.is_leaf() {
                        for face_handle in node.primitives().unwrap() {
                            let face = mesh.structure.get_ref(face_handle.clone());
                            let vertices: Vec<Vec3> = face
                                .adjacent_vertices()
                                .map(|v| mesh.vertex_positions[v.handle()])
                                .collect();

                            match ray_intersects_convex_plane_at(
                                &transformed_ray,
                                &transform,
                                &vertices,
                            ) {
                                Some(t) => {
                                    if let Some((_, value)) = closest {
                                        if t < value {
                                            closest = Some((face_handle.clone(), t));
                                        }
                                    } else {
                                        closest = Some((face_handle.clone(), t));
                                    }
                                }
                                _ => {}
                            }
                        }
                    } else {
                        let [left, right] = node.children().unwrap();

                        let left_node = &self.nodes[left as usize];
                        let right_node = &self.nodes[right as usize];

                        let left_t = left_node.intersects_ray(&transformed_ray, transform);
                        let right_t = right_node.intersects_ray(&transformed_ray, transform);

                        match (left_t, right_t) {
                            (Some(left_t), Some(right_t)) => {
                                if left_t < right_t {
                                    stack.push_back(right);
                                    stack.push_back(left);
                                } else {
                                    stack.push_back(left);
                                    stack.push_back(right);
                                }
                            }
                            _ => {
                                stack.push_back(left);
                                stack.push_back(right);
                            }
                        }
                    }
                }
                None => {}
            }
        }

        closest
    }

    fn split_node(
        &mut self,
        node_index: u32,
        face_centroid_cache: &DenseMap<FaceHandle, Vec3>,
        face_aabb_cache: &DenseMap<FaceHandle, Aabb3d>,
    ) -> Option<(u32, u32)> {
        let root = &self.nodes[node_index as usize];

        let (root_aabb, root_primitive_list) = match root {
            Node::Leaf {
                aabb,
                primitive_list,
            } => (aabb.clone(), primitive_list),
            _ => panic!("Node is not a leaf node"),
        };

        let primitive_count = root_primitive_list.len();

        let (
            mut best_split_axis,
            mut best_split_position,
            mut best_split_cost,
            mut best_split_left_size,
            mut best_split_right_size,
            mut best_split_left_aabb,
            mut best_split_right_aabb,
        ) = (None, None, None, None, None, None, None);

        for axis in 0..3 {
            let mut buckets = [Bucket::default(); Self::BUCKET_COUNT];

            let min = root_aabb.min[axis] as f64;
            let max = root_aabb.max[axis] as f64;
            let range = max - min;

            for face_handle in root_primitive_list.iter().cloned() {
                let centroid = face_centroid_cache[face_handle];

                let bucket_index =
                    ((centroid[axis] as f64 - min) / (max - min) * Self::BUCKET_COUNT as f64)
                        .min(Self::BUCKET_COUNT as f64 - 1.0) as usize;

                let bucket = &mut buckets[bucket_index];

                if let Some(bucket_aabb) = bucket.aabb {
                    bucket.aabb = Some(bucket_aabb.merge(&face_aabb_cache[face_handle]));
                } else {
                    bucket.aabb = Some(face_aabb_cache[face_handle]);
                }

                bucket.primitive_count += 1;
            }

            for i in 1..(Self::BUCKET_COUNT - 1) {
                let left_split = 0..i;
                let right_split = i..Self::BUCKET_COUNT;

                let combined_left_bucket =
                    &buckets[left_split]
                        .iter()
                        .fold(Bucket::default(), |acc, bucket| Bucket {
                            primitive_count: acc.primitive_count + bucket.primitive_count,
                            aabb: if acc.aabb.is_none() {
                                bucket.aabb
                            } else if bucket.aabb.is_none() {
                                acc.aabb
                            } else {
                                Some(acc.aabb.unwrap().merge(&bucket.aabb.unwrap()))
                            },
                        });

                if combined_left_bucket.aabb.is_none() {
                    assert_eq!(combined_left_bucket.primitive_count, 0);
                    continue;
                }

                let combined_right_bucket =
                    &buckets[right_split]
                        .iter()
                        .fold(Bucket::default(), |acc, bucket| Bucket {
                            primitive_count: acc.primitive_count + bucket.primitive_count,
                            aabb: if acc.aabb.is_none() {
                                bucket.aabb
                            } else if bucket.aabb.is_none() {
                                acc.aabb
                            } else {
                                Some(acc.aabb.unwrap().merge(&bucket.aabb.unwrap()))
                            },
                        });

                if combined_right_bucket.aabb.is_none() {
                    assert_eq!(combined_right_bucket.primitive_count, 0);
                    continue;
                }

                let root_surface_area = root_aabb.visible_area();

                let split_cost = ((combined_left_bucket.aabb.unwrap().visible_area()
                    / root_surface_area)
                    * combined_left_bucket.primitive_count as f32)
                    + ((combined_right_bucket.aabb.unwrap().visible_area() / root_surface_area)
                        * combined_right_bucket.primitive_count as f32);

                if let Some(value) = best_split_cost {
                    if split_cost < value {
                        best_split_axis = Some(axis);
                        best_split_position =
                            Some(min + ((i as f64 / Self::BUCKET_COUNT as f64) * range));
                        best_split_cost = Some(split_cost);
                        best_split_left_size = Some(combined_left_bucket.primitive_count);
                        best_split_right_size = Some(combined_right_bucket.primitive_count);
                        best_split_left_aabb = Some(combined_left_bucket.aabb.unwrap());
                        best_split_right_aabb = Some(combined_right_bucket.aabb.unwrap());
                    }
                } else {
                    best_split_axis = Some(axis);
                    best_split_position =
                        Some(min + ((i as f64 / Self::BUCKET_COUNT as f64) * range));
                    best_split_cost = Some(split_cost);
                    best_split_left_size = Some(combined_left_bucket.primitive_count);
                    best_split_right_size = Some(combined_right_bucket.primitive_count);
                    best_split_left_aabb = Some(combined_left_bucket.aabb.unwrap());
                    best_split_right_aabb = Some(combined_right_bucket.aabb.unwrap());
                }
            }
        }

        if let None = best_split_cost {
            return None;
        }

        let best_split_axis = best_split_axis.unwrap();
        let best_split_position = best_split_position.unwrap();
        let best_split_left_size = best_split_left_size.unwrap();
        let best_split_right_size = best_split_right_size.unwrap();
        let best_split_left_aabb = best_split_left_aabb.unwrap();
        let best_split_right_aabb = best_split_right_aabb.unwrap();

        let mut left_primitive_list: Vec<FaceHandle> =
            Vec::with_capacity(best_split_left_size as usize);
        let mut right_primitive_list: Vec<FaceHandle> =
            Vec::with_capacity(best_split_right_size as usize);

        for face_handle in root_primitive_list.iter().cloned() {
            let centroid = face_centroid_cache[face_handle];
            if (centroid[best_split_axis] as f64) < best_split_position {
                left_primitive_list.push(face_handle);
            } else {
                right_primitive_list.push(face_handle);
            }
        }

        assert_eq!(
            primitive_count,
            best_split_left_size as usize + best_split_right_size as usize
        );

        assert_eq!(left_primitive_list.len(), best_split_left_size as usize);
        assert_eq!(right_primitive_list.len(), best_split_right_size as usize);

        let left_node_index = self.nodes.len() as u32;
        let right_node_index = left_node_index + 1;

        self.nodes.push(Node::Leaf {
            aabb: best_split_left_aabb,
            primitive_list: left_primitive_list,
        });

        self.nodes.push(Node::Leaf {
            aabb: best_split_right_aabb,
            primitive_list: right_primitive_list,
        });

        self.nodes[node_index as usize] = Node::NonLeaf {
            left: left_node_index,
            right: right_node_index,
            aabb: root_aabb,
        };

        Some((left_node_index, right_node_index))
    }
}

impl From<&EditableMesh> for BoundingVolumeHierarchy {
    fn from(mesh: &EditableMesh) -> Self {
        let maximum_primitive_per_leaf = 8u32;

        let mut face_centroid_cache =
            DenseMap::<FaceHandle, Vec3>::with_capacity(mesh.structure.num_faces());
        let mut face_aabb_cache =
            DenseMap::<FaceHandle, Aabb3d>::with_capacity(mesh.structure.num_faces());

        for face in mesh.structure.faces() {
            let vertex_count = face.adjacent_vertices().count();

            let centroid = face
                .adjacent_vertices()
                .map(|v| mesh.vertex_positions[v.handle()])
                .fold(Vec3::ZERO, |accumulator, position| position + accumulator)
                / vertex_count as f32;

            face_centroid_cache.insert(face.handle(), centroid);

            let mut vertices = face.adjacent_vertices();

            let first = mesh.vertex_positions[vertices.next().unwrap().handle()];

            let (min, max) = vertices
                .map(|v| mesh.vertex_positions[v.handle()])
                .fold((first, first), |(min, max), position| {
                    (position.min(min), position.max(max))
                });

            face_aabb_cache.insert(face.handle(), Aabb3d { min, max });
        }

        let mut face_aabbs = face_aabb_cache.values();
        let first = face_aabbs.next().unwrap().clone();

        let root_aabb = face_aabb_cache
            .values()
            .fold(first, |acc, curr| acc.merge(curr));

        let mut bvh = BoundingVolumeHierarchy::new();

        let node = Node::Leaf {
            aabb: root_aabb,
            primitive_list: mesh.structure.face_handles().collect(),
        };

        bvh.nodes.push(node);

        let mut queue = VecDeque::<u32>::new();

        queue.push_back(0);

        while !queue.is_empty() {
            let top = queue.pop_front().unwrap();

            let node = bvh.nodes.get(top as usize).unwrap();

            let Node::Leaf { primitive_list, .. } = node else {
                panic!("This should not happen, likely a bug in the code");
            };

            let primitive_count = primitive_list.len();

            if primitive_count <= maximum_primitive_per_leaf as usize {
                continue;
            }

            match bvh.split_node(top, &face_centroid_cache, &face_aabb_cache) {
                Some((left, right)) => {
                    queue.push_back(left);
                    queue.push_back(right);
                }
                None => {
                    warn!(
                        "Failed to split node, leaving at size {} when max is {}. Consider increasing the maximum primitive per leaf",
                        primitive_count, maximum_primitive_per_leaf
                    );
                }
            };
        }

        info!("{}, {}", bvh.nodes.len(), mesh.structure.num_faces());

        return bvh;
    }
}

#[cfg(test)]
mod test {
    use bevy::prelude::{Meshable, Sphere};

    use crate::core::editable_mesh::EditableMesh;

    use super::BoundingVolumeHierarchy;

    #[test]
    fn test_bvh_from_mesh() {
        let mesh = Sphere::new(0.05).mesh().ico(5).unwrap();

        let editable_mesh = EditableMesh::from(&mesh);

        let bvh = BoundingVolumeHierarchy::from(&editable_mesh);

        print!("{}", bvh.nodes.len());
    }
}
