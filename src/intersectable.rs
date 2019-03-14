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

#[derive(Clone, Copy, Debug)]
pub struct Intersection {
    pub normal: Vec3,
    pub p: Vec3,
    pub t: f64,
    pub material: Material,
}
