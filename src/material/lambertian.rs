use super::{Scatterable, Scattered};
use crate::intersectable::Intersection;
use crate::ray::Ray;
use crate::Color;
use crate::Texture;
use crate::Vec3;

#[derive(Clone, Debug)]
pub struct Lambertian {
    pub albedo: Texture,
}

impl Scatterable for Lambertian {
    fn emit(&self, _u: f64, _v: f64, _p: Vec3) -> Color {
        Color::new(0.0, 0.0, 0.0)
    }

    fn scatter(&self, _ray: &Ray, intersection: &Intersection) -> Option<Scattered> {
        let direction = super::random_cosine_direction(intersection.normal);
        let pdf = direction.dot(intersection.normal).max(0.0) / std::f64::consts::PI;

        let scattered = Ray {
            origin: intersection.p,
            direction,
        };

        Some(Scattered {
            scattered,
            attenuation: self
                .albedo
                .value(intersection.u, intersection.v, intersection.p),
            pdf: Some(pdf),
        })
    }
}
