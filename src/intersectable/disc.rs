use super::Intersectable;
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
    fn intersect(&self, ray: Ray, min: f64, max: f64) -> Option<f64> {
        let denom = self.normal.dot(ray.direction);

        if denom.abs() > std::f64::EPSILON {
            let v = self.center - ray.origin;

            let distance = v.dot(self.normal) / denom;

            let p = ray.origin + distance * ray.direction;
            let d = p - self.center;

            if d.norm() <= self.radius * self.radius && distance < max && distance > min {
                Some(distance)
            } else {
                None
            }
        } else {
            None
        }
    }

    fn material(&self) -> Material {
        self.material
    }

    fn normal(&self, _point: Vec3) -> Vec3 {
        self.normal
    }
}
