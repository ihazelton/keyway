import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

interface KeyInfo {
  row: number;
  col: number;
  code: number;
  code_hex: string;
  label: string;
  is_encoder: boolean;
  encoder_id: number | null;
}

interface EncoderAction {
  code: number;
  code_hex: string;
  label: string;
}

interface EncoderInfo {
  id: number;
  cw: EncoderAction;
  ccw: EncoderAction;
}

interface LayerData {
  index: number;
  keys: KeyInfo[];
  encoders: EncoderInfo[];
}

interface ProbeResult {
  product_name: string;
  vendor_id: string;
  product_id: string;
  protocol_version: string;
  layer_count: number;
  layers: LayerData[];
}

// Key positions as percentages of the cropped SVG viewBox.
const VB_X = 300;
const VB_Y = 270;
const VB_W = 600;
const VB_H = 620;
const KEY_SIZE_PCT = (83 / VB_W) * 100;
const svgPos = (svgX: number, svgY: number) => ({
  x: ((svgX - VB_X) / VB_W) * 100,
  y: ((svgY - VB_Y) / VB_H) * 100,
});

const KEY_POSITIONS: { row: number; col: number; x: number; y: number }[] = [
  // Order matches layout_keys() in Rust
  { row: 0, col: 0, ...svgPos(414.5, 414.5) },   // encoder 0 in SVG (but may differ physically)
  { row: 0, col: 3, ...svgPos(701.5, 414.5) },   // encoder 1
  { row: 0, col: 1, ...svgPos(510.5, 414.5) },
  { row: 0, col: 2, ...svgPos(605.5, 414.5) },
  { row: 1, col: 0, ...svgPos(414.5, 509.5) },
  { row: 1, col: 1, ...svgPos(510.5, 509.5) },
  { row: 1, col: 2, ...svgPos(605.5, 509.5) },
  { row: 1, col: 3, ...svgPos(701.5, 509.5) },
  { row: 2, col: 0, ...svgPos(414.5, 605.5) },
  { row: 2, col: 1, ...svgPos(510.5, 605.5) },
  { row: 2, col: 2, ...svgPos(605.5, 605.5) },
  { row: 2, col: 3, ...svgPos(701.5, 605.5) },
  { row: 3, col: 0, ...svgPos(414.5, 701.5) },   // encoder in SVG (rx=41.5)
  { row: 3, col: 1, ...svgPos(510.5, 701.5) },
  { row: 3, col: 2, ...svgPos(605.5, 701.5) },
  { row: 3, col: 3, ...svgPos(701.5, 701.5) },
];

// Encoder labels: encoder 0 is horizontal (left/right), encoder 1 is vertical (up/down)
const ENCODER_LABELS: Record<number, { ccw: string; cw: string; icon: string }> = {
  0: { ccw: "\u21D0", cw: "\u21D2", icon: "\u21C6" },  // ⇐ ⇒ ⇆
  1: { ccw: "\u21D1", cw: "\u21D3", icon: "\u21BB" },  // ⇑ ⇓ ↻
};

// JS key → QMK basic keycode
const JS_TO_QMK: Record<string, number> = {
  KeyA: 0x04, KeyB: 0x05, KeyC: 0x06, KeyD: 0x07, KeyE: 0x08, KeyF: 0x09,
  KeyG: 0x0a, KeyH: 0x0b, KeyI: 0x0c, KeyJ: 0x0d, KeyK: 0x0e, KeyL: 0x0f,
  KeyM: 0x10, KeyN: 0x11, KeyO: 0x12, KeyP: 0x13, KeyQ: 0x14, KeyR: 0x15,
  KeyS: 0x16, KeyT: 0x17, KeyU: 0x18, KeyV: 0x19, KeyW: 0x1a, KeyX: 0x1b,
  KeyY: 0x1c, KeyZ: 0x1d,
  Digit1: 0x1e, Digit2: 0x1f, Digit3: 0x20, Digit4: 0x21, Digit5: 0x22,
  Digit6: 0x23, Digit7: 0x24, Digit8: 0x25, Digit9: 0x26, Digit0: 0x27,
  Enter: 0x28, Escape: 0x29, Backspace: 0x2a, Tab: 0x2b, Space: 0x2c,
  Minus: 0x2d, Equal: 0x2e, BracketLeft: 0x2f, BracketRight: 0x30,
  Backslash: 0x31, Semicolon: 0x33, Quote: 0x34, Backquote: 0x35,
  Comma: 0x36, Period: 0x37, Slash: 0x38, CapsLock: 0x39,
  F1: 0x3a, F2: 0x3b, F3: 0x3c, F4: 0x3d, F5: 0x3e, F6: 0x3f,
  F7: 0x40, F8: 0x41, F9: 0x42, F10: 0x43, F11: 0x44, F12: 0x45,
  PrintScreen: 0x46, ScrollLock: 0x47, Pause: 0x48,
  Insert: 0x49, Home: 0x4a, PageUp: 0x4b, Delete: 0x4c, End: 0x4d,
  PageDown: 0x4e, ArrowRight: 0x4f, ArrowLeft: 0x50, ArrowDown: 0x51, ArrowUp: 0x52,
};

