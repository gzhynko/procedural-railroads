#import bevy_pbr::mesh_view_bindings
#import bevy_pbr::mesh_bindings

#import bevy_pbr::utils
#import bevy_pbr::clustered_forward
#import bevy_pbr::lighting
#import bevy_pbr::shadows
#import bevy_pbr::pbr_types
#import bevy_pbr::pbr_functions

struct Fog {
    color: vec4<f32>,
    density_or_start: f32,
    end: f32,
}
struct TerrainMaterial {
    fog: Fog,
}

@group(1) @binding(0)
var<uniform> terrain_material: TerrainMaterial;
@group(1) @binding(1)
var<uniform> grass_pbr_material: StandardMaterial;
@group(1) @binding(2)
var<uniform> rock_pbr_material: StandardMaterial;

@group(1) @binding(3)
var grass_albedo: texture_2d<f32>;
@group(1) @binding(4)
var grass_albedo_sampler: sampler;

@group(1) @binding(5)
var rock_albedo: texture_2d<f32>;
@group(1) @binding(6)
var rock_albedo_sampler: sampler;

@group(1) @binding(7)
var grass_normal: texture_2d<f32>;
@group(1) @binding(8)
var grass_normal_sampler: sampler;

@group(1) @binding(9)
var rock_normal: texture_2d<f32>;
@group(1) @binding(10)
var rock_normal_sampler: sampler;

fn exponential_fog(
    distance: f32,
) -> vec4<f32> {
    var result = terrain_material.fog.color;
    result.a *= 1.0 - 1.0 / exp(pow(distance * terrain_material.fog.density_or_start, 2.0));
    return result;
}

struct FragmentInput {
    @builtin(front_facing) is_front: bool,
    @builtin(position) frag_coord: vec4<f32>,
    #import bevy_pbr::mesh_vertex_output
};

// A copy of https://github.com/bevyengine/bevy/blob/master/crates/bevy_pbr/src/render/pbr.wgsl
fn get_pbr_color(
    base_color: vec4<f32>,
    in: FragmentInput,
    material: StandardMaterial
) -> vec4<f32> {
    var output_color: vec4<f32> = base_color;
    var pbr_input: PbrInput;

    pbr_input.material.base_color = base_color;
    pbr_input.material.reflectance = material.reflectance;
    pbr_input.material.flags = material.flags;
    pbr_input.material.alpha_cutoff = material.alpha_cutoff;

    var emissive: vec4<f32> = material.emissive;
    pbr_input.material.emissive = emissive;

    var metallic: f32 = material.metallic;
    var perceptual_roughness: f32 = material.perceptual_roughness;
    pbr_input.material.metallic = metallic;
    pbr_input.material.perceptual_roughness = perceptual_roughness;

    var occlusion: f32 = 1.0;
    pbr_input.occlusion = occlusion;

    pbr_input.frag_coord = in.frag_coord;
    pbr_input.world_position = in.world_position;
    pbr_input.world_normal = in.world_normal;

    pbr_input.is_orthographic = view.projection[3].w == 1.0;

    pbr_input.N = prepare_normal(
        material.flags,
        in.world_normal,
#ifdef VERTEX_TANGENTS
#ifdef STANDARDMATERIAL_NORMAL_MAP
        in.world_tangent,
#endif
#endif
        in.is_front,
    );
    pbr_input.V = calculate_view(in.world_position, pbr_input.is_orthographic);
    output_color = pbr(pbr_input);
#ifdef TONEMAP_IN_SHADER
    output_color = tone_mapping(output_color);
#endif

    return output_color;
}

@fragment
fn fragment(
    in: FragmentInput,
) -> @location(0) vec4<f32> {
    let slope = 1.0 - in.world_normal.y;
    let blend_amount = slope * 10.;
    let scale = 1.;

    let grass = textureSample(grass_albedo, grass_albedo_sampler, in.world_position.xz * scale);
    let grass_pbr = get_pbr_color(grass, in, grass_pbr_material);

    //let rock = textureSample(rock_tex, rock_sampler, in.world_position.xz * scale);
    //let rock_pbr = get_pbr_color(rock, in, rock_pbr_material);

    let distance = length(view.world_position.xyz - in.world_position.xyz);
    let fog_contrib = exponential_fog(distance);

    var output_color = grass_pbr;
    output_color = vec4<f32>(mix(output_color.rgb, fog_contrib.rgb, fog_contrib.a), output_color.a);
    return output_color;
}
