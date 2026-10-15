// Directional slide transition. `extra.x` selects the slide direction.

struct TransitionUniforms {
    progress: f32,
    params: vec4<f32>,
    // x: direction (0=left, 1=right, 2=up, 3=down)
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
    let travel = 1.0 - p;

    var slid: vec2<f32>;
    if (transition.extra.x < 0.5) {
        slid = vec2<f32>(in.uv.x + travel, in.uv.y);
    } else if (transition.extra.x < 1.5) {
        slid = vec2<f32>(in.uv.x - travel, in.uv.y);
    } else if (transition.extra.x < 2.5) {
        slid = vec2<f32>(in.uv.x, in.uv.y - travel);
    } else {
        slid = vec2<f32>(in.uv.x, in.uv.y + travel);
    }

    let a = textureSample(fromTexture, sampler, in.uv);

    if (slid.x < 0.0 || slid.x > 1.0 || slid.y < 0.0 || slid.y > 1.0) {
        return a;
    }
    return textureSample(toTexture, sampler, slid);
}