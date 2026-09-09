//! Lights: emissive shapes plus the sky and sun, and how to sample them.

use crate::Vec3;

use std::f64::consts::PI;

/// Geometry of an emissive shape, enough to sample it and evaluate its pdf.
#[derive(Clone, Copy, Debug)]
pub enum LightShape {
    Sphere { center: Vec3, radius: f64 },
    Disc { center: Vec3, normal: Vec3, radius: f64 },
}

#[derive(Clone, Copy, Debug)]
pub enum LightKind {
    /// Shape `shape_id` in the scene's BVH.
    Shape { shape_id: usize, shape: LightShape },
    Sky,
    Sun,
}

/// A light and how likely light selection is to pick it (proportional to
/// emitted power).
#[derive(Clone, Copy, Debug)]
pub struct Light {
    pub kind: LightKind,
    pub select_pdf: f64,
}

/// A sampled direction towards a light from a shading point.
#[derive(Clone, Copy, Debug)]
pub struct LightSample {
    /// Unit direction from the shading point towards the light.
    pub direction: Vec3,
    /// A point on the light along `direction` (used for pdf evaluation).
    pub point: Vec3,
    /// Solid-angle pdf of `direction`.
    pub pdf: f64,
}

impl LightShape {
    pub fn area(&self) -> f64 {
        match *self {
            LightShape::Sphere { radius, .. } => 4.0 * PI * radius * radius,
            LightShape::Disc { radius, .. } => PI * radius * radius,
        }
    }

    /// Sample a direction from `p` towards this shape using two uniforms.
    /// Returns `None` when the sample carries no energy (e.g. a disc seen
    /// edge-on).
    pub fn sample(&self, p: Vec3, u1: f64, u2: f64) -> Option<LightSample> {
        match *self {
            LightShape::Disc { center, normal, radius } => {
                let (tangent, bitangent) = normal.orthonormal_basis();
                let r = radius * u1.sqrt();
                let phi = 2.0 * PI * u2;
                let point = center + tangent * (r * phi.cos()) + bitangent * (r * phi.sin());

                let to_light = point - p;
                let dist_sq = to_light.norm();
                if dist_sq == 0.0 {
                    return None;
                }
                let direction = to_light / dist_sq.sqrt();
                let cos_light = direction.dot(normal).abs();
                if cos_light < 1e-9 {
                    return None;
                }
                let area = PI * radius * radius;
                Some(LightSample {
                    direction,
                    point,
                    pdf: dist_sq / (area * cos_light),
                })
            }
            LightShape::Sphere { center, radius } => {
                let to_center = center - p;
                let dist_sq = to_center.norm();

                if dist_sq <= radius * radius {
                    // Inside the sphere: every direction hits it.
                    let z = 1.0 - 2.0 * u1;
                    let r = (1.0 - z * z).max(0.0).sqrt();
                    let phi = 2.0 * PI * u2;
                    let direction = Vec3::new(r * phi.cos(), r * phi.sin(), z);
                    return Some(LightSample {
                        direction,
                        point: p + direction,
                        pdf: 1.0 / (4.0 * PI),
                    });
                }

                // Outside: uniform direction inside the subtended cone.
                let cos_theta_max = (1.0 - radius * radius / dist_sq).sqrt();
                let cos_theta = 1.0 - u1 * (1.0 - cos_theta_max);
                let sin_theta = (1.0 - cos_theta * cos_theta).max(0.0).sqrt();
                let phi = 2.0 * PI * u2;

                let axis = to_center / dist_sq.sqrt();
                let (tangent, bitangent) = axis.orthonormal_basis();
                let direction = tangent * (sin_theta * phi.cos())
                    + bitangent * (sin_theta * phi.sin())
                    + axis * cos_theta;

                Some(LightSample {
                    direction,
                    point: p + direction,
                    pdf: 1.0 / (2.0 * PI * (1.0 - cos_theta_max)),
                })
            }
        }
    }

    /// Solid-angle pdf that `sample` from `p` would produce the unit
    /// `direction`, which is known to reach the shape at `point`.
    pub fn pdf(&self, p: Vec3, point: Vec3, direction: Vec3) -> f64 {
        match *self {
            LightShape::Disc { normal, radius, .. } => {
                let dist_sq = (point - p).norm();
                let cos_light = direction.dot(normal).abs();
                if cos_light < 1e-9 {
                    return 0.0;
                }
                dist_sq / (PI * radius * radius * cos_light)
            }
            LightShape::Sphere { center, radius } => {
                let dist_sq = (center - p).norm();
                if dist_sq <= radius * radius {
                    return 1.0 / (4.0 * PI);
                }
                let cos_theta_max = (1.0 - radius * radius / dist_sq).sqrt();
                1.0 / (2.0 * PI * (1.0 - cos_theta_max))
            }
        }
    }
}

/// Power heuristic (β = 2) for combining two sampling strategies.
#[inline]
pub fn power_heuristic(pdf_a: f64, pdf_b: f64) -> f64 {
    let a = pdf_a * pdf_a;
    let b = pdf_b * pdf_b;
    if a + b == 0.0 {
        0.0
    } else {
        a / (a + b)
    }
}
