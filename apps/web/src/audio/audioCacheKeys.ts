import type { AudioProcessParams } from "./audioCacheTypes";

const CACHE_VERSION = 1;

export function buildDecodedCacheKey(fileId: string, sampleRate: number): string {
  return `dec:v${CACHE_VERSION}:${fileId}:${sampleRate}`;
}

export function buildProcessedCacheKey(
  fileId: string,
  sampleRate: number,
  params: AudioProcessParams,
): string {
  const sp = params.speedRatio.toFixed(4);
  const pt = params.pitchSemitones.toFixed(2);
  const pp = params.preservePitch ? "1" : "0";
  return `proc:v${CACHE_VERSION}:${fileId}:${sampleRate}:sp${sp}:pt${pt}:pp${pp}:q${params.quality}`;
}

/** True when params match the identity transform (no processing needed). */
export function isIdentityTransform(params: AudioProcessParams): boolean {
  return (
    params.speedRatio === 1 &&
    params.pitchSemitones === 0
  );
}
