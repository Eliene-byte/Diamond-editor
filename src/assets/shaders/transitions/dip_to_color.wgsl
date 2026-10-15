// Dip to color: fade out to a flat colour, then fade back in from it.

struct TransitionUniforms {
    progress: f32,
    // rgb: dip colour, a: dip amount (0 = plain dissolve, 1 = full dip)
    params: vec4<f32>,
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
    let amount = transition.params.a;
    let dipColor = vec4<f32>(transition.params.rgb, 1.0);

    let a = textureSample(fromTexture, sampler, in.uv);
    let b = textureSample(toTexture, sampler, in.uv);

    // plain = linear dissolve; dipped = triangular weights peaking at p = 0.5.
    let plain = mix(a, b, p);

    // outWeight is 1 at p=0, 0 at p=0.5; inWeight is the mirror.
    let outWeight = clamp(1.0 - p * 2.0, 0.0, 1.0);
    let inWeight = clamp(p * 2.0 - 1.0, 0.0, 1.0);

    // Sum of weights is 1 everywhere, so this blends toward the dip colour
    // exactly at the midpoint, and is equivalent to `plain` at the endpoints.
    let dipped = (a * outWeight + b * inWeight) + dipColor * (1.0 - (outWeight + inWeight));

    return mix(plain, dipped, amount);
}