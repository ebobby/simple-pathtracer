//! Sky and sun lights must be sampled consistently and produce the analytic
//! answers for simple configurations.

use crate::environment::{EnvironmentMap, Sky, Sun, Environment};
use crate::ray::Ray;
use crate::shape::Sphere;
use crate::{radiance_with, Integrator};
use crate::{Camera, Color, Hitable, Material, Sampler, Scene, Texture, Vec3, BVH};

use std::f64::consts::PI;

fn grey(albedo: f64) -> Material {
    Material::lambertian(Texture::constant_color(Color::new(albedo, albedo, albedo)))
}

#[test]
fn lambertian_sphere_under_constant_sky_reflects_its_albedo() {
    let albedo = 0.6;
    let objects: Vec<Hitable> = vec![Box::new(Sphere {
        center: Vec3::zero(),
        radius: 1.0,
        material: grey(albedo),
    })];
    let scene = Scene::new(
        Camera::new(Vec3::new(0.0, 0.0, 5.0), Vec3::zero(), 30.0, 1.0, 0.0),
        BVH::from_vec(objects),
    )
    .with_environment(Environment::new().sky(Sky::Constant(Color::new(1.0, 1.0, 1.0))));

    let n = 40_000;
    let mut sum = 0.0;
    for i in 0..n {
        let sampler = Sampler::new(41, i);
        let (jx, jy) = sampler.get_2d(0);
        let ray = Ray {
            origin: Vec3::new(0.0, 0.0, 5.0),
            direction: Vec3::new((jx - 0.5) * 0.3, (jy - 0.5) * 0.3, -1.0),
        };
        sum += radiance_with(&scene, &ray, 1, 1, Integrator::NextEventEstimation, &sampler).r;
    }
    let mean = sum / n as f64;
    assert!((mean - albedo).abs() < 0.01, "expected {albedo}, got {mean}");
}

#[test]
fn sun_over_diffuse_plane_matches_analytic_irradiance() {
    let albedo = 0.5;
    let radiance = 1000.0;
    let angular_radius = 0.05f64;
    let cos_max = angular_radius.cos();
    // Irradiance from a small uniform cone at the zenith: E = L * 2π (1 - cos θ)
    // (exactly π L sin²θ, which differs by well under 0.1% here).
    let expected = albedo * radiance * 2.0 * PI * (1.0 - cos_max) / PI;

    let objects: Vec<Hitable> = vec![Box::new(Sphere {
        center: Vec3::new(0.0, -5000.0, 0.0),
        radius: 5000.0,
        material: grey(albedo),
    })];
    let scene = Scene::new(
        Camera::new(Vec3::new(0.0, 1.0, 0.0), Vec3::zero(), 45.0, 1.0, 0.0),
        BVH::from_vec(objects),
    )
    .with_environment(Environment::new().sun(Sun::new(
        Vec3::new(0.0, 1.0, 0.0),
        Color::new(radiance, radiance, radiance),
        angular_radius,
    )));

    let ray = Ray {
        origin: Vec3::new(0.0, 1.0, 0.0),
        direction: Vec3::new(0.0, -1.0, 0.0),
    };
    let n = 20_000;
    let mut sum = 0.0;
    for i in 0..n {
        sum += radiance_with(&scene, &ray, 1, 1, Integrator::NextEventEstimation, &Sampler::new(43, i)).r;
    }
    let mean = sum / n as f64;
    assert!((mean - expected).abs() < 0.02 * expected, "expected {expected:.4}, got {mean:.4}");
}

/// A small synthetic environment map: dark everywhere except a bright patch.
fn patchy_map() -> EnvironmentMap {
    let (w, h) = (16, 8);
    let mut pixels = Vec::with_capacity(w * h);
    for y in 0..h {
        for x in 0..w {
            let bright = (3..6).contains(&x) && (2..4).contains(&y);
            let v = if bright { 50.0 } else { 0.1 + 0.05 * x as f64 };
            pixels.push(Color::new(v, v * 0.9, v * 0.8));
        }
    }
    EnvironmentMap::from_pixels(w, h, pixels)
}

#[test]
fn environment_map_sample_pdf_matches_pdf_evaluation() {
    let map = patchy_map();
    for i in 0..500 {
        let (u1, u2) = Sampler::new(47, i).get_2d(0);
        let (direction, pdf) = map.sample(u1, u2);
        assert!((direction.length() - 1.0).abs() < 1e-9);
        let evaluated = map.pdf(direction);
        assert!((pdf - evaluated).abs() < 1e-6 * pdf, "sample {i}: {pdf} vs {evaluated}");
    }
}

#[test]
fn environment_map_pdf_integrates_to_one() {
    let map = patchy_map();
    let n = 400_000u32;
    let mut sum = 0.0;
    for i in 0..n {
        let (u1, u2) = Sampler::new(53, i).get_2d(0);
        let z = 1.0 - 2.0 * u1;
        let r = (1.0 - z * z).max(0.0).sqrt();
        let phi = 2.0 * PI * u2;
        let direction = Vec3::new(r * phi.cos(), z, r * phi.sin());
        sum += map.pdf(direction) * 4.0 * PI;
    }
    let integral = sum / f64::from(n);
    assert!((integral - 1.0).abs() < 0.02, "pdf integrates to {integral}");
}

#[test]
fn environment_map_samples_concentrate_on_the_bright_patch() {
    let map = patchy_map();
    let mut bright = 0;
    let n = 2000;
    for i in 0..n {
        let (u1, u2) = Sampler::new(59, i).get_2d(0);
        let (direction, _) = map.sample(u1, u2);
        let c = map.radiance(direction);
        if c.r > 10.0 {
            bright += 1;
        }
    }
    // The patch is 6 of 128 texels but holds most of the energy.
    assert!(bright as f64 / n as f64 > 0.8, "only {bright} of {n} samples hit the patch");
}
