// Spatial/temporal noise reduction.
// Bilateral (range-weighted) blur: averages neighbours that are close in both
// space and intensity, which smooths noise while preserving edges.
// params = vec4(spatialSigma, temporalSigma, strength, unused)

@group(0) @binding(0) var outputTexture: texture_storage_2d<rgba16float, write>;
@group(0) @binding(1) var inputTexture: texture_2d<f32>;
@group(0) @binding(2) var<uniform> params: vec4<f32>;

@compute @workgroup_size(16, 16)
fn cs_main(@builtin(global_invocation_id) id: vec3<u32>) {
    let dims = textureDimensions(inputTexture);
    if (id.x >= dims.x || id.y >= dims.y) {
        return;
    }

    let spatialSigma = max(params.x, 1e-3);
    let strength = clamp(params.z, 0.0, 1.0);
    let rangeSigma = max(spatialSigma * 0.35, 1e-3);

    let centre = textureLoad(inputTexture, vec2<i32>(i32(id.x), i32(id.y)), 0);

    let radius = i32(clamp(spatialSigma * 3.0, 1.0, 8.0));

    var accum = vec4<f32>(0.0);
    var weightSum = 0.0;

    for (var dy = -radius; dy <= radius; dy = dy + 1) {
        for (var dx = -radius; dx <= radius; dx = dx + 1) {
            let sx = clamp(i32(id.x) + dx, 0, i32(dims.x) - 1);
            let sy = clamp(i32(id.y) + dy, 0, i32(dims.y) - 1);
            let neighbour = textureLoad(inputTexture, vec2<i32>(sx, sy), 0);

            let dist = length(vec2<f32>(f32(dx), f32(dy)));
            let spatialWeight = exp(-(dist * dist) / (2.0 * spatialSigma * spatialSigma));

            let diff = length(neighbour.rgb - centre.rgb);
            let rangeWeight = exp(-(diff * diff) / (2.0 * rangeSigma * rangeSigma));

            let w = spatialWeight * rangeWeight;
            accum = accum + neighbour * w;
            weightSum = weightSum + w;
        }
    }

    if (weightSum > 0.0) {
        let filtered = accum / weightSum;
        textureStore(outputTexture, vec2<i32>(i32(id.x), i32(id.y)), mix(centre, filtered, strength));
    } else {
        textureStore(outputTexture, vec2<i32>(i32(id.x), i32(id.y)), centre);
    }
}