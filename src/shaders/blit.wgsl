// Blit shader - copies from accumulation buffer to screen texture
// with gamma correction and sample normalization

struct BlitParams {
    sample_count: u32,
    gamma: f32,
    render_width: u32,  // size of the traced grid, upscaled to the window
    render_height: u32,
}

@group(0) @binding(0) var<uniform> params: BlitParams;
@group(0) @binding(1) var<storage, read> accumulation: array<vec4<f32>>;
@group(0) @binding(2) var output_texture: texture_storage_2d<rgba16float, write>;

@compute @workgroup_size(8, 8)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let dims = textureDimensions(output_texture);
    if global_id.x >= dims.x || global_id.y >= dims.y {
        return;
    }

    // Nearest-neighbour upscale when the tracer ran at reduced resolution
    let sx = global_id.x * params.render_width / dims.x;
    let sy = global_id.y * params.render_height / dims.y;
    let idx = sy * params.render_width + sx;
    let accumulated = accumulation[idx];

    // Normalize by sample count
    let sample_scale = 1.0 / f32(max(params.sample_count, 1u));
    var color = accumulated.xyz * sample_scale;

    // Clamp to valid range (sRGB surface handles gamma)
    color = clamp(color, vec3<f32>(0.0), vec3<f32>(1.0));

    textureStore(output_texture, vec2<i32>(global_id.xy), vec4<f32>(color, 1.0));
}
