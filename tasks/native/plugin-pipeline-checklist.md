# Native Plugin Pipeline — Phased Checklist

Status of the audio plugin loading + bus/return routing + native plugin
editor work across Futureboard Native. Tick the boxes as phases land.
Cross-reference:

- [plugin-insert-routing.md](./plugin-insert-routing.md)
- [plugin-view-native-editor.md](./plugin-view-native-editor.md)

Last updated: 2026-05-29.

---

## Phase 0 — Audit & docs

- [x] Inspect Electron plugin flow (`apps/electron/src/native-plugin/PluginHostNative.ts`)
- [x] Inspect SpherePluginHost public surface (napi vs. plain Rust)
- [x] Inspect DAUx `RuntimeInsert` / `RuntimeSend` / `Vst3RuntimeProcessor`
- [x] Document project schema (`ProjectInsert`, `ProjectPluginInstance`, `TrackRouting`, `Bus`/`Return` already present)
- [x] Write `tasks/native/plugin-insert-routing.md`
- [x] Write `tasks/native/plugin-view-native-editor.md`

## Phase 1 — UI insert scaffold

- [x] `InsertPluginFormat`, `InsertLoadStatus`, `PluginParameterState`
- [x] `InsertSlotState { id, plugin_id, plugin_path, plugin_format, display_name, enabled, bypassed, load_status, parameters }`
- [x] `TrackState.inserts: Vec<InsertSlotState>` (all constructors updated)
- [x] `TimelineState::add_insert / set_insert_plugin / remove_insert / toggle_insert_bypass`
- [x] Mixer strip renders real insert chips (name + bypass dot + remove ×)
- [x] `+ Add Insert` button on mixer strip
- [x] `MixerCallbacks::{on_add_insert, on_remove_insert, on_toggle_insert_bypass, on_open_insert_editor}`
- [x] `StudioLayout::build_mixer_callbacks` wires all four
- [x] Bypass and remove flip `engine_project_dirty` (next audio-poll syncs descriptor list)
- [x] Project save/load round-trips inserts (Project ↔ TimelineState mapping)
- [x] `FUTUREBOARD_PLUGIN_DEBUG=1` logs add/set/remove/bypass mutations
- [x] No realtime audio path changes — runtime still no-ops on unrecognised plugin descriptors

## Phase 2a — De-napi SpherePluginHost & registry-driven picker

- [x] `pub mod native_editor` exposes editor C ABI as plain Rust
  - [x] `open_plugin_editor_window` → `Result<u64, String>`
  - [x] `get_plugin_editor_attach_handle` → `u64`
  - [x] `attach_vst3_editor_view` → `Result<bool, String>`
  - [x] `close_plugin_editor_window`, `focus_plugin_editor_window`, `resize_plugin_editor_window`
  - [x] `drain_plugin_editor_param_events` → `Result<Vec<NativeEditorParamEvent>, String>`
  - [x] `stable_id` helper
- [x] Existing `#[cfg(feature = "napi")] mod editor_window` left bit-for-bit unchanged
- [x] Build both feature configs:
  - [x] `cargo check -p sphere-plugin-host --no-default-features` (native rlib)
  - [x] `cargo check -p sphere-plugin-host` (Electron cdylib)
- [x] `StudioLayout::available_plugins: Option<Vec<RegistryPlugin>>` lazy cache
- [x] `StudioLayout::pick_default_insert_plugin` — first call runs `PluginRegistry::scan(None)`
- [x] `on_add_insert` uses real `RegistryPlugin` when available; falls back to documented stub when registry is empty
- [x] Real `class_id`, `plugin_path`, `format`, `display_name` round-trip through project save/load

## Phase 2b — Real audio processing & picker overlay  *(not yet started)*

- [ ] Real picker overlay (combo / popover listing registered plugins, with category filter + search)
- [ ] DAUx `Vst3RuntimeProcessor::new` actually instantiates via `native_editor` / a new
  `native_processor` C ABI hookup
- [ ] `IPluginFactory` → `IComponent` → `IAudioProcessor` lifecycle on **worker thread**
- [ ] Audio thread only sees `process(...)` calls — no allocations, no logging, no locks
- [ ] Plugin instantiation failure → `InsertLoadStatus::Failed(msg)` in UI; no panic
- [ ] Manual test #6–7 (audio passes through plugin) ✓
- [ ] Manual test #8–9 (bypass changes audio) ✓
- [ ] Manual test #24 (bad plugin fails gracefully) ✓

## Phase 3 — Bus / Send / Return routing  *(not yet started)*

