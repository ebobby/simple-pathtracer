use crate::color::Color;
use crate::intersectable::Intersection;
use crate::ray::Ray;
use crate::vector::Vec3;

#[derive(Clone, Copy, Debug)]
pub enum Material {
    None,
    Lambertian(Color),
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
        let p = Vec3::new(
            rand::random::<f64>(),
            rand::random::<f64>(),
            rand::random::<f64>(),
        ) * 2.0
            - one;
        if p.norm() >= 1.0 {
            break p;
        }
    }
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
            Material::None => None,
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
            Material::Dielectric(refractive_index) => {
                let ref_idx = *refractive_index;
                let mut refracted = Vec3::zero();
                let outward_normal;
                let scattered;

                let ni_over_nt;
                let reflect_probability;
                let cosine;

                let attenuation = Color::white();

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
