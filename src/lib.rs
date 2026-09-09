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
pub use gpu::{render_gpu, render_gpu_linear};
pub use gpu::render_realtime;
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
use std::sync::Mutex;
use std::thread;
use std::time::{Duration, Instant};

use indicatif::{ProgressBar, ProgressStyle};

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
    let start = Instant::now();

    let pixels = render_linear(&scene, width, height, samples, max_depth, workers);

    let gamma_correction = gamma.recip();
    let mut imgbuf = image::ImageBuffer::new(width, height);
    for (i, color) in pixels.iter().enumerate() {
        let x = i as u32 % width;
        let y = i as u32 / width;
        imgbuf.put_pixel(x, y, color.to_gamma_rgb(gamma_correction));
    }
    imgbuf.save(filename).unwrap();

    let end = start.elapsed();

    println!();
    println!(
        "Render took {} seconds.",
        f64::from(end.as_secs() as u32) + f64::from(end.subsec_millis()) / 1000.0
    );
}

/// Render the scene and return linear (not gamma corrected) radiance per pixel,
/// in row-major order. See [`render`] for the meaning of the arguments.
pub fn render_linear(
    scene: &Scene,
    width: u32,
    height: u32,
    samples: u32,
    max_depth: u32,
    workers: usize,
) -> Vec<Color> {
    // Shared linear image buffer
    let pixel_count = (width * height) as usize;
    let imgbuf = Mutex::new(vec![Color::new(0.0, 0.0, 0.0); pixel_count]);
    // Progress bar
    let pb = ProgressBar::new(u64::from(width * height));

    pb.set_style(ProgressStyle::default_bar().template(
        "{spinner:.green} [{elapsed_precise}] [{bar:40.red/gray}] {percent}/100% ({eta_precise})",
    ).unwrap());

    let w = f64::from(width).recip();
    let h = f64::from(height).recip();
    let s = (f64::from(samples) * 4.0).recip();

    let work_count = AtomicUsize::new(0);

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

    // Workers pull tiles from a shared counter until none are left.
    let tile_count = tiles_x * tiles_y;
    let next_tile = AtomicUsize::new(0);

    thread::scope(|scope| {
        for _ in 0..workers.max(1) {
            let imgbuf = &imgbuf;
            let work_count = &work_count;
            let next_tile = &next_tile;

            scope.spawn(move || loop {
                let tile = next_tile.fetch_add(1, Ordering::Relaxed) as u32;
                if tile >= tile_count {
                    break;
                }
                let tile_x = tile % tiles_x;
                let tile_y = tile / tiles_x;

                // Calculate tile boundaries
                let x_start = tile_x * TILE_SIZE;
                let y_start = tile_y * TILE_SIZE;
                let x_end = (x_start + TILE_SIZE).min(width);
                let y_end = (y_start + TILE_SIZE).min(height);

                // Collect all pixel results for this tile
                let mut tile_pixels: Vec<(usize, Color)> =
                    Vec::with_capacity(((x_end - x_start) * (y_end - y_start)) as usize);

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

                                    pixel_color += radiance(scene, &ray, 1, max_depth);
                                }
                            }
                        }

                        tile_pixels.push(((y * width + x) as usize, pixel_color * s));
                    }
                }

                // Write all tile pixels at once (single lock acquisition)
                let pixel_count = tile_pixels.len();
                {
                    let mut img = imgbuf.lock().unwrap();
                    for (idx, color) in tile_pixels {
                        img[idx] = color;
                    }
                }

                work_count.fetch_add(pixel_count, Ordering::Relaxed);
            });
        }

        let poll_interval = Duration::from_millis(50);
        loop {
            let done = work_count.load(Ordering::Relaxed);
            pb.set_position(done as u64);
            if done >= pixel_count {
                break;
            }
            thread::sleep(poll_interval);
        }
    });
    pb.finish();

    imgbuf.into_inner().unwrap()
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

            if depth < max_depth {
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

#[cfg(test)]
mod tests;
