use super::Intersectable;
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
    fn intersect(&self, ray: Ray, min: f64, max: f64) -> Option<f64> {
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
                return Some(t0);
            }

            if t1 < max && t1 > min {
                return Some(t1);
            }
        }

        None
    }

    fn material(&self) -> Material {
        self.material
    }

    fn normal(&self, point: Vec3) -> Vec3 {
        (point - self.center) / self.radius
    }
}
