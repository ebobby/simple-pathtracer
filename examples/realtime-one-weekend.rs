//! Real-time One Weekend Scene
//!
//! The iconic scene from Peter Shirley's "Ray Tracing in One Weekend" book,
//! rendered in real-time with interactive camera controls.
//!
//! Controls:
//! - WASD: Move camera
//! - Mouse (left click + drag): Look around
//! - Space/Shift: Move up/down
//! - Escape: Exit

use pathtracer::shape::*;
use pathtracer::Camera;
use pathtracer::Color;
use pathtracer::GPUShape;
use pathtracer::Material;
use pathtracer::Texture;
use pathtracer::Vec3;

use rand::Rng;
use rand::SeedableRng;

fn build_shapes() -> Vec<GPUShape> {
    // Use seeded RNG for reproducible scene generation
    let mut rng = rand::rngs::StdRng::seed_from_u64(42);

    let mut shapes: Vec<GPUShape> = Vec::new();
    let radius = 0.2f64;

    // Ground
    shapes.push(GPUShape::Sphere(Sphere {
        center: Vec3::new(0.0, -1000.0, 0.0),
        radius: 1000.0,
        material: Material::lambertian(Texture::constant_color(Color::new(0.5, 0.5, 0.5))),
    }));

    // Random small spheres
    for a in -11..11 {
        for b in -11..11 {
            let choose_mat: f64 = rng.gen();

            let center = Vec3::new(
                f64::from(a) + 0.9 * rng.gen::<f64>(),
                0.2,
                f64::from(b) + 0.9 * rng.gen::<f64>(),
            );

            if (center - Vec3::new(4.0, 0.2, 0.0)).length() > 0.9 {
                if choose_mat < 0.8 {
                    // Diffuse
                    shapes.push(GPUShape::Sphere(Sphere {
                        center,
                        radius,
                        material: Material::lambertian(Texture::constant_color(Color::new(
                            rng.gen::<f64>() * rng.gen::<f64>(),
                            rng.gen::<f64>() * rng.gen::<f64>(),
                            rng.gen::<f64>() * rng.gen::<f64>(),
                        ))),
                    }));
                } else if choose_mat < 0.95 {
                    // Metal
                    shapes.push(GPUShape::Sphere(Sphere {
                        center,
                        radius,
                        material: Material::metal(
                            Texture::constant_color(Color::new(
                                0.5 * (1.0 + rng.gen::<f64>()),
                                0.5 * (1.0 + rng.gen::<f64>()),
                                0.5 * (1.0 + rng.gen::<f64>()),
                            )),
                            0.5 * rng.gen::<f64>(),
                        ),
                    }));
                } else {
                    // Glass
                    shapes.push(GPUShape::Sphere(Sphere {
                        center,
                        radius,
                        material: Material::dielectric(
                            Texture::constant_color(Color::new(1.0, 1.0, 1.0)),
                            1.5,
                        ),
                    }));
                }
            }
        }
    }

    // Three hero spheres
    shapes.push(GPUShape::Sphere(Sphere {
        center: Vec3::new(0.0, 1.0, 0.0),
        radius: 1.0,
        material: Material::dielectric(Texture::constant_color(Color::new(1.0, 1.0, 1.0)), 1.5),
    }));
    shapes.push(GPUShape::Sphere(Sphere {
        center: Vec3::new(-4.0, 1.0, 0.0),
        radius: 1.0,
        material: Material::lambertian(Texture::constant_color(Color::new(0.4, 0.2, 0.1))),
    }));
    shapes.push(GPUShape::Sphere(Sphere {
        center: Vec3::new(4.0, 1.0, 0.0),
        radius: 1.0,
        material: Material::metal(Texture::constant_color(Color::new(0.7, 0.6, 0.5)), 0.0),
    }));

    // Sky light (large emissive sphere surrounding the scene)
    shapes.push(GPUShape::Sphere(Sphere {
        center: Vec3::new(0.0, 0.0, 0.0),
        radius: 5000.0,
        material: Material::diffuse_light(Texture::constant_color(Color::new(0.5, 0.7, 1.0))),
    }));

    shapes
}

fn build_camera(aspect_ratio: f64) -> Camera {
    let look_from = Vec3::new(13.0, 2.0, 3.0);
    let look_at = Vec3::new(0.0, 0.0, 0.0);
    Camera::new(look_from, look_at, 20.0, aspect_ratio, 0.0)
}

fn main() {
    let width = 1200;
    let height = 800;
    let gamma = 2.2f64;
    let aspect_ratio = f64::from(width) / f64::from(height);

    let shapes = build_shapes();
    let camera = build_camera(aspect_ratio);

    pathtracer::render_realtime(shapes, &camera, width, height, gamma);
}
