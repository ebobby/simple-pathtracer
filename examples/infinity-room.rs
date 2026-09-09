//! Infinity Room - Showcases infinite reflections between parallel mirrors
//!
//! An homage to Yayoi Kusama's infinity mirror rooms. Features parallel
//! mirror walls with floating light spheres, creating endless reflections.
//!
//! Run with --gpu flag for GPU rendering.

use pathtracer::shape::*;
use pathtracer::Camera;
use pathtracer::Color;
use pathtracer::GPUShape;
use pathtracer::Hitable;
use pathtracer::Material;
use pathtracer::Scene;
use pathtracer::Texture;
use pathtracer::Vec3;
use pathtracer::BVH;

fn build_shapes() -> (Vec<Sphere>, Vec<Disc>) {
    let mirror = Color::new(0.98, 0.98, 0.98);
    let dark = Color::new(0.02, 0.02, 0.02);

    let mut spheres = vec![
        // Floor - mirror
        Sphere {
            center: Vec3::new(0.0, -5000.0, 0.0),
            radius: 5000.0,
            material: Material::metal(Texture::constant_color(mirror), 0.0),
        },
        // Ceiling - mirror
        Sphere {
            center: Vec3::new(0.0, 5012.0, 0.0),
            radius: 5000.0,
            material: Material::metal(Texture::constant_color(mirror), 0.0),
        },
        // Left wall - mirror
        Sphere {
            center: Vec3::new(-5008.0, 0.0, 0.0),
            radius: 5000.0,
            material: Material::metal(Texture::constant_color(mirror), 0.0),
        },
        // Right wall - mirror
        Sphere {
            center: Vec3::new(5008.0, 0.0, 0.0),
            radius: 5000.0,
            material: Material::metal(Texture::constant_color(mirror), 0.0),
        },
        // Back wall - mirror
        Sphere {
            center: Vec3::new(0.0, 0.0, -5015.0),
            radius: 5000.0,
            material: Material::metal(Texture::constant_color(mirror), 0.0),
        },
        // Front wall (behind camera) - dark to avoid blinding reflections
        Sphere {
            center: Vec3::new(0.0, 0.0, 5020.0),
            radius: 5000.0,
            material: Material::lambertian(Texture::constant_color(dark)),
        },
        // A few glass spheres to add visual interest
        Sphere {
            center: Vec3::new(-1.5, 1.5, -6.0),
            radius: 1.5,
            material: Material::dielectric(Texture::constant_color(Color::new(1.0, 1.0, 1.0)), 1.5),
        },
        Sphere {
            center: Vec3::new(2.0, 1.0, -9.0),
            radius: 1.0,
            material: Material::dielectric(Texture::constant_color(Color::new(0.95, 0.95, 1.0)), 1.5),
        },
        // Chrome sphere for contrast
        Sphere {
            center: Vec3::new(0.0, 3.0, -10.0),
            radius: 1.2,
            material: Material::metal(Texture::constant_color(Color::new(0.9, 0.9, 0.95)), 0.0),
        },
    ];

    // Floating light spheres - different colors and positions
    // These will be reflected infinitely in the mirrors
    // Lower intensities to prevent overexposure in mirror room
    let lights = [
        // Warm colors
        (0.0, 6.0, -5.0, 0.8, Color::new(1.0, 0.6, 0.2) * 3.0),     // Orange
        (-3.0, 3.0, -3.0, 0.5, Color::new(1.0, 0.2, 0.3) * 3.5),    // Red
        (3.0, 4.0, -7.0, 0.6, Color::new(1.0, 0.9, 0.3) * 2.5),     // Yellow
        // Cool colors
        (-2.0, 8.0, -8.0, 0.4, Color::new(0.3, 0.5, 1.0) * 4.0),    // Blue
        (2.5, 2.0, -4.0, 0.45, Color::new(0.5, 0.2, 1.0) * 3.5),    // Purple
        (0.0, 10.0, -3.0, 0.35, Color::new(0.2, 1.0, 0.8) * 3.0),   // Cyan
        // White accents
        (-4.0, 5.0, -6.0, 0.3, Color::new(1.0, 1.0, 1.0) * 4.0),    // White
        (4.0, 7.0, -4.0, 0.25, Color::new(1.0, 1.0, 1.0) * 5.0),    // White
        // More scattered lights
        (-5.0, 1.5, -10.0, 0.4, Color::new(1.0, 0.4, 0.7) * 3.0),   // Pink
        (5.0, 9.0, -9.0, 0.35, Color::new(0.4, 1.0, 0.4) * 3.5),    // Green
        (0.0, 0.8, -12.0, 0.5, Color::new(1.0, 0.5, 0.0) * 2.5),    // Amber
        (-6.0, 11.0, -7.0, 0.3, Color::new(0.8, 0.3, 1.0) * 4.0),   // Violet
        (6.0, 2.5, -11.0, 0.4, Color::new(0.2, 0.8, 1.0) * 3.0),    // Sky blue
    ];

    for (x, y, z, r, color) in lights {
        spheres.push(Sphere {
            center: Vec3::new(x, y, z),
            radius: r,
            material: Material::diffuse_light(Texture::constant_color(color)),
        });
    }

    let discs = vec![]; // No discs in this scene

    (spheres, discs)
}

fn build_camera(aspect_ratio: f64) -> Camera {
    let look_from = Vec3::new(0.0, 5.0, 10.0);
    let look_at = Vec3::new(0.0, 5.0, -5.0);
    Camera::new(look_from, look_at, 60.0, aspect_ratio, 0.0)
}

fn build_scene_cpu(spheres: Vec<Sphere>, discs: Vec<Disc>, camera: Camera) -> Scene {
    let mut objects: Vec<Hitable> = Vec::new();
    for sphere in spheres {
        objects.push(Box::new(sphere));
    }
    for disc in discs {
        objects.push(Box::new(disc));
    }
    Scene {
        camera,
        world: BVH::from_vec(objects),
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let use_gpu = args.iter().any(|arg| arg == "--gpu");

    let width = 1280;
    let height = 720;
    let samples = 6000; // Higher samples needed for many bounces
    let aspect_ratio = f64::from(width) / f64::from(height);
    let gamma = 2.2f64;
    let max_depth = 100; // High depth for infinite reflections
    let workers: usize = 12;

    let (spheres, discs) = build_shapes();
    let camera = build_camera(aspect_ratio);

    if use_gpu {
        let mut gpu_shapes: Vec<GPUShape> = Vec::new();
        for sphere in spheres {
            gpu_shapes.push(GPUShape::Sphere(sphere));
        }
        for disc in discs {
            gpu_shapes.push(GPUShape::Disc(disc));
        }

        pathtracer::render_gpu(
            gpu_shapes,
            &camera,
            width,
            height,
            samples,
            max_depth,
            gamma,
            "output/infinity-room-gpu.png",
        );
    } else {
        let scene = build_scene_cpu(spheres, discs, camera);

        pathtracer::render(
            scene,
            width,
            height,
            samples,
            max_depth,
            gamma,
            workers,
            "output/infinity-room.png",
        );
    }
}
