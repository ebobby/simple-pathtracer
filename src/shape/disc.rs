use crate::intersectable::*;
use crate::aabb::AABB;
use crate::material::Material;
use crate::ray::Ray;
use crate::vector::Vec3;

#[derive(Debug)]
pub struct Disc {
    pub center: Vec3,
    pub normal: Vec3,
    pub radius: f64,
    pub material: Material,
}

impl Intersectable for Disc {
    fn bounding_box(&self) -> AABB {
        let corner = Vec3::new(self.radius, self.radius, self.radius);

        AABB {
            min: self.center - corner,
            max: self.center + corner,
        }
    }

    fn intersect(&self, ray: Ray, min: f64, max: f64) -> Option<Intersection> {
        let denom = self.normal.dot(ray.direction);

        if denom.abs() > std::f64::EPSILON {
            let v = self.center - ray.origin;

            let distance = v.dot(self.normal) / denom;

            let p = ray.origin + distance * ray.direction;
            let d = p - self.center;

            if d.norm() <= self.radius * self.radius && distance < max && distance > min {
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
