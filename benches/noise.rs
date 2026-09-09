//! Noise benchmark: RMSE of each integrator against a converged reference at
//! an equal, low sample count. Run with `cargo bench --bench noise`.

use std::time::Instant;

use pathtracer::shape::{Disc, Sphere};
use pathtracer::{Camera, Color, Hitable, Integrator, Material, Scene, Texture, Vec3, BVH};

const WIDTH: u32 = 80;
const HEIGHT: u32 = 60;
const SAMPLES: u32 = 8;
const REFERENCE_SAMPLES: u32 = 4000;
const MAX_DEPTH: u32 = 20;

/// Cornell box with a small disc light: the classic hard case for pure
/// path tracing.
fn cornell() -> Scene {
    let red = Color::new(0.75, 0.25, 0.25);
    let white = Color::new(0.75, 0.75, 0.75);
    let blue = Color::new(0.25, 0.25, 0.75);
    let lambert = |c: Color| Material::lambertian(Texture::constant_color(c));

    let walls = [
        (Vec3::new(5006.0, 0.0, 0.0), blue),
        (Vec3::new(-5006.0, 0.0, 0.0), red),
        (Vec3::new(0.0, 5010.0, 0.0), white),
        (Vec3::new(0.0, -5000.0, 0.0), white),
        (Vec3::new(0.0, 0.0, -5010.0), white),
    ];
    let mut objects: Vec<Hitable> = walls
        .iter()
        .map(|&(center, color)| {
            Box::new(Sphere { center, radius: 5000.0, material: lambert(color) }) as Hitable
        })
        .collect();
    objects.push(Box::new(Sphere {
        center: Vec3::new(-3.5, 2.0, -3.0),
        radius: 2.0,
        material: Material::dielectric(Texture::constant_color(Color::new(1.0, 1.0, 1.0)), 1.52),
    }));
    objects.push(Box::new(Sphere {
        center: Vec3::new(3.5, 2.0, -7.0),
        radius: 2.0,
        material: Material::metal(Texture::constant_color(Color::new(0.05, 1.0, 0.05)), 0.25),
    }));
    objects.push(Box::new(Disc {
        center: Vec3::new(0.0, 10.0, -5.0),
        radius: 1.5,
        normal: Vec3::new(0.0, -1.0, 0.0),
        material: Material::diffuse_light(Texture::constant_color(Color::new(15.0, 15.0, 15.0))),
    }));

    Scene {
        camera: Camera::new(
            Vec3::new(0.0, 5.0, 15.0),
            Vec3::new(0.0, 5.0, 0.0),
            45.0,
            f64::from(WIDTH) / f64::from(HEIGHT),
            0.0,
        ),
        world: BVH::from_vec(objects),
    }
}

/// Dark room with one strong and several weak sphere lights: exercises light
/// selection.
fn uneven_lights() -> Scene {
    let lambert = |c: Color| Material::lambertian(Texture::constant_color(c));
    let light = |c: Color| Material::diffuse_light(Texture::constant_color(c));
    let grey = Color::new(0.6, 0.6, 0.6);

    let mut objects: Vec<Hitable> = vec![
        Box::new(Sphere { center: Vec3::new(0.0, -5000.0, 0.0), radius: 5000.0, material: lambert(grey) }),
        Box::new(Sphere { center: Vec3::new(0.0, 0.0, -5010.0), radius: 5000.0, material: lambert(grey) }),
        Box::new(Sphere { center: Vec3::new(0.0, 1.0, -2.0), radius: 1.0, material: lambert(Color::new(0.8, 0.3, 0.3)) }),
        // Strong light
        Box::new(Sphere { center: Vec3::new(-3.0, 5.0, 0.0), radius: 0.4, material: light(Color::new(60.0, 55.0, 50.0)) }),
    ];
    // Weak decorative lights along the back wall
    for i in 0..8 {
        let x = -7.0 + 2.0 * i as f64;
        objects.push(Box::new(Sphere {
            center: Vec3::new(x, 0.3, -8.0),
            radius: 0.15,
            material: light(Color::new(1.5, 0.4 + 0.15 * i as f64, 0.3)),
        }));
    }

    Scene {
        camera: Camera::new(
            Vec3::new(0.0, 3.0, 10.0),
            Vec3::new(0.0, 1.0, -2.0),
            45.0,
            f64::from(WIDTH) / f64::from(HEIGHT),
            0.0,
        ),
        world: BVH::from_vec(objects),
    }
}

fn rmse(image: &[Color], reference: &[Color]) -> f64 {
    let sum: f64 = image
        .iter()
        .zip(reference)
        .map(|(a, b)| (a.r - b.r).powi(2) + (a.g - b.g).powi(2) + (a.b - b.b).powi(2))
        .sum();
    (sum / (3.0 * image.len() as f64)).sqrt()
}

fn measure(name: &str, scene: &Scene, workers: usize) {
    println!("[{name}] rendering reference ({REFERENCE_SAMPLES} spp x4)...");
    let reference = pathtracer::render_linear_with(
        scene, WIDTH, HEIGHT, REFERENCE_SAMPLES, MAX_DEPTH, workers, Integrator::NextEventEstimation,
    );

    let mut results = Vec::new();
    for (label, integrator) in [
        ("BSDF only", Integrator::BsdfOnly),
        ("NEE + MIS", Integrator::NextEventEstimation),
    ] {
        let start = Instant::now();
        let image = pathtracer::render_linear_with(
            scene, WIDTH, HEIGHT, SAMPLES, MAX_DEPTH, workers, integrator,
        );
        let seconds = start.elapsed().as_secs_f64();
        results.push((label, rmse(&image, &reference), seconds));
    }

    println!("=== Noise: {name} ({WIDTH}x{HEIGHT}, {SAMPLES} spp x4, depth {MAX_DEPTH}) ===");
    for (label, error, seconds) in &results {
        println!("{label:<10} RMSE {error:.4}  ({seconds:.2} s)");
    }
    let (_, bsdf_err, _) = results[0];
    let (_, nee_err, _) = results[1];
    // Variance falls as 1/N, so the equal-quality sample ratio is (RMSE ratio)².
    println!("NEE needs {:.1}x fewer samples than BSDF-only for equal noise.", (bsdf_err / nee_err).powi(2));
    println!();
}

fn main() {
    let workers = std::thread::available_parallelism().map(|n| n.get()).unwrap_or(1);
    measure("Cornell box", &cornell(), workers);
    measure("Uneven lights", &uneven_lights(), workers);
}
