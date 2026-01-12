//! GPU-compatible data types for wgpu compute shaders.
//!
//! These types use `#[repr(C)]` for predictable memory layout and
//! implement bytemuck traits for safe GPU buffer uploads.

use bytemuck::{Pod, Zeroable};

use crate::camera::Camera;
use crate::material::{Dielectric, DiffuseLight, Lambertian, Metal};
use crate::texture::{Checker, ConstantColor};
use crate::Color;
use crate::Material;
use crate::Texture;
use crate::Vec3;

/// GPU-compatible 3D vector with padding for 16-byte alignment.
#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct GPUVec3 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub _pad: f32,
}

impl GPUVec3 {
    pub fn new(x: f32, y: f32, z: f32) -> Self {
        Self { x, y, z, _pad: 0.0 }
    }

    pub fn zero() -> Self {
        Self { x: 0.0, y: 0.0, z: 0.0, _pad: 0.0 }
    }
}

impl From<Vec3> for GPUVec3 {
    fn from(v: Vec3) -> Self {
        Self::new(v.x as f32, v.y as f32, v.z as f32)
    }
}

impl From<Color> for GPUVec3 {
    fn from(c: Color) -> Self {
        Self::new(c.r as f32, c.g as f32, c.b as f32)
    }
}

/// GPU-compatible BVH node for iterative traversal.
#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct GPUBVHNode {
    pub aabb_min: GPUVec3,
    pub aabb_max: GPUVec3,
    pub left_idx: u32,
    pub right_idx: u32,
    pub shape_idx: u32,
    pub is_leaf: u32,
}

impl GPUBVHNode {
    pub fn interior(aabb_min: GPUVec3, aabb_max: GPUVec3, left_idx: u32, right_idx: u32) -> Self {
        Self {
            aabb_min,
            aabb_max,
            left_idx,
            right_idx,
            shape_idx: u32::MAX,
            is_leaf: 0,
        }
    }

    pub fn leaf(aabb_min: GPUVec3, aabb_max: GPUVec3, shape_idx: u32) -> Self {
        Self {
            aabb_min,
            aabb_max,
            left_idx: u32::MAX,
            right_idx: u32::MAX,
            shape_idx,
            is_leaf: 1,
        }
    }
}

/// GPU-compatible sphere representation.
/// Must be 48 bytes to match WGSL array stride (16-byte alignment of vec4).
#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct GPUSphere {
    pub center: GPUVec3,
    pub radius: f32,
    pub material_idx: u32,
    pub _pad: [u32; 6], // 24 bytes padding to reach 48 bytes total
}

impl GPUSphere {
    pub fn new(center: GPUVec3, radius: f32, material_idx: u32) -> Self {
        Self {
            center,
            radius,
            material_idx,
            _pad: [0; 6],
        }
    }
}

/// GPU-compatible disc representation.
#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct GPUDisc {
    pub center: GPUVec3,
    pub normal: GPUVec3,
    pub radius: f32,
    pub material_idx: u32,
    pub _pad: [u32; 2],
}

impl GPUDisc {
    pub fn new(center: GPUVec3, normal: GPUVec3, radius: f32, material_idx: u32) -> Self {
        Self {
            center,
            normal,
            radius,
            material_idx,
            _pad: [0; 2],
        }
    }
}

/// Material type constants for GPU shader branching.
pub const MATERIAL_LAMBERTIAN: u32 = 0;
pub const MATERIAL_METAL: u32 = 1;
pub const MATERIAL_DIELECTRIC: u32 = 2;
pub const MATERIAL_DIFFUSE_LIGHT: u32 = 3;

/// GPU-compatible material representation.
/// Must be 32 bytes to match WGSL array stride (16-byte alignment of vec4).
#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct GPUMaterial {
    pub color: GPUVec3,
    pub material_type: u32,
    pub fuzz: f32,
    pub ior: f32,
    pub _pad: f32, // 4 bytes padding to reach 32 bytes total
}

impl GPUMaterial {
    pub fn lambertian(color: GPUVec3) -> Self {
        Self {
            color,
            material_type: MATERIAL_LAMBERTIAN,
            fuzz: 0.0,
            ior: 1.0,
            _pad: 0.0,
        }
    }

    pub fn metal(color: GPUVec3, fuzz: f32) -> Self {
        Self {
            color,
            material_type: MATERIAL_METAL,
            fuzz,
            ior: 1.0,
            _pad: 0.0,
        }
    }

    pub fn dielectric(color: GPUVec3, ior: f32) -> Self {
        Self {
            color,
            material_type: MATERIAL_DIELECTRIC,
            fuzz: 0.0,
            ior,
            _pad: 0.0,
        }
    }

    pub fn diffuse_light(color: GPUVec3) -> Self {
        Self {
            color,
            material_type: MATERIAL_DIFFUSE_LIGHT,
            fuzz: 0.0,
            ior: 1.0,
            _pad: 0.0,
        }
    }
}

/// Extract color from a texture (supports only ConstantColor for now).
fn texture_to_color(texture: &Texture) -> GPUVec3 {
    match texture {
        Texture::ConstantColor(ConstantColor { color }) => GPUVec3::new(
            color.r as f32,
            color.g as f32,
            color.b as f32,
        ),
        Texture::Checker(Checker { even, .. }) => {
            // Use the "even" color as fallback
            GPUVec3::new(
                even.r as f32,
                even.g as f32,
                even.b as f32,
            )
        }
        Texture::Bitmap(_) => {
            // Fallback to white for bitmap textures
            GPUVec3::new(1.0, 1.0, 1.0)
        }
    }
}

impl From<&Material> for GPUMaterial {
    fn from(material: &Material) -> Self {
        match material {
            Material::Lambertian(Lambertian { albedo }) => {
                GPUMaterial::lambertian(texture_to_color(albedo))
            }
            Material::Metal(Metal { albedo, fuzz }) => {
                GPUMaterial::metal(texture_to_color(albedo), *fuzz as f32)
            }
            Material::Dielectric(Dielectric { attenuation, refractive_index }) => {
                GPUMaterial::dielectric(
                    texture_to_color(attenuation),
                    *refractive_index as f32,
                )
            }
            Material::DiffuseLight(DiffuseLight { texture }) => {
                GPUMaterial::diffuse_light(texture_to_color(texture))
            }
        }
    }
}

/// GPU-compatible camera representation.
#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct GPUCamera {
    pub origin: GPUVec3,
    pub corner: GPUVec3,
    pub horizontal: GPUVec3,
    pub vertical: GPUVec3,
}

impl From<&Camera> for GPUCamera {
    fn from(camera: &Camera) -> Self {
        Self {
            origin: camera.look_from().into(),
            corner: camera.corner().into(),
            horizontal: camera.horizontal().into(),
            vertical: camera.vertical().into(),
        }
    }
}

/// Render parameters passed to GPU shader.
#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct GPURenderParams {
    pub width: u32,
    pub height: u32,
    pub samples: u32,
    pub max_depth: u32,
    pub frame_seed: u32,
    pub num_spheres: u32,
    pub num_discs: u32,
    pub _pad: u32,
}

impl GPURenderParams {
    pub fn new(
        width: u32,
        height: u32,
        samples: u32,
        max_depth: u32,
        num_spheres: u32,
        num_discs: u32,
    ) -> Self {
        Self {
            width,
            height,
            samples,
            max_depth,
            frame_seed: 0,
            num_spheres,
            num_discs,
            _pad: 0,
        }
    }
}
