/**
 * NativeSphereAudioEngineAdapter
 *
 * Implements AudioEngineAdapter by forwarding commands to the native
 * SphereDirectAudioEngine via window.dawElectron.sphereAudio (Electron preload).
 *
 * Safe to construct in a Web context — all methods check for the bridge and
 * fail gracefully when it's absent rather than throwing.
 *
 * UI code must never import this file directly. Use createAudioEngineAdapter()
 * or the active adapter singleton instead.
 */
import type {
  AudioEngineAdapter,
  AudioEngineStatus,
  MeterCallback,
  TransportCallback,
} from "../AudioEngineAdapter";
import type { DawProject, DawTrack, DawClip, InsertDevice, TrackId } from "../../types/daw";
import type {
  EngineProjectSnapshot,
  EngineTrackSnapshot,
  EngineClipSnapshot,
  MeterSnapshot,
  StereoMeterLevel,
} from "./types";
import { platform } from "../../platform";
// ── Bridge accessor ───────────────────────────────────────────────────────────

// eslint-disable-next-line @typescript-eslint/no-explicit-any
function getSphere(): any | null {
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  return (window as any).dawElectron?.sphereAudio ?? null;
}

// ── Meters polling ────────────────────────────────────────────────────────────
const METER_POLL_MS = 50; // ~20 fps

// ── Snapshot builders ─────────────────────────────────────────────────────────

function buildTrackSnapshot(track: DawTrack): EngineTrackSnapshot {
  return {
    id:            track.id,
    type:          track.type,
    volume:        track.volume,
    pan:           track.pan,
    muted:         track.muted ?? false,
    solo:          track.solo  ?? false,
    armed:         track.armed ?? false,
    outputTrackId: track.output ?? null,
    inserts: (track.inserts ?? []).map((ins) => ({
      id:      ins.id,
      type:    ins.type,
      enabled: ins.enabled ?? true,
      params:  ins.params  ?? {},
    })),
    sends: [],
  };
}

function resolveMediaPath(project: DawProject | null, clip: DawClip): string | null {
  if (!project) return null;
  const file = project.files.find((f) => f.id === clip.fileId);
  if (!file) return null;

  // Folder-based Electron project: absolutePath = projectRoot + relativePath.
  // `relativePath` is set by the folder-import flow (e.g. "Media/Audio/kick.wav").
  // The native Rust engine opens this via fs::read — it needs a real filesystem path.
  if (file.relativePath) {
    const root = platform.folderProject.getProjectRoot();
    if (root) {
      // Forward-slash join; Rust fs::read accepts both / and \ on Windows.
      return `${root}/${file.relativePath}`.replace(/\\/g, "/");
    }
  }

  // IndexedDB / OPFS / blob-URL sources: the Rust process cannot reach these.
  // Return null — the clip will be silently skipped in the native engine
  // (audio still plays through the WebAudio fallback path if needed).
  return null;
}

function buildClipSnapshot(project: DawProject | null, clip: DawClip, bpm: number): EngineClipSnapshot {
  // Convert timeline seconds → beats for the native engine.
  const bps = bpm / 60;
  return {
    id:            clip.id,
    trackId:       clip.trackId,
    assetId:       clip.fileId,
    mediaPath:     resolveMediaPath(project, clip),
    startBeat:     clip.startTime  * bps,
    durationBeats: clip.duration   * bps,
    offsetSeconds: clip.offset,
    gain:          clip.gain ?? 1,
    fades:         null,
    audioProcess:  clip.audioProcess
      ? {
          speedRatio:     clip.audioProcess.speedRatio,
          pitchSemitones: clip.audioProcess.pitchSemitones,
          preservePitch:  clip.audioProcess.preservePitch,
          mode:           clip.audioProcess.mode,
          quality:        clip.audioProcess.quality ?? "balanced",
        }
      : null,
  };
}

