//! Glass Marbles - Showcases dielectric materials with different refractive indices
//!
//! A collection of glass spheres demonstrating how different IOR values affect
//! refraction. Features colored glass and clear glass with varying indices.
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
    let white = Color::new(0.85, 0.85, 0.85);
    let soft_blue = Color::new(0.7, 0.8, 0.95);
    let warm_light = Color::new(1.0, 0.95, 0.85) * 15.0;
    let accent_light = Color::new(0.9, 0.95, 1.0) * 8.0;

    // Glass colors (tinted)
    let clear = Color::new(1.0, 1.0, 1.0);
    let amber = Color::new(1.0, 0.85, 0.6);
    let emerald = Color::new(0.6, 1.0, 0.7);
    let sapphire = Color::new(0.7, 0.8, 1.0);
    let ruby = Color::new(1.0, 0.7, 0.75);

    let mut spheres = vec![
        // Floor - slightly reflective white
        Sphere {
            center: Vec3::new(0.0, -5000.0, 0.0),
            radius: 5000.0,
            material: Material::lambertian(Texture::constant_color(white)),
        },
        // Back wall - soft blue gradient feel
        Sphere {
            center: Vec3::new(0.0, 0.0, -5015.0),
            radius: 5000.0,
            material: Material::lambertian(Texture::constant_color(soft_blue)),
        },
        // Central large clear glass sphere (crown glass IOR 1.52)
        Sphere {
            center: Vec3::new(0.0, 2.5, -3.0),
            radius: 2.5,
            material: Material::dielectric(Texture::constant_color(clear), 1.52),
        },
        // A metal sphere for contrast
        Sphere {
            center: Vec3::new(5.0, 1.5, -5.0),
            radius: 1.5,
            material: Material::metal(
                Texture::constant_color(Color::new(0.9, 0.9, 0.95)),
                0.0,
            ),
        },
    ];

    let discs = vec![
        // Main overhead light
        Disc {
            center: Vec3::new(0.0, 12.0, 0.0),
            radius: 3.0,
            normal: Vec3::new(0.0, -1.0, 0.0),
            material: Material::diffuse_light(Texture::constant_color(warm_light)),
        },
        // Side accent light
        Disc {
            center: Vec3::new(-8.0, 8.0, -3.0),
            radius: 1.0,
            normal: Vec3::new(1.0, -0.5, 0.0),
            material: Material::diffuse_light(Texture::constant_color(accent_light)),
        },
    ];

    // Front row - increasing IOR demonstration
    let front_iors = [1.3, 1.45, 1.7, 2.0]; // Water, glass, flint glass, diamond-like
    let front_colors = [sapphire, clear, amber, clear];
    for (i, (&ior, &color)) in front_iors.iter().zip(front_colors.iter()).enumerate() {
        let x = -4.5 + (i as f64) * 3.0;
        spheres.push(Sphere {
            center: Vec3::new(x, 1.0, 2.0),
            radius: 1.0,
            material: Material::dielectric(Texture::constant_color(color), ior),
        });
    }

    // Back row - colored glass collection
    let back_colors = [ruby, emerald, amber, sapphire, ruby];
    for (i, &color) in back_colors.iter().enumerate() {
        let x = -6.0 + (i as f64) * 3.0;
        spheres.push(Sphere {
            center: Vec3::new(x, 1.2, -8.0),
            radius: 1.2,
            material: Material::dielectric(Texture::constant_color(color), 1.52),
        });
    }

    // Small scattered marbles
    let small_positions = [
        (3.5, 0.4, 0.5, emerald, 1.5),
        (-3.0, 0.35, 1.0, ruby, 1.45),
        (5.0, 0.5, -2.0, sapphire, 1.6),
        (-5.5, 0.45, -1.0, amber, 1.55),
        (1.5, 0.3, 3.5, clear, 2.4), // Diamond IOR
        (-1.5, 0.35, 3.0, emerald, 1.52),
    ];

    for (x, r, z, color, ior) in small_positions {
        spheres.push(Sphere {
            center: Vec3::new(x, r, z),
            radius: r,
            material: Material::dielectric(Texture::constant_color(color), ior),
        });
    }

    (spheres, discs)
}

fn build_camera(aspect_ratio: f64) -> Camera {
    let look_from = Vec3::new(0.0, 6.0, 14.0);
    let look_at = Vec3::new(0.0, 1.5, -2.0);
    Camera::new(look_from, look_at, 40.0, aspect_ratio, 0.0)
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
    let samples = 5000;
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
            "output/glass-marbles-gpu.png",
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
            "output/glass-marbles.png",
        );
    }
}
