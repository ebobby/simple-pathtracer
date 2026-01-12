//! GPU rendering module using wgpu compute shaders.

mod context;
mod scene;
mod render;

pub use render::render_gpu;
pub use scene::{GPUScene, GPUShape};
