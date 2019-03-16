use super::{Scatterable, Scattered};
use crate::color::Color;
use crate::intersectable::Intersection;
use crate::ray::Ray;

#[derive(Clone, Copy, Debug)]
pub struct Lambertian {
    pub albedo: Color,
}

impl Scatterable for Lambertian {
    fn emit(&self) -> Color {
        Color::new(0.0, 0.0, 0.0)
    }

    fn scatter(&self, _ray: Ray, intersection: &Intersection) -> Option<Scattered> {
        let target = intersection.p + intersection.normal + super::random_in_unit_sphere();

        let scattered = Ray {
            origin: intersection.p,
            direction: target - intersection.p,
        };

        Some(Scattered {
            scattered,
            attenuation: self.albedo,
        })
    }
}
