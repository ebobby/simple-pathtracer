use crate::color::Color;
use crate::intersectable::Intersection;
use crate::ray::Ray;
use crate::vector::Vec3;

/// Material object.
///
/// # Notes
/// Even though by convention all color components are assumed to be between 0.0
/// and 1.0 and they're clamped when converted to `Rgb` it doens't mean they
/// can't be declared to have larger values if needed to. This is usually the
/// case for light intensity.
#[derive(Clone, Copy, Debug)]
pub enum Material {
    Lambertian(Color),
    Dielectric(Color, f64),
    Metal(Color, f64),
    DiffuseLight(Color),
}

#[derive(Debug)]
pub struct Scattered {
    pub scattered: Ray,
    pub attenuation: Color,
}

fn random_in_unit_sphere() -> Vec3 {
    let u = rand::random::<f64>();
    let v = rand::random::<f64>();
    let theta = u * 2.0 * std::f64::consts::PI;
    let phi = (2.0 * v - 1.0).acos();
    let r = rand::random::<f64>().cbrt();
    let sin_theta = theta.sin();
    let cos_theta = theta.cos();
    let sin_phi = phi.sin();
    let cos_phi = phi.cos();

    Vec3::new(r * sin_phi * cos_theta, r * sin_phi * sin_theta, cos_phi)
}

fn refract(v: Vec3, n: Vec3, ni_over_nt: f64) -> Option<Vec3> {
    let uv = v.normalize();
    let dt = uv.dot(n);

    let discriminant = 1.0 - ni_over_nt.powi(2) * (1.0 - dt.powi(2));

    if discriminant > 0.0 {
        Some(ni_over_nt * (uv - n * dt) - n * discriminant.sqrt())
    } else {
        None
    }
}

fn schlick(cosine: f64, ref_idx: f64) -> f64 {
    let r0 = ((1.0 - ref_idx) / (1.0 + ref_idx)).powi(2);

    r0 + (1.0 - r0) * (1.0 - cosine).powi(5)
}

impl Material {
    pub fn emit(&self) -> Color {
        if let Material::DiffuseLight(color) = self {
            *color
        } else {
            Color::black()
        }
    }

    pub fn scatter(&self, ray: Ray, intersection: &Intersection) -> Option<Scattered> {
        match self {
            Material::DiffuseLight(_) => None,
            Material::Lambertian(albedo) => {
                let target = intersection.p + intersection.normal + random_in_unit_sphere();

                let scattered = Ray {
                    origin: intersection.p,
                    direction: target - intersection.p,
                };

                Some(Scattered {
                    scattered,
                    attenuation: *albedo,
                })
            }
            Material::Metal(albedo, fuzz) => {
                let reflected = ray.direction.normalize().reflect(intersection.normal);

                let scattered = Ray {
                    origin: intersection.p,
                    direction: reflected + (*fuzz * random_in_unit_sphere()),
                };

                if scattered.direction.dot(intersection.normal) > 0.0 {
                    Some(Scattered {
                        scattered,
                        attenuation: *albedo,
                    })
                } else {
                    None
                }
            }
            Material::Dielectric(color, refractive_index) => {
                let ref_idx = *refractive_index;
                let mut refracted = Vec3::zero();
                let outward_normal;
                let scattered;

                let ni_over_nt;
                let reflect_probability;
                let cosine;

                let attenuation = *color;

                let reflected = ray.direction.reflect(intersection.normal);

                let d = ray.direction.dot(intersection.normal);

                if d > 0.0 {
                    outward_normal = -intersection.normal;
                    ni_over_nt = ref_idx;
                    cosine = ref_idx * d / ray.direction.length();
                } else {
                    outward_normal = intersection.normal;
                    ni_over_nt = ref_idx.recip();
                    cosine = -d / ray.direction.length();
                }

                if let Some(r) = refract(ray.direction, outward_normal, ni_over_nt) {
                    refracted = r;
                    reflect_probability = schlick(cosine, ref_idx);
                } else {
                    reflect_probability = 1.0;
                }

                if rand::random::<f64>() < reflect_probability {
                    scattered = Ray {
                        origin: intersection.p.correct(reflected.normalize()),
                        direction: reflected,
                    };
                } else {
                    scattered = Ray {
                        origin: intersection.p.correct(refracted.normalize()),
                        direction: refracted,
                    };
                }

                Some(Scattered {
                    scattered,
                    attenuation,
                })
            }
        }
    }
}
