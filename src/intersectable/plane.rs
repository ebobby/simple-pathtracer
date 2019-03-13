use super::Intersectable;
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

    fn intersect(&self, ray: Ray, min: f64, max: f64) -> Option<f64> {
        let denom = self.normal.dot(ray.direction);

        if denom.abs() > std::f64::EPSILON {
            let v = self.point - ray.origin;

            let distance = v.dot(self.normal) / denom;

            if distance < max && distance > min {
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
