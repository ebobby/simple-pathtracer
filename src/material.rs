use crate::intersectable::Intersection;
use crate::ray::Ray;
use crate::rng;
use crate::Color;
use crate::Texture;
use crate::Vec3;

mod dielectric;
mod diffuse_light;
mod lambertian;
mod metal;

pub use dielectric::Dielectric;
pub use diffuse_light::DiffuseLight;
pub use lambertian::Lambertian;
pub use metal::Metal;

/// Material object.
///
/// # Notes
/// Even though by convention all color components are assumed to be between 0.0
/// and 1.0 and they're clamped when converted to `Rgb` it doens't mean they
/// can't be declared to have larger values if needed to. This is usually the
/// case for light intensity.
#[derive(Clone, Debug)]
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
    fn emit(&self, u: f64, v: f64, p: Vec3) -> Color;
    fn scatter(&self, ray: &Ray, intersection: &Intersection) -> Option<Scattered>;
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

    pub fn diffuse_light(texture: Texture) -> Material {
        Material::DiffuseLight(DiffuseLight { texture })
    }

    pub fn emit(&self, u: f64, v: f64, p: Vec3) -> Color {
        match self {
            Material::DiffuseLight(light) => light.emit(u, v, p),
            _ => Color::new(0.0, 0.0, 0.0),
        }
    }

    pub fn scatter(&self, ray: &Ray, intersection: &Intersection) -> Option<Scattered> {
        match self {
            Material::Lambertian(lambertian) => lambertian.scatter(ray, intersection),
            Material::Metal(metal) => metal.scatter(ray, intersection),
            Material::Dielectric(dielectric) => dielectric.scatter(ray, intersection),
            Material::DiffuseLight(diffuse_light) => diffuse_light.scatter(ray, intersection),
        }
    }
}

/// Generate a random point inside a unit sphere using rejection sampling.
/// This is faster than the spherical coordinate approach as it avoids
/// expensive acos() and cbrt() calls.
#[inline]
fn random_in_unit_sphere() -> Vec3 {
    loop {
        let x = 2.0 * rng::get_random_number() - 1.0;
        let y = 2.0 * rng::get_random_number() - 1.0;
        let z = 2.0 * rng::get_random_number() - 1.0;
        if x * x + y * y + z * z <= 1.0 {
            return Vec3::new(x, y, z);
        }
    }
}

/// Cosine-weighted direction on the hemisphere around the unit `normal`.
/// Uses the same orthonormal basis construction as the GPU shader.
#[inline]
pub(crate) fn random_cosine_direction(normal: Vec3) -> Vec3 {
    let r1 = rng::get_random_number();
    let r2 = rng::get_random_number();

    let phi = 2.0 * std::f64::consts::PI * r1;
    let sqrt_r2 = r2.sqrt();

    let x = phi.cos() * sqrt_r2;
    let y = phi.sin() * sqrt_r2;
    let z = (1.0 - r2).sqrt();

    let (tangent, bitangent) = normal.orthonormal_basis();
    tangent * x + bitangent * y + normal * z
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