const TEXT_TO_QMK: Record<string, number> = {
  a: 0x04, b: 0x05, c: 0x06, d: 0x07, e: 0x08, f: 0x09,
  g: 0x0a, h: 0x0b, i: 0x0c, j: 0x0d, k: 0x0e, l: 0x0f,
  m: 0x10, n: 0x11, o: 0x12, p: 0x13, q: 0x14, r: 0x15,
  s: 0x16, t: 0x17, u: 0x18, v: 0x19, w: 0x1a, x: 0x1b,
  y: 0x1c, z: 0x1d,
  "1": 0x1e, "2": 0x1f, "3": 0x20, "4": 0x21, "5": 0x22,
  "6": 0x23, "7": 0x24, "8": 0x25, "9": 0x26, "0": 0x27,
  enter: 0x28, return: 0x28, esc: 0x29, escape: 0x29,
  backspace: 0x2a, tab: 0x2b, space: 0x2c,
  "-": 0x2d, minus: 0x2d, "=": 0x2e, equal: 0x2e, equals: 0x2e,
  "[": 0x2f, "]": 0x30, "\\": 0x31, ";": 0x33, "'": 0x34,
  "`": 0x35, ",": 0x36, ".": 0x37, "/": 0x38,
  capslock: 0x39, "caps lock": 0x39,
  f1: 0x3a, f2: 0x3b, f3: 0x3c, f4: 0x3d, f5: 0x3e, f6: 0x3f,
  f7: 0x40, f8: 0x41, f9: 0x42, f10: 0x43, f11: 0x44, f12: 0x45,
  printscreen: 0x46, scrolllock: 0x47, pause: 0x48,
  insert: 0x49, home: 0x4a, pageup: 0x4b, delete: 0x4c, del: 0x4c,
  end: 0x4d, pagedown: 0x4e,
  right: 0x4f, left: 0x50, down: 0x51, up: 0x52,
  mute: 0xa8, "volume up": 0xa9, "vol up": 0xa9,
  "volume down": 0xaa, "vol down": 0xaa,
  "play/pause": 0xcd, play: 0xcd, "next track": 0xb5, "prev track": 0xb6, stop: 0xb7,
};

const MOD_NAMES: Record<string, number> = {
  ctrl: 0x01, control: 0x01, lctrl: 0x01,
  shift: 0x02, lshift: 0x02,
  alt: 0x04, opt: 0x04, option: 0x04, lalt: 0x04,
  cmd: 0x08, command: 0x08, gui: 0x08, win: 0x08, super: 0x08, lgui: 0x08,
  rctrl: 0x11, rshift: 0x12, ralt: 0x14, ropt: 0x14,
  rcmd: 0x18, rgui: 0x18,
};

// Selection target: either a key press or an encoder rotation
type SelectionTarget =
  | { type: "key"; keyIdx: number }
  | { type: "encoder"; encoderId: number; direction: "cw" | "ccw" };

function jsEventToQmk(e: KeyboardEvent): number | null {
  if (["Control", "Shift", "Alt", "Meta"].includes(e.key)) return null;
  const base = JS_TO_QMK[e.code];
  if (base === undefined) return null;
  const mods = (e.ctrlKey ? 0x01 : 0) | (e.shiftKey ? 0x02 : 0) | (e.altKey ? 0x04 : 0) | (e.metaKey ? 0x08 : 0);
  return mods === 0 ? base : (mods << 8) | base;
}

