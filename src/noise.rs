use bevy::math::vec2;
use bevy::prelude::*;
use noisy_bevy::simplex_noise_2d_seeded;

const SEED: u32 = 1354251456;

#[derive(Copy, Clone, Resource)]
pub struct NoiseSettings {
    pub amplitude: f32,
    pub frequency: f32,
    pub num_octaves: u32,
    pub scale: f32,
    pub seed: u32,
}

impl Default for NoiseSettings {
    fn default() -> Self {
        Self {
            amplitude: 0.8,
            frequency: 0.001,
            num_octaves: 5,
            scale: 4.0,
            seed: SEED
        }
    }
}

pub(crate) fn get_heightmap_function(chunk_size: f32, noise_settings: NoiseSettings, offset: Vec3) -> impl Fn(f32, f32) -> f32 {
    let heightmap_fn = move |x: f32, y: f32| -> f32 {
        let base_pos_x = x - chunk_size / 2. + offset.x;
        let base_pos_y = y - chunk_size / 2. + offset.z;

        let mut freq = noise_settings.frequency;
        let mut amplitude = noise_settings.amplitude;
        let mut result = 0.0;
        let mut scalar = 1.0;
        for _ in 0..noise_settings.num_octaves {
            let noise_val = simplex_noise_2d_seeded(vec2(base_pos_x / noise_settings.scale, base_pos_y / noise_settings.scale) * freq, noise_settings.seed as f32);
            result += noise_val * scalar * amplitude;
            scalar *= noise_val * 0.5 + 1.0;
            freq *= 2.0;
            amplitude *= 0.45;
        }

        result * 100.0
    };

    heightmap_fn
}