function buildProjectSnapshot(project: DawProject): EngineProjectSnapshot {
  const allClips = project.tracks.flatMap((t) => t.clips);
  return {
    projectId:     project.id,
    projectRoot:   platform.folderProject.getProjectRoot(),
    bpm:           project.bpm,
    timeSignature: [
      project.timeSignature?.numerator   ?? 4,
      project.timeSignature?.denominator ?? 4,
    ],
    sampleRate:    project.sampleRate ?? 44100,
    tracks:        project.tracks.map(buildTrackSnapshot),
    clips:         allClips.map((c) => buildClipSnapshot(project, c, project.bpm)),
    routing: {
      masterOutputDevice: null,
      sampleRate:         project.sampleRate ?? 44100,
      bufferSize:         256,
    },
  };
}

// ── Adapter ───────────────────────────────────────────────────────────────────

export class NativeSphereAudioEngineAdapter implements AudioEngineAdapter {
  private _status:              AudioEngineStatus   = "uninitialized";
  private _meterCallbacks       = new Set<MeterCallback>();
  private _transportCallbacks   = new Set<TransportCallback>();
  private _meterPollId:         ReturnType<typeof setInterval> | null = null;
  private _transportPollId:     ReturnType<typeof setInterval> | null = null;
  private _lastTransport        = { playing: false, positionSeconds: 0 };
  // Debounce timer for syncProject — rapid edits batch into one Rust rebuild.
  private _syncTimer:           ReturnType<typeof setTimeout> | null = null;

  // ── Lifecycle ──────────────────────────────────────────────────────────────

  async init(): Promise<void> {
    const sphere = getSphere();
    if (!sphere) {
      console.warn("[NativeSphere] Preload bridge absent — adapter inactive");
      this._status = "error";
      return;
    }
    try {
      // Check if the engine is already running (auto-started by ipc-handlers).
      // If not, open the default device and start the stream ourselves.
      const status = await sphere.getStatus() as { running: boolean; streamOpen: boolean };
      if (!status.running) {
        if (!status.streamOpen) {
          // Open default output device/config.  User-selected device/buffer is
          // applied from Preferences via the same native bridge.
          await sphere.openDevice({});
        }
        await sphere.start();
      }
      this._status = "running";
      this._startPolling();
      console.log("[NativeSphere] Native audio engine ready");
    } catch (e) {
      console.error("[NativeSphere] Failed to start native engine:", e);
      this._status = "error";
      throw e;
    }
  }

  dispose(): void {
    this._stopPolling();
    this._meterCallbacks.clear();
    this._transportCallbacks.clear();
    const sphere = getSphere();
    if (sphere) {
      sphere.stop().catch((e: unknown) =>
        console.warn("[NativeSphere] stop() error during dispose:", e),
      );
    }
    this._status = "closed";
  }

  getStatus(): AudioEngineStatus {
    return this._status;
  }

  // ── Project sync ───────────────────────────────────────────────────────────

  async loadProject(project: DawProject): Promise<void> {
    const sphere = getSphere();
    if (!sphere) return;
    const snapshot = buildProjectSnapshot(project);
    await sphere.loadProject(snapshot);
  }

  syncProject(project: DawProject): void {
    // Debounce: batch rapid edits (clip drags, fader moves) into one Rust rebuild.
    // 120 ms keeps the engine in sync without decoding audio files on every frame.
    if (this._syncTimer !== null) clearTimeout(this._syncTimer);
    this._syncTimer = setTimeout(() => {
      this._syncTimer = null;
      const sphere = getSphere();
      if (!sphere) return;
      const snapshot = buildProjectSnapshot(project);
      sphere.loadProject(snapshot).catch((e: unknown) =>
        console.warn("[NativeSphere] syncProject error:", e),
      );
    }, 120);
  }

  // ── Transport ──────────────────────────────────────────────────────────────

  async play(positionSeconds?: number): Promise<void> {
    const sphere = getSphere();
    if (!sphere) return;
    await sphere.setTransportState({ playing: true, positionSeconds });
  }

  pause(): void {
    const sphere = getSphere();
    if (!sphere) return;
    sphere
      .setTransportState({ playing: false })
      .catch((e: unknown) => console.warn("[NativeSphere] pause error:", e));
  }

