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

impl Material {
    pub fn emit(&self) -> Color {
        if let Material::DiffuseLight(color) = self {
            *color
        } else {
            Color::new(0.0, 0.0, 0.0)
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
                let reflected = reflect(ray.direction.normalize(), intersection.normal);

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
                let attenuation = *color;
                let reflected = reflect(ray.direction.normalize(), intersection.normal);

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
                        1.0/ref_idx,
                        -1.0 * d / ray.direction.length(),
                    )
                };

                let (refracted, reflect_probability) =
                    if let Some(r) = refract(ray.direction, outward_normal, ni_over_nt) {
                        (r, schlick(cosine, ref_idx))
                    } else {
                        (Vec3::zero(), 1.0)
                    };

                let scattered = if rand::random::<f64>() < reflect_probability {
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
    }
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

fn reflect(v: Vec3, n: Vec3) -> Vec3 {
    v - 2.0 * v.dot(n) * n
}

fn refract(v: Vec3, n: Vec3, ni_over_nt: f64) -> Option<Vec3> {
    let uv = v.normalize();
    let dt = uv.dot(n);

    let discriminant = 1.0 - ni_over_nt*ni_over_nt * (1.0 - dt*dt);

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
