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

/// Build shapes for the Cornell box scene.
/// Returns (spheres, discs) as separate vectors for GPU compatibility.
fn build_shapes() -> (Vec<Sphere>, Vec<Disc>) {
    let red = Color::new(0.75, 0.25, 0.25);
    let white = Color::new(0.75, 0.75, 0.75);
    let blue = Color::new(0.25, 0.25, 0.75);
    let light = Color::new(0.9373, 0.9216, 0.8471) * 23.0;

    let spheres = vec![
        // right wall
        Sphere {
            center: Vec3::new(5006.0, 0.0, 0.0),
            radius: 5000.0,
            material: Material::lambertian(Texture::constant_color(blue)),
        },
        // left wall
        Sphere {
            center: Vec3::new(-5006.0, 0.0, 0.0),
            radius: 5000.0,
            material: Material::lambertian(Texture::constant_color(red)),
        },
        // ceiling
        Sphere {
            center: Vec3::new(0.0, 5010.0, 0.0),
            radius: 5000.0,
            material: Material::lambertian(Texture::constant_color(white)),
        },
        // floor
        Sphere {
            center: Vec3::new(0.0, -5000.0, 0.0),
            radius: 5000.0,
            material: Material::lambertian(Texture::constant_color(white)),
        },
        // back wall
        Sphere {
            center: Vec3::new(0.0, 0.0, -5010.0),
            radius: 5000.0,
            material: Material::lambertian(Texture::constant_color(white)),
        },
        // glass sphere
        Sphere {
            center: Vec3::new(-3.5, 2.0, -3.0),
            radius: 2.0,
            material: Material::dielectric(
                Texture::constant_color(Color::new(1.0, 1.0, 1.0)),
                1.52,
            ),
        },
        // gold metal sphere
        Sphere {
            center: Vec3::new(3.5, 2.0, -7.0),
            radius: 2.0,
            material: Material::metal(Texture::constant_color(Color::new(0.95, 0.82, 0.42)), 0.25),
        },
        // red metal sphere
        Sphere {
            center: Vec3::new(3.8, 2.0, -1.5),
            radius: 2.0,
            material: Material::metal(Texture::constant_color(Color::new(1.0, 0.05, 0.05)), 0.0),
        },
        // green metal sphere
        Sphere {
            center: Vec3::new(0.0, 1.2, -7.8),
            radius: 1.2,
            material: Material::metal(Texture::constant_color(Color::new(0.05, 1.0, 0.05)), 0.25),
        },
        // purple metal sphere
        Sphere {
            center: Vec3::new(0.0, 7.5, -5.0),
            radius: 1.8,
            material: Material::metal(Texture::constant_color(Color::new(0.52, 0.05, 0.52)), 0.0),
        },
    ];

    let discs = vec![
        // light
        Disc {
            center: Vec3::new(0.0, 0.0, -5.0),
            radius: 1.5,
            normal: Vec3::new(0.0, 1.0, 0.0),
            material: Material::diffuse_light(Texture::constant_color(light)),
        },
    ];

    (spheres, discs)
}

fn build_camera(aspect_ratio: f64) -> Camera {
    let look_from = Vec3::new(0.0, 9.95, 8.0);
    let look_at = Vec3::new(0.0, 3.0, -5.0);
    Camera::new(look_from, look_at, 55.0, aspect_ratio, 0.0)
}

fn cornell_box_cpu(spheres: Vec<Sphere>, discs: Vec<Disc>, camera: Camera) -> Scene {
    let mut objects: Vec<Hitable> = Vec::new();

    for disc in discs {
        objects.push(Box::new(disc));
    }

    for sphere in spheres {
        objects.push(Box::new(sphere));
    }

    Scene {
        camera,
        world: BVH::from_vec(objects),
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let use_gpu = args.iter().any(|arg| arg == "--gpu");

    let width = 640;
    let height = 480;
    let samples = 4000;
    let aspect_ratio = f64::from(width) / f64::from(height);
    let gamma = 2.2f64;
    let max_depth = 50;
    let workers: usize = 12;

    let (spheres, discs) = build_shapes();
    let camera = build_camera(aspect_ratio);

    if use_gpu {
        // GPU rendering
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
            "output/inverted-light-cornell-gpu.png",
        );
    } else {
        // CPU rendering
        let scene = cornell_box_cpu(spheres, discs, camera);

        pathtracer::render(
            scene,
            width,
            height,
            samples,
            max_depth,
            gamma,
            workers,
            "output/inverted-light-cornell.png",
        );
    }
}