  stop(): void {
    const sphere = getSphere();
    if (!sphere) return;
    sphere
      .setTransportState({ playing: false, positionSeconds: 0 })
      .catch((e: unknown) => console.warn("[NativeSphere] stop error:", e));
    this._notifyTransport({ playing: false, positionSeconds: 0 });
  }

  seekSeconds(seconds: number): void {
    const sphere = getSphere();
    if (!sphere) return;
    sphere
      .setTransportState({ positionSeconds: seconds })
      .catch((e: unknown) => console.warn("[NativeSphere] seek error:", e));
  }

  setBpm(bpm: number): void {
    const sphere = getSphere();
    if (!sphere) return;
    sphere
      .updateTrackParam("__transport__", "bpm", bpm)
      .catch((e: unknown) => console.warn("[NativeSphere] setBpm error:", e));
  }

  setLoop(enabled: boolean, startSeconds: number, endSeconds: number): void {
    const sphere = getSphere();
    if (!sphere) return;
    sphere
      .setTransportState({ loop: enabled, loopStart: startSeconds, loopEnd: endSeconds })
      .catch((e: unknown) => console.warn("[NativeSphere] setLoop error:", e));
  }

  // ── Track management ───────────────────────────────────────────────────────

  createTrack(track: DawTrack): void {
    const sphere = getSphere();
    if (!sphere) return;
    sphere
      .updateTrackParam(track.id, "__create__", buildTrackSnapshot(track))
      .catch((e: unknown) => console.warn("[NativeSphere] createTrack error:", e));
  }

  removeTrack(trackId: TrackId): void {
    const sphere = getSphere();
    if (!sphere) return;
    sphere
      .updateTrackParam(trackId, "__remove__", true)
      .catch((e: unknown) => console.warn("[NativeSphere] removeTrack error:", e));
  }

  // ── Clip management ────────────────────────────────────────────────────────

  scheduleClip(trackId: TrackId, clip: DawClip): void {
    const sphere = getSphere();
    if (!sphere) return;
    // BPM is unknown at call site; use 120 as a safe default.
    // The full project snapshot (loadProject) keeps the engine in sync accurately.
    const snapshot = buildClipSnapshot(null, clip, 120);
    sphere
      .updateClip(clip.id, { ...snapshot, trackId })
      .catch((e: unknown) => console.warn("[NativeSphere] scheduleClip error:", e));
  }

  unscheduleClip(clipId: string): void {
    const sphere = getSphere();
    if (!sphere) return;
    sphere
      .updateClip(clipId, { __remove__: true })
      .catch((e: unknown) => console.warn("[NativeSphere] unscheduleClip error:", e));
  }

  // ── Audio files ────────────────────────────────────────────────────────────
  // Native engine loads files from disk paths — no buffer transfer over IPC.

  loadAudioFile(_fileId: string, _buffer: AudioBuffer): void {
    // No-op: native engine resolves media paths from the project snapshot.
  }

  unloadAudioFile(_fileId: string): void {
    // No-op.
  }

  // ── Mixer ──────────────────────────────────────────────────────────────────

  setTrackVolume(trackId: TrackId, volume: number): void {
    this._paramUpdate(trackId, "volume", volume);
  }

  setTrackPan(trackId: TrackId, pan: number): void {
    this._paramUpdate(trackId, "pan", pan);
  }

  setTrackMute(trackId: TrackId, muted: boolean): void {
    this._paramUpdate(trackId, "muted", muted);
  }

  setTrackSolo(trackId: TrackId, solo: boolean): void {
    this._paramUpdate(trackId, "solo", solo);
  }

  setTrackPhaseInvert(trackId: TrackId, inverted: boolean): void {
    this._paramUpdate(trackId, "phaseInvert", inverted);
  }

  setTrackOutput(trackId: TrackId, output: string): void {
    this._paramUpdate(trackId, "outputTrackId", output);
  }

  setMasterVolume(volume: number): void {
    this._paramUpdate("__master__", "volume", volume);
  }

  // ── Insert devices ─────────────────────────────────────────────────────────

