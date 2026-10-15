// Video stabilisation resample.
// Applies the per-frame transform produced by the stabiliser to the source frame.
// params = vec4(translateX, translateY, scale, rotation)

@group(0) @binding(0) var outputTexture: texture_storage_2d<rgba16float, write>;
@group(0) @binding(1) var inputTexture: texture_2d<f32>;
@group(0) @binding(2) var<uniform> params: vec4<f32>;

@compute @workgroup_size(16, 16)
fn cs_main(@builtin(global_invocation_id) id: vec3<u32>) {
    let dims = textureDimensions(inputTexture);
    if (id.x >= dims.x || id.y >= dims.y) {
        return;
    }

    let scale = max(params.z, 1e-3);
    let rot = params.w;

    // Inverse-transform this output texel into source space.
    let centre = vec2<f32>(0.5, 0.5);
    let p = (vec2<f32>(f32(id.x) + 0.5, f32(id.y) + 0.5) / vec2<f32>(dims)) - centre;

    let c = cos(rot);
    let s = sin(rot);
    let rotated = vec2<f32>(p.x * c + p.y * s, -p.x * s + p.y * c);

    let sourceUV = rotated / scale + centre + vec2<f32>(params.x, params.y);

    if (sourceUV.x < 0.0 || sourceUV.x > 1.0 || sourceUV.y < 0.0 || sourceUV.y > 1.0) {
        // Outside the valid area after transform: fill with black.
        textureStore(outputTexture, vec2<i32>(i32(id.x), i32(id.y)), vec4<f32>(0.0, 0.0, 0.0, 1.0));
        return;
    }

    let px = vec2<i32>(
        clamp(sourceUV * vec2<f32>(dims), vec2<f32>(0.0), vec2<f32>(dims) - vec2<f32>(1.0))
    );
    let colour = textureLoad(inputTexture, px, 0);
    textureStore(outputTexture, vec2<i32>(i32(id.x), i32(id.y)), colour);
}