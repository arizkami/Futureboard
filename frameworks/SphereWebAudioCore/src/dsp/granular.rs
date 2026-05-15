//! Simple overlap-add (OLA) granular time stretcher.
//!
//! stretch_ratio = output_duration / input_duration
//!   2.0 → output is twice as long (slower).
//!   0.5 → output is half as long (faster).
//!
//! Draft quality; suitable for preview and offline pre-processing.

use super::resample::resample_linear;
use std::f32::consts::PI;

pub fn time_stretch_granular(input: &[f32], stretch_ratio: f32, grain_size: usize) -> Vec<f32> {
    let ratio = stretch_ratio.clamp(0.25, 4.0);
    if input.is_empty() {
        return Vec::new();
    }
    if input.len() < grain_size {
        // Too short to grain-process: fall back to resampling.
        return resample_linear(input, 1.0 / ratio);
    }

    let hop_in = (grain_size / 4).max(1);
    let hop_out = ((hop_in as f32) * ratio).round() as usize;
    let hop_out = hop_out.max(1);
    let out_len = ((input.len() as f32 * ratio).ceil() as usize).max(1);

    let mut output = vec![0.0_f32; out_len];
    let mut window_sum = vec![0.0_f32; out_len];
    let win = hann_window(grain_size);

    let mut in_pos = 0_usize;
    let mut out_pos = 0_usize;

    while in_pos + grain_size <= input.len() && out_pos < out_len {
        let copy_len = grain_size.min(out_len - out_pos);
        for i in 0..copy_len {
            let w = win[i];
            output[out_pos + i] += input[in_pos + i] * w;
            window_sum[out_pos + i] += w;
        }
        in_pos += hop_in;
        out_pos += hop_out;
    }

    // Normalize by accumulated window weight.
    for i in 0..out_len {
        if window_sum[i] > 1e-6 {
            output[i] /= window_sum[i];
        }
    }

    output
}

fn hann_window(size: usize) -> Vec<f32> {
    let n1 = (size - 1) as f32;
    (0..size)
        .map(|i| 0.5 * (1.0 - (2.0 * PI * i as f32 / n1).cos()))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_input_safe() {
        assert!(time_stretch_granular(&[], 2.0, 2048).is_empty());
    }

    #[test]
    fn stretch_2_roughly_doubles_length() {
        let input: Vec<f32> = (0..4096).map(|i| (i as f32 * 0.01).sin()).collect();
        let out = time_stretch_granular(&input, 2.0, 512);
        let expected = (input.len() as f32 * 2.0) as usize;
        let tolerance = (expected as f32 * 0.1) as usize; // 10% tolerance
        assert!(
            out.len().abs_diff(expected) <= tolerance,
            "expected ~{expected}, got {}",
            out.len()
        );
    }

    #[test]
    fn no_nan_or_inf() {
        let input: Vec<f32> = (0..2048).map(|i| (i as f32).sin()).collect();
        for val in time_stretch_granular(&input, 1.5, 512) {
            assert!(val.is_finite(), "output contains non-finite value");
        }
    }
}
