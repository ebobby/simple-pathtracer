pub mod reversed_normal;
pub mod sphere;

use crate::material::Material;
use crate::ray::Ray;
use crate::vector::Vec3;

use std::fmt::Debug;
use std::marker::{Send, Sync};

pub trait Intersectable: Debug + Send + Sync {
    fn intersect(&self, ray: Ray, min: f64, max: f64) -> Option<f64>;
    fn normal(&self, point: Vec3) -> Vec3;
    fn material(&self) -> Material;
}

#[derive(Clone, Copy, Debug)]
pub struct Intersection {
    pub normal: Vec3,
    pub p: Vec3,
    pub t: f64,
    pub material: Material,
}

#[derive(Debug)]
pub struct IntersectableList {
    intersectables: Vec<Box<dyn Intersectable + Send>>,
}

impl IntersectableList {
    pub fn new() -> IntersectableList {
        IntersectableList {
            intersectables: Vec::new(),
        }
    }

    pub fn push(&mut self, object: Box<dyn Intersectable + Send>) {
        self.intersectables.push(object);
    }

    pub fn intersect(&self, ray: Ray) -> Option<Intersection> {
        let mut normal = Vec3::zero();
        let mut t = std::f64::INFINITY;
        let mut p = Vec3::zero();
        let mut material = Material::None;

        for object in &self.intersectables {
            if let Some(dist) = object.intersect(ray, std::f64::EPSILON, t) {
                t = dist;
                p = ray.origin + t * ray.direction;
                normal = object.normal(p);
                material = object.material();
            }
        }

        if t < std::f64::INFINITY {
            Some(Intersection {
                normal,
                p,
                t,
                material,
            })
        } else {
            None
        }
    }
}
