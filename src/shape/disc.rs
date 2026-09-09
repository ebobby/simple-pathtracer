use crate::aabb::AABB;
use crate::intersectable::*;
use crate::light::LightShape;
use crate::ray::Ray;
use crate::Material;
use crate::Vec3;

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

    fn intersect(&self, ray: &Ray, min: f64, max: f64) -> Option<Intersection<'_>> {
        let denom = self.normal.dot(ray.direction);

        if denom.abs() > std::f64::EPSILON {
            let v = self.center - ray.origin;

            let distance = v.dot(self.normal) / denom;

            let p = ray.origin + distance * ray.direction;
            let d = p - self.center;

            if d.norm() < self.radius * self.radius && distance < max && distance > min {
                let (tangent, bitangent) = self.normal.orthonormal_basis();
                let diameter = 2.0 * self.radius;

                Some(Intersection {
                    t: distance,
                    p,
                    u: 0.5 + d.dot(tangent) / diameter,
                    v: 0.5 + d.dot(bitangent) / diameter,
                    normal: self.normal,
                    material: &self.material,
                    shape_id: 0,
                })
            } else {
                None
            }
        } else {
            None
        }
    }

    fn as_light(&self) -> Option<(LightShape, crate::Color)> {
        if self.material.is_emissive() {
            let emission = self.material.emit(0.5, 0.5, self.center);
            Some((LightShape::Disc {
                center: self.center,
                normal: self.normal,
                radius: self.radius,
            }, emission))
        } else {
            None
        }
    }
}
