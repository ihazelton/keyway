# Keyway — agent context

Read SPEC.md first. It is the source of truth for scope and build order.

## What this is

macOS (Apple Silicon) app that remaps a Work Louder Micro keyboard in plain language, replacing VIA's UI. Talks the VIA raw HID protocol directly — no firmware changes. Layer support is a core requirement, not an add-on.

## Working agreements

- UK English. No emoji. Keep explanations brief: the point, not the preamble.
- Ask before big architectural decisions; don't ask about small implementation details.
- Correct the owner rather than guessing what they meant.
- Personal project. Do not reference or reuse anything from day-job work.

## Technical anchors

- Stack: Tauri (Rust backend + web frontend). hidapi for HID.
- VIA raw HID: usage page 0xFF60, usage 0x61. Protocol reference: github.com/the-via/app (open source).
- Board definition: Work Louder Micro VIA JSON from github.com/the-via/keyboards — vendor/product ID, matrix, layout, layer count live there.
- Keycodes are 16-bit QMK codes. The plain-language translation table is bidirectional and must round-trip.
- Press-to-assign keystroke capture needs macOS Input Monitoring permission.

## Hardware loop

The agent cannot see the keyboard. Milestone M1 (SPEC.md) is: connect and dump the keymap to terminal. Do not build UI until M1 passes on real hardware. When hardware behaviour is unknown, write a small probe script and ask the owner to run it and paste output.

## Current status

- [x] M1–M6 done and verified on hardware. Keymap read/render, plain-language translation, click-to-assign (capture/type/layer actions), encoder rotation mapping, press-to-identify highlighting, lighting controls (underglow/backlight/indicators).
- [x] M7 — ambient status light done. `keyway light` CLI + hooks installed in ~/.claude/settings.json (60s amber flash on Notification, green on Stop, 300s red flash on Bash failure, restore on prompt submit — interruptible via ~/.keyway/flash-stop marker).
- [ ] M8 — polish: guardrails (stranded-layer check), transparent-key rendering, error states.

## Hardware lessons learnt (do not rediscover these)

- The Micro's matrix is 4×4 (16 keys incl. 2 encoders), not 1×4. VIA definition: `src-tauri/src/micro.json`.
- Keep ONE persistent HID connection (Tauri managed state + Mutex). Opening per-command causes hangs/beachballs.
- Never save to EEPROM on slider drag — debounce writes, save on release only.
- `<input type="color">` does not work in WKWebView — use hue/sat range sliders.
- Webview keystroke capture can't see macOS system shortcuts (e.g. ctrl+cmd+q) — text input fallback exists for those.
- Pause switch-matrix polling while lighting tab is active (HID contention).
