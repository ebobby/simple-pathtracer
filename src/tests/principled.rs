//! The principled material must sample consistently with its `eval`, conserve
//! energy, and shade back-facing surfaces.

use crate::intersectable::Intersection;
use crate::material::Principled;
use crate::ray::Ray;
use crate::shape::{Disc, Sphere};
use crate::{radiance_with, Integrator};
use crate::{Camera, Color, Hitable, Material, Sampler, Scene, Texture, Vec3, BVH};

fn white() -> Texture {
    Texture::constant_color(Color::new(1.0, 1.0, 1.0))
}

fn hit_at_origin<'a>(material: &'a Material, normal: Vec3) -> Intersection<'a> {
    Intersection {
        p: Vec3::zero(),
        t: 1.0,
        normal,
        u: 0.0,
        v: 0.0,
        material,
        shape_id: 0,
    }
}

/// Emissive dome of radiance 1 around a single sphere with `material`.
fn furnace(material: Material) -> Scene {
    let objects: Vec<Hitable> = vec![
        Box::new(Sphere {
            center: Vec3::zero(),
            radius: 1.0,
            material,
        }),
        Box::new(Sphere {
            center: Vec3::zero(),
            radius: 50.0,
            material: Material::diffuse_light(white()),
        }),
    ];
    Scene {
        camera: Camera::new(Vec3::new(0.0, 0.0, 5.0), Vec3::zero(), 30.0, 1.0, 0.0),
        world: BVH::from_vec(objects),
    }
}

fn furnace_radiance(material: Material, integrator: Integrator) -> f64 {
    let scene = furnace(material);
    let n = 40_000;
    let mut sum = 0.0;
    for i in 0..n {
        let sampler = Sampler::new(31, i);
        let (jx, jy) = sampler.get_2d(0);
        // Rays spread over the sphere's disc so many view angles are covered
        let ray = Ray {
            origin: Vec3::new(0.0, 0.0, 5.0),
            direction: Vec3::new((jx - 0.5) * 0.3, (jy - 0.5) * 0.3, -1.0),
        };
        sum += radiance_with(&scene, &ray, 1, 50, integrator, &sampler).r;
    }
    sum / n as f64
}

#[test]
fn principled_sampled_pdf_matches_eval() {
    let material = Material::Principled(
        Principled::new(Texture::constant_color(Color::new(0.8, 0.5, 0.3)))
            .metallic(0.3)
            .roughness(0.4),
    );
    let normal = Vec3::new(0.1, 1.0, 0.2).normalize();
    let hit = hit_at_origin(&material, normal);
    let incoming = Ray {
        origin: Vec3::new(1.0, 2.0, 0.5),
        direction: Vec3::new(-1.0, -2.0, -0.5),
    };
    let wo = -incoming.direction.normalize();

    let mut checked = 0;
    for i in 0..500 {
        let s = Sampler::new(13, i);
        let (u1, u2) = s.get_2d(0);
        let (u3, _) = s.get_2d(1);
        let (u4, u5) = s.get_2d(2);
        let Some(scattered) = material.scatter(&incoming, &hit, [u1, u2, u3, u4, u5]) else {
            continue;
        };
        let wi = scattered.scattered.direction.normalize();
        let pdf = scattered.pdf.expect("reflection lobes must report a pdf");
        let (_, eval_pdf) = material.eval(wo, wi, &hit).expect("eval on a valid direction");
        assert!((pdf - eval_pdf).abs() < 1e-6 * pdf.max(1.0), "sample {i}: {pdf} vs {eval_pdf}");
        checked += 1;
    }
    assert!(checked > 450, "only {checked} valid samples");
}

#[test]
fn principled_reflection_pdf_integrates_to_one() {
    let material = Material::Principled(
        Principled::new(white()).metallic(0.5).roughness(0.6),
    );
    let normal = Vec3::new(0.0, 1.0, 0.0);
    let hit = hit_at_origin(&material, normal);
    let wo = Vec3::new(0.4, 0.7, 0.1).normalize();

    let n = 400_000u32;
    let mut sum = 0.0;
    for i in 0..n {
        let (u1, u2) = Sampler::new(19, i).get_2d(0);
        let z = 1.0 - 2.0 * u1;
        let r = (1.0 - z * z).max(0.0).sqrt();
        let phi = 2.0 * std::f64::consts::PI * u2;
        let wi = Vec3::new(r * phi.cos(), z, r * phi.sin());
        sum += material.pdf_over_sphere(wo, wi, &hit) * 4.0 * std::f64::consts::PI;
    }
    let integral = sum / f64::from(n);
    assert!((integral - 1.0).abs() < 0.02, "pdf integrates to {integral}");
}

#[test]
fn white_glass_in_a_white_furnace_is_nearly_white() {
    let glass = Material::Principled(
        Principled::new(white()).transmission(1.0).roughness(0.1).ior(1.5),
    );
    let value = furnace_radiance(glass, Integrator::BsdfOnly);
    assert!((0.95..=1.02).contains(&value), "furnace value {value}");
}

#[test]
fn white_rough_diffuse_in_a_white_furnace_conserves_energy() {
    let matte = Material::Principled(Principled::new(white()).roughness(1.0));
    let value = furnace_radiance(matte, Integrator::NextEventEstimation);
    assert!((0.9..=1.02).contains(&value), "furnace value {value}");
}

#[test]
fn diffuse_disc_lit_from_its_back_matches_analytic_irradiance() {
    let (albedo, l, r, h) = (0.5, 10.0, 1.0, 2.0);
    let expected = albedo * l * r * r / (r * r + h * h);

    let objects: Vec<Hitable> = vec![
        // Ground disc whose normal points away from the light and the camera
        Box::new(Disc {
            center: Vec3::zero(),
            normal: Vec3::new(0.0, -1.0, 0.0),
            radius: 100.0,
            material: Material::lambertian(Texture::constant_color(Color::new(albedo, albedo, albedo))),
        }),
        Box::new(Disc {
            center: Vec3::new(0.0, h, 0.0),
            normal: Vec3::new(0.0, -1.0, 0.0),
            radius: r,
            material: Material::diffuse_light(Texture::constant_color(Color::new(l, l, l))),
        }),
    ];
    let scene = Scene {
        camera: Camera::new(Vec3::new(0.0, 1.0, 0.0), Vec3::zero(), 45.0, 1.0, 0.0),
        world: BVH::from_vec(objects),
    };
    let ray = Ray {
        origin: Vec3::new(0.0, 1.0, 0.0),
        direction: Vec3::new(0.0, -1.0, 0.0),
    };
    let n = 20_000;
    let mut sum = 0.0;
    for i in 0..n {
        sum += radiance_with(&scene, &ray, 1, 1, Integrator::NextEventEstimation, &Sampler::new(7, i)).r;
    }
    let mean = sum / n as f64;
    assert!((mean - expected).abs() < 0.02 * expected, "expected {expected:.4}, got {mean:.4}");
}
