use crate::aabb::AABB;
use crate::intersectable::*;
use crate::material::Material;
use crate::ray::Ray;
use crate::vector::Vec3;

#[derive(Debug)]
pub struct Sphere {
    pub center: Vec3,
    pub radius: f64,
    pub material: Material,
}

impl Intersectable for Sphere {
    fn bounding_box(&self) -> AABB {
        let corner = Vec3::new(self.radius, self.radius, self.radius);

        AABB {
            min: self.center - corner,
            max: self.center + corner,
        }
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
