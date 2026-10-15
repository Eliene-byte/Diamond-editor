// Primary/secondary colour grading shader.
// Consumes the `ColorGradeUniforms` block shared with `gpu::color`.

struct ColorGradeUniforms {
    lift: vec4<f32>,
    gamma: vec4<f32>,
    gain: vec4<f32>,
    offset: vec4<f32>,
    contrast: f32,
    pivot: f32,
    saturation: f32,
    temperature: f32,
    tint: f32,
    shadows: vec4<f32>,
    midtones: vec4<f32>,
    highlights: vec4<f32>,
    _pad: vec3<f32>,
}

@group(0) @binding(0) var inputTexture: texture_2d<f32>;
@group(0) @binding(1) var<uniform> grade: ColorGradeUniforms;
@group(0) @binding(2) var lut3d: texture_3d<f32>;
@group(0) @binding(3) var lut1d: texture_1d<f32>;

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
    let dims = vec2<f32>(textureDimensions(inputTexture));
    let px = vec2<i32>(clamp(in.uv * dims, vec2<f32>(0.0), dims - vec2<f32>(1.0)));
    var color = textureLoad(inputTexture, px, 0);

    // --- Lift / Gamma / Gain ---
    color.rgb = (color.rgb + grade.lift.rgb) * grade.gain.rgb;
    color.rgb = pow(max(color.rgb, vec3<f32>(0.0)), vec3<f32>(1.0) / max(grade.gamma.rgb, vec3<f32>(1e-4)));

    // --- Offset / Contrast / Pivot ---
    color.rgb += grade.offset.rgb;
    color.rgb = (color.rgb - grade.pivot) * grade.contrast + grade.pivot;

    // --- Log wheels: shadows / midtones / highlights ---
    let luma = dot(color.rgb, vec3<f32>(0.2126, 0.7152, 0.0722));
    let shadowMask = smoothstep(0.5, 0.0, luma);
    let highlightMask = smoothstep(0.5, 1.0, luma);
    let midtoneMask = clamp(1.0 - shadowMask - highlightMask, 0.0, 1.0);
    color.rgb += grade.shadows.rgb * shadowMask
               + grade.midtones.rgb * midtoneMask
               + grade.highlights.rgb * highlightMask;

    // --- Temperature / Tint ---
    color.rgb *= vec3<f32>(1.0 + grade.temperature, 1.0, 1.0 - grade.temperature);
    color.rgb *= vec3<f32>(1.0 - grade.tint, 1.0 + grade.tint, 1.0 - grade.tint);

    // --- Saturation ---
    let gradedLuma = dot(color.rgb, vec3<f32>(0.2126, 0.7152, 0.0722));
    color.rgb = mix(vec3<f32>(gradedLuma), color.rgb, grade.saturation);

    // --- 3D LUT (cube) ---
    let lutCoord = clamp(color.rgb, vec3<f32>(0.0), vec3<f32>(1.0));
    color.rgb = textureSampleLevel(lut3d, sampler, lutCoord, 0.0).rgb;

    // --- 1D LUT (per channel shaper) ---
    color.r = textureSampleLevel(lut1d, sampler, color.r, 0.0).r;
    color.g = textureSampleLevel(lut1d, sampler, color.g, 0.0).g;
    color.b = textureSampleLevel(lut1d, sampler, color.b, 0.0).b;

    return color;
}