// Deliver/transcode colour conversion.
// Converts the working-space float frame to an 8-bit output target, applying a
// gamma encode when the destination is a display-referred transfer function.
// params = vec4(bitDepth, encodeGamma, colourSpace, unused)

@group(0) @binding(0) var outputTexture: texture_storage_2d<rgba8unorm, write>;
@group(0) @binding(1) var inputTexture: texture_2d<f32>;
@group(0) @binding(2) var<uniform> params: vec4<f32>;

fn linearToSrgb(c: f32) -> f32 {
    if (c <= 0.0031308) {
        return c * 12.92;
    }
    return 1.055 * pow(c, 1.0 / 2.4) - 0.055;
}

@compute @workgroup_size(16, 16)
fn cs_main(@builtin(global_invocation_id) id: vec3<u32>) {
    let dims = textureDimensions(inputTexture);
    if (id.x >= dims.x || id.y >= dims.y) {
        return;
    }

    var colour = textureLoad(inputTexture, vec2<i32>(i32(id.x), i32(id.y)), 0);

    // Clamp out-of-range HDR values before quantisation.
    colour = clamp(colour, vec4<f32>(0.0), vec4<f32>(1.0));

    if (params.y > 0.5) {
        colour = vec4<f32>(linearToSrgb(colour.r), linearToSrgb(colour.g), linearToSrgb(colour.b), colour.a);
    }

    textureStore(outputTexture, vec2<i32>(i32(id.x), i32(id.y)), colour);
}