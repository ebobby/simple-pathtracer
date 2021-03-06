//! Color module.
use std::ops::{Add, AddAssign, Div, Mul};

/// Color object.
///
/// # Notes
/// Even though by convention all color components are assumed to be between 0.0
/// and 1.0 and they're clamped when converted to `Rgb` it doens't mean they
/// can't be declared to have larger values if needed to. This is usually the
/// case for light intensity.
#[derive(Clone, Copy, Debug)]
pub struct Color {
    pub r: f64,
    pub g: f64,
    pub b: f64,
}

impl Color {
    pub fn new(r: f64, g: f64, b: f64) -> Self {
        Color { r, g, b }
    }

    pub fn from_u8(r: u8, g: u8, b: u8) -> Self {
        Color {
            r: f64::from(r) / 255.0,
            g: f64::from(g) / 255.0,
            b: f64::from(b) / 255.0,
        }
    }

    pub fn to_rgb(&self) -> image::Rgb<u8> {
        image::Rgb([
            (self.r.min(1.0).max(0.0) * 255.0) as u8,
            (self.g.min(1.0).max(0.0) * 255.0) as u8,
            (self.b.min(1.0).max(0.0) * 255.0) as u8,
        ])
    }

    pub fn to_gamma_rgb(&self, gamma_correction: f64) -> image::Rgb<u8> {
        image::Rgb([
            (self.r.min(1.0).max(0.0).powf(gamma_correction) * 255.0) as u8,
            (self.g.min(1.0).max(0.0).powf(gamma_correction) * 255.0) as u8,
            (self.b.min(1.0).max(0.0).powf(gamma_correction) * 255.0) as u8,
        ])
    }
}

impl Add for Color {
    type Output = Self;

    fn add(self, rhs: Color) -> Color {
        Color {
            r: self.r + rhs.r,
            g: self.g + rhs.g,
            b: self.b + rhs.b,
        }
    }
}

impl AddAssign for Color {
    fn add_assign(&mut self, rhs: Color) {
        *self = Color {
            r: self.r + rhs.r,
            g: self.g + rhs.g,
            b: self.b + rhs.b,
        };
    }
}

impl Mul<f64> for Color {
    type Output = Self;

    fn mul(self, factor: f64) -> Color {
        Color {
            r: self.r * factor,
            g: self.g * factor,
            b: self.b * factor,
        }
    }
}

impl Mul for Color {
    type Output = Self;

    fn mul(self, rhs: Color) -> Color {
        Color {
            r: self.r * rhs.r,
            g: self.g * rhs.g,
            b: self.b * rhs.b,
        }
    }
}

impl Div<f64> for Color {
    type Output = Self;

    fn div(self, rhs: f64) -> Color {
        Color {
            r: self.r / rhs,
            g: self.g / rhs,
            b: self.b / rhs,
        }
    }
}
