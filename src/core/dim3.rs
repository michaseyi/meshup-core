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
    pub fn intersets_ray_at(&self, ray: &RayCast3d) -> Option<f32> {
        let d = self.orientation * (ray.ray.origin - self.position);
        let e: Vec3 = self.orientation * Vec3::from(ray.ray.direction);
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

        use utils::quartic::QuarticRoots;

        let closest_t = match roots {
            QuarticRoots::Four(roots) => roots
                .into_iter()
                .filter(|t| t.is_sign_positive())
                .reduce(f32::min),
            QuarticRoots::Three(roots) => roots
                .into_iter()
                .filter(|t| t.is_sign_positive())
                .reduce(f32::min),
            QuarticRoots::Two(roots) => roots
                .into_iter()
                .filter(|t| t.is_sign_positive())
                .reduce(f32::min),
            QuarticRoots::One(roots) => {
                if roots[0].is_sign_positive() {
                    Some(roots[0])
                } else {
                    None
                }
            }
            _ => None,
        };

        match closest_t {
            Some(t) => {
                if t > ray.max {
                    return None;
                }
                Some(t)
            }
            None => None,
        }
    }
    
    pub fn intersects_ray(&self, ray: &RayCast3d) -> bool {
        self.intersets_ray_at(ray).is_some()
    }
}

pub struct Triangle3d {
    pub vertices: [Vec3; 3],
}

impl Triangle3d {
    pub fn new(vertices: [Vec3; 3]) -> Self {
        Self { vertices }
    }

    pub fn intersects_ray_at(&self, ray: &RayCast3d) -> Option<f32> {
        let edge1 = self.vertices[1] - self.vertices[0];
        let edge2 = self.vertices[2] - self.vertices[0];
        let h = ray.ray.direction.cross(edge2);
        let a = edge1.dot(h);

        if a > -0.00001 && a < 0.00001 {
            return None;
        }

        let f = 1.0 / a;
        let s = ray.ray.origin - self.vertices[0];
        let u = f * s.dot(h);

        if u < 0.0 || u > 1.0 {
            return None;
        }

        let q = s.cross(edge1);
        let v = f * ray.ray.direction.dot(q);

        if v < 0.0 || u + v > 1.0 {
            return None;
        }

        let t = f * edge2.dot(q);

        if t > 0.00001 {
            return Some(t);
        }

        None
    }
}

/// Input vertices must be in counter-clockwise order.
pub fn ray_intersects_convex_plane_at(
    ray: &RayCast3d,
    transform: &Transform,
    vertices: &[Vec3],
) -> Option<f32> {
    let first = vertices[0];
    for triangle in vertices[1..].windows(2).map(|vertices| {
        Triangle3d::new([
            first.clone() * transform.scale + transform.translation,
            vertices[0] * transform.scale + transform.translation,
            vertices[1] * transform.scale + transform.translation,
        ])
    }) {
        let result = triangle.intersects_ray_at(ray);

        if let Some(t) = result {
            if t < ray.max {
                return Some(t);
            } else {
                return None;
            }
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use bevy::math::Ray3d;

    use super::*;
    use core::f32;

    #[test]
    fn test_ray_intersect_convex_plane() {
        let vertices = [
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(1.0, 0.0, 0.0),
            Vec3::new(1.0, 1.0, 0.0),
            Vec3::new(0.0, 1.0, 0.0),
        ];

        let ray = RayCast3d::from_ray(
            Ray3d::new(Vec3::new(0.5, 0.5, 1.0), Vec3::new(0.0, 0.0, -1.0)),
            1000.0,
        );

        let transform = Transform::IDENTITY;

        assert_eq!(
            ray_intersects_convex_plane_at(&ray, &transform, &vertices),
            Some(1.0)
        );

        let ray = RayCast3d::from_ray(
            Ray3d::new(Vec3::new(0.5, -0.5, 1.0), Vec3::new(0.0, 0.0, -1.0)),
            1000.0,
        );
        assert_eq!(
            ray_intersects_convex_plane_at(&ray, &transform, &vertices),
            None
        );
    }

    #[test]
    fn test_default_orientation_torus_ray_intersection() {
        let torus = Torus::new(1.0, 0.5, Vec3::ZERO, Quat::IDENTITY);
        let ray = RayCast3d::from_ray(
            Ray3d {
                origin: Vec3::new(0.0, 0.0, 0.0),
                direction: Direction3d::new(Vec3::new(0.0, 0.0, -1.0)).unwrap(),
            },
            1000.,
        );
        assert!(torus.intersects_ray(&ray));
        assert_eq!(torus.intersets_ray_at(&ray), Some(0.5));
    }

    #[test]
    fn test_non_default_position_torus_ray_intersection() {
        let torus = Torus::new(1.0, 0.5, Vec3::new(0.0, 0.0, -10.0), Quat::IDENTITY);
        let ray = RayCast3d::from_ray(
            Ray3d {
                origin: Vec3::new(0.0, 0.0, 0.0),
                direction: Direction3d::new(Vec3::new(0.0, 0.0, -1.0)).unwrap(),
            },
            1000.,
        );
        assert!(torus.intersects_ray(&ray));
        assert_eq!(torus.intersets_ray_at(&ray), Some(8.5));
    }

    #[test]
    fn test_non_default_orientation_torus_ray_intersection() {
        let torus = Torus::new(
            1.0,
            0.5,
            Vec3::ZERO,
            Quat::from_rotation_z(f32::consts::FRAC_PI_2),
        );
        let ray = RayCast3d::from_ray(
            Ray3d {
                origin: Vec3::new(0.0, 0.0, 0.0),
                direction: Direction3d::new(Vec3::new(0.0, 1.0, 0.0)).unwrap(),
            },
            1000.,
        );
        assert!(torus.intersects_ray(&ray));
        assert_eq!(torus.intersets_ray_at(&ray), Some(0.5));
    }
}
