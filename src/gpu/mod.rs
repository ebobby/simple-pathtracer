//! GPU rendering module using wgpu compute shaders.

mod context;
mod scene;
mod render;
mod realtime;

pub use render::{render_gpu, render_gpu_linear};
pub use realtime::render_realtime;
pub use scene::{GPUScene, GPUShape};
