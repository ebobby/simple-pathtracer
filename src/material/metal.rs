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

    /// `fuzz` is the GGX roughness α. Below `ggx::MIN_ALPHA` the metal is a
    /// perfect mirror (a delta reflection with no pdf).
    fn scatter(
        &self,
        ray: &Ray,
        intersection: &Intersection,
        uniforms: ScatterUniforms,
    ) -> Option<Scattered> {
        let wo = -ray.direction.normalize();
        let normal = intersection.facing_normal(wo);
        let albedo = self
            .albedo
            .value(intersection.u, intersection.v, intersection.p);

        if self.fuzz < super::ggx::MIN_ALPHA {
            let reflected = super::reflect(-wo, normal);
            if reflected.dot(normal) <= 0.0 {
                return None;
            }
            return Some(Scattered {
                scattered: Ray {
                    origin: intersection.p,
                    direction: reflected,
                },
                attenuation: albedo,
                pdf: None,
            });
        }

        let cos_o = wo.dot(normal);
        if cos_o <= 0.0 {
            return None;
        }

        // Sample a visible microfacet normal in the local frame and reflect
        let (tangent, bitangent) = normal.orthonormal_basis();
        let wo_local = Vec3::new(wo.dot(tangent), wo.dot(bitangent), cos_o);
        let h_local = super::ggx::sample_visible_normal(wo_local, self.fuzz, uniforms[0], uniforms[1]);
        let h = tangent * h_local.x + bitangent * h_local.y + normal * h_local.z;
        let wi = h * (2.0 * wo.dot(h)) - wo;

        let cos_i = wi.dot(normal);
        if cos_i <= 0.0 {
            return None;
        }
        let (f, pdf) = super::ggx::eval(self.fuzz, wo, wi, normal)?;

        Some(Scattered {
            scattered: Ray {
                origin: intersection.p,
                direction: wi,
            },
            // f * cos / pdf, which for visible-normal sampling is G2 / G1
            attenuation: albedo * (f * cos_i / pdf),
            pdf: Some(pdf),
        })
    }
}
