#![allow(dead_code)]

mod aabb;
mod bvh;
mod camera;
mod color;
mod intersectable;
mod material;
mod ray;
mod rng;
mod scene;
mod texture;
mod vector;

pub mod gpu;
pub mod gpu_types;
pub mod shape;

pub use aabb::AABB;
pub use bvh::BVH;
pub use camera::Camera;
pub use color::Color;
pub use gpu::render_gpu;
pub use gpu::GPUScene;
pub use gpu::GPUShape;
pub use gpu_types::*;
pub use material::Material;
pub use scene::Scene;
pub use texture::Texture;
pub use vector::Vec3;

use intersectable::*;
use ray::Ray;

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use indicatif::{ProgressBar, ProgressStyle};
use threadpool::ThreadPool;

/// Hitable is a boxed trait object that implements `Intersectable`.
pub type Hitable = Box<dyn Intersectable + Send + Sync>;

/// Tile size for tile-based rendering (16x16 pixels per tile)
const TILE_SIZE: u32 = 16;

/// Path tracer renderer
///
/// # Arguments
///
/// * `scene` - Scene to render
/// * `width` - Width of the resulting image.
/// * `height` - Height of the resulting image.
/// * `samples` - Samples per pixel to take.
/// * `max_depth` - Hard limit of ray bouncing for the scene.
/// * `gamma` - Gamma value used for gamma correction of the final image.
/// * `workers` - How many threads to use.
/// * `image` - Filename of the saved image.
///
/// # Remarks
/// The path tracer does subpixel sampling (4 samples) using a tent distribution
/// so it traces `4 * samples` rays per pixel. It uses a russian roulette
/// implementation to optimize how many rays are required to render a given
/// pixel. Uses tile-based rendering for reduced scheduling overhead.
pub fn render(
    scene: Scene,
    width: u32,
    height: u32,
    samples: u32,
    max_depth: u32,
    gamma: f64,
    workers: usize,
    filename: &str,
) {
    // Shared mutable image buffer
    let imgbuf = Arc::new(Mutex::new(image::ImageBuffer::new(width, height)));
    // Shared scene buffer
    let scene = Arc::new(scene);
    // Progress bar
    let pb = ProgressBar::new(u64::from(width * height));

    pb.set_style(ProgressStyle::default_bar().template(
        "{spinner:.green} [{elapsed_precise}] [{bar:40.red/gray}] {percent}/100% ({eta_precise})",
    ).unwrap());

    let gamma_correction = gamma.recip();

    let w = f64::from(width).recip();
    let h = f64::from(height).recip();
    let s = (f64::from(samples) * 4.0).recip();

    let pool = ThreadPool::new(workers);

    let work_count = Arc::new(AtomicUsize::new(0));

    // Calculate number of tiles
    let tiles_x = (width + TILE_SIZE - 1) / TILE_SIZE;
    let tiles_y = (height + TILE_SIZE - 1) / TILE_SIZE;

    println!("Simple path tracer.");
    println!(
        "Rendering a {}x{}x{}spp image, max depth of {}, using {} workers.",
        width, height, samples, max_depth, workers
    );
    println!(
        "Using {}x{} tiles ({}x{} pixels each).",
        tiles_x, tiles_y, TILE_SIZE, TILE_SIZE
    );
    println!();

    let start = Instant::now();

    // Iterate over tiles instead of individual pixels
    for tile_y in 0..tiles_y {
        for tile_x in 0..tiles_x {
            let img = Arc::clone(&imgbuf);
            let scene = Arc::clone(&scene);
            let work_count = Arc::clone(&work_count);

            pool.execute(move || {
                // Calculate tile boundaries
                let x_start = tile_x * TILE_SIZE;
                let y_start = tile_y * TILE_SIZE;
                let x_end = (x_start + TILE_SIZE).min(width);
                let y_end = (y_start + TILE_SIZE).min(height);

                // Collect all pixel results for this tile
                let mut tile_pixels: Vec<(u32, u32, image::Rgb<u8>)> = Vec::with_capacity(
                    ((x_end - x_start) * (y_end - y_start)) as usize
                );

                for y in y_start..y_end {
                    for x in x_start..x_end {
                        let mut pixel_color = Color::new(0.0, 0.0, 0.0);

                        for sy in 0..2 {
                            for sx in 0..2 {
                                for _i in 0..samples {
                                    let dx = tent_filter_factor();
                                    let dy = tent_filter_factor();

                                    let u = ((f64::from(sx) + 0.5 + dx) * 0.5 + f64::from(x)) * w;
                                    let v = ((f64::from(sy) + 0.5 + dy) * 0.5 + f64::from(y)) * h;

                                    let ray = scene.camera.get_ray(u, v);

                                    pixel_color += radiance(scene.as_ref(), &ray, 1, max_depth);
                                }
                            }
                        }

                        pixel_color = pixel_color * s;
                        tile_pixels.push((x, y, pixel_color.to_gamma_rgb(gamma_correction)));
                    }
                }

                // Write all tile pixels at once (single lock acquisition)
                let pixel_count = tile_pixels.len();
                {
                    let mut img = img.lock().unwrap();
                    for (x, y, rgb) in tile_pixels {
                        img.put_pixel(x, y, rgb);
                    }
                }

                work_count.fetch_add(pixel_count, Ordering::Relaxed);
            });
        }
    }

    let ten_millis = Duration::from_millis(50);
    loop {
        pb.set_position(work_count.as_ref().load(Ordering::Relaxed) as u64);

        thread::sleep(ten_millis);

        if pool.queued_count() == 0 {
            break;
        }
    }

    pool.join();
    pb.finish();

    imgbuf.lock().unwrap().save(filename).unwrap();

    let end = start.elapsed();

    println!();
    println!(
        "Render took {} seconds.",
        f64::from(end.as_secs() as u32) + f64::from(end.subsec_millis()) / 1000.0
    );
}

fn radiance(scene: &Scene, ray: &Ray, depth: u32, max_depth: u32) -> Color {
    if let Some(intersection) = scene.world.intersect(ray, 0.0001, std::f64::INFINITY) {
        let emitted = intersection
            .material
            .emit(intersection.u, intersection.v, intersection.p);

        if let Some(scattered) = intersection.material.scatter(ray, &intersection) {
            let mut attenuation = scattered.attenuation;
            let p = (attenuation.r + attenuation.g + attenuation.b) / 3.0;

            if depth > 5 {
                if rng::get_random_number() < p {
                    attenuation = attenuation / p;
                } else {
                    return emitted;
                }
            }

            if depth < 100 {
                emitted + attenuation * radiance(scene, &scattered.scattered, depth + 1, max_depth)
            } else {
                emitted
            }
        } else {
            emitted
        }
    } else {
        Color::new(0.0, 0.0, 0.0)
    }
}

fn tent_filter_factor() -> f64 {
    let r = 2.0 * rng::get_random_number();

    if r < 1.0 {
        r.sqrt() - 1.0
    } else {
        1.0 - (2.0 - r).sqrt()
    }
}
