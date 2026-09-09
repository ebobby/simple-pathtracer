use super::{ScatterUniforms, Scatterable, Scattered};
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

    fn scatter(
        &self,
        ray: &Ray,
        intersection: &Intersection,
        uniforms: ScatterUniforms,
    ) -> Option<Scattered> {
        let normal = intersection.facing_normal(-ray.direction);
        let direction = super::random_cosine_direction(normal, uniforms[0], uniforms[1]);
        let pdf = direction.dot(normal).max(0.0) / std::f64::consts::PI;

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
