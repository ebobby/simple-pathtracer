use super::{Scatterable, Scattered};
use crate::color::Color;
use crate::intersectable::Intersection;
use crate::ray::Ray;

#[derive(Clone, Copy, Debug)]
pub struct Metal {
    pub albedo: Color,
    pub fuzz: f64,
}

impl Scatterable for Metal {
    fn emit(&self) -> Color {
        Color::new(0.0, 0.0, 0.0)
    }

    fn scatter(&self, ray: Ray, intersection: &Intersection) -> Option<Scattered> {
        let reflected = super::reflect(ray.direction.normalize(), intersection.normal);

        let scattered = Ray {
            origin: intersection.p,
            direction: reflected + (self.fuzz * super::random_in_unit_sphere()),
        };

        if scattered.direction.dot(intersection.normal) > 0.0 {
            Some(Scattered {
                scattered,
                attenuation: self.albedo,
            })
        } else {
            None
        }
    }
}
