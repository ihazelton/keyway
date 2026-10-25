# Keyway ambient status light — Claude Code hooks

Use Keyway's CLI to make the Work Louder Micro act as a status lamp during Claude Code sessions.

## Setup

### 1. Build the release binary

```bash
cd src-tauri && cargo build --release
```

Binary lands at `src-tauri/target/release/keyway`. Optionally symlink:

```bash
ln -sf "$(pwd)/src-tauri/target/release/keyway" /usr/local/bin/keyway
```

### 2. Configure Claude Code hooks

Add the following to `~/.claude/settings.json` (create the file if it doesn't exist). Adjust the path if you didn't symlink.

```json
{
  "hooks": {
    "Notification": [
      {
        "hooks": [
          {
            "type": "command",
            "command": "(keyway light --flash --color amber --duration 30 && keyway light --color amber) 2>/dev/null || true",
            "async": true,
            "timeout": 45
          }
        ]
      }
    ],
    "Stop": [
      {
        "hooks": [
          {
            "type": "command",
            "command": "keyway light --color green 2>/dev/null || true",
            "async": true,
            "timeout": 10
          }
        ]
      }
    ],
    "UserPromptSubmit": [
      {
        "hooks": [
          {
            "type": "command",
            "command": "keyway light --restore 2>/dev/null || true",
            "async": true,
            "timeout": 10
          }
        ]
      }
    ],
    "PostToolUseFailure": [
      {
        "matcher": "Bash",
        "hooks": [
          {
            "type": "command",
            "command": "keyway light --flash --color red --duration 300 2>/dev/null || true",
            "async": true,
            "timeout": 310
          }
        ]
      }
    ]
  }
}
```

**What this does:**

- **Notification** (Claude is waiting for input) — board flashes amber for 30s, then holds solid amber.
- **Stop** (Claude finishes a response) — board turns solid green.
- **UserPromptSubmit** (you send a new prompt) — board restores to your saved lighting, interrupting any running flash.
- **PostToolUseFailure** on Bash (a command/build fails) — board flashes red for up to 5 minutes, or until you submit a prompt.

All hooks are `async` so they never delay Claude, and errors are suppressed (`|| true`) so a busy HID device (e.g. Keyway GUI open) fails silently rather than breaking your session.

### 3. Manual usage

```bash
# Set a colour
keyway light --color red
keyway light --color green
keyway light --color blue
keyway light --color amber
keyway light --color 170        # raw hue 0-255

# Flash for attention (default 10s, then auto-restores)
keyway light --flash
keyway light --flash --color blue
keyway light --flash --color red --duration 300   # flash for 5 minutes

# Restore saved lighting
keyway light --restore
```

The first `--color` or `--flash` call snapshots the current lighting state to `~/.keyway/lighting-snapshot.json`. Subsequent calls do not overwrite the snapshot, so chaining `--color amber` then `--color green` still lets `--restore` return to the original state.

`--restore` re-applies the snapshot and deletes it. If no snapshot exists, it exits with an error (exit code 1).

**Interrupting a flash:** a running flash holds the HID device exclusively, so `--restore` cannot reach the board directly. Instead it writes `~/.keyway/flash-stop`; the flash loop notices within one cycle (400ms), stops, restores the snapshot itself, and cleans up. This means a long error flash (e.g. `--duration 300`) ends the moment you submit a new prompt via the UserPromptSubmit hook.

## HID contention

macOS enforces exclusive access to HID devices. The CLI opens a short-lived connection, which means:

- **GUI closed:** CLI works without issues.
- **GUI open:** CLI will fail with "exclusive access and device already open". Close the Keyway GUI before using the CLI, or quit the GUI when running Claude Code sessions with hooks enabled.

This is a macOS IOKit limitation, not a Keyway bug. Both VIA and Keyway's GUI hold the device exclusively. A future version could route CLI commands through the running GUI's IPC, but that is out of scope for M7.

## Colour names

red, orange, amber, yellow, green, cyan, blue, purple, magenta, white — or any integer 0-255 for a raw QMK hue value.
