#import bevy_sprite::mesh2d_vertex_output::VertexOutput

@group(2) @binding(0) var<uniform> time: f32;
@group(2) @binding(1) var base_texture: texture_2d<f32>;
@group(2) @binding(2) var base_sampler: sampler;

@fragment
fn fragment(
    input: VertexOutput,
) -> @location(0) vec4<f32> {
    let wave = 0.05 * sin(20 * input.uv.x + time * 10);
    let wave_uv = vec2f(input.uv.x, input.uv.y + wave);

    return textureSample(base_texture, base_sampler, wave_uv);
}
