//! GGX microfacet reflection with Smith height-correlated masking-shadowing
//! and visible-normal sampling (Heitz 2018). The GPU shader mirrors this.

use crate::Vec3;
use std::f64::consts::PI;

/// Roughness below this is treated as a perfect mirror.
pub const MIN_ALPHA: f64 = 1e-3;

/// Normal distribution D(h). Uses `sin²θ_h = |h × n|²` rather than
/// `1 - cos²θ_h`, which cancels catastrophically near the normal for small α.
fn distribution(h: Vec3, normal: Vec3, alpha: f64) -> f64 {
    let cos_h = h.dot(normal);
    if cos_h <= 0.0 {
        return 0.0;
    }
    let a2 = alpha * alpha;
    let sin2_h = h.cross(normal).norm();
    let t = a2 * cos_h * cos_h + sin2_h;
    a2 / (PI * t * t)
}

/// Smith Λ for a direction with cosine `cos` to the normal.
fn lambda(cos: f64, alpha: f64) -> f64 {
    let cos2 = cos * cos;
    let tan2 = (1.0 - cos2).max(0.0) / cos2;
    (-1.0 + (1.0 + alpha * alpha * tan2).sqrt()) * 0.5
}

pub fn g1(cos: f64, alpha: f64) -> f64 {
    1.0 / (1.0 + lambda(cos, alpha))
}

pub fn g2(cos_o: f64, cos_i: f64, alpha: f64) -> f64 {
    1.0 / (1.0 + lambda(cos_o, alpha) + lambda(cos_i, alpha))
}

/// Sample a visible microfacet normal in the local frame (z = normal).
pub fn sample_visible_normal(wo: Vec3, alpha: f64, u1: f64, u2: f64) -> Vec3 {
    // Stretch the view direction into the α = 1 configuration
    let vh = Vec3::new(alpha * wo.x, alpha * wo.y, wo.z).normalize();

    let len_sq = vh.x * vh.x + vh.y * vh.y;
    let t1 = if len_sq > 0.0 {
        Vec3::new(-vh.y, vh.x, 0.0) / len_sq.sqrt()
    } else {
        Vec3::new(1.0, 0.0, 0.0)
    };
    let t2 = vh.cross(t1);

    let r = u1.sqrt();
    let phi = 2.0 * PI * u2;
    let p1 = r * phi.cos();
    let mut p2 = r * phi.sin();
    let s = 0.5 * (1.0 + vh.z);
    p2 = (1.0 - s) * (1.0 - p1 * p1).max(0.0).sqrt() + s * p2;

    let nh = t1 * p1 + t2 * p2 + vh * (1.0 - p1 * p1 - p2 * p2).max(0.0).sqrt();

    // Unstretch
    Vec3::new(alpha * nh.x, alpha * nh.y, nh.z.max(0.0)).normalize()
}

/// Solid-angle pdf of `wi` under visible-normal sampling from `wo`, defined
/// over the whole sphere (directions below the surface can be sampled and
/// are then discarded, so their mass is real): `D_v(h) / (4 wo·h)` with
/// `D_v(h) = G1(wo) (wo·h) D(h) / cos_o`.
pub fn pdf(alpha: f64, wo: Vec3, wi: Vec3, normal: Vec3) -> f64 {
    let cos_o = wo.dot(normal);
    if cos_o <= 0.0 {
        return 0.0;
    }
    let h = (wo + wi).normalize();
    if wo.dot(h) <= 0.0 {
        return 0.0;
    }
    g1(cos_o, alpha) * distribution(h, normal, alpha) / (4.0 * cos_o)
}

/// BRDF value without the albedo factor, and the visible-normal-sampling pdf,
/// for unit directions `wo` (towards the viewer) and `wi` (towards the light)
/// around unit `normal`. `None` when either direction is below the surface.
pub fn eval(alpha: f64, wo: Vec3, wi: Vec3, normal: Vec3) -> Option<(f64, f64)> {
    let cos_o = wo.dot(normal);
    let cos_i = wi.dot(normal);
    if cos_o <= 0.0 || cos_i <= 0.0 {
        return None;
    }
    let h = (wo + wi).normalize();
    if wo.dot(h) <= 0.0 {
        return None;
    }

    let d = distribution(h, normal, alpha);
    let f = d * g2(cos_o, cos_i, alpha) / (4.0 * cos_o * cos_i);
    Some((f, pdf(alpha, wo, wi, normal)))
}