function parseTextKeycode(text: string): number | null {
  const raw = text.trim().toLowerCase();
  if (!raw) return null;
  const parts = raw.split("+").map((p) => p.trim());
  let mods = 0;
  let baseKey: number | null = null;
  for (const part of parts) {
    if (MOD_NAMES[part] !== undefined) { mods |= MOD_NAMES[part]; }
    else if (TEXT_TO_QMK[part] !== undefined) { baseKey = TEXT_TO_QMK[part]; }
    else { return null; }
  }
  if (baseKey === null) {
    if (mods === 0) return null;
    const singleModMap: Record<number, number> = {
      0x01: 0xe0, 0x02: 0xe1, 0x04: 0xe2, 0x08: 0xe3,
      0x11: 0xe4, 0x12: 0xe5, 0x14: 0xe6, 0x18: 0xe7,
    };
    return singleModMap[mods] ?? null;
  }
  return mods === 0 ? baseKey : (mods << 8) | baseKey;
}

function buildLayerKeycode(action: string, layer: number): number | null {
  switch (action) {
    case "MO": return 0x5100 | layer;
    case "TG": return 0x5300 | layer;
    case "DF": return 0x5200 | layer;
    case "TO": return 0x5800 | layer;
    case "OSL": return 0x5700 | layer;
    default: return null;
  }
}

interface LightingState {
  underglow_brightness: number;
  underglow_effect: number;
  underglow_speed: number;
  underglow_color: [number, number];
  backlight_brightness: number;
  backlight_effect: number;
  backlight_speed: number;
  backlight_color: [number, number];
  indicator_brightness: number;
}

const UNDERGLOW_EFFECTS = ["All Off","Solid Color","Breathing 1","Breathing 2","Breathing 3","Breathing 4","Rainbow Mood 1","Rainbow Mood 2","Rainbow Mood 3","Rainbow Swirl 1","Rainbow Swirl 2","Rainbow Swirl 3","Rainbow Swirl 4","Rainbow Swirl 5","Rainbow Swirl 6","Snake 1","Snake 2","Snake 3","Snake 4","Snake 5","Snake 6","Knight 1","Knight 2","Knight 3","Christmas","Gradient 1","Gradient 2","Gradient 3","Gradient 4","Gradient 5","Gradient 6","Gradient 7","Gradient 8","Gradient 9","Gradient 10","RGB Test","Alternating","Twinkle 1","Twinkle 2","Twinkle 3","Twinkle 4","Twinkle 5","Twinkle 6"];
const BACKLIGHT_EFFECTS = ["All Off","Solid Color","Alphas Mods","Gradient Up/Down","Gradient Left/Right","Breathing","Band Sat.","Band Val.","Pinwheel Sat.","Pinwheel Val.","Spiral Sat.","Spiral Val.","Cycle All","Cycle Left/Right","Cycle Up/Down","Rainbow Moving Chevron","Cycle Out/In","Cycle Out/In Dual","Cycle Pinwheel","Cycle Spiral","Dual Beacon","Rainbow Beacon","Rainbow Pinwheels","Raindrops","Jellybean Raindrops","Hue Breathing","Hue Pendulum","Hue Wave","Pixel Rain","Pixel Flow","Pixel Fractal","Typing Heatmap","Digital Rain"];

let state: ProbeResult | null = null;
let activeLayer = 0;
let selection: SelectionTarget | null = null;
let captureMode = false;
let captureHandler: ((e: KeyboardEvent) => void) | null = null;
let pendingKeycode: number | null = null;
let pendingLabel = "";
let pressedKeys: Set<string> = new Set();
let pollTimer: ReturnType<typeof setInterval> | null = null;
let sidebarTab: "keys" | "lighting" = "keys";
let lighting: LightingState | null = null;

function getSelectedInfo(): { label: string; code_hex: string; description: string } | null {
  if (!selection || !state) return null;
  const layer = state.layers[activeLayer];
  if (selection.type === "key") {
    const key = layer.keys[selection.keyIdx];
    const desc = key.is_encoder ? `Encoder ${key.encoder_id} press` : `Key (row ${key.row}, col ${key.col})`;
    return { label: key.label, code_hex: key.code_hex, description: desc };
  } else {
    const sel = selection;
    const enc = layer.encoders.find((e) => e.id === sel.encoderId);
    if (!enc) return null;
    const act = selection.direction === "cw" ? enc.cw : enc.ccw;
    const dirLabel = selection.encoderId === 0
      ? (selection.direction === "cw" ? "rotate right" : "rotate left")
      : (selection.direction === "cw" ? "rotate down" : "rotate up");
    return { label: act.label, code_hex: act.code_hex, description: `Encoder ${selection.encoderId} ${dirLabel}` };
  }
}

