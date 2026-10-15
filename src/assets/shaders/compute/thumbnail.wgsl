// Thumbnail generator: box-downsamples the source frame into a smaller texture.
// params.x = output width, params.y = output height.

@group(0) @binding(0) var outputTexture: texture_storage_2d<rgba8unorm, write>;
@group(0) @binding(1) var inputTexture: texture_2d<f32>;
@group(0) @binding(2) var<uniform> params: vec4<f32>;

@compute @workgroup_size(16, 16)
fn cs_main(@builtin(global_invocation_id) id: vec3<u32>) {
    let outDims = textureDimensions(outputTexture);
    if (id.x >= outDims.x || id.y >= outDims.y) {
        return;
    }

    let srcDims = vec2<f32>(textureDimensions(inputTexture));
    let outDimsF = vec2<f32>(outDims);

    // Map the destination texel onto the corresponding source rectangle.
    let scale = srcDims / outDimsF;
    let origin = vec2<f32>(f32(id.x), f32(id.y)) * scale;
    let extent = scale;

    let x0 = u32(max(floor(origin.x), 0.0));
    let y0 = u32(max(floor(origin.y), 0.0));
    let x1 = u32(min(ceil(origin.x + extent.x), srcDims.x));
    let y1 = u32(min(ceil(origin.y + extent.y), srcDims.y));

    var accum = vec4<f32>(0.0);
    var count = 0.0;
    for (var y = y0; y < max(y1, y0 + 1u); y = y + 1u) {
        for (var x = x0; x < max(x1, x0 + 1u); x = x + 1u) {
            accum = accum + textureLoad(inputTexture, vec2<i32>(i32(x), i32(y)), 0);
            count = count + 1.0;
        }
    }

    if (count > 0.0) {
        textureStore(outputTexture, vec2<i32>(i32(id.x), i32(id.y)), accum / count);
    } else {
        textureStore(outputTexture, vec2<i32>(i32(id.x), i32(id.y)), vec4<f32>(0.0, 0.0, 0.0, 1.0));
    }
}