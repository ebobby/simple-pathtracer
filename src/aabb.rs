//! Axis-aligned minimum bounding box.

use crate::ray::Ray;
use crate::vector::Vec3;

#[derive(Clone, Copy, Debug)]
pub struct Aabb {
    min: Vec3,
    max: Vec3,
}

impl Aabb {
    pub fn new(min: Vec3, max: Vec3) -> Aabb {
        Aabb { min, max }
    }

    pub fn intersect(&self, ray: Ray, min: f64, max: f64) -> bool {
        axis_intersect(
            self.min.x,
            self.max.x,
            ray.origin.x,
            ray.direction.x,
            min,
            max,
        ) && axis_intersect(
            self.min.y,
            self.max.y,
            ray.origin.y,
            ray.direction.y,
            min,
            max,
        ) && axis_intersect(
            self.min.z,
            self.max.z,
            ray.origin.z,
            ray.direction.z,
            min,
            max,
        )
    }
}

fn axis_intersect(
    min_val: f64,
    max_val: f64,
    ray_val: f64,
    ray_d: f64,
    tmin: f64,
    tmax: f64,
) -> bool {
    let inv_d = ray_d.recip();

    let mut t0 = (min_val - ray_val) * inv_d;
    let mut t1 = (max_val - ray_val) * inv_d;

    if inv_d < 0.0 {
        std::mem::swap(&mut t0, &mut t1)
    }

    let tmin = if t0 > tmin {
        t0
    } else {
        tmin
    };

    let tmax = if t1 < tmax {
        t1
    } else {
        tmax
    };

    tmax > tmin
}