function render() {
  const app = document.querySelector<HTMLDivElement>("#app")!;

  if (!state) {
    app.innerHTML = `
      <h1>Keyway</h1>
      <p class="subtitle">Remap your Work Louder Micro in plain language</p>
      <div class="status-bar">
        <span class="status-dot"></span>
        <span class="info">No keyboard connected</span>
        <button id="probe-btn">Connect</button>
      </div>
      <div id="error-area"></div>
    `;
    app.querySelector("#probe-btn")!.addEventListener("click", probe);
    return;
  }

  const tabsHtml = state.layers
    .map((l) =>
      `<button class="layer-tab ${l.index === activeLayer ? "active" : ""}" data-layer="${l.index}">Layer ${l.index}</button>`
    )
    .join("");

  const layer = state.layers[activeLayer];

  const keysHtml = layer.keys
    .map((key, i) => {
      const pos = KEY_POSITIONS[i];
      const isSelected = selection?.type === "key" && selection.keyIdx === i;
      const isPressed = pressedKeys.has(`${key.row},${key.col}`);
      const classes = [
        "key",
        key.is_encoder ? "encoder" : "",
        key.label === "transparent" ? "transparent" : "",
        isSelected ? "selected" : "",
        isPressed ? "pressed" : "",
      ].filter(Boolean).join(" ");

      const displayLabel = key.label === "transparent" ? "\u2193" : key.label;

      let rotationHtml = "";
      if (key.is_encoder && key.encoder_id !== null) {
        const enc = layer.encoders.find((e) => e.id === key.encoder_id);
        if (enc) {
          const labels = ENCODER_LABELS[key.encoder_id] || { ccw: "CCW", cw: "CW" };
          const ccwSel = selection?.type === "encoder" && selection.encoderId === key.encoder_id && selection.direction === "ccw";
          const cwSel = selection?.type === "encoder" && selection.encoderId === key.encoder_id && selection.direction === "cw";

          // Encoder 0: both labels stacked on LEFT of board
          // Encoder 1: both labels stacked on RIGHT of board
          const side = key.encoder_id === 0 ? "off-left" : "off-right";
          rotationHtml = `
            <div class="enc-label ${side} ${side}-top ${ccwSel ? "enc-selected" : ""}" data-enc-id="${key.encoder_id}" data-enc-dir="ccw" title="${enc.ccw.label}">
              <span class="enc-arrow">${labels.ccw}</span>
              <span class="enc-action">${enc.ccw.label}</span>
            </div>
            <div class="enc-label ${side} ${side}-bottom ${cwSel ? "enc-selected" : ""}" data-enc-id="${key.encoder_id}" data-enc-dir="cw" title="${enc.cw.label}">
              <span class="enc-arrow">${labels.cw}</span>
              <span class="enc-action">${enc.cw.label}</span>
            </div>`;

        }
      }

      // Clean up display labels
      let keyLabel = displayLabel;
      if (key.is_encoder) {
        keyLabel = key.label;
      }
      // Hide custom keycodes (can't be changed via VIA)
      if (keyLabel.startsWith("custom")) keyLabel = "";
      // Simplify layer labels
      keyLabel = keyLabel
        .replace("default layer ", "layer ")
        .replace("hold for layer ", "layer ")
        .replace("toggle layer ", "layer ")
        .replace("go to layer ", "layer ")
        .replace("one-shot layer ", "layer ");

      const KEY_H_PCT = (83 / VB_H) * 100;
      return `<div class="key-wrapper" style="left:${pos.x}%;top:${pos.y}%;width:${KEY_SIZE_PCT}%;height:${KEY_H_PCT}%">
        ${rotationHtml}
        <div class="${classes}" data-key-idx="${i}">
          <span class="label">${keyLabel}</span>
        </div>
      </div>`;
    })
    .join("");

  const selInfo = getSelectedInfo();
  const assignPanelHtml = selInfo ? renderAssignPanel(selInfo) : "";

  app.innerHTML = `
    <h1>Keyway</h1>
    <div class="status-bar">
      <span class="status-dot connected"></span>
      <span class="info"><strong>${state.product_name}</strong> &middot; VIA v${state.protocol_version} &middot; ${state.layer_count} layers</span>
      <button id="probe-btn">Refresh</button>
    </div>
    <div class="main-layout">
      <div class="board-side">
        <div class="layer-tabs">${tabsHtml}</div>
        <div class="board-outer">
          <div class="board-container">
            <img src="/board.svg" class="board-svg" alt="Creator Micro" draggable="false" />
            <div class="layout-container">${keysHtml}</div>
          </div>
        </div>
      </div>
      <div class="sidebar">
        <div class="sidebar-tabs">
          <button class="sidebar-tab ${sidebarTab === "keys" ? "active" : ""}" data-sidebar="keys">Keys</button>
          <button class="sidebar-tab ${sidebarTab === "lighting" ? "active" : ""}" data-sidebar="lighting">Lighting</button>
        </div>
        <div class="sidebar-content">
          ${sidebarTab === "keys"
            ? (assignPanelHtml || '<div class="sidebar-empty">Select a key to assign</div>')
            : renderLightingPanel()}
        </div>
        <div id="error-area"></div>
      </div>
    </div>
  `;

  // Event bindings
  app.querySelector("#probe-btn")!.addEventListener("click", probe);

  app.querySelectorAll(".layer-tab").forEach((tab) => {
    tab.addEventListener("click", () => {
      activeLayer = Number((tab as HTMLElement).dataset.layer);
      selection = null;
      stopCapture();
      render();
    });
  });

  app.querySelectorAll(".key").forEach((el) => {
    el.addEventListener("click", (e) => {
      e.stopPropagation();
      const idx = Number((el as HTMLElement).dataset.keyIdx);
      if (selection?.type === "key" && selection.keyIdx === idx) {
        selection = null;
      } else {
        selection = { type: "key", keyIdx: idx };
      }
      stopCapture();
      render();
    });
  });

  app.querySelectorAll(".enc-label").forEach((el) => {
    el.addEventListener("click", (e) => {
      e.stopPropagation();
      const encId = Number((el as HTMLElement).dataset.encId);
      const dir = (el as HTMLElement).dataset.encDir as "cw" | "ccw";
      if (selection?.type === "encoder" && selection.encoderId === encId && selection.direction === dir) {
        selection = null;
      } else {
        selection = { type: "encoder", encoderId: encId, direction: dir };
      }
      stopCapture();
      render();
    });
  });

  app.addEventListener("click", (e) => {
    if (!(e.target as HTMLElement).closest(".key, .enc-label, .assign-panel")) {
      selection = null;
      stopCapture();
      render();
    }
  });

  app.querySelectorAll(".sidebar-tab").forEach((tab) => {
    tab.addEventListener("click", () => {
      stopCapture(); // never leave capture armed on a tab with no Done button
      sidebarTab = (tab as HTMLElement).dataset.sidebar as "keys" | "lighting";
      if (sidebarTab === "lighting") {
        stopPolling();
        if (!lighting) loadLighting();
        else render();
      } else {
        startPolling();
        render();
      }
    });
  });

  bindLightingEvents(app);

  const captureBtn = app.querySelector("#capture-btn");
  if (captureBtn) captureBtn.addEventListener("click", () => {
    if (captureMode) finishCapture();
    else startCapture();
  });

  const clearBtn = app.querySelector("#clear-btn");
  if (clearBtn) clearBtn.addEventListener("click", () => assignKeycode(0x0000));

  const textForm = app.querySelector("#text-input-form") as HTMLFormElement | null;
  if (textForm) {
    const textInput = textForm.querySelector("#text-input") as HTMLInputElement | null;
    if (textInput) {
      // Focusing the manual box cancels an armed capture so typing isn't grabbed.
      textInput.addEventListener("focus", () => {
        if (!captureMode) return;
        stopCapture();
        const btn = document.querySelector<HTMLButtonElement>("#capture-btn");
        if (btn) { btn.classList.remove("capturing"); btn.textContent = "Capture keystroke"; }
        document.querySelector("#capture-preview")?.remove();
      });
    }
    textForm.addEventListener("submit", (e) => {
      e.preventDefault();
      const input = textForm.querySelector("#text-input") as HTMLInputElement;
      const keycode = parseTextKeycode(input.value);
      if (keycode !== null) {
        assignKeycode(keycode);
      } else {
        const errorArea = document.querySelector<HTMLDivElement>("#error-area");
        if (errorArea) errorArea.innerHTML = `<div class="error">Could not parse "${input.value}". Use format like: ctrl+cmd+q, cmd+shift+4, space, f5</div>`;
      }
    });
  }

  const layerForm = app.querySelector("#layer-action-form") as HTMLFormElement | null;
  if (layerForm) {
    layerForm.addEventListener("submit", (e) => {
      e.preventDefault();
      const action = (layerForm.querySelector("#layer-action") as HTMLSelectElement).value;
      const layerNum = Number((layerForm.querySelector("#layer-num") as HTMLSelectElement).value);
      const keycode = buildLayerKeycode(action, layerNum);
      if (keycode !== null) assignKeycode(keycode);
    });
  }
}

