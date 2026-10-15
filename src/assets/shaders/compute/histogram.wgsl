// Histogram scope data generator.
// Bins every pixel into 256 buckets per channel (R, G, B, Luma).
// params.x = enabled flag (unused for now, reserved).

@group(0) @binding(0) var<storage, read_write> histogramData: array<atomic<u32>>;
@group(0) @binding(1) var inputTexture: texture_2d<f32>;
@group(0) @binding(2) var<uniform> params: vec4<f32>;

@compute @workgroup_size(16, 16)
fn cs_main(@builtin(global_invocation_id) id: vec3<u32>) {
    let dims = textureDimensions(inputTexture);
    if (id.x >= dims.x || id.y >= dims.y) {
        return;
    }

    let texel = textureLoad(inputTexture, vec2<i32>(i32(id.x), i32(id.y)), 0);

    let rBin = u32(clamp(texel.r, 0.0, 1.0) * 255.0);
    let gBin = u32(clamp(texel.g, 0.0, 1.0) * 255.0);
    let bBin = u32(clamp(texel.b, 0.0, 1.0) * 255.0);

    let luma = clamp(0.2126 * texel.r + 0.7152 * texel.g + 0.0722 * texel.b, 0.0, 1.0);
    let lBin = u32(luma * 255.0);

    atomicAdd(&histogramData[rBin], 1u);
    atomicAdd(&histogramData[256u + gBin], 1u);
    atomicAdd(&histogramData[512u + bBin], 1u);
    atomicAdd(&histogramData[768u + lBin], 1u);
}