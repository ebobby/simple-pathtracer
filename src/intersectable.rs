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

#[derive(Debug, Default)]
pub struct List {
    intersectables: Vec<Hitable>,
}

impl List {
    pub fn new() -> Self {
        Self {
            intersectables: Vec::new(),
        }
    }

    pub fn from_vec(list: Vec<Hitable>) -> Self {
        Self {
            intersectables: list,
        }
    }

    pub fn push(&mut self, object: Hitable) {
        self.intersectables.push(object);
    }

    pub fn intersect(&self, ray: Ray) -> Option<Intersection> {
        let mut t = std::f64::INFINITY;
        let mut intersection = None;

        for object in &self.intersectables {
//            let box_hit = object
//                .bounding_box()
//                .map_or(true, |aabb| aabb.intersect(ray, std::f64::EPSILON, t));
//
//            if !box_hit {
//                continue;
//            }

            if let Some(int) = object.intersect(ray, std::f64::EPSILON, t) {
                intersection = Some(int);
                t = int.t;
            }
        }

        intersection
    }
}
