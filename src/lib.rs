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

use indicatif::{ProgressBar, ProgressStyle};
use threadpool::ThreadPool;

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
    ));

    let gamma_correction = gamma.recip();

    let w = f64::from(width).recip();
    let h = f64::from(height).recip();
    let s = f64::from(samples).recip();

    let pool = ThreadPool::new(workers);

    let work_count = Arc::new(AtomicUsize::new(0));

    println!("Simple path tracer.");
    println!(
        "Rendering a {}x{}x{}spp image, max depth of {}, using {} workers.",
        width, height, samples, max_depth, workers
    );
    println!();

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
                    let u = (f64::from(x) + rand::random::<f64>()) * w;
                    let v = (f64::from(y) + rand::random::<f64>()) * h;

                    let ray = scene.camera.get_ray(u, v);

                    pixel_color += color(scene.as_ref(), ray, 1, max_depth);
                }

                pixel_color = pixel_color * s;

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

fn color(scene: &Scene, ray: Ray, depth: u32, max_depth: u32) -> Color {
    if let Some(intersection) = scene.objects.intersect(ray) {
        let emitted = intersection.material.emit();

        if let Some(scattered) = intersection.material.scatter(ray, &intersection) {
            let mut attenuation = scattered.attenuation;
            let p = (attenuation.r + attenuation.g + attenuation.b) / 3.0;

            if depth > 5 {
                if rand::random::<f64>() < p {
                    attenuation = attenuation / p;
                } else {
                    return emitted;
                }
            }

            if depth < 100 {
                emitted + attenuation * color(scene, scattered.scattered, depth + 1, max_depth)
            } else {
                emitted
            }
        } else {
            emitted
        }
    } else {
        Color::black()
    }
}
