# Futureboard Studio Task Index

This folder contains implementation task prompts for Futureboard Studio / Mochi DAW.

These files are not meant to be implemented all at once.
They are long-form task specs for AI agents and human-guided development.

Recommended usage:

1. Pick one task file.
2. Ask the agent to read only that file.
3. Ask the agent to implement only one section or one phase.
4. Run build/tests.
5. Commit.
6. Continue with the next focused section.

Hard rule:
Do not let an agent implement an entire long task file in one pass unless the task explicitly says it is small.

============================================================
Workflow
============================================================

Idea / Bug / Design Direction
-> Convert into task prompt
-> Save into tasks folder
-> Agent reads focused task/section
-> Agent patches code
-> Build
-> Smoke test
-> Commit

This is spec-driven vibe coding:
Human-directed.
Agent-assisted.
Prompt-governed.

============================================================
Task Files
============================================================

001-Plugins.txt
Built-in plugin UI, PluginShell, insert chain, and Effect Editor integration.
Focus:
- horizontal plugin UI
- track.inserts as source of truth
- Add Device menu
- Effect Editor device chain
- plugin parameter state

002-WaveformCache.txt
Waveform peak cache for Web and Electron.
Focus:
- Web IndexedDB cache
- Electron cache folder
- waveform cache adapter
- cache key/versioning
- avoid regenerating peaks repeatedly

003-LinkMenuBar.txt
Link already implemented actions to Menu Bar, command registry, shortcuts, and command palette.
Focus:
- no fake menu actions
- command system as single path
- enabled/checked state
- context-aware actions

004-WindowingSystem.txt
Windowed floating panels, dialog windows, Electron external floating windows, Project Wizard, and Unsaved Changes dialog.
Focus:
- internal floating windows
- internal dialogs
- external Electron BrowserWindow skeleton
- project wizard
- dirty project guard

005-ProjectDropdown.txt
Zed-like project dropdown in the Transport Bar.
Focus:
- click project name
- current project
- recent projects
- open local folder / open more project
- dirty guard
- persistent recent projects

006-DawSystem.txt
Foundational DAW system layer using WebAudio first.
Focus:
- project state
- tracks/clips/notes
- mixer/routing
- insert/device state
- transport
- selectors
- commands
- WebAudio engine adapter
- dirty/save state

007-TouchScreenSupport.txt
Touch screen, pen, and stylus support.
Focus:
- pointer events
- touch hit targets
- timeline touch editing
- MIDI piano roll touch editing
- mixer fader touch support
- plugin controls touch support
- long press context menus

999-AudioEngine.txt
Long-term AudioEngine + DSP architecture.
Focus:
- Rust DSP core
- WebAssembly runtime
- AudioWorklet
- Electron native .exe DSP service
- IPC protocol
- mixer/routing/devices
- offline render
- WebAudio fallback

native/000-ReWriteAll.txt
Native UI rewrite roadmap.
Focus:
- native UI first
- app shell
- renderer
- layout
- command system
- DAW views
- feature parity with React prototype
- keep React app as living spec

============================================================
Recommended Implementation Order
============================================================

For the current React/Electron prototype:

1. 003-LinkMenuBar.txt
2. 004-WindowingSystem.txt
3. 005-ProjectDropdown.txt
4. 001-Plugins.txt
5. 002-WaveformCache.txt
6. 006-DawSystem.txt
7. 007-TouchScreenSupport.txt

Long-term:

8. 999-AudioEngine.txt
9. native/000-ReWriteAll.txt

Reason:
Menu and window systems become the backbone.
Plugins and waveform cache plug into the existing UI.
DawSystem cleans the deep architecture after UI flows are clearer.
AudioEngine and Native rewrite are long-term architecture tasks.

============================================================
How to Prompt an Agent
============================================================

Good example:

Read .claude/tasks/006-DawSystem.txt.

Implement only section 3: Time and Musical Math.

Do not touch unrelated UI.
Do not implement WebAudio engine yet.
Run bun run build.
Stop when TypeScript passes.

Bad example:

Read all tasks and implement everything.

Why bad:
The agent will attempt a giant rewrite and likely break the repo.

============================================================
Agent Safety Rules
============================================================

Always tell the agent:

- implement only the requested section
- do not redesign unrelated UI
- do not rewrite audio engine unless the task says so
- keep TypeScript clean
- run build
- leave TODO placeholders for future-heavy features
- disable unimplemented actions instead of faking success
- preserve current working behavior
- prefer small patches

============================================================
Naming Convention
============================================================

Numbered task files:

001-FeatureName.txt
002-FeatureName.txt
003-FeatureName.txt

Reserved high numbers:

999-AudioEngine.txt
Long-term foundational architecture.

Native tasks:

native/000-ReWriteAll.txt
native/001-Renderer.txt
native/002-LayoutEngine.txt
native/003-InputSystem.txt

============================================================
Commit Examples
============================================================

Add/update task files:

git add .claude/tasks
git commit -m "docs(tasks): update DAW implementation backlog"

Add one task:

git add .claude/tasks/007-TouchScreenSupport.txt
git commit -m "docs(tasks): add touchscreen support task"

Native task:

git add .claude/tasks/native/000-ReWriteAll.txt
git commit -m "docs(native): add native UI rewrite roadmap"

============================================================
Project Direction
============================================================

Futureboard Studio is a DAW prototype and living specification.

Current stack:
- React
- Vite
- Electron
- WebAudio prototype
- Canvas for heavy visuals
- Zustand/project store
- command/menu/task-driven development

Future stack:
- Rust/WASM DSP for Web
- Native DSP service for Electron
- SphereEngine/SphereUI native rewrite later

Current priority:
Prototype product behavior first.
Native/audio rewrites later.

The React/Electron app is not throwaway.
It is the living product spec.
