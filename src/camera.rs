//! Camera module.

use super::ray::Ray;
use super::Vec3;

/// The objct that defines where to look from and where to inside the scene.
#[derive(Debug, Clone)]
pub struct Camera {
    look_from: Vec3,
    /// Corner and extents of the image rectangle, placed on the focus plane.
    corner: Vec3,
    horizontal: Vec3,
    vertical: Vec3,
    u: Vec3,
    v: Vec3,
    w: Vec3,
    /// Thin-lens radius (aperture / 2); 0 is a pinhole.
    lens_radius: f64,
    focus_distance: f64,
}

impl Camera {
    /// Creates a new Camera
    ///
    /// # Arguments
    ///
    /// * `look_from` - Position of the camera in world space.
    /// * `look_at` - Where the camera is directly looking at.
    /// * `fov` - Field of view angle.
    /// * `aspect_ratio` - Aspect ratio of the image. Usually width/height.
    /// * `roll` - Angle of rotation on the z (view) coordinate (roll to a side).
    ///
    pub fn new(look_from: Vec3, look_at: Vec3, fov: f64, aspect_ratio: f64, roll: f64) -> Self {
        let roll_angle = roll.to_radians();
        let rotated_up = Vec3::new(-roll_angle.sin(), roll_angle.cos(), 0.0);

        let w = (look_from - look_at).normalize();
        let u = rotated_up.cross(w).normalize();
        let v = w.cross(u);

        let half_height = (fov.to_radians() / 2.0).tan();
        let half_width = half_height * aspect_ratio;

        let corner = look_from - (u * half_width) + (v * half_height) - w;
        let horizontal = u * (2.0 * half_width);
        let vertical = -v * (2.0 * half_height);

        Camera {
            look_from,
            corner,
            horizontal,
            vertical,
            u,
            v,
            w,
            lens_radius: 0.0,
            focus_distance: 1.0,
        }
    }

    /// Thin-lens depth of field: `aperture` is the lens diameter (0 for a
    /// pinhole) and `focus_distance` the distance from the camera to the
    /// plane in sharp focus.
    pub fn with_lens(mut self, aperture: f64, focus_distance: f64) -> Self {
        let focus_distance = focus_distance.max(1e-6);
        // Rescale the image rectangle from its current focus plane to the new one
        let scale = focus_distance / self.focus_distance;
        let to_corner = self.corner - self.look_from;
        self.corner = self.look_from + to_corner * scale;
        self.horizontal = self.horizontal * scale;
        self.vertical = self.vertical * scale;
        self.lens_radius = aperture.max(0.0) * 0.5;
        self.focus_distance = focus_distance;
        self
    }

    pub fn lens_radius(&self) -> f64 {
        self.lens_radius
    }

    pub fn focus_distance(&self) -> f64 {
        self.focus_distance
    }

    /// Generate a ray through screen point `(s, t)` from a point on the lens
    /// chosen by the uniforms `(lu, lv)`. With a pinhole the lens sample is
    /// ignored.
    pub fn get_ray_lens(&self, s: f64, t: f64, lu: f64, lv: f64) -> Ray {
        let target = self.corner + self.horizontal * s + self.vertical * t;
        if self.lens_radius <= 0.0 {
            return Ray {
                origin: self.look_from,
                direction: target - self.look_from,
            };
        }
        // Uniform point on the lens disc
        let r = self.lens_radius * lu.sqrt();
        let phi = 2.0 * std::f64::consts::PI * lv;
        let origin = self.look_from + self.u * (r * phi.cos()) + self.v * (r * phi.sin());
        Ray {
            origin,
            direction: target - origin,
        }
    }

    /// Generate a direction ray from the camera.
    ///
    /// # Arguments
    ///
    /// * `u` - horizontal screen coordinate.
    /// * `v` - vertical screen coordinate.
    ///
    /// *Note*: Screen coordinates are assumed to be between 0.0 and 1.0
    /// inclusive.
    pub fn get_ray(&self, u: f64, v: f64) -> Ray {
        let direction = self.corner + self.horizontal * u + self.vertical * v - self.look_from;

        Ray {
            origin: self.look_from,
            direction,
        }
    }

    /// Returns the camera position.
    pub fn look_from(&self) -> Vec3 {
        self.look_from
    }

    /// Returns the screen corner position.
    pub fn corner(&self) -> Vec3 {
        self.corner
    }

    /// Returns the horizontal screen vector.
    pub fn horizontal(&self) -> Vec3 {
        self.horizontal
    }

    /// Returns the vertical screen vector.
    pub fn vertical(&self) -> Vec3 {
        self.vertical
    }

    /// Returns the vertical field of view in degrees.
    pub fn vfov(&self) -> f64 {
        // vertical.length() = 2 * half_height * focus_distance, half_height = tan(fov/2)
        let half_height = self.vertical.length() / (2.0 * self.focus_distance);
        2.0 * half_height.atan().to_degrees()
    }
}
