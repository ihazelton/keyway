/// Bidirectional QMK 16-bit keycode ↔ plain-language translation.
///
/// QMK keycode structure (16-bit):
///   0x0000–0x00FF  Basic keycodes (HID usage codes)
///   0x0100–0x1FFF  Modifier combos: mods<<8 | keycode
///   0x4000–0x4FFF  Layer-tap: LT(layer, kc) = 0x4000 | (layer<<8) | kc
///   0x5100–0x51FF  Momentary layer: MO(n) = 0x5100 | n
///   0x5200–0x52FF  Default layer: DF(n) = 0x5200 | n
///   0x5300–0x53FF  Toggle layer: TG(n) = 0x5300 | n
///   0x5700–0x57FF  One-shot layer: OSL(n) = 0x5700 | n
///   0x5800–0x58FF  Layer on/off: TO(n) = 0x5800 | n
///   0x7800–0x7FFF  Custom keycodes: 0x7800 + n

/// Convert a 16-bit QMK keycode to human-readable text.
pub fn keycode_to_human(code: u16) -> String {
    // KC_NO / KC_TRNS
    if code == 0x0000 {
        return "none".to_string();
    }
    if code == 0x0001 {
        return "transparent".to_string();
    }

    // Custom keycodes (0x7800+)
    if code >= 0x7800 && code <= 0x7FFF {
        let idx = (code - 0x7800) as usize;
        return custom_keycode_name(idx);
    }

    // Layer-tap: LT(layer, kc) — 0x4000 | (layer << 8) | kc
    if code & 0xF000 == 0x4000 {
        let layer = ((code >> 8) & 0x0F) as u8;
        let kc = (code & 0xFF) as u16;
        let base = basic_keycode_name(kc);
        return format!("tap: {} / hold: layer {}", base, layer);
    }

    // MO(n)
    if code & 0xFF00 == 0x5100 {
        let layer = code & 0xFF;
        return format!("hold for layer {}", layer);
    }

    // DF(n)
    if code & 0xFF00 == 0x5200 {
        let layer = code & 0xFF;
        return format!("default layer {}", layer);
    }

    // TG(n)
    if code & 0xFF00 == 0x5300 {
        let layer = code & 0xFF;
        return format!("toggle layer {}", layer);
    }

    // OSL(n)
    if code & 0xFF00 == 0x5700 {
        let layer = code & 0xFF;
        return format!("one-shot layer {}", layer);
    }

    // TO(n)
    if code & 0xFF00 == 0x5800 {
        let layer = code & 0xFF;
        return format!("go to layer {}", layer);
    }

    // Mod-tap: MT(mod, kc) — 0x6000 | (mod << 8) | kc  (range 0x6000–0x7FFF but < 0x7800)
    if code >= 0x6000 && code < 0x7800 {
        let mods = ((code >> 8) & 0x1F) as u8;
        let kc = (code & 0xFF) as u16;
        let base = basic_keycode_name(kc);
        let mod_str = mod_bits_to_string(mods);
        return format!("tap: {} / hold: {}", base, mod_str);
    }

    // Modifier combos: 0x0100–0x1FFF
    // Format: (mods << 8) | keycode, where mods use bits:
    //   bit 0 = ctrl, bit 1 = shift, bit 2 = alt, bit 3 = gui
    //   bit 4 = right-side modifier
    if code >= 0x0100 && code <= 0x1FFF {
        let mods = ((code >> 8) & 0x1F) as u8;
        let kc = (code & 0xFF) as u16;
        let base = basic_keycode_name(kc);
        let mod_str = mod_bits_to_string(mods);
        return format!("{}+{}", mod_str, base);
    }

    // Basic keycodes
    if code <= 0x00FF {
        return basic_keycode_name(code);
    }

    // Fallback: show raw hex
    format!("0x{:04X}", code)
}

