use crate::aabb::AABB;
use crate::material::Material;
use crate::ray::Ray;
use crate::vector::Vec3;

use std::fmt::Debug;
use std::marker::{Send, Sync};

/// Hitable is a boxed trait object that implements `Intersectable`.
pub type Hitable = Box<dyn Intersectable + Send + Sync>;

pub trait Intersectable: Debug + Send + Sync {
    fn intersect(&self, ray: Ray, min: f64, max: f64) -> Option<Intersection>;
    fn bounding_box(&self) -> AABB;
}

/// Intersection record. When we hit an object, this is where we store that hit.
///
/// # Members
/// * `p` - Point in world space where the we hit the object.
/// * `t` - Distance from the ray origin to the point.
/// * `normal` - Normal from the hit point.
/// * `u` - Texture coordinates.
/// * `v` - Texture coordinates.
/// * `material` - Material of the hit object.
#[derive(Clone, Copy, Debug)]
pub struct Intersection {
    pub p: Vec3,
    pub t: f64,
    pub normal: Vec3,
    pub u: f64,
    pub v: f64,
    pub material: Material,
}
