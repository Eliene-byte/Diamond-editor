// Directional wipe transition. `extra.x` selects the direction:
//   0 = left->right, 1 = right->left, 2 = top->bottom, 3 = bottom->top

struct TransitionUniforms {
    progress: f32,
    // x: border width, y: feather, zw: border colour rgb
    params: vec4<f32>,
    // x: direction, z: unused
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
    let feather = max(transition.params.y, 1e-4);

    // Coordinate that runs 0 -> 1 along the wipe axis.
    var axis: f32;
    if (transition.extra.x < 0.5) {
        axis = in.uv.x;
    } else if (transition.extra.x < 1.5) {
        axis = 1.0 - in.uv.x;
    } else if (transition.extra.x < 2.5) {
        axis = in.uv.y;
    } else {
        axis = 1.0 - in.uv.y;
    }

    let mask = smoothstep(p - feather, p + feather, axis);

    let a = textureSample(fromTexture, sampler, in.uv);
    let b = textureSample(toTexture, sampler, in.uv);
    var color = mix(a, b, mask);

    let border_width = transition.params.x;
    if (border_width > 0.0) {
        if (abs(axis - p) < border_width) {
            color = vec4<f32>(transition.params.zw, 1.0);
        }
    }

    return color;
}