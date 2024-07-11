#import bevy_core_pipeline::fullscreen_vertex_shader::FullscreenVertexOutput

const palette_size = 8;
const SNESPalette = array<vec4<f32>, palette_size>(
    vec4<f32>(0x7 / 255.0, 0xF / 255.0, 0x2b / 255.0, 1.0),     
    //1B1A55
    vec4<f32>(0x1B / 255.0, 0x1A / 255.0, 0x55 / 255.0, 1.0),     
    //535C91
    vec4<f32>(0x53 / 255.0, 0x5C / 255.0, 0x91 / 255.0, 1.0),     
    //9290C3
    vec4<f32>(0x92 / 255.0, 0x90 / 255.0, 0xc3 / 255.0, 1.0),     
    //(0x19, 0x17, 0x3c)
    vec4<f32>(0x19 / 255.0, 0x17 / 255.0, 0x3c / 255.0, 1.0),
    //0F3460
    vec4<f32>(0x0f / 255.0, 0x34 / 255.0, 0x60 / 255.0, 1.0),
    //435585
    vec4<f32>(0x43 / 255.0, 0x55 / 255.0, 0x85 / 255.0, 1.0),
    //1F6E8C
    vec4<f32>(0x1f / 255.0, 0x6e / 255.0, 0x8c / 255.0, 1.0),
);

// Function to find the closest color in the SNES palette
fn closestSNESColor(color: vec4<f32>) -> vec4<f32> {
    var palette: array<vec4<f32>, palette_size> = SNESPalette;
    var closestColor: vec4<f32> = palette[0];
    var closestDistanceSq: f32 = distance(color, closestColor);

    for (var i : i32 = 1; i < palette_size; i = i + 1) {
        let currentColor: vec4<f32> = palette[i];
        let currentDistanceSq: f32 = distance(color, currentColor);

        if (currentDistanceSq < closestDistanceSq) {
            closestColor = currentColor;
            closestDistanceSq = currentDistanceSq;
        }
    }

    return closestColor;
}

@group(0) @binding(0) var screen_texture: texture_2d<f32>;
@group(0) @binding(1) var texture_sampler: sampler;
struct PostProcessSettings {
    intensity: f32,
#ifdef SIXTEEN_BYTE_ALIGNMENT
    // WebGL2 structs must be 16 byte aligned.
    _webgl2_padding: vec3<f32>
#endif
}
@group(0) @binding(2) var<uniform> settings: PostProcessSettings;

@fragment
fn fragment(in: FullscreenVertexOutput) -> @location(0) vec4<f32> {
    // Chromatic aberration strength
    let offset_strength = settings.intensity;
    let curr_color: vec4<f32> = textureSample(screen_texture, texture_sampler, in.uv);
    let color = closestSNESColor(curr_color);
    return vec4<f32>(
        color.r, color.g, color.b, 1.0
    );
}
