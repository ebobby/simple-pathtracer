//! Outdoor - Principled materials under a sky and a sun
//!
//! Principled spheres (matte, glossy plastic, brushed gold, chrome, glass)
//! on a ground plane, lit by a constant sky and a low sun, shot with a thin
//! lens focused on the gold sphere and finished with ACES and bloom.
//!
//! Run with --gpu flag for GPU rendering.

use pathtracer::Principled;
use pathtracer::shape::*;
use pathtracer::{
    Bloom, Camera, Color, Environment, GPUShape, Hitable, Material, Scene, Sky, Sun, Texture,
    ToneCurve, Tonemap, Vec3, BVH,
};

fn principled(color: Color) -> Principled {
    Principled::new(Texture::constant_color(color))
}

fn build_shapes() -> Vec<Sphere> {
    let mut spheres = vec![
        // Ground: slightly rough, dull green-grey
        Sphere {
            center: Vec3::new(0.0, -5000.0, 0.0),
            radius: 5000.0,
            material: Material::Principled(
                principled(Color::new(0.35, 0.4, 0.3)).roughness(0.9),
            ),
        },
        // Brushed gold: the subject, in focus
        Sphere {
            center: Vec3::new(0.0, 1.0, 0.0),
            radius: 1.0,
            material: Material::Principled(
                principled(Color::new(1.0, 0.78, 0.35)).metallic(1.0).roughness(0.35),
            ),
        },
        // Large matte clay, behind and left
        Sphere {
            center: Vec3::new(-3.2, 1.6, -2.5),
            radius: 1.6,
            material: Material::Principled(principled(Color::new(0.8, 0.35, 0.25)).roughness(1.0)),
        },
        // Chrome, behind and right, overlapping the gold from this angle
        Sphere {
            center: Vec3::new(2.6, 1.4, -1.8),
            radius: 1.4,
            material: Material::Principled(
                principled(Color::new(0.9, 0.9, 0.92)).metallic(1.0).roughness(0.12),
            ),
        },
        // Tinted glass, in front of the chrome
        Sphere {
            center: Vec3::new(1.9, 0.8, 1.2),
            radius: 0.8,
            material: Material::Principled(
                principled(Color::new(0.75, 0.95, 0.85)).transmission(1.0).roughness(0.05).ior(1.5),
            ),
        },
        // Small glossy blue plastic, front left
        Sphere {
            center: Vec3::new(-1.7, 0.5, 1.6),
            radius: 0.5,
            material: Material::Principled(principled(Color::new(0.15, 0.3, 0.85)).roughness(0.25)),
        },
        // Frosted glass close to the camera, well outside the focus plane
        Sphere {
            center: Vec3::new(0.9, 0.35, 3.4),
            radius: 0.35,
            material: Material::Principled(
                principled(Color::new(1.0, 1.0, 1.0)).transmission(1.0).roughness(0.2).ior(1.5),
            ),
        },
    ];

    // A scatter of small spheres receding into the distance
    let far = [
        (Vec3::new(-6.0, 0.4, -6.0), 0.4, Color::new(0.9, 0.6, 0.2), 0.0, 0.6),
        (Vec3::new(-2.0, 0.3, -7.0), 0.3, Color::new(0.3, 0.7, 0.4), 0.0, 0.9),
        (Vec3::new(1.5, 0.5, -9.0), 0.5, Color::new(0.85, 0.85, 0.9), 1.0, 0.2),
        (Vec3::new(5.5, 0.35, -7.5), 0.35, Color::new(0.7, 0.2, 0.5), 0.0, 0.4),
        (Vec3::new(8.0, 0.6, -12.0), 0.6, Color::new(0.95, 0.8, 0.3), 1.0, 0.45),
        (Vec3::new(-9.0, 0.5, -11.0), 0.5, Color::new(0.2, 0.4, 0.8), 0.0, 0.7),
    ];
    for (center, radius, color, metallic, roughness) in far {
        spheres.push(Sphere {
            center,
            radius,
            material: Material::Principled(
                principled(color).metallic(metallic).roughness(roughness),
            ),
        });
    }
    spheres
}

fn build_environment() -> Environment {
    Environment::new()
        .sky(Sky::Constant(Color::new(0.45, 0.6, 0.9)))
        // A soft sun (0.1 rad) with irradiance about six times the sky's:
        // L * 2π(1 - cos 0.1) ≈ 11 against π * 0.6 ≈ 1.9 for the sky. A wider
        // cone keeps caustics from the chrome and glass from turning into
        // isolated bright dots.
        .sun(Sun::new(
            Vec3::new(-0.55, 0.45, -0.5),
            Color::new(360.0, 340.0, 290.0),
            0.1,
        ))
}

fn build_camera(aspect_ratio: f64) -> Camera {
    let look_from = Vec3::new(-0.8, 1.3, 7.5);
    let look_at = Vec3::new(0.0, 0.9, 0.0);
    // Focus on the gold sphere; a wide aperture blurs the near frosted
    // glass and the distant spheres.
    let focus_distance = (look_at - look_from).length();
    Camera::new(look_from, look_at, 38.0, aspect_ratio, 0.0).with_lens(0.35, focus_distance)
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let use_gpu = args.iter().any(|arg| arg == "--gpu");

    let width = 800;
    let height = 600;
    let samples = 1000;
    let aspect_ratio = f64::from(width) / f64::from(height);
    let max_depth = 30;
    // The sunlit ground reflects a radiance near 1.6; a quarter-exposure
    // puts it around mid-grey and ACES keeps the highlights from clipping.
    let tonemap = Tonemap::new(0.25, ToneCurve::Aces).with_bloom(Bloom {
        threshold: 1.0,
        intensity: 0.2,
        radius: 0.03,
    });
    let workers = std::thread::available_parallelism().map(|n| n.get()).unwrap_or(4);

    let spheres = build_shapes();
    let camera = build_camera(aspect_ratio);
    let environment = build_environment();

    if use_gpu {
        let gpu_shapes: Vec<GPUShape> = spheres.into_iter().map(GPUShape::Sphere).collect();
        pathtracer::render_gpu_with(
            gpu_shapes,
            &camera,
            &environment,
            &tonemap,
            width,
            height,
            samples,
            max_depth,
            "output/outdoor-gpu.png",
        );
    } else {
        let objects: Vec<Hitable> = spheres
            .into_iter()
            .map(|s| Box::new(s) as Hitable)
            .collect();
        let scene = Scene::new(camera, BVH::from_vec(objects)).with_environment(environment);
        pathtracer::render_with_tonemap(
            scene,
            width,
            height,
            samples,
            max_depth,
            workers,
            &tonemap,
            "output/outdoor.png",
        );
    }
}
