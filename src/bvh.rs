//! Bounding volume hierarchy.

use crate::aabb::AABB;
use crate::intersectable::{Intersectable, Intersection};
use crate::ray::Ray;
use crate::Hitable;

use rand::Rng;

#[derive(Debug)]
pub struct BVH {
    left: Option<Hitable>,
    right: Option<Hitable>,
    bounding_box: AABB,
}

impl BVH {
    pub fn from_vec(mut objects: Vec<Hitable>) -> Self {
        if objects.is_empty() {
            panic!("I need a non-empty object list!");
        }

        let mut rng = rand::thread_rng();
        let axis: usize = rng.gen_range(0..3);

        objects.sort_by(|a, b| {
            let a_box = a.bounding_box();
            let b_box = b.bounding_box();

            match axis {
                0 => a_box.min.x.partial_cmp(&b_box.min.x).unwrap(),
                1 => a_box.min.y.partial_cmp(&b_box.min.y).unwrap(),
                _ => a_box.min.z.partial_cmp(&b_box.min.z).unwrap(),
            }
        });

        let (left, right, bounding_box): (Option<Hitable>, Option<Hitable>, AABB) =
            match objects.len() {
                1 => {
                    let l = objects.remove(0);
                    let bb = l.bounding_box();

                    (Some(l), None, bb)
                }
                2 => {
                    let l = objects.remove(0);
                    let r = objects.remove(0);
                    let bb = AABB::surrounding(l.bounding_box(), r.bounding_box());

                    (Some(l), Some(r), bb)
                }
                size => {
                    let rest = objects.split_off(size / 2);

                    let l = Self::from_vec(objects);
                    let r = Self::from_vec(rest);

                    let bb = AABB::surrounding(l.bounding_box(), r.bounding_box());

                    (Some(Box::new(l)), Some(Box::new(r)), bb)
                }
            };

        Self {
            left,
            right,
            bounding_box,
        }
    }
}

impl Intersectable for BVH {
    fn bounding_box(&self) -> AABB {
        self.bounding_box
    }

    fn intersect(&self, ray: &Ray, min: f64, max: f64) -> Option<Intersection> {
        if !self.bounding_box.intersect(ray, min, max) {
            return None;
        }

        match (&self.left, &self.right) {
            // We have two nodes, check intersection on both.
            (Some(left), Some(right)) => {
                match (
                    left.intersect(ray, min, max),
                    right.intersect(ray, min, max),
                ) {
                    // We have two hits, return the closest one.
                    (Some(left_hit), Some(right_hit)) => {
                        if left_hit.t < right_hit.t {
                            Some(left_hit)
                        } else {
                            Some(right_hit)
                        }
                    }
                    (Some(left_hit), None) => Some(left_hit),
                    (None, Some(right_hit)) => Some(right_hit),
                    (None, None) => None,
                }
            }
            (Some(left), None) => left.intersect(ray, min, max),
            (None, Some(right)) => right.intersect(ray, min, max),
            (None, None) => None,
        }
    }
}
