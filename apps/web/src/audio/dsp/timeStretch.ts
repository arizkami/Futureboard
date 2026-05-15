import { resampleLinear } from "./resample";

type F32 = Float32Array<ArrayBufferLike>;

/**
 * Simple overlap-add (OLA) granular time stretcher.
 *
 * stretchRatio = outputDuration / inputDuration
 *   2.0 → output is twice as long (slower playback)
 *   0.5 → output is half as long (faster playback)
 *
 * Does NOT preserve pitch by itself — combine with pitchShift for that.
 * Quality is draft-grade; suitable for preview and prototyping.
 */
export function timeStretchGranular(
  channels: F32[],
  stretchRatio: number,
  grainSize = 2048,
): Float32Array[] {
  const ratio = Math.max(0.25, Math.min(4.0, stretchRatio));
  if (channels.length === 0) return [];
  return channels.map((ch) => stretchChannel(ch, ratio, grainSize));
}

function stretchChannel(input: F32, stretchRatio: number, grainSize: number): Float32Array {
  const inLen = input.length;
  if (inLen === 0) return new Float32Array(0);

  // For very short inputs fall back to simple resampling
  if (inLen < grainSize) {
    return resampleLinear(input, 1 / stretchRatio);
  }

  const hopIn = Math.max(1, (grainSize >> 2));               // grainSize / 4
  const hopOut = Math.max(1, Math.round(hopIn * stretchRatio));
  const outLen = Math.max(1, Math.ceil(inLen * stretchRatio));
  const output = new Float32Array(outLen);
  const windowSum = new Float32Array(outLen);
  const win = hannWindow(grainSize);

  let inPos = 0;
  let outPos = 0;

  while (inPos + grainSize <= inLen && outPos < outLen) {
    const remaining = outLen - outPos;
    const copyLen = Math.min(grainSize, remaining);
    for (let i = 0; i < copyLen; i++) {
      const w = win[i];
      output[outPos + i] += input[inPos + i] * w;
      windowSum[outPos + i] += w;
    }
    inPos += hopIn;
    outPos += hopOut;
  }

  // Normalize by accumulated window weight to avoid amplitude dips at boundaries
  for (let i = 0; i < outLen; i++) {
    if (windowSum[i] > 1e-6) output[i] /= windowSum[i];
  }

  return output;
}

function hannWindow(size: number): Float32Array {
  const win = new Float32Array(size);
  const n1 = size - 1;
  for (let i = 0; i < size; i++) {
    win[i] = 0.5 * (1 - Math.cos((2 * Math.PI * i) / n1));
  }
  return win;
}
