//! Real-time interactive GPU renderer with camera controls and progressive refinement.

use std::sync::Arc;
use std::time::Instant;

use wgpu::util::DeviceExt;
use winit::application::ApplicationHandler;
use winit::event::{DeviceEvent, ElementState, KeyEvent, MouseButton, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::keyboard::{KeyCode, PhysicalKey};
use winit::window::{Window, WindowId};

use super::context::GPUPipeline;
use super::scene::{GPUScene, GPUShape};
use crate::gpu_types::{GPUCamera, GPURenderParams, GPUVec3};
use crate::Camera;

/// Samples per frame for real-time rendering
const SAMPLES_PER_FRAME: u32 = 4;

/// Camera movement speed (units per second)
const MOVE_SPEED: f64 = 5.0;

/// Mouse sensitivity (radians per pixel)
const MOUSE_SENSITIVITY: f64 = 0.003;

/// Blit shader parameters
#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct BlitParams {
    sample_count: u32,
    gamma: f32,
    _pad0: u32,
    _pad1: u32,
}

/// Camera controller state
struct CameraController {
    // Position and orientation
    position: [f64; 3],
    yaw: f64,   // Rotation around Y axis (left/right)
    pitch: f64, // Rotation around X axis (up/down)
    fov: f64,

    // Movement state
    forward: bool,
    backward: bool,
    left: bool,
    right: bool,
    up: bool,
    down: bool,

    // Mouse state
    mouse_captured: bool,

    // Change tracking
    changed: bool,
}

impl CameraController {
    fn new(camera: &Camera) -> Self {
        let pos = camera.look_from();
        // Compute initial yaw/pitch from camera vectors
        // w points from camera origin to center of image plane = forward direction
        let w = camera.corner() + camera.horizontal() * 0.5 + camera.vertical() * 0.5 - pos;
        let forward = crate::Vec3::new(w.x, w.y, w.z).normalize();

        let yaw = forward.z.atan2(forward.x);
        let pitch = forward.y.asin();

        Self {
            position: [pos.x, pos.y, pos.z],
            yaw,
            pitch,
            fov: camera.vfov(),
            forward: false,
            backward: false,
            left: false,
            right: false,
            up: false,
            down: false,
            mouse_captured: false,
            changed: true,
        }
    }

    fn handle_key(&mut self, key: KeyCode, pressed: bool) {
        match key {
            KeyCode::KeyW => self.forward = pressed,
            KeyCode::KeyS => self.backward = pressed,
            KeyCode::KeyA => self.left = pressed,
            KeyCode::KeyD => self.right = pressed,
            KeyCode::Space => self.up = pressed,
            KeyCode::ShiftLeft | KeyCode::ShiftRight => self.down = pressed,
            _ => {}
        }
    }

    fn handle_mouse_move(&mut self, delta_x: f64, delta_y: f64) {
        if self.mouse_captured {
            self.yaw -= delta_x * MOUSE_SENSITIVITY;
            self.pitch -= delta_y * MOUSE_SENSITIVITY;
            // Clamp pitch to avoid gimbal lock
            self.pitch = self.pitch.clamp(-1.5, 1.5);
            self.changed = true;
        }
    }

    fn update(&mut self, dt: f64) {
        let (sin_yaw, cos_yaw) = self.yaw.sin_cos();
        let (sin_pitch, cos_pitch) = self.pitch.sin_cos();

        // Forward vector (horizontal plane for movement)
        let forward_x = cos_yaw;
        let forward_z = sin_yaw;

        // Right vector
        let right_x = -sin_yaw;
        let right_z = cos_yaw;

        let speed = MOVE_SPEED * dt;
        let mut moved = false;

        if self.forward {
            self.position[0] += forward_x * cos_pitch * speed;
            self.position[1] += sin_pitch * speed;
            self.position[2] += forward_z * cos_pitch * speed;
            moved = true;
        }
        if self.backward {
            self.position[0] -= forward_x * cos_pitch * speed;
            self.position[1] -= sin_pitch * speed;
            self.position[2] -= forward_z * cos_pitch * speed;
            moved = true;
        }
        if self.left {
            self.position[0] += right_x * speed;
            self.position[2] += right_z * speed;
            moved = true;
        }
        if self.right {
            self.position[0] -= right_x * speed;
            self.position[2] -= right_z * speed;
            moved = true;
        }
        if self.up {
            self.position[1] += speed;
            moved = true;
        }
        if self.down {
            self.position[1] -= speed;
            moved = true;
        }

        if moved {
            self.changed = true;
        }
    }

