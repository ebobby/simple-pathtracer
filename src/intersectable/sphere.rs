use super::Intersectable;
use super::Intersection;
use crate::aabb::Aabb;
use crate::material::Material;
use crate::ray::Ray;
use crate::vector::Vec3;

#[derive(Debug)]
pub struct Sphere {
    center: Vec3,
    radius: f64,
    material: Material,
    bounding_box: Aabb,
}

impl Sphere {
    pub fn new(center: Vec3, radius: f64, material: Material) -> Sphere {
        Sphere {
            center,
            radius,
            material,
            bounding_box: sphere_bounding_box(center, radius),
        }
    }
}

impl Intersectable for Sphere {
    fn bounding_box(&self) -> Option<Aabb> {
        Some(self.bounding_box)
    }

    fn intersect(&self, ray: Ray, min: f64, max: f64) -> Option<Intersection> {
        let oc = ray.origin - self.center;
        let a = ray.direction.dot(ray.direction);
        let b = 2.0 * oc.dot(ray.direction);
        let c = oc.dot(oc) - self.radius * self.radius;

        let discriminant = b * b - 4.0 * a * c;

        let divisor = (2.0 * a).recip();
        let dis_sqrt = discriminant.sqrt();

        if discriminant >= 0.0 {
            let t0 = (-b - dis_sqrt) * divisor;
            let t1 = (-b + dis_sqrt) * divisor;

            if t0 < max && t0 > min {
                let p = ray.origin + t0 * ray.direction;

                return Some(Intersection {
                    t: t0,
                    p,
                    normal: (p - self.center) / self.radius,
                    material: self.material,
                });
            }

            if t1 < max && t1 > min {
                let p = ray.origin + t1 * ray.direction;

                return Some(Intersection {
                    t: t1,
                    p,
                    normal: (p - self.center) / self.radius,
                    material: self.material,
                });
            }
        }

        None
    }
}

fn sphere_bounding_box(center: Vec3, radius: f64) -> Aabb {
    let corner = Vec3::new(radius, radius, radius);

    Aabb::new(center - corner, center + corner)
}
