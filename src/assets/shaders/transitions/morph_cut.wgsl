// Morph cut transition.
// The outgoing clip is horizontally scaled toward zero at the midpoint, while the
// incoming clip expands back out from zero, so the two never overlap.

struct TransitionUniforms {
    progress: f32,
    params: vec4<f32>,
    extra: vec4<f32>,
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
    var corners = array<vec2<f32>, 4>(
        vec2<f32>(-1.0, -1.0),
        vec2<f32>( 1.0, -1.0),
        vec2<f32>(-1.0,  1.0),
        vec2<f32>( 1.0,  1.0),
    );
    let corner = corners[vertexIndex];

    var out: VertexOut;
    out.position = vec4<f32>(corner, 0.0, 1.0);
    out.uv = vec2<f32>(corner.x * 0.5 + 0.5, 0.5 - corner.y * 0.5);
    return out;
}

@fragment
fn fs_main(in: VertexOut) -> @location(0) vec4<f32> {
    let p = clamp(transition.progress, 0.0, 1.0);

    // Squeeze factor: 1 at the ends, 0 at the midpoint.
    let squeeze = abs(p * 2.0 - 1.0);
    let scale = max(squeeze, 1e-3);

    // Horizontal compression towards the centre of the frame.
    let centredUV = vec2<f32>((in.uv.x - 0.5) / scale + 0.5, in.uv.y);

    if (p < 0.5) {
        // Outgoing clip shrinking.
        let a = textureSample(fromTexture, sampler, centredUV);
        // Outside the squeezed band, reveal black.
        if (centredUV.x < 0.0 || centredUV.x > 1.0) {
            return vec4<f32>(0.0, 0.0, 0.0, 1.0);
        }
        return a;
    }

    // Incoming clip expanding back out.
    let b = textureSample(toTexture, sampler, centredUV);
    if (centredUV.x < 0.0 || centredUV.x > 1.0) {
        return vec4<f32>(0.0, 0.0, 0.0, 1.0);
    }
    return b;
}