use crate::color::Color;
use crate::intersectable::Intersection;
use crate::ray::Ray;
use crate::vector::Vec3;

#[derive(Clone, Copy, Debug)]
pub enum Material {
    None,
    Lambertian(Color),
    Metal(Color, f64),
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

impl Material {
    pub fn scatter(&self, _ray: Ray, intersection: &Intersection) -> Option<Scattered> {
        match self {
            Material::None => None,
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
        }
    }
}
