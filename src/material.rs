use crate::color::Color;
use crate::intersectable::Intersection;
use crate::ray::Ray;
use crate::vector::Vec3;

#[derive(Clone, Copy, Debug)]
pub enum Material {
    None,
    Lambertian(Color),
    Isotropic(Color),
    Dielectric(f64),
    Metal(Color, f64),
    DiffuseLight(Color),
}

#[derive(Debug)]
pub struct Scattered {
    pub scattered: Ray,
    pub attenuation: Color,
}

fn random_in_unit_sphere() -> Vec3 {
    let one = Vec3::new(1., 1., 1.);

    loop {
        let p = (Vec3::new(
            rand::random::<f64>(),
            rand::random::<f64>(),
            rand::random::<f64>(),
        ) * 2.0)
            - one;
        if p.norm() >= 1.0 {
            break p;
        }
    }
}
fn refract(v: Vec3, n: Vec3, ni_over_nt: f64) -> Option<Vec3> {
    let uv = v.normalize();
    let dt = uv.dot(n);

    let discriminant = 1.0 - ni_over_nt * ni_over_nt * (1.0 - dt * dt);

    if discriminant > 0.0 {
        Some((uv - n * dt) * ni_over_nt - n * discriminant.sqrt())
    } else {
        None
    }
}

fn schlick(cosine: f64, refractive_index: f64) -> f64 {
    let mut r0 = (1.0 - refractive_index) / (1.0 + refractive_index);

    r0 = r0 * r0;

    r0 + (1.0 - r0) * (1.0 - cosine).powf(5.0)
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
            Material::None => None,
            Material::DiffuseLight(_) => None,
            Material::Lambertian(albedo) => {
                let target = intersection.p + intersection.normal + random_in_unit_sphere();
                let scattered = Ray {
                    origin: intersection.p,
                    direction: (target - intersection.p).normalize(),
                };

                Some(Scattered {
                    scattered,
                    attenuation: *albedo,
                })
            }
            Material::Metal(albedo, fuzz) => {
                let reflected = ray.direction.reflect(intersection.normal);

                let scattered = Ray {
                    origin: ray.origin,
                    direction: reflected + (random_in_unit_sphere() * (*fuzz)),
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
            Material::Isotropic(albedo) => {
                let scattered = Ray {
                    origin: intersection.p,
                    direction: random_in_unit_sphere(),
                };

                Some(Scattered {
                    scattered,
                    attenuation: *albedo,
                })
            },
            Material::Dielectric(refractive_index) => {
                let mut refracted = Vec3::zero();
                let outward_normal;
                let scattered;

                let ni_over_nt;
                let reflect_probability;
                let cosine;

                let attenuation = Color::white();

                let reflected = ray.direction.reflect(intersection.normal);

                if ray.direction.dot(intersection.normal) > 0.0 {
                    outward_normal = -intersection.normal;
                    ni_over_nt = *refractive_index;
                    cosine = refractive_index * ray.direction.dot(intersection.normal)
                        / ray.direction.length();
                } else {
                    outward_normal = intersection.normal;
                    ni_over_nt = refractive_index.recip();
                    cosine = -ray.direction.dot(intersection.normal) / ray.direction.length();
                }

                if let Some(r) = refract(ray.direction, outward_normal, ni_over_nt) {
                    reflect_probability = schlick(cosine, *refractive_index);
                    refracted = r;
                } else {
                    reflect_probability = 1.0;
                }

                if rand::random::<f64>() < reflect_probability {
                    scattered = Ray {
                        origin: intersection.p,
                        direction: reflected.normalize(),
                    };
                }
                else {
                    scattered = Ray {
                        origin: intersection.p,
                        direction: refracted.normalize(),
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
