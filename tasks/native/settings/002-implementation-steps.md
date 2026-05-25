# Settings Implementation Checklist

This checklist tracks step-by-step tasks to implement the Settings dialog, JSON configuration storage, and reactive subsystem synchronization.

## Phase 1: Structs & Disk Operations

- [ ] Create `SettingsSchema` struct mirroring the layout structure with `serde` serialize/deserialize annotations.
- [ ] Implement `Default` for `SettingsSchema`.
- [ ] Implement validation/bounds-clamping helper functions for numeric properties.
- [ ] Implement the `SettingsModel` file wrapper:
  - [ ] `load_or_create` logic (resolving platform config path, loading file, backup recovery on parse error).
  - [ ] `save_to_disk` logic (asynchronous writing to minimize main thread lockups).
- [ ] Write unit tests for configuration parsing, clamping, and corrupted backup generation.

---

## Phase 2: Interface Shell & Navigation

- [ ] Register `SettingsModel` as a global model in GPUI.
- [ ] Design the `SettingsDialog` GPUI Window:
  - [ ] Implement the compact titlebar with Search input.
  - [ ] Implement sidebar category navigation list.
  - [ ] Implement the scrollable content view with sticky section headers.
- [ ] Wire category selection to scroll-to-section.
- [ ] Wire Search filtering:
  - [ ] Realtime filter matching text query.
  - [ ] Show matching results and collapse non-matching categories.

---

## Phase 3: Setting Categories Coding

### 1. General Panel
- [ ] Implement Application language, updates, and start wizard toggles.
- [ ] Implement Project Defaults (tempo input, sample rate dropdown, buffer size dropdown).
- [ ] Implement Autosave intervals and backup limits.
- [ ] Implement Notification warning flags.

### 2. Hardware Panel
- [ ] Implement Audio Driver type selector.
- [ ] Implement Audio input/output device selectors.
- [ ] Implement MIDI input/output port checkbox lists.
- [ ] Implement Sync source options.

### 3. Appearance Panel
- [ ] Implement Theme switcher.
- [ ] Implement UI Scale slider.
- [ ] Implement Arrangement grid opacity slider.
- [ ] Implement Piano Roll key labels mode.
- [ ] Implement Mixer peak hold duration and decay settings.

### 4. Editing Panel
- [ ] Implement Mouse zoom/scroll options.
- [ ] Implement Grid default snap values.
- [ ] Implement History max undo limits.

### 5. Recording & Playback Panels
- [ ] Implement Audio export formats and recording locations.
- [ ] Implement Metronome clicks sound type and volume slider.
- [ ] Implement Transport spacebar behavior options.

### 6. Plugins Panel
- [ ] Implement VST3/CLAP directories path management list.
- [ ] Implement manual plug-ins scanning trigger button.
- [ ] Implement scanning errors listing interface.

### 7. Files & Library Panels
- [ ] Implement user soundbanks locations paths editor.
- [ ] Implement indexing filters.

### 8. Cloud & Network Panels
- [ ] Implement Account details, remote control ports, and network MIDI session settings.

### 9. Advanced, Security, & About Panels
- [ ] Implement log verbosity level, dev modes, telemetry toggles, and credits info display.

---

## Phase 4: State Syncing & Verification

- [ ] Connect Settings changes to live systems:
  - [ ] Live theme switcher updates the active layout colors instantly.
  - [ ] Changing audio driver/device dispatches a reconstruction request to `SphereDirectAudioEngine`.
  - [ ] Changing buffer size updates active engine latency.
  - [ ] Changing metronome settings updates live click synthesis parameters.
- [ ] Build verification:
  - [ ] Ensure full workspace checks pass.
  - [ ] Run application manually to confirm settings dialog triggers and loads settings on start.
