//! Axis-aligned minimum bounding box.

use crate::ray::Ray;
use crate::Vec3;

#[derive(Clone, Copy, Debug)]
pub struct AABB {
    pub min: Vec3,
    pub max: Vec3,
}

impl AABB {
    pub fn surrounding(box0: AABB, box1: AABB) -> AABB {
        let min = Vec3 {
            x: box0.min.x.min(box1.min.x),
            y: box0.min.y.min(box1.min.y),
            z: box0.min.z.min(box1.min.z),
        };
        let max = Vec3 {
            x: box0.max.x.max(box1.max.x),
            y: box0.max.y.max(box1.max.y),
            z: box0.max.z.max(box1.max.z),
        };

        AABB { min, max }
    }

    pub fn intersect(&self, ray: &Ray, tmin: f64, tmax: f64) -> bool {
        // Check X axis
        let mut inv_d = ray.direction.x.recip();
        let mut t0 = (self.min.x - ray.origin.x) * inv_d;
        let mut t1 = (self.max.x - ray.origin.x) * inv_d;

        if inv_d < 0.0 {
            std::mem::swap(&mut t0, &mut t1)
        }

        let tmin = if t0 > tmin { t0 } else { tmin };
        let tmax = if t1 < tmax { t1 } else { tmax };

        if tmax <= tmin {
            return false;
        }

        // Check Y axis
        inv_d = ray.direction.y.recip();
        t0 = (self.min.y - ray.origin.y) * inv_d;
        t1 = (self.max.y - ray.origin.y) * inv_d;

        if inv_d < 0.0 {
            std::mem::swap(&mut t0, &mut t1)
        }

        let tmin = if t0 > tmin { t0 } else { tmin };
        let tmax = if t1 < tmax { t1 } else { tmax };

        if tmax <= tmin {
            return false;
        }

        // Check Z axis
        inv_d = ray.direction.z.recip();
        t0 = (self.min.z - ray.origin.z) * inv_d;
        t1 = (self.max.z - ray.origin.z) * inv_d;

        if inv_d < 0.0 {
            std::mem::swap(&mut t0, &mut t1)
        }

        let tmin = if t0 > tmin { t0 } else { tmin };
        let tmax = if t1 < tmax { t1 } else { tmax };

        if tmax <= tmin {
            return false;
        }

        true
    }
}
