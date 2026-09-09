//! Output stage: exposure, tone curve, gamma encoding.

use crate::Color;

/// Glow around bright areas, applied after exposure and before the curve.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Bloom {
    /// Only radiance above this (after exposure) contributes to the glow.
    pub threshold: f64,
    /// Scale of the glow added back to the image.
    pub intensity: f64,
    /// Blur radius as a fraction of the image width.
    pub radius: f64,
}

impl Default for Bloom {
    fn default() -> Self {
        Self {
            threshold: 1.0,
            intensity: 0.15,
            radius: 0.03,
        }
    }
}

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
    /// Optional glow; only applied by `apply_image` (whole-image output).
    pub bloom: Option<Bloom>,
}

impl Default for Tonemap {
    fn default() -> Self {
        Self {
            exposure: 1.0,
            curve: ToneCurve::Clamp,
            gamma: 2.2,
            bloom: None,
        }
    }
}

impl Tonemap {
    pub fn new(exposure: f64, curve: ToneCurve) -> Self {
        Self {
            exposure,
            curve,
            gamma: 2.2,
            bloom: None,
        }
    }

    pub fn with_bloom(mut self, bloom: Bloom) -> Self {
        self.bloom = Some(bloom);
        self
    }

    /// Whole-image output: exposure, bloom, curve and gamma.
    pub fn apply_image(
        &self,
        pixels: &[Color],
        width: u32,
        height: u32,
    ) -> image::ImageBuffer<image::Rgb<u8>, Vec<u8>> {
        let mut imgbuf = image::ImageBuffer::new(width, height);
        let glow = self
            .bloom
            .map(|bloom| bloom_layer(pixels, width as usize, height as usize, self.exposure, bloom));
        let g = self.gamma.recip();
        for (i, color) in pixels.iter().enumerate() {
            let mut c = *color * self.exposure;
            if let Some(glow) = &glow {
                c += glow[i];
            }
            let channel = |v: f64| (self.curve.apply(v).powf(g) * 255.0) as u8;
            let x = i as u32 % width;
            let y = i as u32 / width;
            imgbuf.put_pixel(x, y, image::Rgb([channel(c.r), channel(c.g), channel(c.b)]));
        }
        imgbuf
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

/// The bloom contribution per pixel: radiance above the threshold, blurred
/// with a separable Gaussian, scaled by the intensity.
fn bloom_layer(pixels: &[Color], width: usize, height: usize, exposure: f64, bloom: Bloom) -> Vec<Color> {
    let radius = ((bloom.radius * width as f64).round() as usize).max(1);
    let sigma = radius as f64 / 2.0;
    let kernel: Vec<f64> = (0..=radius)
        .map(|i| (-(i as f64 * i as f64) / (2.0 * sigma * sigma)).exp())
        .collect();
    let norm: f64 = kernel[0] + 2.0 * kernel[1..].iter().sum::<f64>();
    let kernel: Vec<f64> = kernel.iter().map(|k| k / norm).collect();

    let black = Color::new(0.0, 0.0, 0.0);
    let bright: Vec<Color> = pixels
        .iter()
        .map(|c| {
            let c = *c * exposure;
            let lum = 0.2126 * c.r + 0.7152 * c.g + 0.0722 * c.b;
            if lum > bloom.threshold {
                c * ((lum - bloom.threshold) / lum)
            } else {
                black
            }
        })
        .collect();

    // Horizontal pass
    let mut tmp = vec![black; width * height];
    for y in 0..height {
        for x in 0..width {
            let mut acc = bright[y * width + x] * kernel[0];
            for (k, weight) in kernel.iter().enumerate().skip(1) {
                if x >= k {
                    acc += bright[y * width + x - k] * *weight;
                }
                if x + k < width {
                    acc += bright[y * width + x + k] * *weight;
                }
            }
            tmp[y * width + x] = acc;
        }
    }
    // Vertical pass
    let mut out = vec![black; width * height];
    for y in 0..height {
        for x in 0..width {
            let mut acc = tmp[y * width + x] * kernel[0];
            for (k, weight) in kernel.iter().enumerate().skip(1) {
                if y >= k {
                    acc += tmp[(y - k) * width + x] * *weight;
                }
                if y + k < height {
                    acc += tmp[(y + k) * width + x] * *weight;
                }
            }
            out[y * width + x] = acc * bloom.intensity;
        }
    }
    out
}