fn mod_bits_to_string(mods: u8) -> String {
    let right = mods & 0x10 != 0;
    let mut parts = Vec::new();
    let prefix = if right { "r" } else { "" };

    if mods & 0x08 != 0 {
        parts.push(format!("{}cmd", prefix));
    }
    if mods & 0x04 != 0 {
        parts.push(format!("{}alt", prefix));
    }
    if mods & 0x02 != 0 {
        parts.push(format!("{}shift", prefix));
    }
    if mods & 0x01 != 0 {
        parts.push(format!("{}ctrl", prefix));
    }

    if parts.is_empty() {
        "mod?".to_string()
    } else {
        parts.join("+")
    }
}

fn custom_keycode_name(idx: usize) -> String {
    match idx {
        0 => "RGB toggle".to_string(),
        1 => "RGB mode+".to_string(),
        2 => "RGB mode-".to_string(),
        3 => "RGB hue+".to_string(),
        4 => "RGB hue-".to_string(),
        5 => "RGB sat+".to_string(),
        6 => "RGB sat-".to_string(),
        7 => "RGB bright+".to_string(),
        8 => "RGB bright-".to_string(),
        9 => "LED brightness".to_string(),
        _ => format!("custom {}", idx),
    }
}

fn basic_keycode_name(code: u16) -> String {
    match code {
        0x04 => "a",
        0x05 => "b",
        0x06 => "c",
        0x07 => "d",
        0x08 => "e",
        0x09 => "f",
        0x0A => "g",
        0x0B => "h",
        0x0C => "i",
        0x0D => "j",
        0x0E => "k",
        0x0F => "l",
        0x10 => "m",
        0x11 => "n",
        0x12 => "o",
        0x13 => "p",
        0x14 => "q",
        0x15 => "r",
        0x16 => "s",
        0x17 => "t",
        0x18 => "u",
        0x19 => "v",
        0x1A => "w",
        0x1B => "x",
        0x1C => "y",
        0x1D => "z",
        0x1E => "1",
        0x1F => "2",
        0x20 => "3",
        0x21 => "4",
        0x22 => "5",
        0x23 => "6",
        0x24 => "7",
        0x25 => "8",
        0x26 => "9",
        0x27 => "0",
        0x28 => "enter",
        0x29 => "esc",
        0x2A => "backspace",
        0x2B => "tab",
        0x2C => "space",
        0x2D => "-",
        0x2E => "=",
        0x2F => "[",
        0x30 => "]",
        0x31 => "\\",
        0x33 => ";",
        0x34 => "'",
        0x35 => "`",
        0x36 => ",",
        0x37 => ".",
        0x38 => "/",
        0x39 => "caps lock",
        0x3A => "F1",
        0x3B => "F2",
        0x3C => "F3",
        0x3D => "F4",
        0x3E => "F5",
        0x3F => "F6",
        0x40 => "F7",
        0x41 => "F8",
        0x42 => "F9",
        0x43 => "F10",
        0x44 => "F11",
        0x45 => "F12",
        0x46 => "print screen",
        0x47 => "scroll lock",
        0x48 => "pause",
        0x49 => "insert",
        0x4A => "home",
        0x4B => "page up",
        0x4C => "delete",
        0x4D => "end",
        0x4E => "page down",
        0x4F => "right",
        0x50 => "left",
        0x51 => "down",
        0x52 => "up",
        // Media keys
        0xA8 => "mute",
        0xA9 => "volume up",
        0xAA => "volume down",
        0xCD => "play/pause",
        0xB5 => "next track",
        0xB6 => "prev track",
        0xB7 => "stop",
        // Modifiers (as basic keycodes)
        0xE0 => "ctrl",
        0xE1 => "shift",
        0xE2 => "alt",
        0xE3 => "cmd",
        0xE4 => "rctrl",
        0xE5 => "rshift",
        0xE6 => "ralt",
        0xE7 => "rcmd",
        _ => return format!("0x{:02X}", code),
    }
    .to_string()
}
