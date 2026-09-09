// Blit shader - copies from accumulation buffer to screen texture
// with gamma correction and sample normalization

struct BlitParams {
    sample_count: u32,
    exposure: f32,
    render_width: u32,  // size of the traced grid, upscaled to the window
    render_height: u32,
    curve: u32,         // 0 clamp, 1 Reinhard, 2 ACES (mirrors src/tonemap.rs)
    _pad0: u32,
    _pad1: u32,
    _pad2: u32,
}

fn tone_curve(x: vec3<f32>) -> vec3<f32> {
    let v = max(x, vec3<f32>(0.0));
    if params.curve == 1u {
        return v / (1.0 + v);
    }
    if params.curve == 2u {
        return clamp((v * (2.51 * v + 0.03)) / (v * (2.43 * v + 0.59) + 0.14), vec3<f32>(0.0), vec3<f32>(1.0));
    }
    return min(v, vec3<f32>(1.0));
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

    // Normalize by sample count, apply exposure and the tone curve
    // (the sRGB surface handles gamma)
    let sample_scale = 1.0 / f32(max(params.sample_count, 1u));
    let color = tone_curve(accumulated.xyz * sample_scale * params.exposure);

    textureStore(output_texture, vec2<i32>(global_id.xy), vec4<f32>(color, 1.0));
}
