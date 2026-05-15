import type { DecodedAudioData, AudioProcessParams, F32 } from "./audioCacheTypes";
import { audioCacheManager } from "./AudioCacheManager";
import { buildProcessedCacheKey, isIdentityTransform } from "./audioCacheKeys";
import { resampleChannels } from "./dsp/resample";
import { timeStretchGranular } from "./dsp/timeStretch";
import { pitchShiftDraft } from "./dsp/pitchShift";
import {
  ensureRustDsp,
  isRustDspReady,
  rustSpeedChannels,
  rustPitchChannels,
  rustTimeStretchChannels,
} from "./RustDspProcessor";

export type ProcessorKind = "rust-wasm" | "typescript";

// ── AudioProcessingService ────────────────────────────────────────────────────

class AudioProcessingService {
  constructor() {
    // Kick off WASM load in the background so it's ready by the time the user
    // actually triggers processing (saves ~50-100 ms on first call).
    ensureRustDsp().catch(() => {});
  }

  chooseBestProcessor(): ProcessorKind {
    return isRustDspReady() ? "rust-wasm" : "typescript";
  }

  getProcessingCapabilities() {
    return { typescript: true, rustWasm: isRustDspReady() };
  }

  /**
   * Process decoded audio with speed/pitch params.
   * Checks the cache first; processes and caches on miss.
   * Returns decoded source unchanged for identity transforms.
   */
  async processClipAudio(
    decoded: DecodedAudioData,
    params: AudioProcessParams,
  ): Promise<DecodedAudioData> {
    if (isIdentityTransform(params)) return decoded;

    const key = buildProcessedCacheKey(decoded.fileId, decoded.sampleRate, params);
    const cached = audioCacheManager.getProcessedAudio(key);
    if (cached) {
      console.debug("[AudioProcessing] cache hit:", key);
      return cached;
    }

    // Ensure WASM is loaded before deciding which path to use
    await ensureRustDsp();

    const processor = this.chooseBestProcessor();
    console.debug(`[AudioProcessing] processing with ${processor}:`, params);

    let result: DecodedAudioData;
    if (processor === "rust-wasm") {
      result = await this._processRust(decoded, params);
    } else {
      result = await this._processTypeScript(decoded, params);
    }

    audioCacheManager.setProcessedAudio(key, result);
    console.debug(`[AudioProcessing] cached result (${processor}) key:`, key);
    return result;
  }

  /** Return cached processed audio or null without triggering processing. */
  getCachedProcessed(
    decoded: DecodedAudioData,
    params: AudioProcessParams,
  ): DecodedAudioData | null {
    if (isIdentityTransform(params)) return decoded;
    const key = buildProcessedCacheKey(decoded.fileId, decoded.sampleRate, params);
    return audioCacheManager.getProcessedAudio(key) ?? null;
  }

  /** Remove all processed variants for a file so next request reprocesses. */
  invalidateProcessedAudio(fileId: string): void {
    audioCacheManager.clearFileCache(fileId);
  }

  // ── Rust WASM DSP path ────────────────────────────────────────────────────

  private async _processRust(
    decoded: DecodedAudioData,
    params: AudioProcessParams,
  ): Promise<DecodedAudioData> {
    const { speedRatio, pitchSemitones, preservePitch } = params;
    let channels: F32[] = decoded.channelData.map((ch) => new Float32Array(ch));

    if (!preservePitch) {
      if (speedRatio !== 1) {
        const result = rustSpeedChannels(channels, speedRatio);
        if (result) {
          channels = result;
        } else {
          channels = resampleChannels(channels, speedRatio);
        }
      }
      if (pitchSemitones !== 0) {
        const pitchRatio = Math.pow(2, pitchSemitones / 12);
        const result = rustSpeedChannels(channels, pitchRatio);
        if (result) {
          channels = result;
        } else {
          channels = resampleChannels(channels, pitchRatio);
        }
      }
    } else {
      if (speedRatio !== 1) {
        const stretchRatio = 1 / speedRatio;
        const result = rustTimeStretchChannels(channels, stretchRatio);
        if (result) {
          channels = result;
        } else {
          channels = timeStretchGranular(channels, stretchRatio);
        }
      }
      if (pitchSemitones !== 0) {
        const result = rustPitchChannels(channels, pitchSemitones);
        if (result) {
          channels = result;
        } else {
          channels = pitchShiftDraft(channels, pitchSemitones);
        }
      }
    }

    await new Promise<void>((r) => setTimeout(r, 0));

    const outLen = channels[0]?.length ?? 0;
    return {
      fileId: decoded.fileId,
      sampleRate: decoded.sampleRate,
      channels: decoded.channels,
      length: outLen,
      duration: outLen / decoded.sampleRate,
      channelData: channels,
    };
  }

  // ── TypeScript DSP path ───────────────────────────────────────────────────

  private async _processTypeScript(
    decoded: DecodedAudioData,
    params: AudioProcessParams,
  ): Promise<DecodedAudioData> {
    const { speedRatio, pitchSemitones, preservePitch } = params;
    let channels: F32[] = decoded.channelData.map((ch) => new Float32Array(ch));

    if (!preservePitch) {
      if (speedRatio !== 1) {
        channels = resampleChannels(channels, speedRatio);
      }
      if (pitchSemitones !== 0) {
        const pitchRatio = Math.pow(2, pitchSemitones / 12);
        channels = resampleChannels(channels, pitchRatio);
      }
    } else {
      if (speedRatio !== 1) {
        const stretchRatio = 1 / speedRatio;
        channels = timeStretchGranular(channels, stretchRatio);
      }
      if (pitchSemitones !== 0) {
        channels = pitchShiftDraft(channels, pitchSemitones);
      }
    }

    await new Promise<void>((r) => setTimeout(r, 0));

    const outLen = channels[0]?.length ?? 0;
    return {
      fileId: decoded.fileId,
      sampleRate: decoded.sampleRate,
      channels: decoded.channels,
      length: outLen,
      duration: outLen / decoded.sampleRate,
      channelData: channels,
    };
  }
}

export const audioProcessingService = new AudioProcessingService();
