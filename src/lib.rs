#![allow(dead_code)]

mod aabb;
mod bvh;
mod camera;
mod color;
mod environment;
mod intersectable;
mod light;
mod material;
mod ray;
mod rng;
mod sampler;
mod scene;
mod texture;
mod tonemap;
mod vector;

pub mod gpu;
pub mod gpu_types;
pub mod shape;

pub use aabb::AABB;
pub use bvh::BVH;
pub use camera::Camera;
pub use color::Color;
pub use environment::{Environment, EnvironmentMap, Sky, Sun};
pub use gpu::{
    render_gpu, render_gpu_linear, render_gpu_linear_with_environment, render_gpu_with,
    render_gpu_with_environment,
};
pub use gpu::{render_realtime, render_realtime_with, render_realtime_with_environment};
pub use gpu::GPUScene;
pub use gpu::GPUShape;
pub use gpu_types::*;
pub use light::{Light, LightKind, LightShape};
pub use material::{Material, Principled};
pub use sampler::Sampler;
pub use scene::Scene;
pub use texture::Texture;
pub use tonemap::{ToneCurve, Tonemap};
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

/// Which light transport estimator to use.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Integrator {
    /// Pure path tracing: lights are only found by chance. Kept as the
    /// reference implementation.
    BsdfOnly,
    /// Path tracing with next event estimation and multiple importance
    /// sampling at diffuse vertices. The default.
    NextEventEstimation,
}

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
    let tonemap = Tonemap::default().gamma(gamma);
    render_with_tonemap(scene, width, height, samples, max_depth, workers, &tonemap, filename);
}

