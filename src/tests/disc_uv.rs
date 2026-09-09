//! Discs must produce texture coordinates covering the unit square for any orientation.

use crate::intersectable::Intersectable;
use crate::ray::Ray;
use crate::shape::Disc;
use crate::{Color, Material, Texture, Vec3};

fn tilted_disc() -> Disc {
    Disc {
        center: Vec3::new(1.0, 2.0, 3.0),
        normal: Vec3::new(1.0, 1.0, 1.0).normalize(),
        radius: 2.0,
        material: Material::lambertian(Texture::constant_color(Color::new(1.0, 1.0, 1.0))),
    }
}

/// Shoot a ray straight down the normal at `point`, which must lie on the disc plane.
fn uv_at(disc: &Disc, point: Vec3) -> (f64, f64) {
    let ray = Ray {
        origin: point + disc.normal * 5.0,
        direction: -disc.normal,
    };
    let hit = disc.intersect(&ray, 0.0001, f64::INFINITY).expect("ray should hit disc");
    (hit.u, hit.v)
}

#[test]
fn disc_center_maps_to_middle_of_texture() {
    let disc = tilted_disc();
    let (u, v) = uv_at(&disc, disc.center);
    assert!((u - 0.5).abs() < 1e-9 && (v - 0.5).abs() < 1e-9, "({u}, {v})");
}

#[test]
fn disc_rim_maps_to_inscribed_circle() {
    let disc = tilted_disc();
    let tangent = disc.normal.cross(Vec3::new(0.0, 1.0, 0.0)).normalize();
    let bitangent = disc.normal.cross(tangent).normalize();
    let r = disc.radius * 0.999;

    for angle in [0.0f64, 1.0, 2.5, 4.0, 5.5] {
        let point = disc.center + tangent * (r * angle.cos()) + bitangent * (r * angle.sin());
        let (u, v) = uv_at(&disc, point);
        assert!((0.0..=1.0).contains(&u) && (0.0..=1.0).contains(&v), "({u}, {v})");
        let dist = ((u - 0.5).powi(2) + (v - 0.5).powi(2)).sqrt();
        assert!((dist - 0.4995).abs() < 1e-6, "angle {angle}: uv ({u}, {v}) at distance {dist}");
    }
}
