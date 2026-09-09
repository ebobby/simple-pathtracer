//! Infinite lights: a sky (constant colour or equirectangular image) and a
//! sun (small uniform cone).

use crate::{Color, Vec3};
use std::f64::consts::PI;

/// Directional light of uniform radiance inside a small cone.
#[derive(Clone, Debug)]
pub struct Sun {
    /// Unit direction towards the sun.
    pub direction: Vec3,
    pub radiance: Color,
    /// Cosine of the cone's angular radius.
    pub cos_max: f64,
}

impl Sun {
    /// `angular_radius` in radians; the real sun is about 0.00465 rad.
    pub fn new(direction: Vec3, radiance: Color, angular_radius: f64) -> Self {
        Self {
            direction: direction.normalize(),
            radiance,
            cos_max: angular_radius.max(1e-4).cos(),
        }
    }

    /// Solid-angle pdf of any direction inside the cone.
    pub fn pdf(&self) -> f64 {
        1.0 / (2.0 * PI * (1.0 - self.cos_max))
    }

    pub fn contains(&self, direction: Vec3) -> bool {
        direction.dot(self.direction) >= self.cos_max
    }

    /// Uniform direction inside the cone.
    pub fn sample(&self, u1: f64, u2: f64) -> Vec3 {
        let cos_theta = 1.0 - u1 * (1.0 - self.cos_max);
        let sin_theta = (1.0 - cos_theta * cos_theta).max(0.0).sqrt();
        let phi = 2.0 * PI * u2;
        let (t, b) = self.direction.orthonormal_basis();
        t * (sin_theta * phi.cos()) + b * (sin_theta * phi.sin()) + self.direction * cos_theta
    }

    /// Irradiance on a surface facing the sun, for light selection.
    pub fn irradiance(&self) -> f64 {
        luminance(self.radiance) * 2.0 * PI * (1.0 - self.cos_max)
    }
}

/// Equirectangular radiance map with importance sampling by luminance.
/// Row `y` covers polar angle `θ = π (y + 0.5) / height` from +y; column `x`
/// covers azimuth `φ = 2π (x + 0.5) / width` measured from +x towards +z.
#[derive(Clone, Debug)]
pub struct EnvironmentMap {
    pub width: usize,
    pub height: usize,
    pub pixels: Vec<Color>,
    /// Probability density over (u, v) in [0,1]², per texel.
    pub pdf_uv: Vec<f64>,
    /// Cumulative distribution over rows.
    pub marginal_cdf: Vec<f64>,
    /// Cumulative distribution over columns within each row (row-major).
    pub conditional_cdf: Vec<f64>,
    pub average: Color,
}

impl EnvironmentMap {
    pub fn from_pixels(width: usize, height: usize, pixels: Vec<Color>) -> Self {
        assert_eq!(pixels.len(), width * height);

        // Texel weights: luminance times sin θ (the equirectangular Jacobian)
        let mut weights = vec![0.0; width * height];
        let mut sum = Color::new(0.0, 0.0, 0.0);
        for y in 0..height {
            let sin_theta = (PI * (y as f64 + 0.5) / height as f64).sin();
            for x in 0..width {
                let c = pixels[y * width + x];
                sum += c;
                weights[y * width + x] = (luminance(c).max(0.0) * sin_theta).max(1e-12);
            }
        }
        let total: f64 = weights.iter().sum();

        let mut marginal_cdf = Vec::with_capacity(height);
        let mut conditional_cdf = vec![0.0; width * height];
        let mut running = 0.0;
        for y in 0..height {
            let row = &weights[y * width..(y + 1) * width];
            let row_sum: f64 = row.iter().sum();
            let mut acc = 0.0;
            for x in 0..width {
                acc += row[x] / row_sum;
                conditional_cdf[y * width + x] = acc;
            }
            conditional_cdf[y * width + width - 1] = 1.0;
            running += row_sum / total;
            marginal_cdf.push(running);
        }
        marginal_cdf[height - 1] = 1.0;

        let scale = (width * height) as f64 / total;
        let pdf_uv = weights.iter().map(|w| w * scale).collect();

        Self {
            width,
            height,
            pixels,
            pdf_uv,
            marginal_cdf,
            conditional_cdf,
            average: sum / (width * height) as f64,
        }
    }

    /// Load a Radiance HDR (or any image the `image` crate reads).
    pub fn load(path: &str) -> Result<Self, String> {
        let img = image::open(path).map_err(|e| e.to_string())?.to_rgb32f();
        let (w, h) = (img.width() as usize, img.height() as usize);
        let pixels = img
            .pixels()
            .map(|p| Color::new(f64::from(p[0]), f64::from(p[1]), f64::from(p[2])))
            .collect();
        Ok(Self::from_pixels(w, h, pixels))
    }

