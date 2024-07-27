use bevy;
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub struct UvSphereOptions {
    pub radius: f32,
    pub latitudes: u32,
    pub longitudes: u32,
}

#[derive(Clone, Copy, Default, Debug)]
#[wasm_bindgen]
pub struct Vec3 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

impl From<bevy::math::Vec3> for Vec3 {
    fn from(vec3: bevy::math::Vec3) -> Self {
        Self {
            x: vec3.x,
            y: vec3.y,
            z: vec3.z,
        }
    }
}

impl Into<bevy::math::Vec3> for Vec3 {
    fn into(self) -> bevy::math::Vec3 {
        bevy::math::Vec3::new(self.x, self.y, self.z)
    }
}

#[derive(Clone, Copy, Default, Debug)]
#[wasm_bindgen]
pub struct Quat {
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub w: f32,
}

impl From<bevy::math::Quat> for Quat {
    fn from(quat: bevy::math::Quat) -> Self {
        Self {
            x: quat.x,
            y: quat.y,
            z: quat.z,
            w: quat.w,
        }
    }
}

impl Into<bevy::math::Quat> for Quat {
    fn into(self) -> bevy::math::Quat {
        bevy::math::quat(self.x, self.y, self.z, self.w)
    }
}

#[derive(Clone, Copy, Default, Debug)]
#[wasm_bindgen]
pub struct Transform {
    pub translation: Vec3,
    pub rotation: Quat,
    pub scale: Vec3,
}

impl From<bevy::transform::components::Transform> for Transform {
    fn from(transform: bevy::transform::components::Transform) -> Self {
        Self {
            translation: transform.translation.into(),
            rotation: transform.rotation.into(),
            scale: transform.scale.into(),
        }
    }
}

impl Into<bevy::transform::components::Transform> for Transform {
    fn into(self) -> bevy::transform::components::Transform {
        bevy::transform::components::Transform {
            translation: self.translation.into(),
            rotation: self.rotation.into(),
            scale: self.scale.into(),
        }
    }
}

#[wasm_bindgen]
impl UvSphereOptions {
    #[wasm_bindgen(constructor)]
    pub fn new(radius: f32, latitudes: u32, longitudes: u32) -> Self {
        Self {
            radius,
            latitudes,
            longitudes,
        }
    }
}
#[wasm_bindgen]
pub struct CubeOptions {
    size: f32,
}

#[wasm_bindgen]
impl CubeOptions {
    #[wasm_bindgen(constructor)]
    pub fn new(size: f32) -> Self {
        Self { size }
    }
}

#[wasm_bindgen]
pub struct CylinderOptions {
    pub radius: f32,
    pub height: f32,
    pub depth: u32,
}

#[wasm_bindgen]
impl CylinderOptions {
    #[wasm_bindgen(constructor)]
    pub fn new(radius: f32, height: f32, depth: u32) -> Self {
        Self {
            radius,
            height,
            depth,
        }
    }
}

#[wasm_bindgen]
pub struct PlaneOptions {
    pub size: f32,
    pub width_segments: u32,
    pub height_segments: u32,
}

#[wasm_bindgen]
impl PlaneOptions {
    #[wasm_bindgen(constructor)]
    pub fn new(size: f32, width_segments: u32, height_segments: u32) -> Self {
        Self {
            size,
            width_segments,
            height_segments,
        }
    }
}

#[wasm_bindgen]
#[derive(Hash, Eq, PartialEq, Debug)]
pub enum ToolType {
    Move,
    Rotate,
    Scale,
    Cursor,
}
