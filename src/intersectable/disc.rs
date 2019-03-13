use super::Intersectable;
use super::Intersection;
use crate::aabb::Aabb;
use crate::material::Material;
use crate::ray::Ray;
use crate::vector::Vec3;

#[derive(Debug)]
pub struct Disc {
    center: Vec3,
    normal: Vec3,
    radius: f64,
    material: Material,
    bounding_box: Aabb,
}

impl Disc {
    pub fn new(center: Vec3, radius: f64, normal: Vec3, material: Material) -> Disc {
        Disc {
            center,
            radius,
            normal,
            material,
            bounding_box: disc_bounding_box(center, radius),
        }
    }
}

impl Intersectable for Disc {
    fn bounding_box(&self) -> Option<Aabb> {
        Some(self.bounding_box)
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

fn disc_bounding_box(center: Vec3, radius: f64) -> Aabb {
    let corner = Vec3::new(radius, radius, radius);

    Aabb::new(center - corner, center + corner)
}
