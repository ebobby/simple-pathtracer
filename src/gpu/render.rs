//! GPU rendering orchestration.

use std::time::Instant;

use indicatif::{ProgressBar, ProgressStyle};
use wgpu::util::DeviceExt;

use super::context::{create_environment_buffers, GPUContext, GPUPipeline};
use super::scene::{GPUScene, GPUShape};
use crate::gpu_types::GPURenderParams;
use crate::{Camera, Color, Environment, Tonemap};

/// Samples per pass. Kept small so no single dispatch runs long enough to
/// trip the GPU watchdog; dispatch overhead is negligible next to the work.
const SAMPLES_PER_PASS: u32 = 16;

/// Render a scene using GPU compute shaders and save it as an image.
///
/// # Arguments
///
/// * `shapes` - List of shapes to render
/// * `camera` - Camera for the scene
/// * `width` - Output image width
/// * `height` - Output image height
/// * `samples` - Samples per pixel
/// * `max_depth` - Maximum ray bounce depth
/// * `gamma` - Gamma correction value
/// * `filename` - Output filename
pub fn render_gpu(
    shapes: Vec<GPUShape>,
    camera: &Camera,
    width: u32,
    height: u32,
    samples: u32,
    max_depth: u32,
    gamma: f64,
    filename: &str,
) {
    render_gpu_with_environment(
        shapes,
        camera,
        &Environment::default(),
        width,
        height,
        samples,
        max_depth,
        gamma,
        filename,
    );
}

/// [`render_gpu`] with a sky and/or sun.
pub fn render_gpu_with_environment(
    shapes: Vec<GPUShape>,
    camera: &Camera,
    environment: &Environment,
    width: u32,
    height: u32,
    samples: u32,
    max_depth: u32,
    gamma: f64,
    filename: &str,
) {
    let tonemap = Tonemap::default().gamma(gamma);
    render_gpu_with(
        shapes, camera, environment, &tonemap, width, height, samples, max_depth, filename,
    );
}

/// [`render_gpu`] with an environment and an explicit output stage.
pub fn render_gpu_with(
    shapes: Vec<GPUShape>,
    camera: &Camera,
    environment: &Environment,
    tonemap: &Tonemap,
    width: u32,
    height: u32,
    samples: u32,
    max_depth: u32,
    filename: &str,
) {
    let start = Instant::now();

    let pixels = render_gpu_linear_with_environment(
        shapes, camera, environment, width, height, samples, max_depth,
    )
    .expect("Failed to find a suitable GPU adapter");

    // Convert to image with exposure, bloom, tone curve and gamma
    println!("Converting to image...");
    let imgbuf = tonemap.apply_image(&pixels, width, height);

    // Save image
    imgbuf.save(filename).expect("Failed to save image");

    let elapsed = start.elapsed();
    println!();
    println!("Render took {:.3} seconds.", elapsed.as_secs_f64());
    println!("Saved to: {}", filename);
}

/// Render a scene on the GPU and return linear (not gamma corrected) radiance
/// per pixel in row-major order. Returns `None` when no GPU adapter is available.
/// See [`render_gpu`] for the meaning of the arguments.
pub fn render_gpu_linear(
    shapes: Vec<GPUShape>,
    camera: &Camera,
    width: u32,
    height: u32,
    samples: u32,
    max_depth: u32,
) -> Option<Vec<Color>> {
    render_gpu_linear_with_environment(
        shapes,
        camera,
        &Environment::default(),
        width,
        height,
        samples,
        max_depth,
    )
}

