/**
 * registerSphereAudioHandlers — wires IPC channels to the native Rust addon.
 *
 * Call once from Electron main after app.whenReady().
 * Uses `SphereAudioNative` (direct N-API addon) instead of a child process.
 *
 * If the addon fails to load (dev machine without a Rust build, CI without
 * native deps, etc.) every handler returns a safe "unavailable" response
 * rather than throwing, so the rest of the app continues working.
 */
import { ipcMain } from "electron";
import { IpcChannels } from "../ipc/channels.js";
import { sphereAudioNative } from "./SphereAudioNative.js";
import type { SphereDeviceOpenConfig, SphereTransportState } from "../ipc/channels.js";

export function registerSphereAudioHandlers(_appDir: string): void {
  const svc = sphereAudioNative;

  // Try to initialise the native addon on first registration.
  const available = svc.initialize();
  if (!available) {
    console.warn(
      "[SphereAudio] Native addon unavailable — IPC handlers registered in degraded mode"
    );
  } else {
    // Auto-open the system default output device and start the audio stream.
    // This runs immediately so the engine is "running" before the renderer
    // ever queries getStatus(), making the settings panel show the correct state.
    try {
      svc.openDevice({});   // omitted config fields → system default device/config
      svc.start();          // stream.play() — silent until transport play or test tone
      console.log("[SphereAudio] Auto-started on default output device");
    } catch (e) {
      console.warn("[SphereAudio] Auto-start failed (non-fatal):", e);
    }
  }

  // ── Status / version ───────────────────────────────────────────────────────

  ipcMain.handle(IpcChannels.SphereAudioGetStatus, () => {
    return svc.getStatus();
  });

  ipcMain.handle(IpcChannels.SphereAudioGetVersion, () => {
    return svc.getVersion();
  });

  // ── Device enumeration ─────────────────────────────────────────────────────

  ipcMain.handle(IpcChannels.SphereAudioListInputDevices, () => {
    return svc.listInputDevices();
  });

  ipcMain.handle(IpcChannels.SphereAudioListOutputDevices, () => {
    return svc.listOutputDevices();
  });

  // ── Stream lifecycle ───────────────────────────────────────────────────────

  ipcMain.handle(
    IpcChannels.SphereAudioOpenDevice,
    (_event, config: SphereDeviceOpenConfig) => {
      svc.openDevice(config); // throws if addon unavailable
    },
  );

  ipcMain.handle(IpcChannels.SphereAudioCloseDevice, () => {
    svc.closeDevice();
  });

  ipcMain.handle(IpcChannels.SphereAudioStart, () => {
    svc.start(); // opens cpal stream + begins audio output
  });

  ipcMain.handle(IpcChannels.SphereAudioStop, () => {
    svc.stop();
  });

  ipcMain.handle(
    IpcChannels.SphereAudioSetTestTone,
    (_event, enabled: boolean, frequency: number) => {
      svc.setTestTone(enabled, frequency);
    },
  );

  // ── Transport ──────────────────────────────────────────────────────────────
  // The old IPC shape used a `SphereTransportState` bag with optional fields.
  // Map it to the individual engine calls.

  ipcMain.handle(
    IpcChannels.SphereAudioSetTransport,
    (_event, state: SphereTransportState) => {
      if (typeof state.positionSeconds === "number") {
        svc.seek(state.positionSeconds);
      }
      if (state.playing === true)  svc.play();
      if (state.playing === false) svc.pause();
    },
  );

  ipcMain.handle(IpcChannels.SphereAudioGetTransport, () => {
    const st = svc.getStatus();
    return {
      playing:         st.transportPlaying,
      positionSeconds: st.positionSeconds,
    };
  });

  // ── Param updates ──────────────────────────────────────────────────────────

  ipcMain.handle(
    IpcChannels.SphereAudioUpdateTrackParam,
    (_event, trackId: string, paramId: string, value: unknown) => {
      svc.updateTrackParam(trackId, paramId, value);
    },
  );

  ipcMain.handle(
    IpcChannels.SphereAudioUpdateInsertParam,
    (_event, trackId: string, insertId: string, paramId: string, value: unknown) => {
      svc.updateInsertParam(trackId, insertId, paramId, value);
    },
  );

  // ── Project snapshot ───────────────────────────────────────────────────────

  ipcMain.handle(
    IpcChannels.SphereAudioLoadProject,
    (_event, snapshot: unknown) => {
      svc.loadProject(snapshot);
    },
  );

  ipcMain.handle(
    IpcChannels.SphereAudioUpdateClip,
    (_event, clipId: string, patch: unknown) => {
      svc.updateClip(clipId, patch);
    },
  );

  // ── Meters ─────────────────────────────────────────────────────────────────

  ipcMain.handle(IpcChannels.SphereAudioGetMeters, () => {
    return svc.getMeters();
  });

  console.log(
    `[SphereAudio] IPC handlers registered (addon ${available ? "✓ loaded" : "✗ unavailable"})`
  );
}
