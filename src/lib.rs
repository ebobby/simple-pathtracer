#![allow(dead_code)]

pub mod camera;
pub mod color;
pub mod intersectable;
pub mod material;
pub mod ray;
pub mod scene;
pub mod vector;

use color::Color;
use ray::Ray;
use scene::Scene;

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use indicatif::ProgressBar;
use threadpool::ThreadPool;

const EPSILON: f64 = 1e-6;

pub fn render(
    scene: Scene,
    width: u32,
    height: u32,
    samples: u32,
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

    let gamma_correction = gamma.recip();

    let w = f64::from(width);
    let h = f64::from(height);
    let s = f64::from(samples);

    let pool = ThreadPool::new(workers);

    let work_count = Arc::new(AtomicUsize::new(0));

    println!("Simple path tracer.");
    println!(
        "Rendering a {}x{} image, {} samples per pixel using {} workers.",
        width, height, samples, workers
    );
    println!();
    println!("Processing {} pixels...", width * height);

    let start = Instant::now();

    // Iterate over the coordinates and pixels of the image
    for y in 0..height {
        for x in 0..width {
            let img = Arc::clone(&imgbuf);
            let scene = Arc::clone(&scene);
            let work_count = Arc::clone(&work_count);

            pool.execute(move || {
                let mut pixel_color = Color::black();

                for _i in 0..samples {
                    let u = (f64::from(x) + rand::random::<f64>()) / w;
                    let v = (f64::from(y) + rand::random::<f64>()) / h;

                    let ray = scene.camera.get_ray(u, v);

                    pixel_color += color(scene.as_ref(), ray, 1);
                }

                pixel_color = pixel_color / s;

                let mut img = img.lock().unwrap();

                img.put_pixel(x, y, pixel_color.gamma_rgb(gamma_correction));

                work_count.fetch_add(1, Ordering::SeqCst);
            });
        }
    }

    let ten_millis = Duration::from_millis(10);
    loop {
        pb.set_position(work_count.as_ref().load(Ordering::SeqCst) as u64);

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

fn color(scene: &Scene, ray: Ray, depth: u32) -> Color {
    if depth >= 50 {
        return Color::black();
    }

    if let Some(intersection) = scene.objects.intersect(ray) {
        if let Some(scattered) = intersection.material.scatter(ray, &intersection) {
            scattered.attenuation * color(scene, scattered.scattered, depth + 1)
        } else {
            Color::black()
        }
    } else {
        let unit_direction = ray.direction.normalize();
        let t = (unit_direction.y + 1.0) * 0.5;

        Color::new(1.0, 1.0, 1.0) * (1.0 - t) + Color::new(0.5, 0.7, 1.0) * t
    }
}
