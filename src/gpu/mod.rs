//! GPU rendering module using wgpu compute shaders.

mod context;
mod scene;
mod render;
mod realtime;

pub use render::{
    render_gpu, render_gpu_linear, render_gpu_linear_with_environment, render_gpu_with,
    render_gpu_with_environment,
};
pub use realtime::{render_realtime, render_realtime_with, render_realtime_with_environment};
pub use scene::{GPUScene, GPUShape};
