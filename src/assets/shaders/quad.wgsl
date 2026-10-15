// Fullscreen textured-quad shader used by every UI widget and the viewer.
// Draws a quad with optional opacity and blend mode.

struct QuadUniforms {
    // xy: destination rect origin (NDC), zw: destination size (NDC)
    dest_rect: vec4<f32>,
    // xy: source rect origin (uv), zw: source rect size (uv)
    src_rect: vec4<f32>,
    opacity: f32,
    // 0 = normal, 1 = add, 2 = multiply, ...
    blend_mode: u32,
    _pad: vec2<f32>,
}

@group(0) @binding(0) var sourceTexture: texture_2d<f32>;
@group(0) @binding(1) var sourceSampler: sampler;
@group(0) @binding(2) var<uniform> quad: QuadUniforms;

struct VertexOut {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
}

@vertex
fn vs_main(@builtin(vertex_index) vertexIndex: u32) -> VertexOut {
    // Unit quad in [0,1] space, as a triangle strip.
    var corners = array<vec2<f32>, 4>(
        vec2<f32>(0.0, 0.0),
        vec2<f32>(1.0, 0.0),
        vec2<f32>(0.0, 1.0),
        vec2<f32>(1.0, 1.0),
    );

    let corner = corners[vertexIndex];

    // Map the unit quad into the destination rect given in NDC.
    let ndc = quad.dest_rect.xy + corner * quad.dest_rect.zw;

    var out: VertexOut;
    out.position = vec4<f32>(ndc, 0.0, 1.0);
    // Destination Y grows downward on screen; uv origin is top-left.
    out.uv = vec2<f32>(corner.x, 1.0 - corner.y);
    return out;
}

@fragment
fn fs_main(in: VertexOut) -> @location(0) vec4<f32> {
    let uv = quad.src_rect.xy + in.uv * quad.src_rect.zw;
    let texel = textureSample(sourceTexture, sourceSampler, uv);

    if (quad.blend_mode == 0u) {
        // Normal: premultiply-free straight alpha.
        return vec4<f32>(texel.rgb, texel.a * quad.opacity);
    }
    if (quad.blend_mode == 1u) {
        // Add: accumulate luminance, keep alpha.
        return vec4<f32>(texel.rgb * quad.opacity, texel.a);
    }
    if (quad.blend_mode == 2u) {
        // Multiply.
        return vec4<f32>(texel.rgb * texel.rgb * quad.opacity, texel.a);
    }
    return vec4<f32>(texel.rgb, texel.a * quad.opacity);
}

// Plain vertex-colour quad (used for solid rectangles such as panel backgrounds).
struct SolidUniforms {
    rect: vec4<f32>,
    color: vec4<f32>,
}

@group(1) @binding(0) var<uniform> solid: SolidUniforms;

@vertex
fn vs_solid(@builtin(vertex_index) vertexIndex: u32) -> VertexOut {
    var corners = array<vec2<f32>, 4>(
        vec2<f32>(0.0, 0.0),
        vec2<f32>(1.0, 0.0),
        vec2<f32>(0.0, 1.0),
        vec2<f32>(1.0, 1.0),
    );
    let corner = corners[vertexIndex];
    let ndc = solid.rect.xy + corner * solid.rect.zw;

    var out: VertexOut;
    out.position = vec4<f32>(ndc, 0.0, 1.0);
    out.uv = corner;
    return out;
}

@fragment
fn fs_solid(in: VertexOut) -> @location(0) vec4<f32> {
    return solid.color;
}