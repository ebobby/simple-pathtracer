//! Principled material: a metallic-roughness dielectric/metal with optional
//! transmission and emission, in the spirit of glTF's model.
//!
//! The BSDF is a mixture of two branches chosen stochastically:
//!
//! * the *opaque* branch (probability `1 - p_t`): GGX specular with Schlick
//!   Fresnel plus a Lambertian diffuse lobe weighted by `(1 - metallic)` and
//!   `(1 - F̄)`; it has a mixture pdf that `eval` reports exactly, so light
//!   sampling and MIS work on it;
//! * the *glass* branch (probability `p_t = (1 - metallic) transmission`):
//!   Fresnel-weighted GGX reflection or refraction, treated as a delta
//!   bounce for MIS.
//!
//! The GPU shader mirrors this.

use super::ggx;
use super::{ScatterUniforms, Scatterable, Scattered};
use crate::intersectable::Intersection;
use crate::ray::Ray;
use crate::{Color, Texture, Vec3};

use std::f64::consts::PI;

#[derive(Clone, Debug)]
pub struct Principled {
    pub base_color: Texture,
    pub metallic: f64,
    pub roughness: f64,
    pub transmission: f64,
    pub ior: f64,
    pub emission: Color,
}

impl Principled {
    /// A matte dielectric with the given base colour.
    pub fn new(base_color: Texture) -> Self {
        Self {
            base_color,
            metallic: 0.0,
            roughness: 0.5,
            transmission: 0.0,
            ior: 1.5,
            emission: Color::new(0.0, 0.0, 0.0),
        }
    }

    pub fn metallic(mut self, metallic: f64) -> Self {
        self.metallic = metallic.clamp(0.0, 1.0);
        self
    }

    pub fn roughness(mut self, roughness: f64) -> Self {
        self.roughness = roughness.clamp(0.0, 1.0);
        self
    }

    pub fn transmission(mut self, transmission: f64) -> Self {
        self.transmission = transmission.clamp(0.0, 1.0);
        self
    }

    pub fn ior(mut self, ior: f64) -> Self {
        self.ior = ior.max(1.0);
        self
    }

    pub fn emission(mut self, emission: Color) -> Self {
        self.emission = emission;
        self
    }

    /// GGX α, clamped so the distribution stays representable.
    pub fn alpha(&self) -> f64 {
        (self.roughness * self.roughness).max(ggx::MIN_ALPHA)
    }

    fn f0(&self, base: Color) -> Color {
        let d = ((self.ior - 1.0) / (self.ior + 1.0)).powi(2);
        Color::new(d, d, d) * (1.0 - self.metallic) + base * self.metallic
    }

    fn fresnel(f0: Color, cos: f64) -> Color {
        let w = (1.0 - cos.clamp(0.0, 1.0)).powi(5);
        f0 + (Color::new(1.0, 1.0, 1.0) + f0 * -1.0) * w
    }

    fn average(c: Color) -> f64 {
        (c.r + c.g + c.b) / 3.0
    }

    /// Probability of taking the glass branch.
    fn p_transmit(&self) -> f64 {
        (1.0 - self.metallic) * self.transmission
    }

    /// Probability of the specular lobe within the opaque branch; depends
    /// on `wo` only so `eval` can reproduce it.
    fn p_specular(&self, f0: Color, cos_o: f64) -> f64 {
        self.metallic + (1.0 - self.metallic) * Self::average(Self::fresnel(f0, cos_o))
    }

    /// Opaque-branch BRDF and its mixture pdf for unit `wo`, `wi` above the
    /// facing normal `n`, scaled by the branch probability so the result is
    /// the non-delta part of the full BSDF and its sampling density.
    fn eval_opaque(&self, wo: Vec3, wi: Vec3, n: Vec3, base: Color) -> Option<(Color, f64)> {
        let cos_o = wo.dot(n);
        let cos_i = wi.dot(n);
        if cos_o <= 0.0 || cos_i <= 0.0 {
            return None;
        }
        let alpha = self.alpha();
        let f0 = self.f0(base);
        let (spec, pdf_ggx) = ggx::eval(alpha, wo, wi, n)?;
        let h = (wo + wi).normalize();
        let fresnel_h = Self::fresnel(f0, wo.dot(h));
        let fresnel_avg = Self::average(Self::fresnel(f0, cos_o));
        let diffuse_weight = (1.0 - self.metallic) * (1.0 - fresnel_avg);

        let f = fresnel_h * spec + base * (diffuse_weight / PI);
        let p_spec = self.p_specular(f0, cos_o);
        let pdf = p_spec * pdf_ggx + (1.0 - p_spec) * cos_i / PI;

        let p_opaque = 1.0 - self.p_transmit();
        Some((f * p_opaque, pdf * p_opaque))
    }

    /// Sampling density of the non-delta lobes over the whole sphere (for
    /// tests): integrates to `1 - p_transmit`.
    pub fn pdf_over_sphere(&self, wo: Vec3, wi: Vec3, n: Vec3, base: Color) -> f64 {
        let cos_o = wo.dot(n);
        if cos_o <= 0.0 {
            return 0.0;
        }
        let p_spec = self.p_specular(self.f0(base), cos_o);
        let pdf = p_spec * ggx::pdf(self.alpha(), wo, wi, n)
            + (1.0 - p_spec) * wi.dot(n).max(0.0) / PI;
        pdf * (1.0 - self.p_transmit())
    }

