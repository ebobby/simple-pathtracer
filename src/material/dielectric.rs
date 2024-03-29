use super::{Scatterable, Scattered};
use crate::intersectable::Intersection;
use crate::ray::Ray;
use crate::rng;
use crate::Color;
use crate::Texture;
use crate::Vec3;

#[derive(Clone, Debug)]
pub struct Dielectric {
    pub attenuation: Texture,
    pub refractive_index: f64,
}

fn schlick(cosine: f64, ref_idx: f64) -> f64 {
    let r0 = ((1.0 - ref_idx) / (1.0 + ref_idx)).powi(2);

    r0 + (1.0 - r0) * (1.0 - cosine).powi(5)
}

impl Scatterable for Dielectric {
    fn emit(&self, _u: f64, _v: f64, _p: Vec3) -> Color {
        Color::new(0.0, 0.0, 0.0)
    }

    fn scatter(&self, ray: &Ray, intersection: &Intersection) -> Option<Scattered> {
        let ref_idx = self.refractive_index;
        let attenuation = self
            .attenuation
            .value(intersection.u, intersection.v, intersection.p);
        let reflected = super::reflect(ray.direction.normalize(), intersection.normal);

        let d = ray.direction.dot(intersection.normal);

        let (outward_normal, ni_over_nt, cosine) = if d > 0.0 {
            (
                intersection.normal * -1.0,
                ref_idx,
                ref_idx * d / ray.direction.length(),
            )
        } else {
            (
                intersection.normal,
                1.0 / ref_idx,
                -1.0 * d / ray.direction.length(),
            )
        };

        let (refracted, reflect_probability) =
            if let Some(r) = super::refract(ray.direction, outward_normal, ni_over_nt) {
                (r, schlick(cosine, ref_idx))
            } else {
                (Vec3::zero(), 1.0)
            };

        let scattered = if rng::get_random_number() < reflect_probability {
            Ray {
                origin: intersection.p,
                direction: reflected,
            }
        } else {
            Ray {
                origin: intersection.p,
                direction: refracted,
            }
        };

        Some(Scattered {
            scattered,
            attenuation,
        })
    }
}
