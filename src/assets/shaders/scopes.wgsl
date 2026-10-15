// Video scopes rendering shader.
// The scope image is supplied as a 1D strip produced by the compute passes
// (waveform / vectorscope / histogram) and visualised here.

struct ScopesUniforms {
    resolution: vec2<f32>,
    // 0 = waveform, 1 = parade, 2 = vectorscope, 3 = histogram
    mode: u32,
    color_space: u32,
    gain: f32,
    _pad: vec3<f32>,
}

@group(0) @binding(0) var scopeTexture: texture_2d<f32>;
@group(0) @binding(1) var<uniform> scopes: ScopesUniforms;

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

fn gridOverlay(uv: vec2<f32>, divisions: f32, strength: f32) -> f32 {
    let gx = abs(fract(uv.x * divisions) - 0.5);
    let gy = abs(fract(uv.y * divisions) - 0.5);
    let line = min(gx, gy);
    return (1.0 - smoothstep(0.0, 0.02, line)) * strength;
}

@fragment
fn fs_waveform(in: VertexOut) -> @location(0) vec4<f32> {
    let dims = vec2<f32>(textureDimensions(scopeTexture));
    let x = u32(clamp(in.uv.x * dims.x, 0.0, dims.x - 1.0));
    let column = textureLoad(scopeTexture, vec2<i32>(i32(x), 0), 0);

    let rLevel = clamp(column.r * scopes.gain, 0.0, 1.0);
    let gLevel = clamp(column.g * scopes.gain, 0.0, 1.0);
    let bLevel = clamp(column.b * scopes.gain, 0.0, 1.0);

    // 0 at the top of the scope, 1 at the bottom.
    var color = vec3<f32>(0.0);
    if (in.uv.y >= 1.0 - rLevel) { color = color + vec3<f32>(1.0, 0.0, 0.0); }
    if (in.uv.y >= 1.0 - gLevel) { color = color + vec3<f32>(0.0, 1.0, 0.0); }
    if (in.uv.y >= 1.0 - bLevel) { color = color + vec3<f32>(0.0, 0.0, 1.0); }

    color = color + vec3<f32>(gridOverlay(in.uv, 10.0, 0.12));
    return vec4<f32>(color, 1.0);
}

@fragment
fn fs_parade(in: VertexOut) -> @location(0) vec4<f32> {
    let dims = vec2<f32>(textureDimensions(scopeTexture));

    // Three side-by-side thirds, one per channel.
    let third = floor(in.uv.x * 3.0);
    let withinThird = fract(in.uv.x * 3.0);
    let channel = i32(clamp(third, 0.0, 2.0));

    let x = u32(clamp(withinThird * dims.x, 0.0, dims.x - 1.0));
    let column = textureLoad(scopeTexture, vec2<i32>(i32(x), 0), 0);

    var level = column.r;
    if (channel == 1) { level = column.g; }
    if (channel == 2) { level = column.b; }

    var color = vec3<f32>(0.0);
    if (in.uv.y >= 1.0 - clamp(level * scopes.gain, 0.0, 1.0)) {
        if (channel == 0) { color = vec3<f32>(1.0, 0.0, 0.0); }
        else if (channel == 1) { color = vec3<f32>(0.0, 1.0, 0.0); }
        else { color = vec3<f32>(0.0, 0.0, 1.0); }
    }

    // Separators between the three channels.
    if (withinThird < 0.004 || withinThird > 0.996) {
        color = vec3<f32>(0.25);
    }

    return vec4<f32>(color, 1.0);
}

@fragment
fn fs_vectorscope(in: VertexOut) -> @location(0) vec4<f32> {
    let dims = vec2<f32>(textureDimensions(scopeTexture));
    let px = vec2<i32>(clamp(in.uv * dims, vec2<f32>(0.0), dims - vec2<f32>(1.0)));
    let density = textureLoad(scopeTexture, px, 0);

    // Rec.709 luma of the density plot is mapped to brightness.
    let brightness = clamp(density.r * scopes.gain, 0.0, 1.0);
    var color = vec3<f32>(brightness * 0.4, brightness, brightness * 0.8);

    // Target rings at 75% and 100% saturation.
    let centred = (in.uv - vec2<f32>(0.5)) * 2.0;
    let radius = length(centred);
    if (abs(radius - 0.75) < 0.006 || abs(radius - 1.0) < 0.006) {
        color = vec3<f32>(0.35, 0.35, 0.35);
    }
    // Crosshair through the centre.
    if (abs(centred.x) < 0.003 || abs(centred.y) < 0.003) {
        color = vec3<f32>(0.25, 0.25, 0.25);
    }

    return vec4<f32>(color, 1.0);
}

@fragment
fn fs_histogram(in: VertexOut) -> @location(0) vec4<f32> {
    let dims = vec2<f32>(textureDimensions(scopeTexture));
    let x = u32(clamp(in.uv.x * dims.x, 0.0, dims.x - 1.0));
    let column = textureLoad(scopeTexture, vec2<i32>(i32(x), 0), 0);

    var color = vec3<f32>(0.0);
    if (in.uv.y >= 1.0 - clamp(column.r * scopes.gain, 0.0, 1.0)) { color = color + vec3<f32>(1.0, 0.0, 0.0); }
    if (in.uv.y >= 1.0 - clamp(column.g * scopes.gain, 0.0, 1.0)) { color = color + vec3<f32>(0.0, 1.0, 0.0); }
    if (in.uv.y >= 1.0 - clamp(column.b * scopes.gain, 0.0, 1.0)) { color = color + vec3<f32>(0.0, 0.0, 1.0); }
    if (in.uv.y >= 1.0 - clamp(column.a * scopes.gain, 0.0, 1.0)) { color = color + vec3<f32>(1.0); }

    return vec4<f32>(color, 1.0);
}