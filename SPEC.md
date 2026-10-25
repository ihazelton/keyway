# Keyway — Spec

A macOS app for remapping a Work Louder Micro (QMK/VIA keyboard) in plain language. Replaces VIA's UI. No firmware changes — talks the VIA raw HID protocol directly, so remaps persist on the board.

## Why

VIA forces users to write QMK keycode notation like `LGUI(KC_C)`. Keyway shows and accepts `cmd+c`. Better still: press-to-assign — tap the physical key, then press the shortcut you want it to emit.

## Target

- macOS, Apple Silicon
- Work Louder Micro (QMK firmware with VIA enabled)
- Personal-use app, unsigned, no App Store

## Core features (v1)

1. **HID layer** — connect to the Micro over the VIA raw HID protocol (usage page 0xFF60, usage 0x61). Read and write keycodes per layer/row/col. Reference implementation: VIA's open-source code (the-via/app).
2. **Keymap read/render** — read the full keymap (all layers) and render the board layout from the Micro's VIA JSON definition (from the-via/keyboards repo).
3. **Layer support (core requirement)** — data model is layer × row × col. Layer count read from the VIA definition (typically 4). UI: layer tabs above the layout. Transparent keys (`KC_TRNS`) rendered visually as fall-through, not as code.
4. **Plain-language keycode translation** — bidirectional table between QMK 16-bit keycodes and human notation:
   - `LGUI(KC_C)` ↔ `cmd+c`
   - `MO(1)` ↔ "hold for layer 2"
   - `LT(2, KC_SPC)` ↔ "tap: space / hold: layer 2"
   - `TG(3)` ↔ "toggle layer 3"
   - Media keys ↔ "play/pause", "volume up", etc.
5. **Press-to-assign flow**
   - Select physical key: listen mode — tap a key on the Micro; app identifies which switch fired. Active layer tab decides which layer is being edited.
   - Assign output: app captures the next keystroke on the Mac keyboard (e.g. literally press cmd+shift+4) and encodes it as a QMK keycode. Requires macOS Input Monitoring permission (one-time approval in System Settings).
   - Layer actions can't be expressed by keystroke, so the assign flow has a picker branch: capture a keystroke OR choose a layer action (momentary / toggle / tap-hold).
6. **Guardrails** — warn before removing the only key that reaches a layer (stranded-layer check).
7. **Write/persist** — changes written to the board via VIA protocol; survive replug and other machines.

8. **Lighting control** — the Micro's LEDs are currently controlled through VIA. To fully replace VIA, Keyway must support reading and writing lighting settings (brightness, effect, colour) via the VIA protocol's lighting commands. QMK exposes this through the same raw HID channel.

## Out of scope (v1)

- Macro sequences ("type my email") — QMK macro slots, separate feature later.
- Multi-keyboard support. Micro only.
- Signing/notarisation/distribution.

## Stack

Tauri: Rust backend (hidapi for HID, native key capture) + web frontend for the layout view. Alternative: pure Swift if Tauri fights back.

## Build order / milestones

1. **M1 — Plumbing:** connect to the Micro, dump current keymap to terminal. Proves the HID path before any UI. Expect macOS raw-HID quirks here (permissions, usage page filtering).
2. **M2 — Render:** layout view from VIA JSON definition, layer tabs, current assignments shown in plain language.
3. **M3 — Translation:** full keycode ↔ human notation table (mechanical; suitable for a lower model).
4. **M4 — Click-to-assign:** click a key in the UI, pick from keystroke capture or layer-action picker, write to board.
5. **M5 — Press-to-assign:** listen mode for physical key selection.
6. **M6 — Lighting:** read/write lighting settings (brightness, effect, colour) via VIA protocol.
7. **M7 — Ambient status light:** the board doubles as a status lamp for the owner's Claude Code workflow.
   - CLI mode: `keyway light --color red`, `keyway light --flash`, `keyway light --restore` — reuses the existing `via.rs` lighting commands. Lighting changes are NOT saved to EEPROM, so the user's saved settings survive; `--restore` re-reads them.
   - Claude Code hooks integration: `Notification` hook → flash (Claude waiting for input); `Stop` hook → green pulse (done); wrapped build command → red on non-zero exit.
   - Needs care around HID contention with the running GUI app (both can't hold the device simultaneously).
8. **M8 — Polish:** guardrails, transparent-key rendering, error states.

## Hardware testing note

Every milestone past M1 scaffolding needs a human with the Micro plugged in. The loop is: agent writes code → human runs it, taps keys, reports back.
