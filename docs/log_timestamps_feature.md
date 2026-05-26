# Log Timestamps Feature

## Overview

The `log_timestamps` feature adds timestamps to session log files in Blightmud. When enabled, each line in the log file is prefixed with a timestamp in `[HH:MM:SS]` format, making it easy to track when events occurred during your MUD session.

## Configuration

### Setting

| Setting Name | Type | Default | Description |
|---|---|---|---|
| `log_timestamps` | boolean | `false` | Enable/disable timestamps in session log files |

### How to Toggle

**Command line:**

```
/set log_timestamps              # Show current value
/set log_timestamps on           # Enable timestamps
/set log_timestamps off          # Disable timestamps
```

**Lua script:**

```lua
settings.set("log_timestamps", true)   -- Enable
settings.set("log_timestamps", false)  -- Disable
settings.get("log_timestamps")         -- Get current value
```

## Behavior

- When **enabled**, each log line is prefixed with `[HH:MM:SS]`:
  ```
  [14:32:05] The Great Hall
  [14:32:07] A guard salutes you.
  [14:32:10] > look
  ```
- When **disabled** (default), log lines have no prefix:
  ```
  The Great Hall
  A guard salutes you.
  > look
  ```
- The setting takes effect **immediately** — no restart required.
- Only new log lines are affected; existing lines in the log file remain unchanged.
- The timestamp uses your system's local time (24-hour format).

## Implementation Details

### Modified Files

| File | Change |
|---|---|
| `src/model/settings.rs` | Added `LOG_TIMESTAMPS` constant and default value |
| `src/io/logger.rs` | Added `timestamps` field to `Logger`, timestamp prefix in `log_str()`, `set_timestamps()` method |
| `src/session.rs` | Added `log_timestamps` to `SessionBuilder`, initializes `Logger` with setting |
| `src/lib.rs` | Handles `Event::SettingChanged` for `LOG_TIMESTAMPS`, passes setting to `SessionBuilder` |
| `resources/help/logging.md` | Added timestamps section |
| `resources/help/settings.md` | Added `log_timestamps` to settings list |
| `resources/completions.txt` | Added `log_timestamps` to tab completions |
| `resources/lua/types/blightmud.d.lua` | Updated known settings list |

### Data Flow

```
User toggles setting
       │
       ▼
settings.set("log_timestamps", true)
       │
       ▼
Event::SettingChanged("log_timestamps", true)
       │
       ▼
lib.rs: match LOG_TIMESTAMPS → logger.set_timestamps(true)
       │
       ▼
Logger.timestamps = true
       │
       ▼
Next log_str() call writes "[HH:MM:SS] " prefix
```