    fn to_gpu_camera(&self, aspect_ratio: f64) -> GPUCamera {
        let look_from = crate::Vec3::new(self.position[0], self.position[1], self.position[2]);

        let (sin_yaw, cos_yaw) = self.yaw.sin_cos();
        let (sin_pitch, cos_pitch) = self.pitch.sin_cos();

        let forward = crate::Vec3::new(cos_yaw * cos_pitch, sin_pitch, sin_yaw * cos_pitch);
        let look_at = look_from + forward;

        let camera = Camera::new(look_from, look_at, self.fov, aspect_ratio, 0.0);
        GPUCamera::from_camera(&camera)
    }

    fn take_changed(&mut self) -> bool {
        let was_changed = self.changed;
        self.changed = false;
        was_changed
    }
}

/// Application state for winit event loop
struct RealtimeApp {
    // Window and GPU state
    window: Option<Arc<Window>>,
    device: Option<wgpu::Device>,
    queue: Option<wgpu::Queue>,
    surface: Option<wgpu::Surface<'static>>,
    surface_config: Option<wgpu::SurfaceConfiguration>,

    // Pipeline state
    pathtracer_pipeline: Option<GPUPipeline>,
    blit_pipeline: Option<wgpu::ComputePipeline>,
    blit_bind_group_layout: Option<wgpu::BindGroupLayout>,
    fullscreen_pipeline: Option<wgpu::RenderPipeline>,
    fullscreen_bind_group_layout: Option<wgpu::BindGroupLayout>,
    sampler: Option<wgpu::Sampler>,

    // Scene buffers (static)
    camera_buffer: Option<wgpu::Buffer>,
    params_buffer: Option<wgpu::Buffer>,
    bvh_buffer: Option<wgpu::Buffer>,
    spheres_buffer: Option<wgpu::Buffer>,
    discs_buffer: Option<wgpu::Buffer>,
    materials_buffer: Option<wgpu::Buffer>,
    output_buffer: Option<wgpu::Buffer>,
    pathtracer_bind_group: Option<wgpu::BindGroup>,

    // Blit state
    blit_params_buffer: Option<wgpu::Buffer>,

    // Scene data
    scene: Option<GPUScene>,

    // Render state
    camera_controller: Option<CameraController>,
    sample_count: u32,
    gamma: f64,
    last_frame: Instant,
    frame_count: u64,

    // Initial setup data
    shapes: Option<Vec<GPUShape>>,
    initial_camera: Option<Camera>,
    width: u32,
    height: u32,
}

impl RealtimeApp {
    fn new(shapes: Vec<GPUShape>, camera: Camera, width: u32, height: u32, gamma: f64) -> Self {
        Self {
            window: None,
            device: None,
            queue: None,
            surface: None,
            surface_config: None,
            pathtracer_pipeline: None,
            blit_pipeline: None,
            blit_bind_group_layout: None,
            fullscreen_pipeline: None,
            fullscreen_bind_group_layout: None,
            sampler: None,
            camera_buffer: None,
            params_buffer: None,
            bvh_buffer: None,
            spheres_buffer: None,
            discs_buffer: None,
            materials_buffer: None,
            output_buffer: None,
            pathtracer_bind_group: None,
            blit_params_buffer: None,
            scene: None,
            camera_controller: Some(CameraController::new(&camera)),
            sample_count: 0,
            gamma,
            last_frame: Instant::now(),
            frame_count: 0,
            shapes: Some(shapes),
            initial_camera: Some(camera),
            width,
            height,
        }
    }

