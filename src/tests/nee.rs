//! Next event estimation must be unbiased and match analytic direct lighting.

use crate::light::{LightKind, LightShape};
use crate::ray::Ray;
use crate::shape::{Disc, Sphere};
use crate::{radiance_with, render_linear_with, Integrator};
use crate::{Camera, Color, Hitable, Material, Sampler, Scene, Texture, Vec3, BVH};

fn grey(albedo: f64) -> Material {
    Material::lambertian(Texture::constant_color(Color::new(albedo, albedo, albedo)))
}

/// Lambertian ground plane (huge sphere) with a downward-facing disc light
/// of radius `r` at height `h` directly above the origin.
fn plane_under_disc(albedo: f64, radiance: f64, r: f64, h: f64) -> Scene {
    let objects: Vec<Hitable> = vec![
        Box::new(Sphere {
            center: Vec3::new(0.0, -5000.0, 0.0),
            radius: 5000.0,
            material: grey(albedo),
        }),
        Box::new(Disc {
            center: Vec3::new(0.0, h, 0.0),
            normal: Vec3::new(0.0, -1.0, 0.0),
            radius: r,
            material: Material::diffuse_light(Texture::constant_color(Color::new(
                radiance, radiance, radiance,
            ))),
        }),
    ];
    Scene::new(Camera::new(Vec3::new(0.0, 1.0, 0.0), Vec3::zero(), 45.0, 1.0, 0.0), BVH::from_vec(objects))
}

#[test]
fn direct_light_from_disc_matches_analytic_irradiance() {
    let (albedo, l, r, h) = (0.5, 10.0, 1.0, 2.0);
    let scene = plane_under_disc(albedo, l, r, h);
    // Irradiance under the centre of a disc: E = π L sin²α, sin²α = r²/(r²+h²).
    // Reflected radiance from a Lambertian surface: albedo * E / π.
    let expected = albedo * l * r * r / (r * r + h * h);

    let ray = Ray {
        origin: Vec3::new(0.0, 1.0, 0.0),
        direction: Vec3::new(0.0, -1.0, 0.0),
    };
    let n = 20_000;
    let mut sum = 0.0;
    for i in 0..n {
        let sampler = Sampler::new(99, i);
        sum += radiance_with(&scene, &ray, 1, 1, Integrator::NextEventEstimation, &sampler).r;
    }
    let mean = sum / n as f64;
    assert!(
        (mean - expected).abs() < 0.02 * expected,
        "expected {expected:.4}, got {mean:.4}"
    );
}

#[test]
fn nee_and_bsdf_only_agree_on_mean_brightness() {
    let objects: Vec<Hitable> = vec![
        Box::new(Sphere {
            center: Vec3::new(0.0, -5000.0, 0.0),
            radius: 5000.0,
            material: grey(0.7),
        }),
        Box::new(Sphere {
            center: Vec3::new(0.0, 1.0, 0.0),
            radius: 1.0,
            material: grey(0.8),
        }),
        Box::new(Sphere {
            center: Vec3::new(2.5, 0.7, 1.0),
            radius: 0.7,
            material: Material::metal(Texture::constant_color(Color::new(0.9, 0.9, 0.9)), 0.2),
        }),
        Box::new(Sphere {
            center: Vec3::new(-2.5, 0.8, 1.5),
            radius: 0.8,
            material: Material::Principled(
                crate::material::Principled::new(Texture::constant_color(Color::new(0.2, 0.6, 0.9)))
                    .metallic(0.5)
                    .roughness(0.3),
            ),
        }),
        // Small sphere light and a tilted disc light
        Box::new(Sphere {
            center: Vec3::new(-2.0, 4.0, 1.0),
            radius: 0.5,
            material: Material::diffuse_light(Texture::constant_color(Color::new(20.0, 18.0, 15.0))),
        }),
        Box::new(Disc {
            center: Vec3::new(3.0, 4.0, -2.0),
            normal: Vec3::new(-0.5, -1.0, 0.3).normalize(),
            radius: 1.0,
            material: Material::diffuse_light(Texture::constant_color(Color::new(6.0, 8.0, 12.0))),
        }),
    ];
    let scene = Scene::new(Camera::new(
            Vec3::new(0.0, 2.5, 9.0),
            Vec3::new(0.0, 1.0, 0.0),
            45.0,
            4.0 / 3.0,
            0.0,
        ), BVH::from_vec(objects));
    let (w, h, depth) = (32, 24, 12);

    let mean = |pixels: &[Color]| {
        pixels.iter().map(|c| c.r + c.g + c.b).sum::<f64>() / (3.0 * pixels.len() as f64)
    };
    let nee = mean(&render_linear_with(&scene, w, h, 200, depth, 4, Integrator::NextEventEstimation));
    let bsdf = mean(&render_linear_with(&scene, w, h, 2000, depth, 4, Integrator::BsdfOnly));

    let relative = (nee - bsdf).abs() / bsdf;
    assert!(
        relative < 0.03,
        "NEE mean {nee:.4} vs BSDF-only mean {bsdf:.4} differ by {:.1}%",
        relative * 100.0
    );
}

