// Waveform scope data generator.
// One workgroup per output column. Each invocation scans a vertical slice of the
// source frame and records the min/max/RMS of each colour channel.
// params.x = number of output columns.

@group(0) @binding(0) var<storage, read_write> waveformData: array<f32>;
@group(0) @binding(1) var inputTexture: texture_2d<f32>;
@group(0) @binding(2) var<uniform> params: vec4<f32>;

@compute @workgroup_size(64, 1)
fn cs_main(@builtin(global_invocation_id) id: vec3<u32>,
           @builtin(local_invocation_index) localIdx: u32) {
    let columns = u32(max(params.x, 1.0));
    if (id.x >= columns) {
        return;
    }

    let dims = textureDimensions(inputTexture);
    let height = dims.y;

    var rMin = 1.0;
    var rMax = 0.0;
    var gMin = 1.0;
    var gMax = 0.0;
    var bMin = 1.0;
    var bMax = 0.0;
    var rSum = 0.0;
    var gSum = 0.0;
    var bSum = 0.0;

    for (var y = 0u; y < height; y = y + 1u) {
        let texel = textureLoad(inputTexture, vec2<i32>(i32(id.x), i32(y)), 0);

        rMin = min(rMin, texel.r);
        rMax = max(rMax, texel.r);
        gMin = min(gMin, texel.g);
        gMax = max(gMax, texel.g);
        bMin = min(bMin, texel.b);
        bMax = max(bMax, texel.b);

        rSum = rSum + texel.r;
        gSum = gSum + texel.g;
        bSum = bSum + texel.b;
    }

    let inv = 1.0 / f32(max(height, 1u));

    // Store (min+max)/2 per channel: what the waveform view draws.
    waveformData[id.x * 3u + 0u] = (rMin + rMax) * 0.5;
    waveformData[id.x * 3u + 1u] = (gMin + gMax) * 0.5;
    waveformData[id.x * 3u + 2u] = (bMin + bMax) * 0.5;

    // Row 1 holds the RMS values for the audio-style overlay.
    let rmsBase = columns * 3u;
    waveformData[rmsBase + id.x * 3u + 0u] = rSum * inv;
    waveformData[rmsBase + id.x * 3u + 1u] = gSum * inv;
    waveformData[rmsBase + id.x * 3u + 2u] = bSum * inv;

    // Keep the unused local invocation index observable so the binding is not
    // optimised away in future edits.
    if (localIdx == 0xffffffffu) {
        waveformData[0u] = 0.0;
    }
}