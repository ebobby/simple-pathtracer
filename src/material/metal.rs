use super::{ScatterUniforms, Scatterable, Scattered};
use crate::intersectable::Intersection;
use crate::ray::Ray;
use crate::Color;
use crate::Texture;
use crate::Vec3;

#[derive(Clone, Debug)]
pub struct Metal {
    pub albedo: Texture,
    pub fuzz: f64,
}

impl Scatterable for Metal {
    fn emit(&self, _u: f64, _v: f64, _p: Vec3) -> Color {
        Color::new(0.0, 0.0, 0.0)
    }

    fn scatter(
        &self,
        ray: &Ray,
        intersection: &Intersection,
        uniforms: ScatterUniforms,
    ) -> Option<Scattered> {
        let reflected = super::reflect(ray.direction.normalize(), intersection.normal);
        let fuzz = super::random_in_unit_ball(uniforms[0], uniforms[1], uniforms[2]);

        let scattered = Ray {
            origin: intersection.p,
            direction: reflected + (self.fuzz * fuzz),
        };

        if scattered.direction.dot(intersection.normal) > 0.0 {
            Some(Scattered {
                scattered,
                attenuation: self
                    .albedo
                    .value(intersection.u, intersection.v, intersection.p),
                pdf: None,
            })
        } else {
            None
        }
    }
}
