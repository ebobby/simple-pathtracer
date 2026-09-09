//! Rough metal is a GGX microfacet BRDF: its sampled directions must report
//! the same pdf that `eval` computes, and the pdf must integrate to one.

use crate::intersectable::Intersection;
use crate::material::ggx;
use crate::ray::Ray;
use crate::{Color, Material, Sampler, Texture, Vec3};

fn rough_metal(alpha: f64) -> Material {
    Material::metal(Texture::constant_color(Color::new(0.9, 0.8, 0.7)), alpha)
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

#[test]
fn ggx_sampled_direction_pdf_matches_eval() {
    let material = rough_metal(0.3);
    let normal = Vec3::new(0.2, 1.0, -0.1).normalize();
    let hit = hit_at_origin(&material, normal);
    let incoming = Ray {
        origin: Vec3::new(1.0, 2.0, 0.5),
        direction: Vec3::new(-1.0, -2.0, -0.5),
    };
    let wo = -incoming.direction.normalize();

    let mut checked = 0;
    for i in 0..500 {
        let (u1, u2) = Sampler::new(11, i).get_2d(0);
        let Some(scattered) = material.scatter(&incoming, &hit, [u1, u2, 0.5]) else {
            continue;
        };
        let wi = scattered.scattered.direction.normalize();
        let sampled_pdf = scattered.pdf.expect("rough metal must report a pdf");
        let (_, eval_pdf) = material.eval(wo, wi, &hit).expect("eval on a valid direction");
        assert!(
            (sampled_pdf - eval_pdf).abs() < 1e-6 * sampled_pdf.max(1.0),
            "sample {i}: sampled pdf {sampled_pdf} vs eval {eval_pdf}"
        );
        checked += 1;
    }
    assert!(checked > 400, "only {checked} valid samples");
}

#[test]
fn ggx_pdf_integrates_to_one_over_the_sphere() {
    let alpha = 0.5;
    let normal = Vec3::new(0.0, 1.0, 0.0);
    let wo = Vec3::new(0.3, 0.8, 0.2).normalize();

    // Uniform directions over the whole sphere. Directions below the horizon
    // are sampled and discarded, so they carry pdf mass too.
    let n = 400_000u32;
    let mut sum = 0.0;
    let mut below_horizon = 0.0;
    for i in 0..n {
        let (u1, u2) = Sampler::new(17, i).get_2d(0);
        let z = 1.0 - 2.0 * u1;
        let r = (1.0 - z * z).max(0.0).sqrt();
        let phi = 2.0 * std::f64::consts::PI * u2;
        let wi = Vec3::new(r * phi.cos(), z, r * phi.sin());
        let pdf = ggx::pdf(alpha, wo, wi, normal) * 4.0 * std::f64::consts::PI;
        sum += pdf;
        if z <= 0.0 {
            below_horizon += pdf;
        }
    }
    let integral = sum / f64::from(n);
    assert!((integral - 1.0).abs() < 0.02, "pdf integrates to {integral}");
    assert!(below_horizon > 0.0, "some sampled directions should fall below the horizon");
}

#[test]
fn smooth_metal_is_a_delta_reflection() {
    let material = rough_metal(0.0);
    let normal = Vec3::new(0.0, 1.0, 0.0);
    let hit = hit_at_origin(&material, normal);
    let incoming = Ray {
        origin: Vec3::new(0.0, 1.0, 1.0),
        direction: Vec3::new(0.0, -1.0, -1.0),
    };
    let scattered = material.scatter(&incoming, &hit, [0.3, 0.7, 0.1]).unwrap();
    assert!(scattered.pdf.is_none());
    let d = scattered.scattered.direction.normalize();
    let expected = Vec3::new(0.0, 1.0, -1.0).normalize();
    assert!((d - expected).length() < 1e-9, "{d:?}");
    assert!(material.eval(Vec3::new(0.0, 1.0, 1.0).normalize(), expected, &hit).is_none());
}
