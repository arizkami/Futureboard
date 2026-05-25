# Settings Dialog & Configuration System Overview

This document specifies the structure, categories, and roadmap for the Futureboard Native Settings Dialog and underlying JSON configuration files.

## Category Tree Specification

The Settings dialog uses a hierarchical categories tree layout:

```text
Settings
├─ General
│  ├─ Application (language, start screens, update checks)
│  ├─ Project Defaults (default tempo, sample rate, tracks setup)
│  ├─ Autosave & Backup (autosave intervals, backup retain count)
│  └─ Notifications (in-app warnings, system notifications)
│
├─ Hardware
│  ├─ Audio (driver type, device selection, buffer size, active channels)
│  ├─ MIDI (MIDI input/output device enable states, clock sync options)
│  ├─ Control Surfaces (protocol type: MCU, HUI, OSC, device mapping)
│  └─ Sync (external LTC/MTC sync, clock source selection)
│
├─ Appearance
│  ├─ Theme (theme variant, custom CSS injection, editor opacity)
│  ├─ Layout (sidebar placement, status bar show/hide, window frame settings)
│  ├─ Arrangement (clip color assignment, grid line intensity)
│  ├─ Piano Roll (key guide labels, note layout scale colors)
│  ├─ Audio Editor (waveform color schema, grid background shading)
│  └─ Mixer (fader size, peak hold time, meter decay rates)
│
├─ Editing
│  ├─ Mouse & Tools (tool click behavior, scroll direction, zoom factor)
│  ├─ Snap & Grid (default snap value, triple/dotted toggles)
│  ├─ Timeline (playhead auto-scroll, timeline units)
│  ├─ Automation (draw tool smoothness, point editing behavior)
│  └─ Undo History (max undo states, undo history grouping)
│
├─ Recording
│  ├─ Audio Recording (file format, recording folder, input latency compensation)
│  ├─ MIDI Recording (MIDI notes quantize on record, latency compensation)
│  └─ Metronome (click volume, sound type, pre-roll counts)
│
├─ Playback
│  ├─ Transport (spacebar behavior, return to start on stop)
│  ├─ Looping (loop selection bounds snap, pre-roll looping)
│  └─ Performance (multi-core engine usage, audio thread priority)
│
├─ Plugins
│  ├─ Formats (VST3/CLAP formats enable, sandbox level)
│  ├─ Folders (system directories, custom search paths list)
│  ├─ Scanning (automatic background scanning, rescan failed plug-ins)
│  ├─ Windows (floating editors auto-hide, plugin scaling factor)
│  ├─ Processing (threading allocation, offline oversampling factor)
│  └─ Browser (plugin category sorting options: creator, format, tag)
│
├─ Files & Folders
│  ├─ Extra Folders (user library paths, external soundbank locations)
│  ├─ File Management (missing files automatic search, copy audio on import)
│  └─ Cache (waveform preview cache size limits, cleanup on exit)
│
├─ Export
│  ├─ Audio Export (default format, sample rate, bit depth, dither)
│  ├─ Stems (stems naming patterns, track export scopes)
│  ├─ Offline Render (oversampling factor on render, normalize on export)
│  └─ Video Export (video container, codec selection, audio render bitrate)
│
├─ Library
│  ├─ Media Browser (media indexing folders, file scanner filters)
│  ├─ Presets (presets saving locations, default preset path)
│  └─ Tags (user tags definition, color tag assignments)
│
├─ Shortcuts
│  ├─ Keyboard (custom shortcut mapping editor, shortcuts profile)
│  └─ Command Palette (command exclusions list, fuzzy search behavior)
│
├─ Accessibility
│  ├─ Visual (font scale factor, high contrast mode, screen reader support)
│  ├─ Input (single-key shortcuts mode, drag sensitivity helper)
│  └─ Audio (accessible status beeps, mono channel layout helper)
│
├─ Cloud & Account
│  ├─ Account (user profile sign-in, license verification)
│  ├─ Sync (presets and settings cloud backup, automatic synchronization)
│  └─ Collaboration (shared project permissions defaults, presence indicators)
│
├─ Network & Remote
│  ├─ Remote Control (remote web UI host address, remote control ports)
│  └─ Network Audio/MIDI (network audio streaming protocol, MIDI network sessions)
│
├─ Advanced
│  ├─ Engine (WASAPI exclusive flags, hardware DMA transfer sizes)
│  ├─ Debug (logging level verbosity, show performance overlays, crash dump path)
│  └─ Developer (plugin developer mode, script execution sandbox)
│
├─ Security & Privacy
│  ├─ Privacy (telemetry collection enable, usage reports opt-out)
│  └─ Security (sandboxed plugin execution, untrusted binary alerts)
│
└─ About
   ├─ Product Info (version details, system specifications)
   ├─ Links (documentation website, user forum, support email)
   └─ Credits (developer roster, legal notices, third-party licenses)
```

## System Requirements

1. **Native Settings Dialog**: A unified, responsive dialog that adapts to the categories hierarchy. It uses the DAW design language (sleek dark surface, subtle borders, compact inputs).
2. **JSON Settings File**: Settings are saved in `settings.json` in the user configuration directory (e.g. `AppData/Roaming/Futureboard/settings.json` on Windows).
3. **Rust Model Backend**: A centralized settings manager backed by `serde` for loading, editing, saving, and defaulting settings.
4. **Reactive State Syncing**: Changes are instantly reflected across open windows and audio engine states.
