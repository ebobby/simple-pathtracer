//! Mirror Hall - Showcases metal reflections with varying roughness
//!
//! A corridor of mirrors with spheres of different metallic finishes,
//! demonstrating the fuzz parameter from perfectly smooth to rough.
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
    let white = Color::new(0.9, 0.9, 0.9);
    let dark_gray = Color::new(0.15, 0.15, 0.15);
    let gold = Color::new(1.0, 0.84, 0.0);
    let copper = Color::new(0.72, 0.45, 0.20);
    let silver = Color::new(0.95, 0.93, 0.88);
    let chrome = Color::new(0.55, 0.55, 0.55);
    let light = Color::new(1.0, 0.95, 0.9) * 12.0;

    let mut spheres = vec![
        // Floor - dark reflective
        Sphere {
            center: Vec3::new(0.0, -5000.0, 0.0),
            radius: 5000.0,
            material: Material::metal(Texture::constant_color(dark_gray), 0.05),
        },
        // Ceiling
        Sphere {
            center: Vec3::new(0.0, 5010.0, 0.0),
            radius: 5000.0,
            material: Material::lambertian(Texture::constant_color(white)),
        },
        // Left mirror wall
        Sphere {
            center: Vec3::new(-5004.0, 0.0, 0.0),
            radius: 5000.0,
            material: Material::metal(Texture::constant_color(silver), 0.0),
        },
        // Right mirror wall
        Sphere {
            center: Vec3::new(5004.0, 0.0, 0.0),
            radius: 5000.0,
            material: Material::metal(Texture::constant_color(silver), 0.0),
        },
        // Back wall
        Sphere {
            center: Vec3::new(0.0, 0.0, -5030.0),
            radius: 5000.0,
            material: Material::metal(Texture::constant_color(chrome), 0.1),
        },
    ];

    let discs = vec![
        // Ceiling light strip
        Disc {
            center: Vec3::new(0.0, 9.9, -8.0),
            radius: 1.5,
            normal: Vec3::new(0.0, -1.0, 0.0),
            material: Material::diffuse_light(Texture::constant_color(light)),
        },
        Disc {
            center: Vec3::new(0.0, 9.9, -16.0),
            radius: 1.5,
            normal: Vec3::new(0.0, -1.0, 0.0),
            material: Material::diffuse_light(Texture::constant_color(light)),
        },
    ];

    // Row of spheres with increasing fuzz (roughness)
    let fuzz_values = [0.0, 0.05, 0.15, 0.3, 0.5];
    let colors = [gold, silver, copper, chrome, gold];

    for (i, (&fuzz, &color)) in fuzz_values.iter().zip(colors.iter()).enumerate() {
        let z = -6.0 - (i as f64) * 5.0;
        spheres.push(Sphere {
            center: Vec3::new(0.0, 1.5, z),
            radius: 1.5,
            material: Material::metal(Texture::constant_color(color), fuzz),
        });
    }

    // Small accent spheres along the sides
    for i in 0..4 {
        let z = -5.0 - (i as f64) * 6.0;
        // Left side - gold
        spheres.push(Sphere {
            center: Vec3::new(-2.5, 0.5, z),
            radius: 0.5,
            material: Material::metal(Texture::constant_color(gold), 0.0),
        });
        // Right side - copper
        spheres.push(Sphere {
            center: Vec3::new(2.5, 0.5, z),
            radius: 0.5,
            material: Material::metal(Texture::constant_color(copper), 0.0),
        });
    }

    (spheres, discs)
}

fn build_camera(aspect_ratio: f64) -> Camera {
    let look_from = Vec3::new(0.0, 3.0, 8.0);
    let look_at = Vec3::new(0.0, 2.0, -10.0);
    Camera::new(look_from, look_at, 50.0, aspect_ratio, 0.0)
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

    let width = 800;
    let height = 600;
    let samples = 2000;
    let aspect_ratio = f64::from(width) / f64::from(height);
    let gamma = 2.2f64;
    let max_depth = 50;
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
            "output/mirror-hall-gpu.png",
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
            "output/mirror-hall.png",
        );
    }
}
