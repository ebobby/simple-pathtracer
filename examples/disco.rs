//! Disco - Showcases colored emissive lights and their interactions
//!
//! A vibrant scene with multiple colored light sources illuminating
//! reflective and diffuse surfaces, demonstrating color bleeding and
//! light mixing.
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
    let dark = Color::new(0.1, 0.1, 0.1);
    let chrome = Color::new(0.8, 0.8, 0.85);

    // Colored lights - high intensity
    let red_light = Color::new(1.0, 0.1, 0.1) * 30.0;
    let green_light = Color::new(0.1, 1.0, 0.2) * 30.0;
    let blue_light = Color::new(0.2, 0.3, 1.0) * 30.0;
    let magenta_light = Color::new(1.0, 0.1, 0.8) * 22.0;
    let cyan_light = Color::new(0.1, 0.9, 1.0) * 22.0;
    let yellow_light = Color::new(1.0, 0.9, 0.2) * 22.0;
    let white_light = Color::new(1.0, 1.0, 1.0) * 12.0;

    let mut spheres = vec![
        // Floor - reflective dark
        Sphere {
            center: Vec3::new(0.0, -5000.0, 0.0),
            radius: 5000.0,
            material: Material::metal(Texture::constant_color(dark), 0.1),
        },
        // Ceiling
        Sphere {
            center: Vec3::new(0.0, 5015.0, 0.0),
            radius: 5000.0,
            material: Material::lambertian(Texture::constant_color(dark)),
        },
        // Back wall
        Sphere {
            center: Vec3::new(0.0, 0.0, -5020.0),
            radius: 5000.0,
            material: Material::lambertian(Texture::constant_color(white)),
        },
        // Left wall
        Sphere {
            center: Vec3::new(-5012.0, 0.0, 0.0),
            radius: 5000.0,
            material: Material::lambertian(Texture::constant_color(white)),
        },
        // Right wall
        Sphere {
            center: Vec3::new(5012.0, 0.0, 0.0),
            radius: 5000.0,
            material: Material::lambertian(Texture::constant_color(white)),
        },
        // Central disco ball - highly reflective
        Sphere {
            center: Vec3::new(0.0, 8.0, -6.0),
            radius: 2.0,
            material: Material::metal(Texture::constant_color(chrome), 0.0),
        },
        // Glass spheres to show colored caustics
        Sphere {
            center: Vec3::new(0.0, 2.0, -5.0),
            radius: 2.0,
            material: Material::dielectric(Texture::constant_color(Color::new(1.0, 1.0, 1.0)), 1.5),
        },
        Sphere {
            center: Vec3::new(-2.5, 1.0, 0.0),
            radius: 1.0,
            material: Material::dielectric(Texture::constant_color(Color::new(1.0, 0.9, 0.95)), 1.5),
        },
        Sphere {
            center: Vec3::new(2.5, 1.0, 0.0),
            radius: 1.0,
            material: Material::dielectric(Texture::constant_color(Color::new(0.95, 0.95, 1.0)), 1.5),
        },
    ];

    let mut discs = vec![
        // Subtle white fill light
        Disc {
            center: Vec3::new(0.0, 14.8, -6.0),
            radius: 0.8,
            normal: Vec3::new(0.0, -1.0, 0.0),
            material: Material::diffuse_light(Texture::constant_color(white_light)),
        },
    ];

    // Ceiling lights - arranged in a pattern
    let ceiling_lights = [
        (-6.0, 14.5, -8.0, red_light),
        (0.0, 14.5, -8.0, green_light),
        (6.0, 14.5, -8.0, blue_light),
        (-4.0, 14.5, -3.0, magenta_light),
        (4.0, 14.5, -3.0, cyan_light),
        (0.0, 14.5, 2.0, yellow_light),
    ];

    for (x, y, z, color) in ceiling_lights {
        discs.push(Disc {
            center: Vec3::new(x, y, z),
            radius: 1.2,
            normal: Vec3::new(0.0, -1.0, 0.0),
            material: Material::diffuse_light(Texture::constant_color(color)),
        });
    }

    // Mirror spheres to catch and reflect the colored lights
    let mirror_positions = [
        (-5.0, 1.5, -4.0),
        (5.0, 1.5, -4.0),
        (-3.0, 1.0, -10.0),
        (3.0, 1.0, -10.0),
    ];

    for (x, r, z) in mirror_positions {
        spheres.push(Sphere {
            center: Vec3::new(x, r, z),
            radius: r,
            material: Material::metal(Texture::constant_color(chrome), 0.0),
        });
    }

    // Matte colored spheres to show color bleeding
    let matte_spheres = [
        (-7.0, 1.2, -8.0, Color::new(0.9, 0.2, 0.2)), // Red
        (7.0, 1.2, -8.0, Color::new(0.2, 0.2, 0.9)),  // Blue
        (0.0, 1.0, -14.0, Color::new(0.2, 0.9, 0.2)), // Green
    ];

    for (x, r, z, color) in matte_spheres {
        spheres.push(Sphere {
            center: Vec3::new(x, r, z),
            radius: r,
            material: Material::lambertian(Texture::constant_color(color)),
        });
    }

    // Small emissive accent spheres on the ground
    let accent_emissives = [
        (-8.0, 0.3, -2.0, red_light * 0.3),
        (8.0, 0.3, -2.0, blue_light * 0.3),
        (-4.0, 0.25, 3.0, green_light * 0.3),
        (4.0, 0.25, 3.0, magenta_light * 0.3),
    ];

    for (x, r, z, color) in accent_emissives {
        spheres.push(Sphere {
            center: Vec3::new(x, r, z),
            radius: r,
            material: Material::diffuse_light(Texture::constant_color(color)),
        });
    }

    (spheres, discs)
}

fn build_camera(aspect_ratio: f64) -> Camera {
    let look_from = Vec3::new(0.0, 5.0, 12.0);
    let look_at = Vec3::new(0.0, 3.0, -4.0);
    Camera::new(look_from, look_at, 55.0, aspect_ratio, 0.0)
}

fn build_scene_cpu(spheres: Vec<Sphere>, discs: Vec<Disc>, camera: Camera) -> Scene {
    let mut objects: Vec<Hitable> = Vec::new();
    for sphere in spheres {
        objects.push(Box::new(sphere));
    }
    for disc in discs {
        objects.push(Box::new(disc));
    }
    Scene::new(camera, BVH::from_vec(objects))
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let use_gpu = args.iter().any(|arg| arg == "--gpu");

    let width = 800;
    let height = 600;
    let samples = 800;
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
            "output/disco-gpu.png",
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
            "output/disco.png",
        );
    }
}
