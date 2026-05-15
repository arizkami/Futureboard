//! Draft-quality pitch shift that preserves approximate duration.
//!
//! Algorithm:
//!   1. Resample by pitch_ratio (changes pitch + duration together).
//!   2. Time-stretch back to the original duration (OLA granular).
//!   3. Trim or zero-pad to exactly the original length.
//!
//! Not artifact-free. Suitable for preview; replace with phase-vocoder later.
//! semitones clamped to [-24, 24].

use super::granular::time_stretch_granular;
use super::resample::resample_linear;

pub fn pitch_shift_draft(input: &[f32], semitones: f32) -> Vec<f32> {
    let semitones = semitones.clamp(-24.0, 24.0);
    if input.is_empty() {
        return Vec::new();
    }
    if semitones.abs() < 1e-6 {
        return input.to_vec();
    }

    let pitch_ratio = 2.0_f32.powf(semitones / 12.0);
    let original_len = input.len();

    // Step 1: resample to change pitch (also changes duration).
    let resampled = resample_linear(input, pitch_ratio);

    // Step 2: time-stretch back to original duration.
    // resampled.len() ≈ original_len / pitch_ratio
    // stretch_ratio needed = original_len / resampled_len ≈ pitch_ratio
    let stretched = time_stretch_granular(&resampled, pitch_ratio, 2048);

    // Step 3: trim or zero-pad to match exactly.
    let mut output = vec![0.0_f32; original_len];
    let copy_len = stretched.len().min(original_len);
    output[..copy_len].copy_from_slice(&stretched[..copy_len]);
    output
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_input_safe() {
        assert!(pitch_shift_draft(&[], 12.0).is_empty());
    }

    #[test]
    fn zero_semitones_is_identity() {
        let input: Vec<f32> = (0..256).map(|i| (i as f32).sin()).collect();
        let out = pitch_shift_draft(&input, 0.0);
        assert_eq!(out.len(), input.len());
        for (a, b) in input.iter().zip(out.iter()) {
            assert!((a - b).abs() < 1e-5, "zero semitones should be identity");
        }
    }

    #[test]
    fn preserves_length_positive() {
        let input: Vec<f32> = (0..4096).map(|i| (i as f32 * 0.01).sin()).collect();
        let out = pitch_shift_draft(&input, 12.0);
        assert_eq!(out.len(), input.len(), "+12 st must preserve length");
    }

    #[test]
    fn preserves_length_negative() {
        let input: Vec<f32> = (0..4096).map(|i| (i as f32 * 0.01).sin()).collect();
        let out = pitch_shift_draft(&input, -12.0);
        assert_eq!(out.len(), input.len(), "-12 st must preserve length");
    }

    #[test]
    fn no_nan_or_inf() {
        let input: Vec<f32> = (0..2048).map(|i| (i as f32).sin()).collect();
        for val in pitch_shift_draft(&input, 7.0) {
            assert!(val.is_finite(), "output contains non-finite value");
        }
    }
}
