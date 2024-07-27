use bevy::{
    math::bounding::RayCast3d,
    prelude::*,
    render::{mesh::Indices, render_asset::RenderAssetUsages},
};

use crate::utils;

pub struct Cone {
    pub radius: f32,
    pub height: f32,
}

pub struct ConeMeshBuilder {
    pub cone: Cone,
    // number of cone base verties
    pub resolution: u32,
}

impl Default for ConeMeshBuilder {
    fn default() -> Self {
        Self {
            cone: Cone::new(1.0, 1.0),
            resolution: 32,
        }
    }
}

impl From<Cone> for ConeMeshBuilder {
    fn from(value: Cone) -> Self {
        Self {
            cone: value,
            ..default()
        }
    }
}

impl Cone {
    pub fn new(radius: f32, height: f32) -> Self {
        Self { radius, height }
    }

    pub fn mesh(self) -> ConeMeshBuilder {
        ConeMeshBuilder::from(self)
    }
}

impl ConeMeshBuilder {
    pub fn build(&self) -> Mesh {
        let mut mesh = Mesh::new(
            bevy::render::mesh::PrimitiveTopology::TriangleList,
            RenderAssetUsages::RENDER_WORLD | RenderAssetUsages::MAIN_WORLD,
        );

        let mut vertices: Vec<Vec3> = (0..self.resolution + 2).map(|_| Vec3::ZERO).collect();

        let apex_index = 0;
        let base_index = 1;

        for i in 0..self.resolution {
            let angle = 2.0 * std::f32::consts::PI * (i as f32 / self.resolution as f32);
            let x = self.cone.radius * angle.cos();
            let z = self.cone.radius * angle.sin();
            vertices[(i + 2) as usize] = Vec3::new(x, 0.0, z);
        }

        vertices[0] = Vec3::new(0.0, self.cone.height, 0.0);

        let mut indices: Vec<u32> = Vec::new();
        indices.reserve((self.resolution * 6) as usize);

        for i in 0..self.resolution {
            let next = ((i + 1) % self.resolution) + 2;
            indices.push(i + 2);
            indices.push(next);
            indices.push(apex_index);

            indices.push(base_index);
            indices.push(i + 2);
            indices.push(next);
        }
        mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, vertices);
        mesh.insert_indices(Indices::U32(indices));

        mesh.with_duplicated_vertices().with_computed_flat_normals()
    }

    pub fn resolution(mut self, resolution: u32) -> Self {
        self.resolution = resolution;
        self
    }
}

impl From<ConeMeshBuilder> for Mesh {
    fn from(value: ConeMeshBuilder) -> Self {
        value.build()
    }
}

pub struct Torus {
    pub ring_radius: f32,
    pub ring_thickness: f32,
    pub position: Vec3,
    pub orientation: Quat,
}

impl Default for Torus {
    fn default() -> Self {
        Self {
            ring_radius: 1.0,
            ring_thickness: 0.5,
            position: Vec3::ZERO,
            orientation: Quat::IDENTITY,
        }
    }
}

impl Torus {
    pub fn new(ring_radius: f32, ring_thickness: f32, position: Vec3, orientation: Quat) -> Self {
        Self {
            ring_radius,
            ring_thickness,
            position,
            orientation,
        }
    }

    /// Reference: http://cosinekitty.com/raytrace/chapter13_torus.html
    pub fn intersets_ray_at(&self, ray: &RayCast3d) -> Option<Vec3> {
        let d = ray.ray.origin;
        let e: Vec3 = ray.ray.direction.into();
        let a = self.ring_radius;
        let b = self.ring_thickness;

        let a_squared = a * a;
        let b_squared = b * b;
        let e_squared = e * e;
        let d_squared = d * d;
        let d_e = d * e;

        let g = 4. * a_squared * (e_squared.x + e_squared.z);
        let h = 8. * a_squared * (d_e.x + d_e.z);
        let i = 4. * a_squared * (d_squared.x + d_squared.z);
        let j = e.length_squared();
        let k = 2. * d.dot(e);
        let l = d.length_squared() + (a_squared - b_squared);

        let a4 = j * j;
        let a3 = 2. * j * k;
        let a2 = (2. * j * l) + (k * k) - g;
        let a1 = (2. * k * l) - h;
        let a0 = (l * l) - i;

        let roots = utils::quartic::solve_quartic(a4, a3, a2, a1, a0);

        match roots.min() {
            Some(t) => {
                if t > ray.max {
                    return None;
                }
                Some(d + e * t)
            }
            None => None,
        }
    }
    pub fn intersects_ray(&self, ray: &RayCast3d) -> bool {
        self.intersets_ray_at(ray).is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_torus_creation() {
        let torus = Torus::new(1.0, 0.5, Vec3::ZERO, Quat::IDENTITY);
        assert_eq!(torus.ring_radius, 1.0);
        assert_eq!(torus.ring_thickness, 0.5);
        assert_eq!(torus.position, Vec3::ZERO);
        assert_eq!(torus.orientation, Quat::IDENTITY);
    }

    #[test]
    fn test_torus_ray_intersection_in_default_orientation() {
        let torus = Torus::new(1.0, 0.5, Vec3::ZERO, Quat::IDENTITY);
        let ray = RayCast3d::from_ray(
            Ray3d {
                origin: Vec3::new(0.0, 0.0, 2.0),
                direction: Direction3d::new(Vec3::new(0.0, 0.0, -1.0)).unwrap(),
            },
            1000.,
        );
        assert!(torus.intersects_ray(&ray));
        assert_eq!(torus.intersets_ray_at(&ray), Some(Vec3::new(0.0, 0.0, 1.5)));
    }
}
