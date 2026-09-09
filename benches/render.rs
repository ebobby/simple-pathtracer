//! Wall-clock benchmark for the CPU renderer on a deterministic sphere field.
//! Run with `cargo bench`. Reports the best of several runs.

use std::time::Instant;

use pathtracer::shape::Sphere;
use pathtracer::{Camera, Color, Hitable, Material, Scene, Texture, Vec3, BVH};
use rand::{Rng, SeedableRng};

const WIDTH: u32 = 320;
const HEIGHT: u32 = 240;
const SAMPLES: u32 = 4;
const MAX_DEPTH: u32 = 20;
const RUNS: usize = 3;

fn scene() -> Scene {
    let mut rng = rand_xoshiro::Xoshiro256PlusPlus::seed_from_u64(7);
    let mut objects: Vec<Hitable> = Vec::new();

    // Ground
    objects.push(Box::new(Sphere {
        center: Vec3::new(0.0, -5000.0, 0.0),
        radius: 5000.0,
        material: Material::lambertian(Texture::constant_color(Color::new(0.5, 0.5, 0.5))),
    }));
    // Emissive dome so paths pick up light
    objects.push(Box::new(Sphere {
        center: Vec3::new(0.0, 0.0, 0.0),
        radius: 200.0,
        material: Material::diffuse_light(Texture::constant_color(Color::new(1.0, 1.0, 1.0))),
    }));

    for _ in 0..200 {
        let center = Vec3::new(
            rng.gen_range(-12.0..12.0),
            rng.gen_range(0.2..1.0),
            rng.gen_range(-12.0..12.0),
        );
        let radius = rng.gen_range(0.2..0.6);
        let albedo = Texture::constant_color(Color::new(
            rng.gen_range(0.1..0.9),
            rng.gen_range(0.1..0.9),
            rng.gen_range(0.1..0.9),
        ));
        let material = match rng.gen_range(0..3) {
            0 => Material::lambertian(albedo),
            1 => Material::metal(albedo, rng.gen_range(0.0..0.3)),
            _ => Material::dielectric(albedo, 1.5),
        };
        objects.push(Box::new(Sphere { center, radius, material }));
    }

    Scene {
        camera: Camera::new(
            Vec3::new(0.0, 4.0, 16.0),
            Vec3::new(0.0, 0.5, 0.0),
            40.0,
            f64::from(WIDTH) / f64::from(HEIGHT),
            0.0,
        ),
        world: BVH::from_vec(objects),
    }
}

fn best_of(workers: usize) -> f64 {
    let scene = scene();
    (0..RUNS)
        .map(|_| {
            let start = Instant::now();
            let pixels = pathtracer::render_linear(&scene, WIDTH, HEIGHT, SAMPLES, MAX_DEPTH, workers);
            assert_eq!(pixels.len(), (WIDTH * HEIGHT) as usize);
            start.elapsed().as_secs_f64()
        })
        .fold(f64::INFINITY, f64::min)
}

fn main() {
    let all = std::thread::available_parallelism().map(|n| n.get()).unwrap_or(1);
    let paths = f64::from(WIDTH * HEIGHT * SAMPLES * 4);

    let single = best_of(1);
    let multi = best_of(all);

    println!();
    println!("=== CPU render benchmark ({}x{}, {} spp x4, depth {}) ===", WIDTH, HEIGHT, SAMPLES, MAX_DEPTH);
    println!("1 worker:   {:.3} s  ({:.0} paths/s)", single, paths / single);
    println!("{} workers: {:.3} s  ({:.0} paths/s)", all, multi, paths / multi);
}
