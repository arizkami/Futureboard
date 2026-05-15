/**
 * Thin wrapper around the WASM DSP exports (process_speed_mono, process_pitch_mono,
 * process_time_stretch_mono). Lazily initialized — first call triggers the dynamic import.
 *
 * Operates on individual mono channels. Call per-channel and reassemble.
 */

import type { F32 } from "./audioCacheTypes";

type WasmDspModule = {
  process_speed_mono(input: Float32Array, speedRatio: number): Float32Array;
  process_pitch_mono(input: Float32Array, semitones: number): Float32Array;
  process_time_stretch_mono(input: Float32Array, stretchRatio: number): Float32Array;
};

let _module: WasmDspModule | null = null;
let _initPromise: Promise<WasmDspModule | null> | null = null;

async function loadModule(): Promise<WasmDspModule | null> {
  try {
    const mod = await import("../engine/wasm-pkg/futureboard_core.js");
    await mod.default(); // init() — safe to call multiple times
    _module = mod as unknown as WasmDspModule;
    console.debug("[RustDsp] WASM DSP module ready");
    return _module;
  } catch (e) {
    console.warn("[RustDsp] Failed to load WASM DSP module, will use TypeScript fallback:", e);
    return null;
  }
}

export function ensureRustDsp(): Promise<WasmDspModule | null> {
  if (_module) return Promise.resolve(_module);
  if (!_initPromise) _initPromise = loadModule();
  return _initPromise;
}

export function isRustDspReady(): boolean {
  return _module !== null;
}

/** Apply speed resampling to all channels via WASM. Returns null if WASM unavailable. */
export function rustSpeedChannels(channels: F32[], speedRatio: number): Float32Array[] | null {
  if (!_module) return null;
  try {
    return channels.map((ch) => _module!.process_speed_mono(new Float32Array(ch), speedRatio));
  } catch (e) {
    console.warn("[RustDsp] process_speed_mono error:", e);
    return null;
  }
}

/** Apply pitch shift to all channels via WASM. Returns null if WASM unavailable. */
export function rustPitchChannels(channels: F32[], semitones: number): Float32Array[] | null {
  if (!_module) return null;
  try {
    return channels.map((ch) => _module!.process_pitch_mono(new Float32Array(ch), semitones));
  } catch (e) {
    console.warn("[RustDsp] process_pitch_mono error:", e);
    return null;
  }
}

/** Apply time-stretch to all channels via WASM. Returns null if WASM unavailable. */
export function rustTimeStretchChannels(
  channels: F32[],
  stretchRatio: number,
): Float32Array[] | null {
  if (!_module) return null;
  try {
    return channels.map((ch) =>
      _module!.process_time_stretch_mono(new Float32Array(ch), stretchRatio),
    );
  } catch (e) {
    console.warn("[RustDsp] process_time_stretch_mono error:", e);
    return null;
  }
}
