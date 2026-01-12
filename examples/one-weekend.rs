//! Ray Tracing in One Weekend - Classic random spheres scene
//!
//! The iconic scene from Peter Shirley's "Ray Tracing in One Weekend" book.
//! Features hundreds of randomly placed spheres with various materials.
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

use rand::SeedableRng;
use rand::Rng;

fn build_shapes() -> (Vec<Sphere>, Vec<Disc>) {
    // Use seeded RNG for reproducible scene generation
    let mut rng = rand::rngs::StdRng::seed_from_u64(42);

    let mut spheres: Vec<Sphere> = Vec::new();
    let radius = 0.2f64;

    // Ground
    spheres.push(Sphere {
        center: Vec3::new(0.0, -1000.0, 0.0),
        radius: 1000.0,
        material: Material::lambertian(Texture::constant_color(Color::new(0.5, 0.5, 0.5))),
    });

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
                    spheres.push(Sphere {
                        center,
                        radius,
                        material: Material::lambertian(Texture::constant_color(Color::new(
                            rng.gen::<f64>() * rng.gen::<f64>(),
                            rng.gen::<f64>() * rng.gen::<f64>(),
                            rng.gen::<f64>() * rng.gen::<f64>(),
                        ))),
                    });
                } else if choose_mat < 0.95 {
                    // Metal
                    spheres.push(Sphere {
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
                    });
                } else {
                    // Glass
                    spheres.push(Sphere {
                        center,
                        radius,
                        material: Material::dielectric(
                            Texture::constant_color(Color::new(1.0, 1.0, 1.0)),
                            1.5,
                        ),
                    });
                }
            }
        }
    }

    // Three hero spheres
    spheres.push(Sphere {
        center: Vec3::new(0.0, 1.0, 0.0),
        radius: 1.0,
        material: Material::dielectric(Texture::constant_color(Color::new(1.0, 1.0, 1.0)), 1.5),
    });
    spheres.push(Sphere {
        center: Vec3::new(-4.0, 1.0, 0.0),
        radius: 1.0,
        material: Material::lambertian(Texture::constant_color(Color::new(0.4, 0.2, 0.1))),
    });
    spheres.push(Sphere {
        center: Vec3::new(4.0, 1.0, 0.0),
        radius: 1.0,
        material: Material::metal(Texture::constant_color(Color::new(0.7, 0.6, 0.5)), 0.0),
    });

    // Sky light (large emissive sphere surrounding the scene)
    spheres.push(Sphere {
        center: Vec3::new(0.0, 0.0, 0.0),
        radius: 5000.0,
        material: Material::diffuse_light(Texture::constant_color(Color::new(0.5, 0.7, 1.0))),
    });

    let discs = vec![]; // No discs in this scene

    (spheres, discs)
}

fn build_camera(aspect_ratio: f64) -> Camera {
    let look_from = Vec3::new(13.0, 2.0, 3.0);
    let look_at = Vec3::new(0.0, 0.0, 0.0);
    Camera::new(look_from, look_at, 20.0, aspect_ratio, 0.0)
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

    let width = 1200;
    let height = 800;
    let samples = 1000;
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
            "output/one-weekend-gpu.png",
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
            "output/one-weekend.png",
        );
    }
}
