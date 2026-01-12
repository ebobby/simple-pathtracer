// Fullscreen blit shader - renders a fullscreen triangle and samples from a texture

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
}

// Fullscreen triangle trick - no vertex buffer needed
// Creates a single triangle that covers the entire screen
@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32) -> VertexOutput {
    var out: VertexOutput;

    // Generate fullscreen triangle vertices using standard trick
    // vertex 0: (-1, -1), vertex 1: (3, -1), vertex 2: (-1, 3)
    var x: f32;
    var y: f32;

    if vertex_index == 0u {
        x = -1.0;
        y = -1.0;
    } else if vertex_index == 1u {
        x = 3.0;
        y = -1.0;
    } else {
        x = -1.0;
        y = 3.0;
    }

    out.position = vec4<f32>(x, y, 0.0, 1.0);
    // UV coordinates (0,0 top-left to 1,1 bottom-right)
    out.uv = vec2<f32>((x + 1.0) / 2.0, (1.0 - y) / 2.0);

    return out;
}

@group(0) @binding(0) var blit_texture: texture_2d<f32>;
@group(0) @binding(1) var blit_sampler: sampler;

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    return textureSample(blit_texture, blit_sampler, in.uv);
}
