# settings.json Schema Specification

This document details the configuration schema, defaults, and type mapping for Futureboard's JSON configuration file.

## 1. JSON Schema Draft

```json
{
  "general": {
    "language": "en",
    "show_start_screen": true,
    "check_updates": true,
    "project_defaults": {
      "tempo": 120.0,
      "time_signature_num": 4,
      "time_signature_den": 4,
      "sample_rate": 48000,
      "buffer_size": 256,
      "tracks_count": 4
    },
    "autosave": {
      "enabled": true,
      "interval_minutes": 5,
      "max_backups": 10
    },
    "notifications": {
      "enable_warnings": true,
      "enable_system_notifications": false
    }
  },
  "hardware": {
    "audio": {
      "driver_type": "WASAPI Exclusive",
      "device_in": "Built-in Microphone",
      "device_out": "Speakers (Realtek)",
      "active_inputs": [0, 1],
      "active_outputs": [0, 1]
    },
    "midi": {
      "enabled_inputs": ["Keyboard Controller", "Midi Device 2"],
      "enabled_outputs": ["Synth Out"],
      "clock_sync": true
    },
    "control_surfaces": [],
    "sync": {
      "clock_source": "Internal",
      "ltc_enabled": false
    }
  },
  "appearance": {
    "theme": "Fleet Dark",
    "ui_scale": 1.0,
    "arrangement": {
      "grid_line_intensity": 0.4,
      "clip_color_mode": "TrackAccent"
    },
    "piano_roll": {
      "show_key_guides": true
    },
    "mixer": {
      "meter_decay_db_per_sec": 24.0,
      "peak_hold_seconds": 3.0
    }
  },
  "editing": {
    "mouse": {
      "zoom_sensitivity": 1.0,
      "natural_scroll": false
    },
    "snap": {
      "snap_to_grid": true,
      "default_snap_value": "1/16"
    },
    "history": {
      "max_undo_steps": 100
    }
  },
  "recording": {
    "audio": {
      "format": "wav",
      "bit_depth": 24,
      "recording_path": ""
    },
    "metronome": {
      "enabled": false,
      "volume": 0.8,
      "sound_type": "Woodblock",
      "count_in_bars": 1
    }
  },
  "plugins": {
    "vst3": {
      "enabled": true,
      "paths": [
        "C:\\Program Files\\Common Files\\VST3",
        "C:\\Program Files (x86)\\Common Files\\VST3"
      ]
    },
    "clap": {
      "enabled": true,
      "paths": [
        "C:\\Program Files\\Common Files\\CLAP"
      ]
    },
    "scan": {
      "background_scan": true,
      "failed_plugins": []
    }
  }
}
```

---

## 2. Rust Deserialization Validation & Fallbacks

Since settings files can be modified by the user directly, the deserializer must handle invalid values gracefully.

### Deserialization Rules
1. **Fallback on Missing Key**: If a key does not exist, use the hardcoded default from the struct `Default` implementation (supported by `#[serde(default)]` annotation).
2. **Bounds Clamping on Out of Range**:
   - `general.project_defaults.tempo`: Must be clamped between `20.0` and `999.0` bpm.
   - `general.project_defaults.sample_rate`: Restricted to valid values (`44100`, `48000`, `88200`, `96000`, `192000`). Fall back to `48000` if invalid.
   - `general.project_defaults.buffer_size`: Must be a power of two between `32` and `4096`. Fall back to `256` if invalid.
   - `appearance.ui_scale`: Clamped between `0.5` and `2.5`.
3. **Corrupted File Fallback**: If the file is completely unparseable (e.g. malformed JSON syntax), rename it to `settings.json.backup` and generate a clean `settings.json` with system defaults.
