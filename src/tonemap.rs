//! Output stage: exposure, tone curve, gamma encoding.

use crate::Color;

/// How radiance above the displayable range is compressed.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ToneCurve {
    /// Hard clip at 1.0 (the historical behaviour).
    #[default]
    Clamp,
    /// `x / (1 + x)` per channel: never clips, compresses highlights softly.
    Reinhard,
    /// Filmic ACES fit (Narkowicz 2015): contrast in the mids, soft shoulder.
    Aces,
}

impl ToneCurve {
    /// Map a linear value to [0, 1].
    pub fn apply(self, x: f64) -> f64 {
        let x = x.max(0.0);
        match self {
            ToneCurve::Clamp => x.min(1.0),
            ToneCurve::Reinhard => x / (1.0 + x),
            ToneCurve::Aces => {
                let (a, b, c, d, e) = (2.51, 0.03, 2.43, 0.59, 0.14);
                ((x * (a * x + b)) / (x * (c * x + d) + e)).clamp(0.0, 1.0)
            }
        }
    }

    /// Integer id used by the GPU blit shader.
    pub fn gpu_id(self) -> u32 {
        match self {
            ToneCurve::Clamp => 0,
            ToneCurve::Reinhard => 1,
            ToneCurve::Aces => 2,
        }
    }
}

/// Exposure multiplier, tone curve and display gamma applied to linear
/// radiance to produce 8-bit output.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Tonemap {
    /// Linear scale applied before the curve (2.0 = one stop brighter).
    pub exposure: f64,
    pub curve: ToneCurve,
    pub gamma: f64,
}

impl Default for Tonemap {
    fn default() -> Self {
        Self {
            exposure: 1.0,
            curve: ToneCurve::Clamp,
            gamma: 2.2,
        }
    }
}

impl Tonemap {
    pub fn new(exposure: f64, curve: ToneCurve) -> Self {
        Self {
            exposure,
            curve,
            gamma: 2.2,
        }
    }

    /// Exposure given in stops relative to 1.0.
    pub fn from_ev(stops: f64, curve: ToneCurve) -> Self {
        Self::new(2f64.powf(stops), curve)
    }

    pub fn gamma(mut self, gamma: f64) -> Self {
        self.gamma = gamma;
        self
    }

    /// Linear radiance to an 8-bit gamma-encoded pixel.
    pub fn apply(&self, c: Color) -> image::Rgb<u8> {
        let g = self.gamma.recip();
        let channel = |v: f64| (self.curve.apply(v * self.exposure).powf(g) * 255.0) as u8;
        image::Rgb([channel(c.r), channel(c.g), channel(c.b)])
    }
}
