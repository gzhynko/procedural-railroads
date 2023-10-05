#import bevy_pbr::pbr_functions as pbr_functions
#import bevy_pbr::pbr_types as pbr_types

#import bevy_pbr::mesh_vertex_output       MeshVertexOutput
#import bevy_pbr::mesh_bindings            mesh
#import bevy_pbr::mesh_view_bindings       view, fog
#import bevy_pbr::mesh_view_types          FOG_MODE_OFF
#import bevy_core_pipeline::tonemapping    screen_space_dither, powsafe, tone_mapping
#import bevy_pbr::parallax_mapping         parallaxed_uv

#ifdef SCREEN_SPACE_AMBIENT_OCCLUSION
#import bevy_pbr::gtao_utils gtao_multibounce
#endif

@group(1) @binding(1)
var<uniform> grass_pbr_material: pbr_types::StandardMaterial;
@group(1) @binding(2)
var<uniform> rock_pbr_material: pbr_types::StandardMaterial;

@group(1) @binding(3)
var grass_albedo: texture_2d<f32>;
@group(1) @binding(4)
var grass_albedo_sampler: sampler;

@group(1) @binding(5)
var rock_albedo: texture_2d<f32>;
@group(1) @binding(6)
var rock_albedo_sampler: sampler;

@fragment
fn fragment(
    in: MeshVertexOutput,
    @builtin(front_facing) is_front: bool,
) -> @location(0) vec4<f32> {
    let scale = 4.;
    let grass_base_color = textureSample(grass_albedo, grass_albedo_sampler, in.world_position.xz / scale);
    let grass_pbr_color = get_pbr_color(grass_base_color, in, is_front, grass_pbr_material);

    return grass_pbr_color;
}

// A copy of https://github.com/bevyengine/bevy/blob/main/crates/bevy_pbr/src/render/pbr.wgsl.
// TODO: Keep an eye on https://github.com/bevyengine/bevy/pull/7820 and remove this when that PR (or an alternative) gets merged
fn get_pbr_color(
    base_color: vec4<f32>,
    in: MeshVertexOutput,
    is_front: bool,
    material: pbr_types::StandardMaterial,
) -> vec4<f32> {
    var output_color: vec4<f32> = base_color;

    if ((material.flags & pbr_types::STANDARD_MATERIAL_FLAGS_UNLIT_BIT) == 0u) {
        var pbr_input: pbr_functions::PbrInput;

        pbr_input.material.base_color = base_color;
        pbr_input.material.reflectance = material.reflectance;
        pbr_input.material.flags = material.flags;
        pbr_input.material.alpha_cutoff = material.alpha_cutoff;

        // FIXME: this does not support emissive textures
        var emissive: vec4<f32> = material.emissive;
        pbr_input.material.emissive = emissive;

        // FIXME: this does not support metallic/roughness textures
        var metallic: f32 = material.metallic;
        var perceptual_roughness: f32 = material.perceptual_roughness;
        pbr_input.material.metallic = metallic;
        pbr_input.material.perceptual_roughness = perceptual_roughness;

        // FIXME: this does not support occlusion textures
        // FIXME: this does not support SSAO
        var occlusion: vec3<f32> = vec3(1.0);
        pbr_input.occlusion = occlusion;

        pbr_input.frag_coord = in.position;
        pbr_input.world_position = in.world_position;

        pbr_input.world_normal = pbr_functions::prepare_world_normal(
            in.world_normal,
            (material.flags & pbr_types::STANDARD_MATERIAL_FLAGS_DOUBLE_SIDED_BIT) != 0u,
            is_front,
        );

        pbr_input.is_orthographic = view.projection[3].w == 1.0;

#ifdef LOAD_PREPASS_NORMALS
        pbr_input.N = bevy_pbr::prepass_utils::prepass_normal(in.position, 0u);
#else
        pbr_input.N = pbr_functions::apply_normal_mapping(
            material.flags,
            pbr_input.world_normal,
#ifdef VERTEX_TANGENTS
#ifdef STANDARDMATERIAL_NORMAL_MAP
            in.world_tangent,
#endif
#endif
#ifdef VERTEX_UVS
            in.uv,
#endif
            view.mip_bias,
        );
#endif
        pbr_input.V = pbr_functions::calculate_view(in.world_position, pbr_input.is_orthographic);
        output_color = pbr_functions::pbr(pbr_input);
    } else {
        output_color = pbr_functions::alpha_discard(material, output_color);
    }

    // fog
    if (fog.mode != FOG_MODE_OFF && (material.flags & pbr_types::STANDARD_MATERIAL_FLAGS_FOG_ENABLED_BIT) != 0u) {
        output_color = pbr_functions::apply_fog(fog, output_color, in.world_position.xyz, view.world_position.xyz);
    }

#ifdef TONEMAP_IN_SHADER
    output_color = tone_mapping(output_color, view.color_grading);
#ifdef DEBAND_DITHER
    var output_rgb = output_color.rgb;
    output_rgb = powsafe(output_rgb, 1.0 / 2.2);
    output_rgb = output_rgb + screen_space_dither(in.position.xy);
    // This conversion back to linear space is required because our output texture format is
    // SRGB; the GPU will assume our output is linear and will apply an SRGB conversion.
    output_rgb = powsafe(output_rgb, 2.2);
    output_color = vec4(output_rgb, output_color.a);
#endif
#endif
#ifdef PREMULTIPLY_ALPHA
    output_color = pbr_functions::premultiply_alpha(material.flags, output_color);
#endif

    return output_color;
}