function renderAssignPanel(info: { label: string; code_hex: string; description: string }): string {
  const layerOptions = state!.layers
    .map((l) => `<option value="${l.index}">Layer ${l.index}</option>`)
    .join("");

  return `
    <div class="assign-panel">
      <div class="assign-header">
        <strong>Assign: ${info.description}</strong> — currently: <code>${info.label}</code>
      </div>
      <div class="assign-body">
        <div class="assign-section">
          <div class="assign-section-title">Keystroke</div>
          <div class="keystroke-row">
            <button id="capture-btn" class="assign-btn ${captureMode ? "capturing" : ""}">
              ${captureMode ? "Done" : "Capture keystroke"}
            </button>
            <span class="or-label">or type it:</span>
            <form id="text-input-form" class="text-input-row">
              <input id="text-input" type="text" placeholder="e.g. ctrl+cmd+q" spellcheck="false" autocomplete="off" />
              <button type="submit" class="assign-btn">Set</button>
            </form>
          </div>
          ${captureMode ? `<div id="capture-preview" class="capture-preview">${
            pendingKeycode !== null
              ? `Captured: <strong>${pendingLabel}</strong> \u2014 click Done to apply`
              : "Recording\u2026 press your key or combo"
          }</div>` : ""}
        </div>
        <div class="assign-section">
          <div class="assign-section-title">Layer action</div>
          <form id="layer-action-form" class="layer-action-row">
            <select id="layer-action">
              <option value="MO">Hold for layer</option>
              <option value="TG">Toggle layer</option>
              <option value="DF">Default layer</option>
              <option value="TO">Go to layer</option>
              <option value="OSL">One-shot layer</option>
            </select>
            <select id="layer-num">${layerOptions}</select>
            <button type="submit" class="assign-btn">Set</button>
          </form>
        </div>
        <div class="assign-section">
          <button id="clear-btn" class="assign-btn danger">Clear (none)</button>
        </div>
      </div>
    </div>
  `;
}

