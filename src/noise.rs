use noisy_bevy::simplex_noise_2d_seeded;
use crate::{NoiseSettings, Vec2, Vec3};

pub(crate) fn get_heightmap_function(chunk_size: f32, noise_settings: NoiseSettings, offset: Vec3) -> impl Fn(f64, f64) -> f64 {
    let heightmap_fn = move |x: f64, y: f64| -> f64 {
        let base_pos_x = x as f32 - chunk_size / 2. + offset.x;
        let base_pos_y = y as f32 - chunk_size / 2. + offset.z;
        noise_settings.amplitude * simplex_noise_2d_seeded(Vec2::new(base_pos_x / noise_settings.scale.0 as f32, base_pos_y / noise_settings.scale.0 as f32), noise_settings.seed as f32) as f64
            + noise_settings.amplitude / 2. * simplex_noise_2d_seeded(Vec2::new((base_pos_x + 100.) / noise_settings.scale.0 as f32 / 2., (base_pos_y + 100.) / noise_settings.scale.0 as f32 / 2.), noise_settings.seed as f32) as f64
            + noise_settings.amplitude / 4. * simplex_noise_2d_seeded(Vec2::new((base_pos_x + 200.) / noise_settings.scale.0 as f32 / 4., (base_pos_y + 200.) / noise_settings.scale.0 as f32 / 4.), noise_settings.seed as f32) as f64
            + noise_settings.amplitude / 8. * simplex_noise_2d_seeded(Vec2::new((base_pos_x + 400.) / noise_settings.scale.0 as f32 / 8., (base_pos_y + 400.) / noise_settings.scale.0 as f32 / 8.), noise_settings.seed as f32) as f64
        + offset.y as f64
    };

    heightmap_fn
}