    /// BSDF value and pdf for light sampling (non-delta lobes only).
    pub fn eval(&self, wo: Vec3, wi: Vec3, intersection: &Intersection) -> Option<(Color, f64)> {
        let n = intersection.facing_normal(wo);
        let base = self
            .base_color
            .value(intersection.u, intersection.v, intersection.p);
        self.eval_opaque(wo, wi, n, base)
    }

    /// Per-channel transmittance after travelling `distance` inside the
    /// object (Beer-Lambert with `base_color` as transmittance per unit).
    pub fn transmittance(&self, distance: f64, intersection: &Intersection) -> Color {
        let base = self
            .base_color
            .value(intersection.u, intersection.v, intersection.p);
        Color::new(
            base.r.max(0.0).powf(distance),
            base.g.max(0.0).powf(distance),
            base.b.max(0.0).powf(distance),
        )
    }
}

impl Scatterable for Principled {
    fn emit(&self, _u: f64, _v: f64, _p: Vec3) -> Color {
        self.emission
    }

    fn scatter(
        &self,
        ray: &Ray,
        intersection: &Intersection,
        u: ScatterUniforms,
    ) -> Option<Scattered> {
        let n_geom = intersection.normal;
        let wo = -ray.direction.normalize();
        let entering = wo.dot(n_geom) > 0.0;
        let n = if entering { n_geom } else { -n_geom };
        let cos_o = wo.dot(n);
        if cos_o <= 0.0 {
            return None;
        }

        let base = self
            .base_color
            .value(intersection.u, intersection.v, intersection.p);
        let alpha = self.alpha();
        let f0 = self.f0(base);

        let (tangent, bitangent) = n.orthonormal_basis();
        let to_world = |v: Vec3| tangent * v.x + bitangent * v.y + n * v.z;
        let wo_local = Vec3::new(wo.dot(tangent), wo.dot(bitangent), cos_o);

        let p_transmit = self.p_transmit();
        let mut choice = u[2];

        if choice < p_transmit {
            // Glass branch: Fresnel reflection or refraction about a
            // sampled microfacet normal, treated as delta for MIS.
            choice /= p_transmit;
            let h = to_world(ggx::sample_visible_normal(wo_local, alpha, u[0], u[1]));
            let cos_oh = wo.dot(h);
            let fresnel = Self::fresnel(f0, cos_oh);
            let fresnel_avg = Self::average(fresnel);
            let eta = if entering { 1.0 / self.ior } else { self.ior };
            let sin2_t = eta * eta * (1.0 - cos_oh * cos_oh);
            let total_internal_reflection = sin2_t >= 1.0;

            if total_internal_reflection || choice < fresnel_avg {
                let wi = h * (2.0 * cos_oh) - wo;
                let cos_i = wi.dot(n);
                if cos_i <= 0.0 {
                    return None;
                }
                let weight = ggx::g2(cos_o, cos_i, alpha) / ggx::g1(cos_o, alpha);
                let tint = if total_internal_reflection {
                    Color::new(1.0, 1.0, 1.0)
                } else {
                    fresnel / fresnel_avg
                };
                return Some(Scattered {
                    scattered: Ray {
                        origin: intersection.p,
                        direction: wi,
                    },
                    attenuation: tint * weight,
                    pdf: None,
                });
            }

            let cos_t = (1.0 - sin2_t).sqrt();
            let wi = -wo * eta + h * (eta * cos_oh - cos_t);
            let cos_i = -wi.dot(n);
            if cos_i <= 0.0 {
                return None;
            }
            let weight = ggx::g2(cos_o, cos_i, alpha) / ggx::g1(cos_o, alpha);
            let tint = (Color::new(1.0, 1.0, 1.0) + fresnel * -1.0) / (1.0 - fresnel_avg);
            return Some(Scattered {
                scattered: Ray {
                    origin: intersection.p,
                    direction: wi,
                },
                attenuation: tint * weight,
                pdf: None,
            });
        }

        // Opaque branch: specular or diffuse lobe, weighted by f cos / pdf.
        choice = (choice - p_transmit) / (1.0 - p_transmit);
        let p_spec = self.p_specular(f0, cos_o);
        let wi = if choice < p_spec {
            let h = to_world(ggx::sample_visible_normal(wo_local, alpha, u[0], u[1]));
            h * (2.0 * wo.dot(h)) - wo
        } else {
            super::random_cosine_direction(n, u[3], u[4])
        };
        let cos_i = wi.dot(n);
        if cos_i <= 0.0 {
            return None;
        }
        let (f, pdf) = self.eval_opaque(wo, wi, n, base)?;
        if pdf <= 0.0 {
            return None;
        }

        Some(Scattered {
            scattered: Ray {
                origin: intersection.p,
                direction: wi,
            },
            attenuation: f * (cos_i / pdf),
            pdf: Some(pdf),
        })
    }
}
