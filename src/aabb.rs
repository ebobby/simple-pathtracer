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
    let divisor = ray_d.recip();

    let t0 = ((min_val - ray_val) * divisor).min((max_val - ray_val) * divisor);
    let t1 = ((min_val - ray_val) * divisor).max((max_val - ray_val) * divisor);

    let min = tmin.max(t0);
    let max = tmax.min(t1);

    max > min
}