    fn initialize_gpu(&mut self, window: Arc<Window>) {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(),
            dx12_shader_compiler: Default::default(),
            flags: wgpu::InstanceFlags::default(),
            gles_minor_version: wgpu::Gles3MinorVersion::default(),
        });

        let surface = instance.create_surface(window.clone()).unwrap();

        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            compatible_surface: Some(&surface),
            force_fallback_adapter: false,
        }))
        .expect("Failed to find GPU adapter");

        println!("GPU: {}", adapter.get_info().name);

        let (device, queue) = pollster::block_on(adapter.request_device(
            &wgpu::DeviceDescriptor {
                label: Some("Realtime Device"),
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::default(),
                memory_hints: Default::default(),
            },
            None,
        ))
        .expect("Failed to create device");

        let size = window.inner_size();
        let surface_caps = surface.get_capabilities(&adapter);
        let surface_format = surface_caps
            .formats
            .iter()
            .find(|f| f.is_srgb())
            .copied()
            .unwrap_or(surface_caps.formats[0]);

        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: size.width.max(1),
            height: size.height.max(1),
            present_mode: wgpu::PresentMode::AutoVsync,
            alpha_mode: surface_caps.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };
        surface.configure(&device, &config);

        // Build scene
        let shapes = self.shapes.take().unwrap();
        let camera = self.initial_camera.as_ref().unwrap();
        let scene = GPUScene::build(shapes, camera);
        println!(
            "Scene: {} spheres, {} discs, {} materials, {} BVH nodes",
            scene.num_spheres,
            scene.num_discs,
            scene.materials.len(),
            scene.bvh_nodes.len()
        );

        // Create pathtracer pipeline
        let pathtracer_pipeline = GPUPipeline::new(&device);

        // Create blit compute pipeline (accumulation buffer -> texture)
        let blit_shader_source = include_str!("../shaders/blit.wgsl");
        let blit_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Blit Shader"),
            source: wgpu::ShaderSource::Wgsl(blit_shader_source.into()),
        });

        let blit_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Blit Bind Group Layout"),
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: true },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 2,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::StorageTexture {
                            access: wgpu::StorageTextureAccess::WriteOnly,
                            format: wgpu::TextureFormat::Rgba8Unorm,
                            view_dimension: wgpu::TextureViewDimension::D2,
                        },
                        count: None,
                    },
                ],
            });

        let blit_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Blit Pipeline Layout"),
            bind_group_layouts: &[&blit_bind_group_layout],
            push_constant_ranges: &[],
        });

        let blit_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("Blit Pipeline"),
            layout: Some(&blit_pipeline_layout),
            module: &blit_shader,
            entry_point: Some("main"),
            compilation_options: Default::default(),
            cache: None,
        });

        // Create fullscreen render pipeline (texture -> surface)
        let fullscreen_shader_source = include_str!("../shaders/fullscreen_blit.wgsl");
        let fullscreen_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Fullscreen Shader"),
            source: wgpu::ShaderSource::Wgsl(fullscreen_shader_source.into()),
        });

        let fullscreen_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Fullscreen Bind Group Layout"),
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                            view_dimension: wgpu::TextureViewDimension::D2,
                            multisampled: false,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                        count: None,
                    },
                ],
            });

        let fullscreen_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Fullscreen Pipeline Layout"),
                bind_group_layouts: &[&fullscreen_bind_group_layout],
                push_constant_ranges: &[],
            });

        let fullscreen_pipeline =
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("Fullscreen Pipeline"),
                layout: Some(&fullscreen_pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &fullscreen_shader,
                    entry_point: Some("vs_main"),
                    buffers: &[],
                    compilation_options: Default::default(),
                },
                fragment: Some(wgpu::FragmentState {
                    module: &fullscreen_shader,
                    entry_point: Some("fs_main"),
                    targets: &[Some(wgpu::ColorTargetState {
                        format: surface_format,
                        blend: None,
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                    compilation_options: Default::default(),
                }),
                primitive: wgpu::PrimitiveState {
                    topology: wgpu::PrimitiveTopology::TriangleList,
                    strip_index_format: None,
                    front_face: wgpu::FrontFace::Ccw,
                    cull_mode: None,
                    unclipped_depth: false,
                    polygon_mode: wgpu::PolygonMode::Fill,
                    conservative: false,
                },
                depth_stencil: None,
                multisample: wgpu::MultisampleState::default(),
                multiview: None,
                cache: None,
            });

        // Create sampler for fullscreen blit
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("Blit Sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });

        // Create buffers
        let width = config.width;
        let height = config.height;
        let output_size = (width * height * 16) as u64;

        let camera_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Camera Buffer"),
            contents: bytemuck::cast_slice(&[scene.camera]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let params_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Params Buffer"),
            size: std::mem::size_of::<GPURenderParams>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let bvh_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("BVH Buffer"),
            contents: bytemuck::cast_slice(&scene.bvh_nodes),
            usage: wgpu::BufferUsages::STORAGE,
        });

        let spheres_data = if scene.spheres.is_empty() {
            vec![crate::gpu_types::GPUSphere::new(GPUVec3::zero(), 0.0, 0)]
        } else {
            scene.spheres.clone()
        };
        let spheres_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Spheres Buffer"),
            contents: bytemuck::cast_slice(&spheres_data),
            usage: wgpu::BufferUsages::STORAGE,
        });

        let discs_data = if scene.discs.is_empty() {
            vec![crate::gpu_types::GPUDisc::new(
                GPUVec3::zero(),
                GPUVec3::new(0.0, 1.0, 0.0),
                0.0,
                0,
            )]
        } else {
            scene.discs.clone()
        };
        let discs_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Discs Buffer"),
            contents: bytemuck::cast_slice(&discs_data),
            usage: wgpu::BufferUsages::STORAGE,
        });

        let materials_data = if scene.materials.is_empty() {
            vec![crate::gpu_types::GPUMaterial::lambertian(GPUVec3::new(
                0.5, 0.5, 0.5,
            ))]
        } else {
            scene.materials.clone()
        };
        let materials_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Materials Buffer"),
            contents: bytemuck::cast_slice(&materials_data),
            usage: wgpu::BufferUsages::STORAGE,
        });

        let output_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Output Buffer"),
            size: output_size,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        // Create pathtracer bind group
        let pathtracer_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Pathtracer Bind Group"),
            layout: pathtracer_pipeline.bind_group_layout(),
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
            ],
        });

        // Blit params buffer
        let blit_params_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Blit Params Buffer"),
            size: std::mem::size_of::<BlitParams>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        self.window = Some(window);
        self.device = Some(device);
        self.queue = Some(queue);
        self.surface = Some(surface);
        self.surface_config = Some(config);
        self.pathtracer_pipeline = Some(pathtracer_pipeline);
        self.blit_pipeline = Some(blit_pipeline);
        self.blit_bind_group_layout = Some(blit_bind_group_layout);
        self.fullscreen_pipeline = Some(fullscreen_pipeline);
        self.fullscreen_bind_group_layout = Some(fullscreen_bind_group_layout);
        self.sampler = Some(sampler);
        self.camera_buffer = Some(camera_buffer);
        self.params_buffer = Some(params_buffer);
        self.bvh_buffer = Some(bvh_buffer);
        self.spheres_buffer = Some(spheres_buffer);
        self.discs_buffer = Some(discs_buffer);
        self.materials_buffer = Some(materials_buffer);
        self.output_buffer = Some(output_buffer);
        self.pathtracer_bind_group = Some(pathtracer_bind_group);
        self.blit_params_buffer = Some(blit_params_buffer);
        self.scene = Some(scene);
    }

    fn render(&mut self) {
        let device = self.device.as_ref().unwrap();
        let queue = self.queue.as_ref().unwrap();
        let surface = self.surface.as_ref().unwrap();
        let config = self.surface_config.as_ref().unwrap();

        // Update timing
        let now = Instant::now();
        let dt = now.duration_since(self.last_frame).as_secs_f64();
        self.last_frame = now;

        // Update camera
        let camera_controller = self.camera_controller.as_mut().unwrap();
        camera_controller.update(dt);

        if camera_controller.take_changed() {
            // Reset accumulation
            self.sample_count = 0;

            // Clear output buffer
            let output_size = (config.width * config.height * 16) as u64;
            let zeros = vec![0u8; output_size as usize];
            queue.write_buffer(self.output_buffer.as_ref().unwrap(), 0, &zeros);

            // Update camera buffer
            let aspect_ratio = config.width as f64 / config.height as f64;
            let gpu_camera = camera_controller.to_gpu_camera(aspect_ratio);
            queue.write_buffer(
                self.camera_buffer.as_ref().unwrap(),
                0,
                bytemuck::cast_slice(&[gpu_camera]),
            );
        }

        // Get surface texture
        let output = match surface.get_current_texture() {
            Ok(texture) => texture,
            Err(wgpu::SurfaceError::Lost | wgpu::SurfaceError::Outdated) => {
                surface.configure(device, config);
                return;
            }
            Err(e) => {
                eprintln!("Surface error: {:?}", e);
                return;
            }
        };

        let scene = self.scene.as_ref().unwrap();

        // Update render params
        let params = GPURenderParams {
            width: config.width,
            height: config.height,
            samples: SAMPLES_PER_FRAME,
            max_depth: 50,
            frame_seed: self.frame_count as u32,
            num_spheres: scene.num_spheres,
            num_discs: scene.num_discs,
            _pad: 0,
        };
        queue.write_buffer(
            self.params_buffer.as_ref().unwrap(),
            0,
            bytemuck::cast_slice(&[params]),
        );

        // Update blit params
        self.sample_count += SAMPLES_PER_FRAME;
        let blit_params = BlitParams {
            sample_count: self.sample_count,
            gamma: self.gamma as f32,
            _pad0: 0,
            _pad1: 0,
        };
        queue.write_buffer(
            self.blit_params_buffer.as_ref().unwrap(),
            0,
            bytemuck::cast_slice(&[blit_params]),
        );

        // Create intermediate texture for blit output
        let blit_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Blit Texture"),
            size: wgpu::Extent3d {
                width: config.width,
                height: config.height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::STORAGE_BINDING | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        let blit_texture_view = blit_texture.create_view(&wgpu::TextureViewDescriptor::default());

        // Create blit bind group
        let blit_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Blit Bind Group"),
            layout: self.blit_bind_group_layout.as_ref().unwrap(),
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: self.blit_params_buffer.as_ref().unwrap().as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: self.output_buffer.as_ref().unwrap().as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::TextureView(&blit_texture_view),
                },
            ],
        });

        // Create fullscreen bind group
        let fullscreen_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Fullscreen Bind Group"),
            layout: self.fullscreen_bind_group_layout.as_ref().unwrap(),
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&blit_texture_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(self.sampler.as_ref().unwrap()),
                },
            ],
        });

        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Render Encoder"),
        });

        // Pathtracer compute pass
        {
            let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("Pathtracer Pass"),
                timestamp_writes: None,
            });
            compute_pass.set_pipeline(self.pathtracer_pipeline.as_ref().unwrap().pipeline());
            compute_pass.set_bind_group(0, self.pathtracer_bind_group.as_ref().unwrap(), &[]);
            let workgroups_x = (config.width + 7) / 8;
            let workgroups_y = (config.height + 7) / 8;
            compute_pass.dispatch_workgroups(workgroups_x, workgroups_y, 1);
        }

        // Blit compute pass (buffer -> texture)
        {
            let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("Blit Pass"),
                timestamp_writes: None,
            });
            compute_pass.set_pipeline(self.blit_pipeline.as_ref().unwrap());
            compute_pass.set_bind_group(0, &blit_bind_group, &[]);
            let workgroups_x = (config.width + 7) / 8;
            let workgroups_y = (config.height + 7) / 8;
            compute_pass.dispatch_workgroups(workgroups_x, workgroups_y, 1);
        }

        // Fullscreen render pass (texture -> surface)
        let output_view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Fullscreen Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &output_view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });
            render_pass.set_pipeline(self.fullscreen_pipeline.as_ref().unwrap());
            render_pass.set_bind_group(0, &fullscreen_bind_group, &[]);
            render_pass.draw(0..3, 0..1);
        }

        queue.submit(Some(encoder.finish()));
        output.present();

        self.frame_count += 1;

        // Print status periodically
        if self.frame_count % 60 == 0 {
            println!(
                "Frame {}: {} samples, {:.1} FPS",
                self.frame_count,
                self.sample_count,
                1.0 / dt
            );
        }
    }

    fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        if new_size.width > 0 && new_size.height > 0 {
            let device = self.device.as_ref().unwrap();
            let surface = self.surface.as_ref().unwrap();
            let config = self.surface_config.as_mut().unwrap();

            config.width = new_size.width;
            config.height = new_size.height;
            surface.configure(device, config);

            // Recreate output buffer for new size
            let output_size = (new_size.width * new_size.height * 16) as u64;
            let output_buffer = device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("Output Buffer"),
                size: output_size,
                usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });

            // Recreate bind group with new output buffer
            let pathtracer_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("Pathtracer Bind Group"),
                layout: self.pathtracer_pipeline.as_ref().unwrap().bind_group_layout(),
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: self.params_buffer.as_ref().unwrap().as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: self.camera_buffer.as_ref().unwrap().as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: self.bvh_buffer.as_ref().unwrap().as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 3,
                        resource: self.spheres_buffer.as_ref().unwrap().as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 4,
                        resource: self.discs_buffer.as_ref().unwrap().as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 5,
                        resource: self.materials_buffer.as_ref().unwrap().as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 6,
                        resource: output_buffer.as_entire_binding(),
                    },
                ],
            });

            self.output_buffer = Some(output_buffer);
            self.pathtracer_bind_group = Some(pathtracer_bind_group);

            // Reset accumulation
            self.sample_count = 0;
            if let Some(controller) = self.camera_controller.as_mut() {
                controller.changed = true;
            }
        }
    }
}