/// Sampling a light and evaluating `pdf` for the sampled direction must agree.
#[test]
fn light_sample_pdf_matches_pdf_evaluation() {
    let lights = [
        LightShape::Disc {
            center: Vec3::new(1.0, 3.0, -2.0),
            normal: Vec3::new(0.2, -1.0, 0.4).normalize(),
            radius: 1.5,
        },
        LightShape::Sphere {
            center: Vec3::new(-2.0, 2.0, 1.0),
            radius: 0.8,
        },
    ];
    let p = Vec3::new(0.3, 0.0, 0.1);
    for light in &lights {
        for i in 0..100 {
            let (u1, u2) = Sampler::new(5, i).get_2d(0);
            let sample = light.sample(p, u1, u2).expect("light should be sampleable");
            assert!((sample.direction.length() - 1.0).abs() < 1e-9);
            let pdf = light.pdf(p, sample.point, sample.direction);
            assert!(
                (pdf - sample.pdf).abs() < 1e-6 * sample.pdf,
                "{light:?}: sampled pdf {} vs evaluated {}",
                sample.pdf,
                pdf
            );
        }
    }
}

#[test]
fn point_inside_sphere_light_samples_full_sphere() {
    let light = LightShape::Sphere { center: Vec3::zero(), radius: 10.0 };
    let sample = light.sample(Vec3::new(1.0, 2.0, 3.0), 0.3, 0.6).unwrap();
    assert!((sample.pdf - 1.0 / (4.0 * std::f64::consts::PI)).abs() < 1e-12);
}

#[test]
fn lights_are_selected_in_proportion_to_power() {
    // Two sphere lights of equal size: one emits 9x the radiance of the other.
    let objects: Vec<Hitable> = vec![
        Box::new(Sphere {
            center: Vec3::new(-2.0, 0.0, 0.0),
            radius: 0.5,
            material: Material::diffuse_light(Texture::constant_color(Color::new(9.0, 9.0, 9.0))),
        }),
        Box::new(Sphere {
            center: Vec3::new(2.0, 0.0, 0.0),
            radius: 0.5,
            material: Material::diffuse_light(Texture::constant_color(Color::new(1.0, 1.0, 1.0))),
        }),
    ];
    let scene = Scene::new(
        Camera::new(Vec3::new(0.0, 0.0, 5.0), Vec3::zero(), 45.0, 1.0, 0.0),
        BVH::from_vec(objects),
    );

    let mut strong = 0;
    let n = 10_000;
    for i in 0..n {
        let (u, _) = Sampler::new(3, i).get_2d(0);
        let (light, pdf) = scene.pick_light(u);
        let shape_id = match light.kind {
            LightKind::Shape { shape_id, .. } => shape_id,
            _ => panic!("unexpected light kind"),
        };
        if shape_id == 0 {
            strong += 1;
            assert!((pdf - 0.9).abs() < 1e-9, "strong light pdf {pdf}");
        } else {
            assert!((pdf - 0.1).abs() < 1e-9, "weak light pdf {pdf}");
        }
    }
    let fraction = strong as f64 / n as f64;
    assert!((fraction - 0.9).abs() < 0.02, "strong light picked {fraction} of the time");

    assert!((scene.light_of_shape(1).unwrap().select_pdf - 0.1).abs() < 1e-9);
}