    fn direction_to_uv(direction: Vec3) -> (f64, f64) {
        let theta = direction.y.clamp(-1.0, 1.0).acos();
        let mut phi = direction.z.atan2(direction.x);
        if phi < 0.0 {
            phi += 2.0 * PI;
        }
        (phi / (2.0 * PI), theta / PI)
    }

    fn uv_to_direction(u: f64, v: f64) -> Vec3 {
        let theta = PI * v;
        let phi = 2.0 * PI * u;
        let sin_theta = theta.sin();
        Vec3::new(sin_theta * phi.cos(), theta.cos(), sin_theta * phi.sin())
    }

    fn texel(&self, u: f64, v: f64) -> usize {
        let x = ((u * self.width as f64) as usize).min(self.width - 1);
        let y = ((v * self.height as f64) as usize).min(self.height - 1);
        y * self.width + x
    }

    pub fn radiance(&self, direction: Vec3) -> Color {
        let (u, v) = Self::direction_to_uv(direction);
        self.pixels[self.texel(u, v)]
    }

    /// Solid-angle pdf of `direction` under `sample`.
    pub fn pdf(&self, direction: Vec3) -> f64 {
        let (u, v) = Self::direction_to_uv(direction);
        let sin_theta = (PI * v).sin();
        if sin_theta <= 0.0 {
            return 0.0;
        }
        self.pdf_uv[self.texel(u, v)] / (2.0 * PI * PI * sin_theta)
    }

    /// Importance-sampled direction and its solid-angle pdf.
    pub fn sample(&self, u1: f64, u2: f64) -> (Vec3, f64) {
        let y = self
            .marginal_cdf
            .partition_point(|&c| c <= u1)
            .min(self.height - 1);
        let row_start = if y == 0 { 0.0 } else { self.marginal_cdf[y - 1] };
        let row_span = self.marginal_cdf[y] - row_start;
        let v_in_row = if row_span > 0.0 {
            ((u1 - row_start) / row_span).clamp(0.0, 0.999_999)
        } else {
            0.5
        };

        let row = &self.conditional_cdf[y * self.width..(y + 1) * self.width];
        let x = row.partition_point(|&c| c <= u2).min(self.width - 1);
        let col_start = if x == 0 { 0.0 } else { row[x - 1] };
        let col_span = row[x] - col_start;
        let u_in_col = if col_span > 0.0 {
            ((u2 - col_start) / col_span).clamp(0.0, 0.999_999)
        } else {
            0.5
        };

        let u = (x as f64 + u_in_col) / self.width as f64;
        let v = (y as f64 + v_in_row) / self.height as f64;
        let direction = Self::uv_to_direction(u, v);
        let sin_theta = (PI * v).sin();
        let pdf = self.pdf_uv[y * self.width + x] / (2.0 * PI * PI * sin_theta);
        (direction, pdf)
    }
}

#[derive(Clone, Debug)]
pub enum Sky {
    Constant(Color),
    Image(EnvironmentMap),
}

impl Sky {
    pub fn radiance(&self, direction: Vec3) -> Color {
        match self {
            Sky::Constant(c) => *c,
            Sky::Image(map) => map.radiance(direction),
        }
    }

    /// Solid-angle pdf of `direction` under `sample`.
    pub fn pdf(&self, direction: Vec3) -> f64 {
        match self {
            Sky::Constant(_) => 1.0 / (4.0 * PI),
            Sky::Image(map) => map.pdf(direction),
        }
    }

    pub fn sample(&self, u1: f64, u2: f64) -> (Vec3, f64) {
        match self {
            Sky::Constant(_) => {
                let z = 1.0 - 2.0 * u1;
                let r = (1.0 - z * z).max(0.0).sqrt();
                let phi = 2.0 * PI * u2;
                (Vec3::new(r * phi.cos(), r * phi.sin(), z), 1.0 / (4.0 * PI))
            }
            Sky::Image(map) => map.sample(u1, u2),
        }
    }

    /// Irradiance on a surface under the whole sky, for light selection.
    pub fn irradiance(&self) -> f64 {
        let average = match self {
            Sky::Constant(c) => *c,
            Sky::Image(map) => map.average,
        };
        PI * luminance(average)
    }
}

/// What rays see when they leave the scene.
#[derive(Clone, Debug, Default)]
pub struct Environment {
    pub sky: Option<Sky>,
    pub sun: Option<Sun>,
}

impl Environment {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn sky(mut self, sky: Sky) -> Self {
        self.sky = Some(sky);
        self
    }

    pub fn sun(mut self, sun: Sun) -> Self {
        self.sun = Some(sun);
        self
    }
}

pub fn luminance(c: Color) -> f64 {
    0.2126 * c.r + 0.7152 * c.g + 0.0722 * c.b
}
