// Optical flow estimation placeholder.
// Produces a zero flow field so downstream stabilisation has a valid input while
// the multi-scale Lucas-Kanade solver is being implemented.
// params = vec4(scale, iterations, smoothness, unused)

@group(0) @binding(0) var outputFlow: texture_storage_2d<rg32float, write>;
@group(0) @binding(1) var prevFrame: texture_2d<f32>;
@group(0) @binding(2) var currFrame: texture_2d<f32>;
@group(0) @binding(3) var<uniform> params: vec4<f32>;

@compute @workgroup_size(16, 16)
fn cs_main(@builtin(global_invocation_id) id: vec3<u32>) {
    let dims = textureDimensions(prevFrame);
    if (id.x >= dims.x || id.y >= dims.y) {
        return;
    }
    // No motion estimate yet: write a neutral flow vector for this texel.
    textureStore(outputFlow, vec2<i32>(i32(id.x), i32(id.y)), vec2<f32>(0.0, 0.0));
}