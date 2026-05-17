import { create } from "zustand";

const STORAGE_KEY = "futureboard.appSettings.v1";

export type StartupBehavior = "wizard" | "newProject" | "lastProject";
export type PreferredEngine = "auto" | "wasm" | "webAudio" | "native-sphere-direct";
export type PreferredBufferSize = 64 | 128 | 256 | 512 | 1024;

/** DAUx OS-level audio backend selection. */
export type DauxBackend = "wasapi" | "mme" | "coreaudio" | "alsa";

/** Audio engine sample rate override. "device-default" lets the driver choose. */
export type AudioSampleRate = "device-default" | 44100 | 48000 | 96000;

/** Top-level engine kind visible in the UI (derived from runtime, not user-selectable). */
export type AudioEngineKind = "daux" | "wasm";

export type AppSettings = {
  startupBehavior: StartupBehavior;
  autoSave: boolean;
  autoSaveIntervalMin: number;
  preferredEngine: PreferredEngine;
  preferredBufferSize: PreferredBufferSize;
  /** DAUx OS backend (Electron only). Defaults to the platform-appropriate backend. */
  dauxBackend?: DauxBackend;
  /** Audio engine sample rate (Electron / DAUx only). */
  audioSampleRate: AudioSampleRate;
  compactUI: boolean;
  enableDevTools: boolean;
};

const DEFAULTS: AppSettings = {
  startupBehavior: "wizard",
  autoSave: true,
  autoSaveIntervalMin: 5,
  preferredEngine: "auto",
  preferredBufferSize: 256,
  dauxBackend: undefined,   // resolved at runtime per-platform
  audioSampleRate: "device-default",
  compactUI: false,
  enableDevTools: false,
};

function loadFromStorage(): AppSettings {
  try {
    const raw = localStorage.getItem(STORAGE_KEY);
    if (!raw) return { ...DEFAULTS };
    const parsed = JSON.parse(raw) as Partial<AppSettings>;
    return { ...DEFAULTS, ...parsed };
  } catch {
    return { ...DEFAULTS };
  }
}

function saveToStorage(s: AppSettings) {
  try {
    localStorage.setItem(STORAGE_KEY, JSON.stringify(s));
  } catch {
    // ignore quota errors
  }
}

type SettingsStore = AppSettings & {
  applySettings: (patch: Partial<AppSettings>) => void;
  resetToDefaults: () => void;
};

export const useSettingsStore = create<SettingsStore>((set) => ({
  ...loadFromStorage(),

  applySettings(patch) {
    set((s) => {
      const next: AppSettings = {
        startupBehavior:    patch.startupBehavior    ?? s.startupBehavior,
        autoSave:           patch.autoSave           ?? s.autoSave,
        autoSaveIntervalMin:patch.autoSaveIntervalMin ?? s.autoSaveIntervalMin,
        preferredEngine:    patch.preferredEngine    ?? s.preferredEngine,
        preferredBufferSize:patch.preferredBufferSize ?? s.preferredBufferSize,
        dauxBackend:        patch.dauxBackend        ?? s.dauxBackend,
        audioSampleRate:    patch.audioSampleRate    ?? s.audioSampleRate,
        compactUI:          patch.compactUI          ?? s.compactUI,
        enableDevTools:     patch.enableDevTools     ?? s.enableDevTools,
      };
      saveToStorage(next);
      return next;
    });
  },

  resetToDefaults() {
    saveToStorage(DEFAULTS);
    set({ ...DEFAULTS });
  },
}));

export { DEFAULTS as APP_SETTINGS_DEFAULTS };