/// [`render_gpu_linear`] with a sky and/or sun.
pub fn render_gpu_linear_with_environment(
    shapes: Vec<GPUShape>,
    camera: &Camera,
    environment: &Environment,
    width: u32,
    height: u32,
    samples: u32,
    max_depth: u32,
) -> Option<Vec<Color>> {
    println!("Simple path tracer (GPU).");
    println!(
        "Rendering a {}x{}x{}spp image, max depth of {}.",
        width, height, samples, max_depth
    );

    // Build GPU scene
    println!("Building GPU scene...");
    let scene = GPUScene::build_with_environment(shapes, camera, environment);
    println!(
        "Scene: {} spheres, {} discs, {} lights, {} materials, {} BVH nodes",
        scene.num_spheres,
        scene.num_discs,
        scene.lights.len(),
        scene.materials.len(),
        scene.bvh_nodes.len()
    );

    // Initialize GPU
    println!("Initializing GPU...");
    let ctx = pollster::block_on(GPUContext::new())?;
    let pipeline = GPUPipeline::new(&ctx.device);

    // Calculate number of passes
    let total_samples = samples * 4; // Match CPU's 4x subpixel sampling
    let num_passes = (total_samples + SAMPLES_PER_PASS - 1) / SAMPLES_PER_PASS;
    let samples_per_pass = SAMPLES_PER_PASS.min(total_samples);

    println!(
        "Rendering in {} passes ({} samples/pass)...",
        num_passes, samples_per_pass
    );
    println!();

    // Calculate workgroup dispatch size
    let workgroup_size = 8u32;
    let workgroups_x = (width + workgroup_size - 1) / workgroup_size;
    let workgroups_y = (height + workgroup_size - 1) / workgroup_size;

    // Create output buffer
    let output_size = (width * height * 16) as u64; // vec4<f32> = 16 bytes
    let output_buffer = ctx.device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("Output Buffer"),
        size: output_size,
        usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
        mapped_at_creation: false,
    });

    // Create buffers that don't change between passes
    let camera_buffer = ctx
        .device
        .create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Camera Buffer"),
            contents: bytemuck::cast_slice(&[scene.camera]),
            usage: wgpu::BufferUsages::UNIFORM,
        });

    let bvh_buffer = ctx
        .device
        .create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("BVH Buffer"),
            contents: bytemuck::cast_slice(&scene.bvh_nodes),
            usage: wgpu::BufferUsages::STORAGE,
        });

    let spheres_data = if scene.spheres.is_empty() {
        vec![crate::gpu_types::GPUSphere::new(
            crate::gpu_types::GPUVec3::zero(),
            0.0,
            0,
        )]
    } else {
        scene.spheres.clone()
    };
    let spheres_buffer = ctx
        .device
        .create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Spheres Buffer"),
            contents: bytemuck::cast_slice(&spheres_data),
            usage: wgpu::BufferUsages::STORAGE,
        });

    let discs_data = if scene.discs.is_empty() {
        vec![crate::gpu_types::GPUDisc::new(
            crate::gpu_types::GPUVec3::zero(),
            crate::gpu_types::GPUVec3::new(0.0, 1.0, 0.0),
            0.0,
            0,
        )]
    } else {
        scene.discs.clone()
    };
    let discs_buffer = ctx
        .device
        .create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Discs Buffer"),
            contents: bytemuck::cast_slice(&discs_data),
            usage: wgpu::BufferUsages::STORAGE,
        });

    let materials_data = if scene.materials.is_empty() {
        vec![crate::gpu_types::GPUMaterial::lambertian(
            crate::gpu_types::GPUVec3::new(0.5, 0.5, 0.5),
        )]
    } else {
        scene.materials.clone()
    };
    let materials_buffer = ctx
        .device
        .create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Materials Buffer"),
            contents: bytemuck::cast_slice(&materials_data),
            usage: wgpu::BufferUsages::STORAGE,
        });

    // Storage buffers may not be empty; a dummy entry is harmless with num_lights = 0
    let lights_data = if scene.lights.is_empty() {
        vec![crate::gpu_types::GPULight::new(0, 1.0, 1.0)]
    } else {
        scene.lights.clone()
    };
    let lights_buffer = ctx
        .device
        .create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Lights Buffer"),
            contents: bytemuck::cast_slice(&lights_data),
            usage: wgpu::BufferUsages::STORAGE,
        });

    let [env_pixels_buffer, env_cdf_buffer] =
        create_environment_buffers(&ctx.device, &scene.environment);

    // Create params buffer once (will be updated each pass)
    let params_buffer = ctx.device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("Params Buffer"),
        size: std::mem::size_of::<GPURenderParams>() as u64,
        usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });

    // Create bind group once (reused for all passes)
    let bind_group = ctx.device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("Pathtracer Bind Group"),
        layout: pipeline.bind_group_layout(),
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: params_buffer.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: camera_buffer.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 2,
                resource: bvh_buffer.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 3,
                resource: spheres_buffer.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 4,
                resource: discs_buffer.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 5,
                resource: materials_buffer.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 6,
                resource: output_buffer.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 7,
                resource: lights_buffer.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 8,
                resource: env_pixels_buffer.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 9,
                resource: env_cdf_buffer.as_entire_binding(),
            },
        ],
    });

    // Progress bar
    let pb = ProgressBar::new(num_passes as u64);
    pb.set_style(
        ProgressStyle::default_bar()
            .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} passes ({eta})")
            .unwrap()
            .progress_chars("#>-"),
    );

    // Render passes
    for pass in 0..num_passes {
        // Calculate samples for this pass (last pass may have fewer)
        let samples_this_pass = if pass == num_passes - 1 {
            total_samples - (pass * samples_per_pass)
        } else {
            samples_per_pass
        };

        // Update params for this pass
        let params = GPURenderParams {
            width,
            height,
            samples: samples_this_pass,
            max_depth,
            frame_seed: pass, // Different seed for each pass
            num_spheres: scene.num_spheres,
            num_discs: scene.num_discs,
            num_lights: scene.lights.len() as u32,
            sample_offset: pass * samples_per_pass,
            sky_type: scene.environment.sky_type,
            env_width: scene.environment.env_width,
            env_height: scene.environment.env_height,
            sky_color: scene.environment.sky_color,
            sun_direction: scene.environment.sun_direction,
            sun_radiance: scene.environment.sun_radiance,
        };
        ctx.queue.write_buffer(&params_buffer, 0, bytemuck::cast_slice(&[params]));

        // Create command encoder and dispatch
        let mut encoder = ctx
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Pathtracer Encoder"),
            });

        {
            let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("Pathtracer Pass"),
                timestamp_writes: None,
            });
            compute_pass.set_pipeline(pipeline.pipeline());
            compute_pass.set_bind_group(0, &bind_group, &[]);
            compute_pass.dispatch_workgroups(workgroups_x, workgroups_y, 1);
        }

        ctx.queue.submit(Some(encoder.finish()));
        ctx.device.poll(wgpu::Maintain::Wait);

        pb.inc(1);
    }

    pb.finish_with_message("Rendering complete");

    // Read back results
    let staging_buffer = ctx.device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("Staging Buffer"),
        size: output_size,
        usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });

    let mut encoder = ctx
        .device
        .create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Readback Encoder"),
        });
    encoder.copy_buffer_to_buffer(&output_buffer, 0, &staging_buffer, 0, output_size);
    ctx.queue.submit(Some(encoder.finish()));

    let buffer_slice = staging_buffer.slice(..);
    let (sender, receiver) = std::sync::mpsc::channel();
    buffer_slice.map_async(wgpu::MapMode::Read, move |result| {
        sender.send(result).unwrap();
    });

    ctx.device.poll(wgpu::Maintain::Wait);
    receiver.recv().unwrap().expect("Failed to map buffer");

    let data = buffer_slice.get_mapped_range();
    let pixels: &[[f32; 4]] = bytemuck::cast_slice(&data);

    // Normalize by total samples
    let sample_scale = 1.0 / (total_samples as f32);
    let colors: Vec<Color> = pixels
        .iter()
        .map(|p| {
            Color::new(
                f64::from(p[0] * sample_scale),
                f64::from(p[1] * sample_scale),
                f64::from(p[2] * sample_scale),
            )
        })
        .collect();

    drop(data);
    staging_buffer.unmap();

    Some(colors)
}
