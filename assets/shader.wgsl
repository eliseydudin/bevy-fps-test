#import bevy_core_pipeline::fullscreen_vertex_shader::FullscreenVertexOutput

@group(0) @binding(0) var screen_texture: texture_2d<f32>;
@group(0) @binding(1) var texture_sampler: sampler;
struct PostProcessSettings {
    intensity: f32,
    block_size: f32
#ifdef SIXTEEN_BYTE_ALIGNMENT
    // WebGL2 structs must be 16 byte aligned.
    _webgl2_padding: vec2<f32>
#endif
}
@group(0) @binding(2) var<uniform> settings: PostProcessSettings;

fn limit(f: f32) -> f32 {
    let color_limit = settings.intensity;
    return trunc(255.0 * f) / color_limit / 255.0;
}

@fragment
fn fragment(in: FullscreenVertexOutput) -> @location(0) vec4<f32> {
    let resolution = vec2<f32>(textureDimensions(screen_texture));

    let width_height_over_block_size = resolution / max(1.0, settings.block_size);

    var uv = in.uv + 0.5;
    uv *= width_height_over_block_size;
    uv = floor(uv);
    uv /= width_height_over_block_size;
    uv -= 0.5;

    let color = textureSample(screen_texture, texture_sampler, uv); 

    return vec4<f32>(limit(color.r), limit(color.g), limit(color.b), 1.0);
}
