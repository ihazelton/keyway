# Keyway

Remap a Work Louder Micro in plain language: `cmd+c`, not `LGUI(KC_C)`.

Keyway is a macOS app that replaces VIA's interface. It speaks the VIA raw HID
protocol directly, so remaps are saved on the board itself and need no
firmware changes. Tap a key on the board, press the shortcut you want it to
send, done.

It also turns the board into a status light for Claude Code: amber when Claude
needs you, green when it's done, red when something it ran has failed.

The story behind it: [Repurposing Figma hardware](https://www.ianhazelton.com/work/figma-hardware).

## What it does

- Shows the board's keymap, every layer, in plain language
- Assigns a key by pressing the shortcut on your Mac keyboard, typing it, or
  picking a layer action (hold, toggle, tap or hold)
- Maps the two encoders' turns
- Finds a key on screen when you press it on the board
- Controls the lighting: underglow, backlight and indicators
- `keyway light`, a command-line status light for Claude Code hooks
  (see [hooks/README.md](hooks/README.md))

## Requirements

- macOS on Apple Silicon
- A Work Louder Micro running QMK with VIA enabled
- [Rust](https://rustup.rs), Node 20+ and [pnpm](https://pnpm.io)

## Run it

```bash
pnpm install
pnpm tauri dev
```

Build the app:

```bash
pnpm tauri build
```

The first time you capture a shortcut, macOS asks for Input Monitoring
permission (System Settings → Privacy & Security).

Close VIA (or any other app holding the board) first: macOS lets only one app
talk to the board at a time.

## Status

Personal project, unsigned, not on the App Store. Remapping, layers, encoders
and lighting all work on hardware. Still to do: a warning before you remove the
only key that reaches a layer, clearer see-through keys, and better error
messages. Scope and build order are in [SPEC.md](SPEC.md).

Not affiliated with Work Louder or Figma.
