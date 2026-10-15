// Linear wipe: a hard edge sweeps across from left to right.

struct TransitionUniforms {
    progress: f32,
    // x: border width (0..0.2), y: feather (0..0.2), zw: border color rgb
    params: vec4<f32>,
    // x: direction (0 = left->right), z: unused
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

    // Distance from the wipe edge, in uv space, measured along x.
    let edge = p;
    let feather = max(transition.params.y, 1e-4);

    // 0 while the wipe edge is to the right of this pixel, 1 once it has passed.
    var mask = smoothstep(edge - feather, edge + feather, in.uv.x);

    let a = textureSample(fromTexture, sampler, in.uv);
    let b = textureSample(toTexture, sampler, in.uv);
    var color = mix(a, b, mask);

    // Optional border line drawn right on the wipe edge.
    let border_width = transition.params.x;
    if (border_width > 0.0) {
        let dist = abs(in.uv.x - edge);
        if (dist < border_width) {
            color = vec4<f32>(transition.params.zw, 1.0);
        }
    }

    return color;
}