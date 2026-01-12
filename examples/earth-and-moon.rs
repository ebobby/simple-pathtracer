//! Earth and Moon - Showcases bitmap textures
//!
//! A simple scene with Earth and Moon spheres using bitmap textures,
//! lit by a large area light representing the Sun.
//!
//! Run with --gpu flag for GPU rendering.
//! NOTE: GPU rendering does NOT support bitmap textures - spheres will appear white.
//! Use CPU rendering for proper texture display.

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
    let spheres = vec![
        // Earth
        Sphere {
            center: Vec3::new(-9.0, 0.0, 0.0),
            radius: 10.0,
            material: Material::lambertian(Texture::bitmap("examples/textures/earth.jpg")),
        },
        // Moon
        Sphere {
            center: Vec3::new(13.0, 0.0, 0.0),
            radius: 5.0,
            material: Material::lambertian(Texture::bitmap("examples/textures/moon.jpg")),
        },
    ];

    let discs = vec![
        // Sun (large area light)
        Disc {
            center: Vec3::new(1000.0, 0.0, 0.0),
            normal: Vec3::new(-1.0, 0.0, 0.0),
            radius: 1000.0,
            material: Material::diffuse_light(Texture::constant_color(
                Color::new(1.0, 0.90, 0.75) * 5.0,
            )),
        },
    ];

    (spheres, discs)
}

fn build_camera(aspect_ratio: f64) -> Camera {
    let look_from = Vec3::new(0.0, 20.0, 30.0);
    let look_at = Vec3::new(0.0, 0.0, 0.0);
    Camera::new(look_from, look_at, 45.0, aspect_ratio, 0.0)
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
    let samples = 3000;
    let aspect_ratio = f64::from(width) / f64::from(height);
    let gamma = 2.2f64;
    let max_depth = 50;
    let workers: usize = 12;

    let (spheres, discs) = build_shapes();
    let camera = build_camera(aspect_ratio);

    if use_gpu {
        println!("WARNING: Bitmap textures are not supported on GPU - spheres will appear white.");
        println!("Use CPU rendering for proper texture display.");
        println!();

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
            "output/earth-moon-gpu.png",
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
            "output/earth-moon.png",
        );
    }
}
