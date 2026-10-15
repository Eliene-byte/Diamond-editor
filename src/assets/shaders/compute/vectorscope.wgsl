// Vectorscope density plot generator.
// Converts each pixel to Cb/Cr and accumulates into a 256x256 polar grid.

@group(0) @binding(0) var<storage, read_write> vectorscopeData: array<atomic<u32>>;
@group(0) @binding(1) var inputTexture: texture_2d<f32>;
@group(0) @binding(2) var<uniform> params: vec4<f32>;

fn rgbToCbCr(rgb: vec3<f32>) -> vec2<f32> {
    let luma = 0.2126 * rgb.r + 0.7152 * rgb.g + 0.0722 * rgb.b;
    let cb = 0.5 + 0.564 * (rgb.b - luma);
    let cr = 0.5 + 0.713 * (rgb.r - luma);
    return vec2<f32>(cb, cr);
}

@compute @workgroup_size(16, 16)
fn cs_main(@builtin(global_invocation_id) id: vec3<u32>) {
    let dims = textureDimensions(inputTexture);
    if (id.x >= dims.x || id.y >= dims.y) {
        return;
    }

    let texel = textureLoad(inputTexture, vec2<i32>(i32(id.x), i32(id.y)), 0);
    let cbcr = rgbToCbCr(texel.rgb);

    let ix = u32(clamp(cbcr.x, 0.0, 1.0) * 255.0);
    let iy = u32(clamp(cbcr.y, 0.0, 1.0) * 255.0);

    atomicAdd(&vectorscopeData[iy * 256u + ix], 1u);
}