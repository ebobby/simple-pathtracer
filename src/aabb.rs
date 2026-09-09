//! Axis-aligned minimum bounding box.

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

    pub fn centroid(&self) -> Vec3 {
        (self.min + self.max) * 0.5
    }

    /// Slab test with a precomputed inverse ray direction.
    ///
    /// Returns the entry distance of the ray into the box, clamped to `tmin`,
    /// when the ray overlaps the box within `(tmin, tmax)`.
    #[inline]
    pub fn intersect(&self, origin: Vec3, inv_dir: Vec3, tmin: f64, tmax: f64) -> Option<f64> {
        let t0 = (self.min - origin) * inv_dir;
        let t1 = (self.max - origin) * inv_dir;

        let t_enter = tmin
            .max(t0.x.min(t1.x))
            .max(t0.y.min(t1.y))
            .max(t0.z.min(t1.z));
        let t_exit = tmax
            .min(t0.x.max(t1.x))
            .min(t0.y.max(t1.y))
            .min(t0.z.max(t1.z));

        if t_exit > t_enter {
            Some(t_enter)
        } else {
            None
        }
    }
}