/// [`render`] with an explicit output stage (exposure, tone curve, gamma).
pub fn render_with_tonemap(
    scene: Scene,
    width: u32,
    height: u32,
    samples: u32,
    max_depth: u32,
    workers: usize,
    tonemap: &Tonemap,
    filename: &str,
) {
    let start = Instant::now();

    let pixels = render_linear(&scene, width, height, samples, max_depth, workers);

    let mut imgbuf = image::ImageBuffer::new(width, height);
    for (i, color) in pixels.iter().enumerate() {
        let x = i as u32 % width;
        let y = i as u32 / width;
        imgbuf.put_pixel(x, y, tonemap.apply(*color));
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
    render_linear_with(
        scene,
        width,
        height,
        samples,
        max_depth,
        workers,
        Integrator::NextEventEstimation,
    )
}

/// [`render_linear`] with an explicit choice of estimator.
pub fn render_linear_with(
    scene: &Scene,
    width: u32,
    height: u32,
    samples: u32,
    max_depth: u32,
    workers: usize,
    integrator: Integrator,
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
                        let pixel_seed = sampler::hash(y * width + x);

                        for sy in 0..2 {
                            for sx in 0..2 {
                                for i in 0..samples {
                                    let sampler =
                                        Sampler::new(pixel_seed, (sy * 2 + sx) * samples + i);
                                    let (jx, jy) = sampler.get_2d(sampler::SLOT_PIXEL);
                                    let dx = tent_filter_factor(jx);
                                    let dy = tent_filter_factor(jy);

                                    let u = ((f64::from(sx) + 0.5 + dx) * 0.5 + f64::from(x)) * w;
                                    let v = ((f64::from(sy) + 0.5 + dy) * 0.5 + f64::from(y)) * h;

                                    let ray = scene.camera.get_ray(u, v);

                                    pixel_color += radiance_with(
                                        scene, &ray, 1, max_depth, integrator, &sampler,
                                    );
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

/// Estimate the radiance arriving along `ray` with the default estimator.
///
/// `depth` is the number of the surface interaction the ray is about to
/// have (1 for camera rays); the walk stops after `max_depth` interactions.
fn radiance(scene: &Scene, ray: &Ray, depth: u32, max_depth: u32, sampler: &Sampler) -> Color {
    radiance_with(scene, ray, depth, max_depth, Integrator::NextEventEstimation, sampler)
}

/// [`radiance`] with an explicit choice of estimator. Random decisions come
/// from `sampler`; bounce `b` (0-based) uses pair slots `bounce_slot(b)..+3`.
fn radiance_with(
    scene: &Scene,
    ray: &Ray,
    depth: u32,
    max_depth: u32,
    integrator: Integrator,
    sampler: &Sampler,
) -> Color {
    let use_nee = integrator == Integrator::NextEventEstimation && !scene.lights().is_empty();

    let mut ray = ray.clone();
    let mut depth = depth;
    let mut color = Color::new(0.0, 0.0, 0.0);
    let mut throughput = Color::new(1.0, 1.0, 1.0);

    // How the current ray was generated: camera and specular bounces add any
    // emission they find at full weight; diffuse bounces carry their pdf so
    // emission can be weighted against light sampling.
    let mut prev_specular = true;
    let mut prev_pdf = 0.0;

    loop {
        let Some(intersection) = scene.world.intersect(&ray, 0.0001, f64::INFINITY) else {
            // Left the scene: sky and sun, each MIS-weighted against the
            // chance that light sampling would have picked this direction.
            let direction = ray.direction.normalize();
            if let Some(sky) = &scene.environment.sky {
                let mut weight = 1.0;
                if use_nee && !prev_specular {
                    let light = scene.sky_light().unwrap();
                    weight = light::power_heuristic(prev_pdf, sky.pdf(direction) * light.select_pdf);
                }
                color += throughput * sky.radiance(direction) * weight;
            }
            if let Some(sun) = &scene.environment.sun {
                if sun.contains(direction) {
                    let mut weight = 1.0;
                    if use_nee && !prev_specular {
                        let light = scene.sun_light().unwrap();
                        weight = light::power_heuristic(prev_pdf, sun.pdf() * light.select_pdf);
                    }
                    color += throughput * sun.radiance * weight;
                }
            }
            break;
        };

        // Beer-Lambert absorption when leaving the inside of a transmissive
        // principled object.
        if let Material::Principled(principled) = intersection.material {
            if principled.transmission > 0.0 && ray.direction.dot(intersection.normal) > 0.0 {
                throughput = throughput * principled.transmittance(intersection.t, &intersection);
            }
        }

        let emitted = intersection
            .material
            .emit(intersection.u, intersection.v, intersection.p);
        if emitted.r > 0.0 || emitted.g > 0.0 || emitted.b > 0.0 {
            let weight = if use_nee && !prev_specular {
                match scene.light_of_shape(intersection.shape_id) {
                    Some(light) => {
                        let direction = ray.direction.normalize();
                        let pdf_light = scene
                            .light_pdf(light, ray.origin, intersection.p, direction)
                            * light.select_pdf;
                        light::power_heuristic(prev_pdf, pdf_light)
                    }
                    None => 1.0,
                }
            } else {
                1.0
            };
            color += throughput * emitted * weight;
        }

        // Sample slots for this bounce: BSDF direction, light sample, and
        // (light selection or Fresnel, Russian roulette). The scalar slot is
        // only generated when something consumes it.
        let slot = sampler::bounce_slot(depth - 1);
        let u_bsdf = sampler.get_2d(slot);
        let (needs_scalar, needs_secondary) = match intersection.material {
            Material::Dielectric(_) => (true, false),
            Material::Principled(_) => (true, true),
            _ => (false, false),
        };
        let mut u_scalar = if needs_scalar {
            Some(sampler.get_2d(slot + 2))
        } else {
            None
        };
        let u_secondary = if needs_secondary {
            sampler.get_2d(slot + 3)
        } else {
            (0.0, 0.0)
        };

        let Some(scattered) = intersection.material.scatter(
            &ray,
            &intersection,
            [
                u_bsdf.0,
                u_bsdf.1,
                u_scalar.map_or(0.0, |u| u.0),
                u_secondary.0,
                u_secondary.1,
            ],
        ) else {
            break;
        };

        match scattered.pdf {
            Some(pdf) if use_nee => {
                // At the last allowed interaction there is no BSDF-sampled
                // continuation to share the light with, so the light sample
                // takes full weight.
                let last_vertex = depth >= max_depth;
                let wo = -ray.direction.normalize();
                let u_light = sampler.get_2d(slot + 1);
                let u_select = *u_scalar.get_or_insert_with(|| sampler.get_2d(slot + 2));
                color += throughput
                    * sample_direct_light(
                        scene,
                        &intersection,
                        wo,
                        last_vertex,
                        u_light,
                        u_select.0,
                    );
                prev_specular = false;
                prev_pdf = pdf;
            }
            _ => {
                prev_specular = true;
            }
        }

        let mut attenuation = scattered.attenuation;

        // Russian roulette: after a few bounces, terminate paths with a
        // probability proportional to how little light they still carry.
        if depth > 5 {
            let p = (attenuation.r + attenuation.g + attenuation.b) / 3.0;
            let u_roulette = u_scalar.get_or_insert_with(|| sampler.get_2d(slot + 2)).1;
            if u_roulette < p {
                attenuation = attenuation / p;
            } else {
                break;
            }
        }

        if depth >= max_depth {
            break;
        }

        throughput = throughput * attenuation;
        ray = scattered.scattered;
        depth += 1;
    }

    color
}

/// Direct lighting at a non-delta vertex seen from unit direction `wo`, from
/// one light chosen in proportion to its power, weighted against BSDF
/// sampling with the power heuristic unless `full_weight` says no BSDF
/// continuation will follow.
fn sample_direct_light(
    scene: &Scene,
    intersection: &Intersection,
    wo: Vec3,
    full_weight: bool,
    u_light: (f64, f64),
    u_select: f64,
) -> Color {
    let (light, select_pdf) = scene.pick_light(u_select);

    let Some(sample) = scene.sample_light(light, intersection.p, u_light.0, u_light.1) else {
        return Color::new(0.0, 0.0, 0.0);
    };
    let cos_theta = sample.direction.dot(intersection.facing_normal(wo));
    let Some((f, pdf_bsdf)) = intersection
        .material
        .eval(wo, sample.direction, intersection)
    else {
        return Color::new(0.0, 0.0, 0.0);
    };

    let shadow_ray = Ray {
        origin: intersection.p,
        direction: sample.direction,
    };
    let shadow_hit = scene.world.intersect(&shadow_ray, 0.0001, f64::INFINITY);
    let emitted = match (light.kind, shadow_hit) {
        // Shape lights: the nearest hit must be that shape
        (light::LightKind::Shape { shape_id, .. }, Some(hit)) if hit.shape_id == shape_id => {
            hit.material.emit(hit.u, hit.v, hit.p)
        }
        // Infinite lights: the shadow ray must leave the scene
        (light::LightKind::Sky, None) => {
            scene.environment.sky.as_ref().unwrap().radiance(sample.direction)
        }
        (light::LightKind::Sun, None) => scene.environment.sun.as_ref().unwrap().radiance,
        _ => return Color::new(0.0, 0.0, 0.0),
    };
    let pdf_light = sample.pdf * select_pdf;
    let weight = if full_weight {
        1.0
    } else {
        light::power_heuristic(pdf_light, pdf_bsdf)
    };

    f * emitted * (cos_theta / pdf_light * weight)
}

/// Map a uniform in [0, 1) to a tent-distributed offset in (-1, 1).
fn tent_filter_factor(u: f64) -> f64 {
    let r = 2.0 * u;

    if r < 1.0 {
        r.sqrt() - 1.0
    } else {
        1.0 - (2.0 - r).sqrt()
    }
}

#[cfg(test)]
mod tests;
