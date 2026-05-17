import type {
  AudioEngineAdapter,
  MeterCallback,
  TransportCallback,
} from "./AudioEngineAdapter";
import type { DawProject, DawTrack, TrackId } from "../types/daw";
import { createAudioEngineAdapter, type AdapterSelection } from "./native/createAudioEngineAdapter";
import type { PreferredEngine } from "../store/settingsStore";
import { useSettingsStore } from "../store/settingsStore";
import { useProjectStore } from "../store/projectStore";
import { useTransportStore } from "../store/transportStore";
import { platform } from "../platform";

type ActiveBackend = AdapterSelection["backend"] | "uninitialized";

class ActiveAudioEngine {
  private _adapter: AudioEngineAdapter | null = null;
  private _backend: ActiveBackend = "uninitialized";
  private _initPromise: Promise<void> | null = null;
  private _transportUnsub: (() => void) | null = null;
  private _pendingProject: DawProject | null = null;
  private _transport = { playing: false, positionSeconds: 0 };

  async init(preferredEngine = useSettingsStore.getState().preferredEngine): Promise<void> {
    if (this._adapter) return;
    if (this._initPromise) return this._initPromise;

    this._initPromise = this._init(preferredEngine);
    return this._initPromise;
  }

  async reconfigure(preferredEngine = useSettingsStore.getState().preferredEngine): Promise<void> {
    this.dispose();
    await this.init(preferredEngine);
  }

  dispose(): void {
    this._transportUnsub?.();
    this._transportUnsub = null;
    this._adapter?.dispose();
    this._adapter = null;
    this._backend = "uninitialized";
    this._initPromise = null;
  }

  get backend(): ActiveBackend {
    return this._backend;
  }

  get isNative(): boolean {
    return this._backend === "native-sphere-direct";
  }

  get isPlaying(): boolean {
    return this._transport.playing;
  }

  get projectTime(): number {
    return this._transport.positionSeconds;
  }

  async play(positionSeconds?: number): Promise<void> {
    const adapter = await this._ensureAdapter();
    if (positionSeconds !== undefined) {
      this._transport.positionSeconds = Math.max(0, positionSeconds);
    }
    await adapter.play(positionSeconds);
    this._setTransport({ playing: true, positionSeconds: this._transport.positionSeconds });
  }

  pause(): void {
    this._adapter?.pause();
    this._setTransport({ ...this._transport, playing: false });
  }

  stop(): void {
    this._adapter?.stop();
    this._setTransport({ playing: false, positionSeconds: 0 });
  }

  seekSeconds(seconds: number): void {
    const positionSeconds = Math.max(0, seconds);
    this._adapter?.seekSeconds(positionSeconds);
    this._setTransport({ ...this._transport, positionSeconds });
  }

  setBpm(bpm: number): void {
    this._adapter?.setBpm(bpm);
  }

  setLoop(enabled: boolean, startSeconds: number, endSeconds: number): void {
    this._adapter?.setLoop(enabled, startSeconds, endSeconds);
  }

  loadProject(project: DawProject): Promise<void> {
    this._pendingProject = project;
    if (!this._adapter) return Promise.resolve();
    return this._adapter.loadProject(project);
  }

  syncProject(project: DawProject): void {
    this._pendingProject = project;
    this._adapter?.syncProject(project);
  }

  createTrack(track: DawTrack): void {
    this._adapter?.createTrack(track);
  }

  removeTrack(trackId: TrackId): void {
    this._adapter?.removeTrack(trackId);
  }

  setTrackVolume(trackId: TrackId, volume: number): void {
    this._adapter?.setTrackVolume(trackId, volume);
  }

  setTrackPan(trackId: TrackId, pan: number): void {
    this._adapter?.setTrackPan(trackId, pan);
  }

  setTrackMute(trackId: TrackId, muted: boolean): void {
    this._adapter?.setTrackMute(trackId, muted);
  }

  setTrackSolo(trackId: TrackId, solo: boolean): void {
    this._adapter?.setTrackSolo(trackId, solo);
  }

  setTrackPhaseInvert(trackId: TrackId, inverted: boolean): void {
    this._adapter?.setTrackPhaseInvert(trackId, inverted);
  }

  setMasterVolume(volume: number): void {
    this._adapter?.setMasterVolume(volume);
  }

  subscribeMeters(callback: MeterCallback): () => void {
    if (!this._adapter) return () => {};
    return this._adapter.subscribeMeters(callback);
  }

  subscribeTransport(callback: TransportCallback): () => void {
    if (!this._adapter) return () => {};
    return this._adapter.subscribeTransport(callback);
  }

  private async _init(preferredEngine: PreferredEngine): Promise<void> {
    const selection = await createAudioEngineAdapter(preferRuntimeEngine(preferredEngine));
    this._adapter = selection.adapter;
    this._backend = selection.backend;
    this._transportUnsub = selection.adapter.subscribeTransport((state) => {
      this._setTransport(state);
    });

    const project = this._pendingProject ?? useProjectStore.getState().project;
    await selection.adapter.loadProject(project);
    this._pendingProject = project;
  }

  private async _ensureAdapter(): Promise<AudioEngineAdapter> {
    await this.init();
    if (!this._adapter) throw new Error("Audio engine failed to initialize");
    return this._adapter;
  }

  private _setTransport(next: { playing: boolean; positionSeconds: number }): void {
    this._transport = {
      playing: next.playing,
      positionSeconds: Math.max(0, next.positionSeconds),
    };
    const store = useTransportStore.getState();
    store.setIsPlaying(this._transport.playing);
    store.setPlayheadTime(this._transport.positionSeconds);
  }
}

function preferRuntimeEngine(preferredEngine: PreferredEngine): PreferredEngine {
  if (platform.kind === "electron") return "native-sphere-direct";
  return preferredEngine === "native-sphere-direct" ? "auto" : preferredEngine;
}

export const activeAudioEngine = new ActiveAudioEngine();
