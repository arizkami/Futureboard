export type TrackMeterSnapshot = {
  trackId: string;
  peakL: number;
  peakR: number;
  rmsL?: number;
  rmsR?: number;
  holdL?: number;
  holdR?: number;
  updatedAt: number;
};

export type MeterState = {
  tracks: Record<string, TrackMeterSnapshot>;
  master: TrackMeterSnapshot;
};

type MeterListener = (snapshot: TrackMeterSnapshot) => void;

class MeterStore {
  private state: MeterState = {
    tracks: {},
    master: zeroTrack("master"),
  };
  private listeners = new Map<string, Set<MeterListener>>();

  getSnapshot(trackId: string): TrackMeterSnapshot {
    return trackId === "master" ? this.state.master : this.state.tracks[trackId] ?? zeroTrack(trackId);
  }

  getState(): MeterState {
    return this.state;
  }

  subscribe(trackId: string, listener: MeterListener): () => void {
    const listeners = this.listeners.get(trackId) ?? new Set<MeterListener>();
    listeners.add(listener);
    this.listeners.set(trackId, listeners);
    listener(this.getSnapshot(trackId));
    return () => {
      listeners.delete(listener);
      if (listeners.size === 0) this.listeners.delete(trackId);
    };
  }

  updateTrack(trackId: string, level: { l: number; r: number; rmsL?: number; rmsR?: number }): void {
    if (trackId === "master") {
      this.updateMaster(level);
      return;
    }
    const snapshot: TrackMeterSnapshot = {
      trackId,
      peakL: clampMeter(level.l),
      peakR: clampMeter(level.r),
      rmsL: level.rmsL,
      rmsR: level.rmsR,
      updatedAt: performance.now(),
    };
    this.state.tracks = { ...this.state.tracks, [trackId]: snapshot };
    this.emit(trackId, snapshot);
  }

  updateMaster(level: { l: number; r: number; rmsL?: number; rmsR?: number }): void {
    const snapshot: TrackMeterSnapshot = {
      trackId: "master",
      peakL: clampMeter(level.l),
      peakR: clampMeter(level.r),
      rmsL: level.rmsL,
      rmsR: level.rmsR,
      updatedAt: performance.now(),
    };
    this.state.master = snapshot;
    this.emit("master", snapshot);
  }

  clearTrack(trackId: string): void {
    const next = { ...this.state.tracks };
    delete next[trackId];
    this.state.tracks = next;
    this.emit(trackId, zeroTrack(trackId));
  }

  private emit(trackId: string, snapshot: TrackMeterSnapshot): void {
    const listeners = this.listeners.get(trackId);
    if (!listeners) return;
    for (const listener of listeners) listener(snapshot);
  }
}

function zeroTrack(trackId: string): TrackMeterSnapshot {
  return {
    trackId,
    peakL: 0,
    peakR: 0,
    rmsL: 0,
    rmsR: 0,
    updatedAt: 0,
  };
}

function clampMeter(value: number): number {
  if (!Number.isFinite(value)) return 0;
  return Math.max(0, Math.min(1, value));
}

export const meterStore = new MeterStore();
