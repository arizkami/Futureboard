import { resampleLinear } from "./resample";
import { timeStretchGranular } from "./timeStretch";

type F32 = Float32Array<ArrayBufferLike>;

/**
 * Draft-quality pitch shift that preserves approximate duration.
 *
 * Algorithm:
 *   1. Resample by pitchRatio (changes pitch + duration).
 *   2. Time-stretch back to original duration (OLA granular).
 *   3. Trim/pad to exactly the original length.
 *
 * Not artifact-free. Suitable for preview; replace with phase-vocoder later.
 *
 * semitones: -24 to +24
 */
export function pitchShiftDraft(
  channels: F32[],
  semitones: number,
): Float32Array[] {
  const clamped = Math.max(-24, Math.min(24, semitones));
  if (clamped === 0 || channels.length === 0) return channels.map((ch) => new Float32Array(ch));

  const pitchRatio = Math.pow(2, clamped / 12);
  const originalLength = channels[0].length;

  // Step 1: resample to change pitch (also changes duration)
  // pitchRatio > 1 (pitch up) → fewer samples → higher pitch, shorter buffer
  // pitchRatio < 1 (pitch down) → more samples → lower pitch, longer buffer
  const resampled = channels.map((ch) => resampleLinear(ch, pitchRatio));

  // Step 2: time-stretch back to the original duration
  // resampled length ≈ originalLength / pitchRatio
  // stretch ratio = originalLength / resampledLength ≈ pitchRatio
  const stretched = timeStretchGranular(resampled, pitchRatio);

  // Step 3: ensure output length matches original exactly
  return stretched.map((ch) => {
    if (ch.length === originalLength) return ch;
    const out = new Float32Array(originalLength);
    out.set(ch.subarray(0, Math.min(ch.length, originalLength)));
    return out;
  });
}
