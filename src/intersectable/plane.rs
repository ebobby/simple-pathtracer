use super::Intersectable;
use super::Intersection;
use crate::aabb::Aabb;
use crate::material::Material;
use crate::ray::Ray;
use crate::vector::Vec3;

#[derive(Debug)]
pub struct Plane {
    point: Vec3,
    normal: Vec3,
    material: Material,
}

impl Plane {
    pub fn new(point: Vec3, normal: Vec3, material: Material) -> Plane {
        Plane {
            point,
            normal,
            material,
        }
    }
}

impl Intersectable for Plane {
    fn bounding_box(&self) -> Option<Aabb> {
        None
    }

    fn intersect(&self, ray: Ray, min: f64, max: f64) -> Option<Intersection> {
        let denom = self.normal.dot(ray.direction);

        if denom.abs() > std::f64::EPSILON {
            let v = self.point - ray.origin;

            let distance = v.dot(self.normal) / denom;

            if distance < max && distance > min {
                Some(Intersection {
                    t: distance,
                    p: ray.origin + distance * ray.direction,
                    normal: self.normal,
                    material: self.material,
                })
            } else {
                None
            }
        } else {
            None
        }
    }
}
