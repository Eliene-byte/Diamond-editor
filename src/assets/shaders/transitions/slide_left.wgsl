// Slide: the incoming clip slides in on top of the outgoing clip, which stays put.

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

    let a = textureSample(fromTexture, sampler, in.uv);
    // Slide in from the right: uv is offset left by (1 - progress).
    let slid_uv = vec2<f32>(in.uv.x + (1.0 - p), in.uv.y);

    // Outside [0,1] the incoming clip is off-screen, so show `a` there.
    if (slid_uv.x < 0.0 || slid_uv.x > 1.0) {
        return a;
    }

    let b = textureSample(toTexture, sampler, slid_uv);
    return b;
}