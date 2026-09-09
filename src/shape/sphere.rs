use crate::aabb::AABB;
use crate::intersectable::*;
use crate::ray::Ray;
use crate::Material;
use crate::Vec3;

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

    fn intersect(&self, ray: &Ray, min: f64, max: f64) -> Option<Intersection<'_>> {
        let oc = ray.origin - self.center;
        let a = ray.direction.norm();
        let half_b = oc.dot(ray.direction);
        let c = oc.norm() - self.radius * self.radius;

        let discriminant = half_b * half_b - a * c;
        if discriminant <= 0.0 {
            return None;
        }

        let sqrtd = discriminant.sqrt();
        let inv_a = a.recip();

        let mut t = (-half_b - sqrtd) * inv_a;
        if t <= min || t >= max {
            t = (-half_b + sqrtd) * inv_a;
            if t <= min || t >= max {
                return None;
            }
        }

        let p = ray.point_at(t);
        let normal = (p - self.center) / self.radius;
        let (u, v) = sphere_texture_uv(normal);

        Some(Intersection {
            p,
            t,
            u,
            v,
            normal,
            material: &self.material,
        })
    }
}

fn sphere_texture_uv(p: Vec3) -> (f64, f64) {
    let phi = p.z.atan2(p.x);
    let theta = p.y.asin();
    let u = 1.0 - (phi + std::f64::consts::PI) / (2.0 * std::f64::consts::PI);
    let v = (theta + std::f64::consts::PI / 2.0) / std::f64::consts::PI;

    (u, v)
}
