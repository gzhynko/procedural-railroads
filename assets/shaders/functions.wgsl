/// A copy of https://github.com/bevyengine/bevy/blob/main/crates/bevy_pbr/src/render/pbr_functions.wgsl ::prepare_normal().
/// Removed all ifdefs and made the function accept normal map texture and sampler
/// This is done because in terrain_texturing.wgsl there are multiple textures and so multiple normal maps.
fn prepare_normal_with_normal_map(
    normal_map_texture: texture_2d<f32>,
    normal_map_sampler: sampler,
    standard_material_flags: u32,
    world_normal: vec3<f32>,
    world_tangent: vec4<f32>,
    uv: vec2<f32>,
    is_front: bool,
) -> vec3<f32> {
    var N: vec3<f32> = normalize(world_normal);

    // NOTE: The mikktspace method of normal mapping explicitly requires that these NOT be
    // normalized nor any Gram-Schmidt applied to ensure the vertex normal is orthogonal to the
    // vertex tangent! Do not change this code unless you really know what you are doing.
    // http://www.mikktspace.com/
    var T: vec3<f32> = world_tangent.xyz;
    var B: vec3<f32> = world_tangent.w * cross(N, T);

    if ((standard_material_flags & STANDARD_MATERIAL_FLAGS_DOUBLE_SIDED_BIT) != 0u) {
        if (!is_front) {
            N = -N;
            T = -T;
            B = -B;
        }
    }

    // Nt is the tangent-space normal.
    var Nt = textureSample(normal_map_texture, normal_map_sampler, uv).rgb;
    if ((standard_material_flags & STANDARD_MATERIAL_FLAGS_TWO_COMPONENT_NORMAL_MAP) != 0u) {
        // Only use the xy components and derive z for 2-component normal maps.
        Nt = vec3<f32>(Nt.rg * 2.0 - 1.0, 0.0);
        Nt.z = sqrt(1.0 - Nt.x * Nt.x - Nt.y * Nt.y);
    } else {
        Nt = Nt * 2.0 - 1.0;
    }
    // Normal maps authored for DirectX require flipping the y component
    if ((standard_material_flags & STANDARD_MATERIAL_FLAGS_FLIP_NORMAL_MAP_Y) != 0u) {
        Nt.y = -Nt.y;
    }
    // NOTE: The mikktspace method of normal mapping applies maps the tangent-space normal from
    // the normal map texture in this way to be an EXACT inverse of how the normal map baker
    // calculates the normal maps so there is no error introduced. Do not change this code
    // unless you really know what you are doing.
    // http://www.mikktspace.com/
    N = normalize(Nt.x * T + Nt.y * B + Nt.z * N);

    return N;
}