impl ApplicationHandler for RealtimeApp {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_none() {
            let window_attrs = Window::default_attributes()
                .with_title("Pathtracer - WASD to move, mouse to look, ESC to quit")
                .with_inner_size(winit::dpi::LogicalSize::new(self.width, self.height));

            let window = Arc::new(event_loop.create_window(window_attrs).unwrap());
            self.initialize_gpu(window.clone());
        }
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: WindowId,
        event: WindowEvent,
    ) {
        match event {
            WindowEvent::CloseRequested => {
                event_loop.exit();
            }
            WindowEvent::Resized(new_size) => {
                self.resize(new_size);
            }
            WindowEvent::KeyboardInput {
                event:
                    KeyEvent {
                        physical_key: PhysicalKey::Code(key_code),
                        state,
                        ..
                    },
                ..
            } => {
                if key_code == KeyCode::Escape && state == ElementState::Pressed {
                    event_loop.exit();
                    return;
                }
                if let Some(controller) = self.camera_controller.as_mut() {
                    controller.handle_key(key_code, state == ElementState::Pressed);
                }
            }
            WindowEvent::MouseInput { state, button, .. } => {
                if button == MouseButton::Left {
                    if let Some(controller) = self.camera_controller.as_mut() {
                        controller.mouse_captured = state == ElementState::Pressed;
                    }
                }
            }
            WindowEvent::RedrawRequested => {
                self.render();
                if let Some(window) = &self.window {
                    window.request_redraw();
                }
            }
            _ => {}
        }
    }

    fn device_event(
        &mut self,
        _event_loop: &ActiveEventLoop,
        _device_id: winit::event::DeviceId,
        event: DeviceEvent,
    ) {
        if let DeviceEvent::MouseMotion { delta } = event {
            if let Some(controller) = self.camera_controller.as_mut() {
                controller.handle_mouse_move(delta.0, delta.1);
            }
        }
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        if let Some(window) = &self.window {
            window.request_redraw();
        }
    }
}

/// Launch real-time interactive renderer.
///
/// Controls:
/// - WASD: Move camera
/// - Mouse (left click + drag): Look around
/// - Space/Shift: Move up/down
/// - Escape: Exit
pub fn render_realtime(
    shapes: Vec<GPUShape>,
    camera: &Camera,
    width: u32,
    height: u32,
    gamma: f64,
) {
    println!("Starting real-time renderer...");
    println!("Controls: WASD to move, mouse drag to look, Space/Shift for up/down, ESC to quit");

    let event_loop = EventLoop::new().unwrap();
    event_loop.set_control_flow(ControlFlow::Poll);

    let mut app = RealtimeApp::new(shapes, camera.clone(), width, height, gamma);
    event_loop.run_app(&mut app).unwrap();
}
