// Push: the outgoing clip slides out while the incoming clip slides in behind it,
// creating the illusion that the incoming clip is pushing the outgoing one off.

struct TransitionUniforms {
    progress: f32,
    _pad: vec3<f32>,
}

@group(0) @binding(0) var fromTexture: texture_2d<f32>;
@group(0) @binding(1) var toTexture: texture_2d<f32>;
@group(0) @binding(2) var<uniform> transition: TransitionUniforms;

struct VertexOut {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
}

@vertex
fn vs_main(@builtin(vertex_index) vertexIndex: u32) -> VertexOut {
    var positions = array<vec2<f32>, 4>(
        vec2<f32>(-1.0, -1.0),
        vec2<f32>( 1.0, -1.0),
        vec2<f32>(-1.0,  1.0),
        vec2<f32>( 1.0,  1.0),
    );

    var out: VertexOut;
    let pos = positions[vertexIndex];
    out.position = vec4<f32>(pos, 0.0, 1.0);
    out.uv = pos * vec2<f32>(0.5, -0.5) + vec2<f32>(0.5, 0.5);
    return out;
}

@fragment
fn fs_main(in: VertexOut) -> @location(0) vec4<f32> {
    let p = clamp(transition.progress, 0.0, 1.0);

    // Outgoing clip is pushed to the left by p.
    let out_uv = vec2<f32>(in.uv.x + p, in.uv.y);
    // Incoming clip travels with it, from off-screen right, so its uv is unchanged
    // relative to screen space while it is entering.
    let in_uv = vec2<f32>(in.uv.x, in.uv.y);

    if (out_uv.x <= 1.0) {
        // Still (partly) showing the outgoing clip on top.
        return textureSample(fromTexture, sampler, out_uv);
    }

    // Outgoing clip has left the frame; incoming clip is now fully visible.
    return textureSample(toTexture, sampler, in_uv);
}