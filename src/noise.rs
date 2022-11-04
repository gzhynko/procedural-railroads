use noisy_bevy::simplex_noise_2d_seeded;
use crate::{NoiseSettings, Vec2};

pub(crate) fn get_heightmap_function(chunk_size: f32, noise_settings: NoiseSettings) -> impl Fn(f64, f64) -> f64 {
    let heightmap_fn = move |x: f64, y: f64| -> f64 {
        noise_settings.amplitude * simplex_noise_2d_seeded(Vec2::new((x as f32 - chunk_size / 2.) / noise_settings.scale.0 as f32, (y as f32 - chunk_size / 2.)  / noise_settings.scale.1 as f32), noise_settings.seed as f32) as f64
            + 5. * simplex_noise_2d_seeded(Vec2::new((x as f32 - chunk_size / 2.) / noise_settings.scale.0 as f32, (y as f32 - chunk_size / 2.)  / noise_settings.scale.0 as f32), noise_settings.seed as f32 + 1.) as f64
    };

    heightmap_fn
}