function renderLightingPanel(): string {
  if (!lighting) return '<div class="sidebar-empty">Loading lighting...</div>';

  const ugEffectOpts = UNDERGLOW_EFFECTS.map((e, i) =>
    `<option value="${i}" ${i === lighting!.underglow_effect ? "selected" : ""}>${e}</option>`
  ).join("");

  const blEffectOpts = BACKLIGHT_EFFECTS.map((e, i) =>
    `<option value="${i}" ${i === lighting!.backlight_effect ? "selected" : ""}>${e}</option>`
  ).join("");

  return `
    <div class="lighting-panel">
      <div class="lighting-section">
        <div class="lighting-title">Underglow</div>
        <label>Brightness<input type="range" min="0" max="255" value="${lighting.underglow_brightness}" data-light="ug-brightness" /></label>
        <label>Effect<select data-light="ug-effect">${ugEffectOpts}</select></label>
        ${lighting.underglow_effect > 0 ? `
          <label>Speed<input type="range" min="0" max="3" value="${lighting.underglow_speed}" data-light="ug-speed" /></label>
          <label>Hue<input type="range" min="0" max="255" value="${lighting.underglow_color[0]}" data-light="ug-hue" class="hue-slider" /></label>
          <label>Saturation<input type="range" min="0" max="255" value="${lighting.underglow_color[1]}" data-light="ug-sat" /></label>
        ` : ""}
      </div>
      <div class="lighting-section">
        <div class="lighting-title">Backlight</div>
        <label>Brightness<input type="range" min="0" max="255" value="${lighting.backlight_brightness}" data-light="bl-brightness" /></label>
        <label>Effect<select data-light="bl-effect">${blEffectOpts}</select></label>
        ${lighting.backlight_effect > 0 ? `
          <label>Speed<input type="range" min="0" max="255" value="${lighting.backlight_speed}" data-light="bl-speed" /></label>
          <label>Hue<input type="range" min="0" max="255" value="${lighting.backlight_color[0]}" data-light="bl-hue" class="hue-slider" /></label>
          <label>Saturation<input type="range" min="0" max="255" value="${lighting.backlight_color[1]}" data-light="bl-sat" /></label>
        ` : ""}
      </div>
      <div class="lighting-section">
        <div class="lighting-title">Indicators</div>
        <label>Brightness<input type="range" min="0" max="5" value="${lighting.indicator_brightness}" data-light="ind-brightness" /></label>
      </div>
    </div>
  `;
}

