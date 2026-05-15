// ─────────────────────────────────────────────────────────────────────────────
//  Mochi DAW · native shell layout constants
//
//  Mirrors the web prototype so the native and web shells stay visually
//  aligned while the C++ port is incremental.  Values are sourced from
//  apps/web/src/theme.ts and the web AppShell / TransportBar / StatusBar
//  components, expressed here in logical (CSS) pixels.
// ─────────────────────────────────────────────────────────────────────────────
#pragma once

namespace MochiDaw {

// Top transport bar (TransportBar.tsx — `h-9` → 36 px).
constexpr float TRANSPORT_BAR_H   = 36.f;

// Bottom status bar (StatusBar.tsx).
constexpr float STATUSBAR_H       = 22.f;

// Browser panel default width  (theme.ts → BROWSER_WIDTH = 272).
constexpr float BROWSER_W         = 272.f;

// Inspector panel default width (matches AppShell right rail).
constexpr float INSPECTOR_W       = 280.f;

// Bottom mixer panel default height (uiStore mixer panel size).
constexpr float MIXER_H           = 300.f;

// Arrangement timeline ruler / track header / track row dimensions.
constexpr float ARRANGEMENT_RULER_H = 30.f;   // theme.ts → RULER_HEIGHT
constexpr float ARRANGEMENT_HEADER_W = 272.f; // theme.ts → HEADER_WIDTH
constexpr float ARRANGEMENT_TRACK_H = 76.f;   // theme.ts → TRACK_HEIGHT

// Window sizing — matches the web app's `FULL_MENU_MIN_WIDTH = 1600` and a
// comfortable workstation height; minimum size avoids the partial / overflow
// menu layout for the native preview.
constexpr int   WINDOW_W          = 1600;
constexpr int   WINDOW_H          = 1000;
constexpr int   WINDOW_MIN_W      = 1100;
constexpr int   WINDOW_MIN_H      = 720;

} // namespace MochiDaw
