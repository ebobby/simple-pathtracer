//! The CPU and GPU renderers must converge to the same image.

use crate::shape::{Disc, Sphere};
use crate::{render_gpu_linear, render_linear};
use crate::{Camera, Color, GPUShape, Hitable, Material, Scene, Texture, Vec3, BVH};

const WIDTH: u32 = 32;
const HEIGHT: u32 = 24;
const SAMPLES: u32 = 250;
const MAX_DEPTH: u32 = 20;

/// Open scene: a huge ground sphere, three small spheres of each material,
/// and a disc light. Rays that escape see black on both backends.
fn shapes() -> (Vec<Sphere>, Vec<Disc>) {
    let grey = Texture::constant_color(Color::new(0.6, 0.6, 0.6));
    let spheres = vec![
        Sphere {
            center: Vec3::new(0.0, -5000.0, 0.0),
            radius: 5000.0,
            material: Material::lambertian(grey.clone()),
        },
        Sphere {
            center: Vec3::new(-2.0, 1.0, 0.0),
            radius: 1.0,
            material: Material::lambertian(Texture::constant_color(Color::new(0.8, 0.2, 0.2))),
        },
        Sphere {
            center: Vec3::new(0.0, 1.0, 0.0),
            radius: 1.0,
            material: Material::metal(Texture::constant_color(Color::new(0.9, 0.9, 0.9)), 0.1),
        },
        Sphere {
            center: Vec3::new(2.0, 1.0, 0.0),
            radius: 1.0,
            material: Material::dielectric(Texture::constant_color(Color::new(1.0, 1.0, 1.0)), 1.5),
        },
        Sphere {
            center: Vec3::new(0.0, 0.6, 2.5),
            radius: 0.6,
            material: Material::Principled(
                crate::material::Principled::new(Texture::constant_color(Color::new(0.9, 0.4, 0.2)))
                    .metallic(0.6)
                    .roughness(0.35),
            ),
        },
        Sphere {
            center: Vec3::new(-1.6, 0.5, 2.2),
            radius: 0.5,
            material: Material::Principled(
                crate::material::Principled::new(Texture::constant_color(Color::new(0.8, 0.9, 1.0)))
                    .transmission(1.0)
                    .roughness(0.15),
            ),
        },
    ];
    let discs = vec![Disc {
        center: Vec3::new(0.0, 6.0, 0.0),
        normal: Vec3::new(0.0, -1.0, 0.0),
        radius: 3.0,
        material: Material::diffuse_light(Texture::constant_color(Color::new(8.0, 8.0, 8.0))),
    }];
    (spheres, discs)
}

fn camera() -> Camera {
    Camera::new(
        Vec3::new(0.0, 2.0, 8.0),
        Vec3::new(0.0, 1.0, 0.0),
        45.0,
        f64::from(WIDTH) / f64::from(HEIGHT),
        0.0,
    )
}

fn mean(pixels: &[Color]) -> f64 {
    pixels.iter().map(|c| c.r + c.g + c.b).sum::<f64>() / (3.0 * pixels.len() as f64)
}

#[test]
fn cpu_and_gpu_renders_agree() {
    let (spheres, discs) = shapes();
    let gpu_shapes: Vec<GPUShape> = spheres
        .into_iter()
        .map(GPUShape::Sphere)
        .chain(discs.into_iter().map(GPUShape::Disc))
        .collect();
    let Some(gpu) = render_gpu_linear(gpu_shapes, &camera(), WIDTH, HEIGHT, SAMPLES, MAX_DEPTH)
    else {
        eprintln!("no GPU adapter, skipping parity test");
        return;
    };

    let (spheres, discs) = shapes();
    let objects: Vec<Hitable> = spheres
        .into_iter()
        .map(|s| Box::new(s) as Hitable)
        .chain(discs.into_iter().map(|d| Box::new(d) as Hitable))
        .collect();
    let scene = Scene {
        camera: camera(),
        world: BVH::from_vec(objects),
    };
    let cpu = render_linear(&scene, WIDTH, HEIGHT, SAMPLES, MAX_DEPTH, 4);

    assert_eq!(cpu.len(), gpu.len());

    let cpu_mean = mean(&cpu);
    let gpu_mean = mean(&gpu);
    let relative = (gpu_mean - cpu_mean).abs() / cpu_mean;
    assert!(
        relative < 0.02,
        "mean brightness differs by {:.1}% (cpu {cpu_mean:.4}, gpu {gpu_mean:.4})",
        relative * 100.0
    );

    let mad = cpu
        .iter()
        .zip(&gpu)
        .map(|(a, b)| (a.r - b.r).abs() + (a.g - b.g).abs() + (a.b - b.b).abs())
        .sum::<f64>()
        / (3.0 * cpu.len() as f64);
    assert!(mad < 0.03, "mean absolute per-channel difference {mad:.4}");
}
