//! Camera module.

use super::ray::Ray;
use super::Vec3;

/// The objct that defines where to look from and where to inside the scene.
#[derive(Debug, Clone)]
pub struct Camera {
    look_from: Vec3,
    corner: Vec3,
    horizontal: Vec3,
    vertical: Vec3,
    u: Vec3,
    v: Vec3,
    w: Vec3,
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
        // vertical.length() = 2 * half_height, and half_height = tan(fov/2)
        let half_height = self.vertical.length() / 2.0;
        2.0 * half_height.atan().to_degrees()
    }
}
