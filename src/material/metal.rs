use super::{Scatterable, Scattered};
use crate::intersectable::Intersection;
use crate::ray::Ray;
use crate::Color;
use crate::Texture;

#[derive(Clone, Copy, Debug)]
pub struct Metal {
    pub albedo: Texture,
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
                attenuation: self
                    .albedo
                    .value(intersection.u, intersection.v, intersection.p),
            })
        } else {
            None
        }
    }
}
