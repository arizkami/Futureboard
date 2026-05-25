# Settings Dialog — User Interface Design

The Settings Dialog must maintain visual consistency with the Futureboard Studio DAW design language.

## 1. UI Layout Structure

The layout is a two-column modal window with a compact titlebar:

```text
┌─ Settings Dialog ────────────────────────────────────────────────────────┐
│  Search Settings [ Q ]                                          [ Close ]│
├──────────────────────────────────────────────────────────────────────────┤
│  Sidebar Navigation              │  Content Pane (Scrollable)            │
│  ┌────────────────────────────┐  │  ┌─────────────────────────────────┐  │
│  │ General                    │  │  │ General > Application           │  │
│  │   Application              │  │  │   Language: [ English   | v ]   │  │
│  │   Project Defaults         │  │  │   Start Screen: [x] Show wizard  │  │
│  │   Autosave & Backup        │  │  ├─────────────────────────────────┤  │
│  │   Notifications            │  │  │ General > Project Defaults      │  │
│  │                            │  │  │   Default Tempo: [ 120.0  ] bpm │  │
│  │ Hardware                   │  │  │   Sample Rate: [ 48000 Hz | v ] │  │
│  │   Audio                    │  │  │   Buffer Size: [ 256 smpl | v ] │  │
│  │   MIDI                     │  │  │                                 │  │
│  │   Control Surfaces         │  │  │                                 │  │
│  │   Sync                     │  │  │                                 │  │
│  └────────────────────────────┘  │  └─────────────────────────────────┘  │
└──────────────────────────────────┴───────────────────────────────────────┘
```

---

## 2. Dialog Elements

### A. Sidebar Navigation
- **Width**: Fixed at `160 px`.
- **Background**: `Colors::bottom_panel_header_bg()` or `Colors::surface_panel_alt()`.
- **Layout**: Tree structure of categories. Selecting a category highlights the item and scrolls the right content panel to the corresponding section.
- **Scroll Behavior**: Shows a custom scrollbar if categories exceed height.

### B. Right Content Panel
- **Background**: `Colors::bottom_panel_bg()` or `Colors::surface_panel()`.
- **Divider**: `Colors::panel_border()`.
- **Layout**: Vertical list of settings sections. Each section has:
  - Section Header (Title, icon, separator line).
  - Settings Controls (labeled dropdowns, text inputs, checkboxes, slider controls).
- **Scroll Behavior**: Scrollable viewport with section headers pinning at the top (sticky headers) to maintain context while scrolling.

### C. Search Bar
- **Position**: Pinned to the top of the sidebar.
- **Shortcut**: `Ctrl + F` or `Q` when dialog is active.
- **Filter Action**: Realtime filtering of settings. Typing a query collapses the categories tree to show only sections or properties matching the search term, highlighting matching labels.

---

## 3. UI Control Specifications

All inputs must be compact and themed:

### Checkbox
- **Size**: `12x12 px`.
- **Border**: `Colors::border_default()`.
- **Active State**: Fill with `Colors::accent_primary()` and display a tick icon in `Colors::text_inverse()`.

### Dropdown (Combobox)
- **Height**: `22 px`.
- **Background**: `Colors::slot_bg()`.
- **Border**: `Colors::slot_border()`.
- **Text**: `Colors::text_primary()`.

### Text & Numeric Inputs
- **Height**: `22 px`.
- **Background**: `Colors::surface_input()`.
- **Border**: `Colors::border_subtle()`.
- **Focused State**: Border `Colors::border_focus()`, shadow focus ring `Colors::with_alpha(Colors::border_focus(), 0.15)`.