let lightDebounce: ReturnType<typeof setTimeout> | null = null;

function bindLightingEvents(app: HTMLElement) {
  app.querySelectorAll<HTMLElement>("[data-light]").forEach((el) => {
    const key = el.dataset.light!;
    const isRange = el.getAttribute("type") === "range";

    if (isRange) {
      el.addEventListener("input", () => {
        if (lightDebounce) clearTimeout(lightDebounce);
        lightDebounce = setTimeout(() => applyLight(key, (el as HTMLInputElement).value), 100);
      });
      el.addEventListener("change", async () => {
        await applyLight(key, (el as HTMLInputElement).value);
        invoke("save_lighting").catch(() => {});
      });
    } else {
      el.addEventListener("change", async () => {
        await applyLight(key, (el as HTMLInputElement).value);
        invoke("save_lighting").catch(() => {});
      });
    }
  });
}

async function applyLight(key: string, val: string) {
  try {
    switch (key) {
      case "ug-brightness": await setLight(2, 1, [Number(val)]); lighting!.underglow_brightness = Number(val); break;
      case "ug-effect": await setLight(2, 2, [Number(val)]); lighting!.underglow_effect = Number(val); setTimeout(render, 50); break;
      case "ug-speed": await setLight(2, 3, [Number(val)]); lighting!.underglow_speed = Number(val); break;
      case "ug-hue": lighting!.underglow_color[0] = Number(val); await setLight(2, 4, lighting!.underglow_color as unknown as number[]); break;
      case "ug-sat": lighting!.underglow_color[1] = Number(val); await setLight(2, 4, lighting!.underglow_color as unknown as number[]); break;
      case "bl-brightness": await setLight(3, 1, [Number(val)]); lighting!.backlight_brightness = Number(val); break;
      case "bl-effect": await setLight(3, 2, [Number(val)]); lighting!.backlight_effect = Number(val); setTimeout(render, 50); break;
      case "bl-speed": await setLight(3, 3, [Number(val)]); lighting!.backlight_speed = Number(val); break;
      case "bl-hue": lighting!.backlight_color[0] = Number(val); await setLight(3, 4, lighting!.backlight_color as unknown as number[]); break;
      case "bl-sat": lighting!.backlight_color[1] = Number(val); await setLight(3, 4, lighting!.backlight_color as unknown as number[]); break;
      case "ind-brightness": await setLight(0, 1, [Number(val)]); lighting!.indicator_brightness = Number(val); break;
    }
  } catch (e) {
    const errorArea = document.querySelector<HTMLDivElement>("#error-area");
    if (errorArea) errorArea.innerHTML = `<div class="error">${e}</div>`;
  }
}

async function setLight(channel: number, valueId: number, value: number[]) {
  await invoke("set_lighting_value", { channel, valueId, value });
}

async function loadLighting() {
  try {
    lighting = await invoke<LightingState>("get_lighting");
    render();
  } catch (e) {
    const errorArea = document.querySelector<HTMLDivElement>("#error-area");
    if (errorArea) errorArea.innerHTML = `<div class="error">${e}</div>`;
  }
}

// Serialise arm/disarm so rapid toggles apply in submission order — otherwise
// two threadpool-dispatched invokes could land reversed and leave capture stuck.
let captureSync: Promise<unknown> = Promise.resolve();
function syncCaptureFlag(enabled: boolean) {
  captureSync = captureSync.then(() => invoke("set_capture", { enabled }).catch(() => {}));
}

function startCapture() {
  captureMode = true;
  pendingKeycode = null;
  pendingLabel = "";
  syncCaptureFlag(true);
  render();
  // JS fallback: catches ordinary keys if the native monitor isn't active.
  // Cmd/Ctrl combos never reach here — the native monitor handles those.
  captureHandler = (e: KeyboardEvent) => {
    const t = e.target as HTMLElement | null;
    // Never hijack typing in a text field — that's the manual-entry fallback.
    if (t && (t.tagName === "INPUT" || t.tagName === "TEXTAREA" || t.isContentEditable)) {
      stopCapture();
      render();
      return;
    }
    e.preventDefault();
    e.stopPropagation();
    const keycode = jsEventToQmk(e);
    if (keycode !== null) setPending(keycode, eventLabel(e));
  };
  window.addEventListener("keydown", captureHandler, true);
}

function stopCapture() {
  if (!captureMode) return;
  captureMode = false;
  pendingKeycode = null;
  pendingLabel = "";
  syncCaptureFlag(false);
  if (captureHandler) {
    window.removeEventListener("keydown", captureHandler, true);
    captureHandler = null;
  }
}

