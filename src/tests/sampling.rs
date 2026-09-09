//! Lambertian scattering must be cosine distributed: E[cos θ] = 2/3.

use crate::material::random_cosine_direction;
use crate::rng;
use crate::Vec3;

#[test]
fn lambertian_scatter_is_cosine_weighted() {
    let normal = Vec3::new(0.3, -0.5, 0.8).normalize();
    let n = 200_000;
    let mut sum = 0.0;
    let mut below = 0;
    for _ in 0..n {
        let d = random_cosine_direction(normal, rng::get_random_number(), rng::get_random_number());
        assert!((d.length() - 1.0).abs() < 1e-9, "not unit length: {d:?}");
        let cos = d.dot(normal);
        if cos < 0.0 {
            below += 1;
        }
        sum += cos;
    }
    let mean = sum / n as f64;
    assert_eq!(below, 0, "{below} samples below the surface");
    assert!((mean - 2.0 / 3.0).abs() < 0.005, "E[cos] = {mean}");
}
