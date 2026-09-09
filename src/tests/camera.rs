//! Thin-lens camera: every lens sample must see the focus plane the same way,
//! and bloom must spread bright pixels without touching dark images.

use crate::{Bloom, Camera, Color, Sampler, ToneCurve, Tonemap, Vec3};

fn camera_with_lens(aperture: f64, focus_distance: f64) -> Camera {
    Camera::new(Vec3::new(0.0, 1.0, 8.0), Vec3::new(0.0, 1.0, 0.0), 40.0, 1.5, 0.0)
        .with_lens(aperture, focus_distance)
}

#[test]
fn pinhole_ray_is_unchanged_by_lens_samples() {
    let camera = camera_with_lens(0.0, 8.0);
    let a = camera.get_ray_lens(0.3, 0.6, 0.1, 0.9);
    let b = camera.get_ray_lens(0.3, 0.6, 0.8, 0.2);
    assert_eq!(a.origin, b.origin);
    assert!((a.direction.normalize() - b.direction.normalize()).length() < 1e-12);
}

#[test]
fn all_lens_samples_converge_on_the_focus_plane() {
    let focus = 6.0;
    let camera = camera_with_lens(0.5, focus);
    let pinhole = camera_with_lens(0.0, focus).get_ray_lens(0.3, 0.6, 0.5, 0.5);
    // Point on the focus plane seen by the pinhole ray
    let forward = Vec3::new(0.0, 0.0, -1.0);
    let t = focus / pinhole.direction.dot(forward);
    let focal_point = pinhole.point_at(t);

    for i in 0..50 {
        let (lu, lv) = Sampler::new(61, i).get_2d(0);
        let ray = camera.get_ray_lens(0.3, 0.6, lu, lv);
        let t = focus / ray.direction.dot(forward);
        let p = ray.point_at(t);
        assert!((p - focal_point).length() < 1e-9, "sample {i}: {p:?} vs {focal_point:?}");
        // The origin moved off the pinhole position, within the aperture
        assert!((ray.origin - pinhole.origin).length() <= 0.25 + 1e-9);
    }
}

#[test]
fn field_of_view_survives_focusing() {
    let camera = camera_with_lens(0.3, 12.0);
    assert!((camera.vfov() - 40.0).abs() < 1e-9, "{}", camera.vfov());
}

fn black_with_one_bright_pixel(w: usize, h: usize) -> Vec<Color> {
    let mut pixels = vec![Color::new(0.0, 0.0, 0.0); w * h];
    pixels[(h / 2) * w + w / 2] = Color::new(100.0, 100.0, 100.0);
    pixels
}

#[test]
fn bloom_spreads_a_bright_pixel_to_its_neighbours() {
    let (w, h) = (32, 32);
    let pixels = black_with_one_bright_pixel(w, h);
    let tonemap = Tonemap::new(1.0, ToneCurve::Clamp).with_bloom(Bloom {
        threshold: 1.0,
        intensity: 0.5,
        radius: 0.1,
    });
    let image = tonemap.apply_image(&pixels, w as u32, h as u32);
    let centre = image.get_pixel(w as u32 / 2, h as u32 / 2).0;
    let neighbour = image.get_pixel(w as u32 / 2 + 2, h as u32 / 2).0;
    let far = image.get_pixel(1, 1).0;
    assert_eq!(centre, [255, 255, 255]);
    assert!(neighbour[0] > 0, "neighbour should glow: {neighbour:?}");
    assert!(neighbour[0] > far[0], "glow should fall off with distance");
}

#[test]
fn bloom_leaves_pixels_below_the_threshold_alone() {
    let (w, h) = (16, 16);
    let pixels = vec![Color::new(0.3, 0.3, 0.3); w * h];
    let with = Tonemap::new(1.0, ToneCurve::Clamp)
        .with_bloom(Bloom { threshold: 1.0, intensity: 1.0, radius: 0.2 })
        .apply_image(&pixels, w as u32, h as u32);
    let without = Tonemap::new(1.0, ToneCurve::Clamp).apply_image(&pixels, w as u32, h as u32);
    assert_eq!(with.as_raw(), without.as_raw());
}

#[test]
fn no_bloom_matches_per_pixel_output() {
    let (w, h) = (8, 4);
    let pixels: Vec<Color> = (0..w * h).map(|i| Color::new(i as f64 * 0.05, 0.2, 1.5)).collect();
    let tonemap = Tonemap::new(0.8, ToneCurve::Aces);
    let image = tonemap.apply_image(&pixels, w as u32, h as u32);
    for (i, c) in pixels.iter().enumerate() {
        let expected = tonemap.apply(*c).0;
        assert_eq!(image.get_pixel(i as u32 % w as u32, i as u32 / w as u32).0, expected);
    }
}