// Record a captured keystroke without applying it — the user clicks Done to apply.
function setPending(keycode: number, label: string) {
  if (!captureMode) return;
  pendingKeycode = keycode;
  pendingLabel = label;
  updateCapturePreview();
}

// Apply the captured keystroke (if any) and leave capture mode.
function finishCapture() {
  const kc = pendingKeycode;
  stopCapture();
  if (kc !== null) assignKeycode(kc);
  else render();
}

function updateCapturePreview() {
  const el = document.querySelector<HTMLDivElement>("#capture-preview");
  if (!el) return;
  el.innerHTML = pendingKeycode !== null
    ? `Captured: <strong>${pendingLabel}</strong> — click Done to apply`
    : "Recording… press your key or combo";
}

// Build a quick preview label from a JS keyboard event (fallback path only).
function eventLabel(e: KeyboardEvent): string {
  const parts: string[] = [];
  if (e.ctrlKey) parts.push("⌃");
  if (e.altKey) parts.push("⌥");
  if (e.shiftKey) parts.push("⇧");
  if (e.metaKey) parts.push("⌘");
  parts.push(e.key.length === 1 ? e.key.toUpperCase() : e.key);
  return parts.join("");
}

async function assignKeycode(keycode: number) {
  if (!selection || !state) return;
  const errorArea = document.querySelector<HTMLDivElement>("#error-area");

  try {
    if (selection.type === "key") {
      const key = state.layers[activeLayer].keys[selection.keyIdx];
      await invoke("set_key", { layer: activeLayer, row: key.row, col: key.col, keycode });
    } else {
      const dir = selection.direction === "cw" ? 0 : 1;
      await invoke("set_encoder", {
        layer: activeLayer,
        encoderId: selection.encoderId,
        direction: dir,
        keycode,
      });
    }
    await probe();
  } catch (e) {
    if (errorArea) {
      errorArea.innerHTML = `<div class="error">${e}</div>`;
    }
  }
}

async function probe() {
  const btn = document.querySelector<HTMLButtonElement>("#probe-btn");
  const errorArea = document.querySelector<HTMLDivElement>("#error-area");
  if (btn) { btn.disabled = true; btn.textContent = "Connecting\u2026"; }
  if (errorArea) errorArea.innerHTML = "";

  try {
    const prevSelection = selection;
    state = await invoke<ProbeResult>("probe_keyboard");
    selection = prevSelection;
    if (activeLayer >= state.layer_count) activeLayer = 0;
    render();
    startPolling();
  } catch (e) {
    if (errorArea) errorArea.innerHTML = `<div class="error">${e}</div>`;
    if (btn) { btn.disabled = false; btn.textContent = state ? "Refresh" : "Connect"; }
  }
}

function startPolling() {
  if (pollTimer) return;
  pollTimer = setInterval(async () => {
    if (!state) return;
    try {
      const result = await invoke<{ keys: [number, number][] }>("get_pressed_keys");
      const newPressed = new Set(result.keys.map(([r, c]) => `${r},${c}`));
      // Only update DOM if state changed
      const changed = newPressed.size !== pressedKeys.size ||
        [...newPressed].some((k) => !pressedKeys.has(k));
      if (changed) {
        pressedKeys = newPressed;
        // Update classes directly without full re-render
        document.querySelectorAll<HTMLElement>(".key").forEach((el) => {
          const idx = Number(el.dataset.keyIdx);
          if (isNaN(idx) || !state) return;
          const key = state.layers[activeLayer]?.keys[idx];
          if (!key) return;
          const isPressed = pressedKeys.has(`${key.row},${key.col}`);
          el.classList.toggle("pressed", isPressed);
          // Auto-select pressed key — but never retarget while capturing,
          // or a stray Micro press would steal the pending assignment.
          if (isPressed && selection?.type !== "key" && !captureMode) {
            selection = { type: "key", keyIdx: idx };
            render();
          }
        });
      }
    } catch {
      // Device disconnected or busy — ignore
    }
  }, 50);
}

function stopPolling() {
  if (pollTimer) {
    clearInterval(pollTimer);
    pollTimer = null;
  }
}

window.addEventListener("DOMContentLoaded", () => render());

// Native macOS key monitor forwards captured keystrokes (incl. Cmd/Ctrl combos).
listen<{ keycode: number; label: string }>("captured-keystroke", (e) => {
  setPending(e.payload.keycode, e.payload.label);
});
