use crate::intersectable::Intersection;
use crate::ray::Ray;
use crate::Vec3;
use crate::Color;
use crate::Texture;

mod dielectric;
mod diffuse_light;
mod lambertian;
mod metal;

use dielectric::Dielectric;
use diffuse_light::DiffuseLight;
use lambertian::Lambertian;
use metal::Metal;

/// Material object.
///
/// # Notes
/// Even though by convention all color components are assumed to be between 0.0
/// and 1.0 and they're clamped when converted to `Rgb` it doens't mean they
/// can't be declared to have larger values if needed to. This is usually the
/// case for light intensity.
#[derive(Clone, Copy, Debug)]
pub enum Material {
    Lambertian(Lambertian),
    Dielectric(Dielectric),
    Metal(Metal),
    DiffuseLight(DiffuseLight),
}

#[derive(Debug)]
pub struct Scattered {
    pub scattered: Ray,
    pub attenuation: Color,
}

pub trait Scatterable {
    fn emit(&self) -> Color;
    fn scatter(&self, ray: Ray, intersection: &Intersection) -> Option<Scattered>;
}

impl Material {
    pub fn lambertian(albedo: Texture) -> Material {
        Material::Lambertian(Lambertian { albedo })
    }

    pub fn metal(albedo: Texture, fuzz: f64) -> Material {
        Material::Metal(Metal { albedo, fuzz })
    }

    pub fn dielectric(attenuation: Texture, refractive_index: f64) -> Material {
        Material::Dielectric(Dielectric {
            attenuation,
            refractive_index,
        })
    }

    pub fn diffuse_light(color: Color) -> Material {
        Material::DiffuseLight(DiffuseLight { color })
    }

    pub fn emit(&self) -> Color {
        match *self {
            Material::DiffuseLight(light) => light.emit(),
            _ => Color::new(0.0, 0.0, 0.0),
        }
    }

    pub fn scatter(&self, ray: Ray, intersection: &Intersection) -> Option<Scattered> {
        match self {
            Material::Lambertian(lambertian) => lambertian.scatter(ray, intersection),
            Material::Metal(metal) => metal.scatter(ray, intersection),
            Material::Dielectric(dielectric) => dielectric.scatter(ray, intersection),
            Material::DiffuseLight(diffuse_light) => diffuse_light.scatter(ray, intersection),
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

    let discriminant = 1.0 - ni_over_nt * ni_over_nt * (1.0 - dt * dt);

    if discriminant > 0.0 {
        Some(ni_over_nt * (uv - n * dt) - n * discriminant.sqrt())
    } else {
        None
    }
}
