/**
 * createAudioEngineAdapter — selects the best AudioEngineAdapter for the
 * current runtime and user settings.
 *
 * Selection logic:
 *
 *   Web browser:
 *     → always WebAudioEngineAdapter (native engine unavailable)
 *
 *   Electron:
 *     preferredEngine === "native-sphere-direct"  → native only, error if unavailable
 *     preferredEngine === "auto"                  → native if available, else WebAudio
 *     preferredEngine === "wasm" | "webAudio"     → WebAudioEngineAdapter
 *
 * Electron desktop forces native through the caller; WebAudio remains the
 * browser runtime and the explicit web fallback.
 */
import type { AudioEngineAdapter } from "../AudioEngineAdapter";
import type { PreferredEngine } from "../../store/settingsStore";
import { webAudioEngineAdapter } from "../WebAudioEngineAdapter";
import { NativeSphereAudioEngineAdapter } from "./NativeSphereAudioEngineAdapter";
import { detectAudioEngineBackends } from "./detection";
import { showToast } from "../../components/ui/Toast";

export type AdapterSelection = {
  adapter:  AudioEngineAdapter;
  backend:  "web-audio" | "native-sphere-direct";
  fallback: boolean;
};

/**
 * Build and return an initialised adapter.
 * The adapter is already init()'d when this promise resolves.
 */
export async function createAudioEngineAdapter(
  preferredEngine: PreferredEngine,
): Promise<AdapterSelection> {
  // ── Web browser: always WebAudio ──────────────────────────────────────────
  if (!window.dawElectron) {
    const adapter = webAudioEngineAdapter;
    await adapter.init();
    return { adapter, backend: "web-audio", fallback: false };
  }

  // ── Electron: check preference ────────────────────────────────────────────
  const wantNative =
    preferredEngine === "native-sphere-direct" ||
    preferredEngine === "auto";

  if (wantNative) {
    const backends = await detectAudioEngineBackends();
    const nativeStatus = backends.find((b) => b.backend === "native-sphere-direct");

    if (nativeStatus?.available) {
      try {
        const adapter = new NativeSphereAudioEngineAdapter();
        await adapter.init();
        console.log("[EngineFactory] Using SphereDirectAudioEngine (native)");
        return { adapter, backend: "native-sphere-direct", fallback: false };
      } catch (e) {
        console.error("[EngineFactory] Native engine init failed:", e);
        showToast("Native audio engine failed", true);
        if (preferredEngine === "native-sphere-direct") throw e;
      }
    } else if (preferredEngine === "native-sphere-direct") {
      const reason = nativeStatus?.reason ?? "unknown reason";
      console.warn(`[EngineFactory] Native engine requested but unavailable (${reason}).`);
      showToast(`Native audio engine unavailable: ${reason}`, true);
      throw new Error(`Native audio engine unavailable: ${reason}`);
    }
  }

  // ── WebAudio fallback (or explicit preference) ────────────────────────────
  const adapter = webAudioEngineAdapter;
  await adapter.init();
  const isFallback = wantNative;
  console.log(
    `[EngineFactory] Using WebAudioEngineAdapter${isFallback ? " (fallback)" : ""}`,
  );
  return { adapter, backend: "web-audio", fallback: isFallback };
}
