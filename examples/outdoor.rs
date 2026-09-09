//! Outdoor - Principled materials under a sky and a sun
//!
//! A ground plane and a row of spheres with different principled settings
//! (matte, glossy plastic, brushed gold, chrome, glass), lit by a constant
//! sky and a low sun. Exercises the material model and the infinite lights.
//!
//! Run with --gpu flag for GPU rendering.

use pathtracer::shape::*;
use pathtracer::Principled;
use pathtracer::{
    Camera, Color, Environment, GPUShape, Hitable, Material, Scene, Sky, Sun, Texture, Vec3, BVH,
};

fn principled(color: Color) -> Principled {
    Principled::new(Texture::constant_color(color))
}

fn build_shapes() -> Vec<Sphere> {
    vec![
        // Ground: slightly rough, dull green-grey
        Sphere {
            center: Vec3::new(0.0, -5000.0, 0.0),
            radius: 5000.0,
            material: Material::Principled(
                principled(Color::new(0.35, 0.4, 0.3)).roughness(0.9),
            ),
        },
        // Matte clay
        Sphere {
            center: Vec3::new(-4.5, 1.0, 0.0),
            radius: 1.0,
            material: Material::Principled(principled(Color::new(0.8, 0.35, 0.25)).roughness(1.0)),
        },
        // Glossy blue plastic
        Sphere {
            center: Vec3::new(-2.2, 1.0, 0.0),
            radius: 1.0,
            material: Material::Principled(principled(Color::new(0.15, 0.3, 0.85)).roughness(0.25)),
        },
        // Brushed gold
        Sphere {
            center: Vec3::new(0.0, 1.0, 0.0),
            radius: 1.0,
            material: Material::Principled(
                principled(Color::new(1.0, 0.78, 0.35)).metallic(1.0).roughness(0.35),
            ),
        },
        // Chrome
        Sphere {
            center: Vec3::new(2.2, 1.0, 0.0),
            radius: 1.0,
            material: Material::Principled(
                principled(Color::new(0.9, 0.9, 0.92)).metallic(1.0).roughness(0.12),
            ),
        },
        // Tinted glass
        Sphere {
            center: Vec3::new(4.5, 1.0, 0.0),
            radius: 1.0,
            material: Material::Principled(
                principled(Color::new(0.75, 0.95, 0.85)).transmission(1.0).roughness(0.05).ior(1.5),
            ),
        },
        // Small frosted glass in front
        Sphere {
            center: Vec3::new(1.0, 0.4, 2.2),
            radius: 0.4,
            material: Material::Principled(
                principled(Color::new(1.0, 1.0, 1.0)).transmission(1.0).roughness(0.35).ior(1.5),
            ),
        },
    ]
}

fn build_environment() -> Environment {
    Environment::new()
        .sky(Sky::Constant(Color::new(0.45, 0.6, 0.9)))
        // A soft sun (0.1 rad) with irradiance about six times the sky's:
        // L * 2π(1 - cos 0.1) ≈ 11 against π * 0.6 ≈ 1.9 for the sky. A wider
        // cone keeps caustics from the chrome and glass from turning into
        // isolated bright dots.
        .sun(Sun::new(
            Vec3::new(-0.6, 0.5, 0.4),
            Color::new(360.0, 340.0, 290.0),
            0.1,
        ))
}

fn build_camera(aspect_ratio: f64) -> Camera {
    Camera::new(
        Vec3::new(0.0, 2.2, 10.0),
        Vec3::new(0.0, 0.9, 0.0),
        45.0,
        aspect_ratio,
        0.0,
    )
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let use_gpu = args.iter().any(|arg| arg == "--gpu");

    let width = 800;
    let height = 600;
    let samples = 200;
    let aspect_ratio = f64::from(width) / f64::from(height);
    let gamma = 2.2f64;
    let max_depth = 30;
    let workers = std::thread::available_parallelism().map(|n| n.get()).unwrap_or(4);

    let spheres = build_shapes();
    let camera = build_camera(aspect_ratio);
    let environment = build_environment();

    if use_gpu {
        let gpu_shapes: Vec<GPUShape> = spheres.into_iter().map(GPUShape::Sphere).collect();
        pathtracer::render_gpu_with_environment(
            gpu_shapes,
            &camera,
            &environment,
            width,
            height,
            samples,
            max_depth,
            gamma,
            "output/outdoor-gpu.png",
        );
    } else {
        let objects: Vec<Hitable> = spheres
            .into_iter()
            .map(|s| Box::new(s) as Hitable)
            .collect();
        let scene = Scene::new(camera, BVH::from_vec(objects)).with_environment(environment);
        pathtracer::render(
            scene,
            width,
            height,
            samples,
            max_depth,
            gamma,
            workers,
            "output/outdoor.png",
        );
    }
}