  addInsertDevice(trackId: TrackId, device: InsertDevice): void {
    const sphere = getSphere();
    if (!sphere) return;
    sphere
      .updateInsertParam(trackId, device.id, "__create__", device)
      .catch((e: unknown) => console.warn("[NativeSphere] addInsertDevice error:", e));
  }

  removeInsertDevice(trackId: TrackId, deviceId: string): void {
    const sphere = getSphere();
    if (!sphere) return;
    sphere
      .updateInsertParam(trackId, deviceId, "__remove__", true)
      .catch((e: unknown) => console.warn("[NativeSphere] removeInsertDevice error:", e));
  }

  setInsertEnabled(trackId: TrackId, deviceId: string, enabled: boolean): void {
    const sphere = getSphere();
    if (!sphere) return;
    sphere
      .updateInsertParam(trackId, deviceId, "enabled", enabled)
      .catch((e: unknown) => console.warn("[NativeSphere] setInsertEnabled error:", e));
  }

  setInsertParam(
    trackId: TrackId,
    deviceId: string,
    param: string,
    value: number | string | boolean,
  ): void {
    const sphere = getSphere();
    if (!sphere) return;
    sphere
      .updateInsertParam(trackId, deviceId, param, value)
      .catch((e: unknown) => console.warn("[NativeSphere] setInsertParam error:", e));
  }

  // ── Metering ───────────────────────────────────────────────────────────────

  subscribeMeters(callback: MeterCallback): () => void {
    this._meterCallbacks.add(callback);
    return () => this._meterCallbacks.delete(callback);
  }

  subscribeTransport(callback: TransportCallback): () => void {
    this._transportCallbacks.add(callback);
    return () => this._transportCallbacks.delete(callback);
  }

  // ── Internal: realtime param update ───────────────────────────────────────

  private _paramUpdate(trackId: string, paramId: string, value: number | string | boolean): void {
    const sphere = getSphere();
    if (!sphere) return;
    sphere
      .updateTrackParam(trackId, paramId, value)
      .catch((e: unknown) =>
        console.warn(`[NativeSphere] updateTrackParam(${trackId}, ${paramId}) error:`, e),
      );
  }

  // ── Internal: polling loops ────────────────────────────────────────────────

  private _startPolling(): void {
    this._stopPolling();

    // Meter poll — ~20 fps
    this._meterPollId = setInterval(() => {
      if (this._meterCallbacks.size === 0) return;
      const sphere = getSphere();
      if (!sphere) return;
      sphere
        .getMeters()
        .then((snap: MeterSnapshot) => {
          for (const [trackId, level] of Object.entries(snap.tracks)) {
            const sl: { left: number; right: number } = level as StereoMeterLevel;
            for (const cb of this._meterCallbacks) {
              cb(trackId, { l: sl.left, r: sl.right });
            }
          }
          for (const cb of this._meterCallbacks) {
            cb("master", { l: snap.master.left, r: snap.master.right });
          }
        })
        .catch(() => {/* native engine may be busy — ignore */});
    }, METER_POLL_MS);

    // Transport poll — ~20 fps
    this._transportPollId = setInterval(() => {
      if (this._transportCallbacks.size === 0) return;
      const sphere = getSphere();
      if (!sphere) return;
      sphere
        .getTransportState()
        .then((state: { playing: boolean; positionSeconds: number }) => {
          if (
            state.playing         !== this._lastTransport.playing ||
            Math.abs(state.positionSeconds - this._lastTransport.positionSeconds) > 0.01
          ) {
            this._lastTransport = state;
            this._notifyTransport(state);
          }
        })
        .catch(() => {});
    }, METER_POLL_MS);
  }

  private _stopPolling(): void {
    if (this._meterPollId     !== null) clearInterval(this._meterPollId);
    if (this._transportPollId !== null) clearInterval(this._transportPollId);
    this._meterPollId     = null;
    this._transportPollId = null;
  }

  private _notifyTransport(state: { playing: boolean; positionSeconds: number }): void {
    for (const cb of this._transportCallbacks) {
      cb(state);
    }
  }
}