Schema already exists in `crates/SphereUIComponents/src/project/mod.rs`
(`Bus`, `Return`, `Group` in `ProjectTrackType`; `TrackRouting`,
`output_bus`, `sends`). Gap is in `timeline_state::TrackType` and the
runtime topology walk.

- [ ] Add `TrackType::Bus`, `TrackType::Return` to `timeline_state.rs`
- [ ] Add Track dialog rows for Bus / Return
- [ ] `SendSlotState` on `TrackState` (id, target_track_id, enabled, pre_fader, gain_db)
- [ ] Mixer strip renders sends section (currently empty placeholder)
- [ ] Visual differentiation: Bus / Return strip styling
- [ ] Inspector shows routing info per track
- [ ] DAUx runtime: topological scheduling of buses/returns
- [ ] Cycle detection — reject `Master → normal track` loops
- [ ] Send accumulation buffers (no per-block allocation)
- [ ] `FUTUREBOARD_ROUTING_DEBUG=1` logs graph nodes, order, sends, cycle rejections
- [ ] Manual tests #14–20

## Phase 4 — Native PluginView shell  *(not yet started)*

- [ ] `PluginViewCommand::{Open, Close, Resize, Focus}` enum
- [ ] GPUI window opens on `OpenInsertEditor` click
- [ ] External native plugin editor window (not embedded in GPUI) opened via
      `native_editor::open_plugin_editor_window`
- [ ] `attach_vst3_editor_view` on Windows path
- [ ] macOS path scaffolded (NSView platform type)
- [ ] Linux path deferred — fallback panel
- [ ] Resize forwards to `resize_plugin_editor_window`
- [ ] Close calls `close_plugin_editor_window` — no leaked handles
- [ ] Fallback GPUI panel when attach fails (plugin name + error + bypass/remove)
- [ ] `FUTUREBOARD_PLUGIN_VIEW_DEBUG=1` logs open/attach/resize/close/error
- [ ] Manual tests #10–12

## Phase 5 — Parameter event drain pump  *(not yet started)*

- [ ] `cx.spawn` loop at ~30 Hz on UI thread
- [ ] `drain_plugin_editor_param_events` → `InsertSlotState.parameters`
- [ ] UI parameter change → plugin controller (reverse direction)
- [ ] Automation hookup deferred to a later round
- [ ] No audio thread interaction

---

## Hard rules carried across all phases

- Plugin instantiation runs on a worker thread, never the audio thread.
- No `LoadProject` for UI-only actions (bypass toggle, slot select).
- Bad plugin → `InsertLoadStatus::Failed(msg)`; never panic.
- Audio callback never allocates, never JSON-parses, never logs.
- IPlugView calls run on the UI/main thread, never the audio thread.
- Every `open_plugin_editor_window` pairs with `close_plugin_editor_window`.
- Theme tokens only — no hardcoded colors.
- Cross-process plugin isolation deferred per `SKILL.md` §13; documented as
  a long-term goal.

## Manual test status (against the original spec checklist)

| # | Test | Status | Phase |
|---|---|---|---|
| 1 | Start app | ✅ | — |
| 2 | Add Audio Track | ✅ | — |
| 3 | Load audio clip | ✅ | — |
| 4 | Add VST3 plugin to insert | ✅ real names if scanned, else stub | 1 / 2a |
| 5 | Confirm insert name appears | ✅ | 1 |
| 6 | Press play | ✅ | — |
| 7 | Audio passes through plugin | ❌ | 2b |
| 8 | Bypass plugin | ⚠️ UI only | 2b for audio |
| 9 | Bypass changes audio | ❌ | 2b |
| 10 | Open plugin editor | ❌ | 4 |
| 11 | Resize editor | ❌ | 4 |
| 12 | Close editor | ❌ | 4 |
| 13 | Remove plugin | ✅ | 1 |
| 14 | Add Return Track | ❌ | 3 |
| 15 | Send Audio → Return | ❌ | 3 |
| 16 | Plugin on Return Track | ❌ | 3 |
| 17 | Wet signal on return | ❌ | 3 |
| 18 | Add Bus Track | ❌ | 3 |
| 19 | Route Audio → Bus | ❌ | 3 |
| 20 | Bus → Master | ❌ | 3 |
| 21 | Save project | ✅ | 1 |
| 22 | Reopen project | ✅ | 1 |
| 23 | Inserts / routing restored | ✅ inserts; ❌ routing | 1 / 3 |
| 24 | Bad plugin fails gracefully | n/a until 2b | 2b |

Legend: ✅ done · ⚠️ partial · ❌ pending · n/a not relevant yet
