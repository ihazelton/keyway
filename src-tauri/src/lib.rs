#[cfg(target_os = "macos")]
mod capture;
pub mod cli;
mod keycodes;
pub(crate) mod via;

use keycodes::keycode_to_human;
use serde::Serialize;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use tauri::State;
use via::ViaDevice;

/// Whether native keystroke capture is armed. Shared with the macOS event monitor.
struct CaptureFlag(Arc<AtomicBool>);

const MICRO_ROWS: usize = 4;
const MICRO_COLS: usize = 4;
const NUM_ENCODERS: u8 = 2;

struct DeviceState(Mutex<Option<ViaDevice>>);

fn with_device<T>(state: &State<DeviceState>, f: impl Fn(&ViaDevice) -> Result<T, String>) -> Result<T, String> {
    let mut guard = state.0.lock().map_err(|e| format!("Lock error: {e}"))?;
    // Open if not connected
    if guard.is_none() {
        *guard = Some(ViaDevice::open()?);
    }
    match f(guard.as_ref().unwrap()) {
        Ok(val) => Ok(val),
        Err(e) => {
            // Connection may be stale — drop and retry once
            *guard = None;
            *guard = Some(ViaDevice::open()?);
            f(guard.as_ref().unwrap())
        }
    }
}

#[derive(Serialize)]
struct KeyInfo {
    row: usize,
    col: usize,
    code: u16,
    code_hex: String,
    label: String,
    is_encoder: bool,
    encoder_id: Option<u8>,
}

#[derive(Serialize)]
struct EncoderAction {
    code: u16,
    code_hex: String,
    label: String,
}

#[derive(Serialize)]
struct EncoderInfo {
    id: u8,
    cw: EncoderAction,
    ccw: EncoderAction,
}

#[derive(Serialize)]
struct LayerData {
    index: usize,
    keys: Vec<KeyInfo>,
    encoders: Vec<EncoderInfo>,
}

#[derive(Serialize)]
struct ProbeResult {
    product_name: String,
    vendor_id: String,
    product_id: String,
    protocol_version: String,
    layer_count: usize,
    layers: Vec<LayerData>,
}

fn layout_keys() -> Vec<(usize, usize, f32, f32, bool, Option<u8>)> {
    vec![
        (0, 0, 0.0, 0.0, true, Some(0)),
        (0, 3, 3.5, 0.0, true, Some(1)),
        (0, 1, 1.25, 0.25, false, None),
        (0, 2, 2.25, 0.25, false, None),
        (1, 0, 0.25, 1.0, false, None),
        (1, 1, 1.25, 1.0, false, None),
        (1, 2, 2.25, 1.0, false, None),
        (1, 3, 3.25, 1.0, false, None),
        (2, 0, 0.25, 2.0, false, None),
        (2, 1, 1.25, 2.0, false, None),
        (2, 2, 2.25, 2.0, false, None),
        (2, 3, 3.25, 2.0, false, None),
        (3, 0, 0.0, 3.25, false, None),
        (3, 1, 1.25, 3.0, false, None),
        (3, 2, 2.25, 3.0, false, None),
        (3, 3, 3.5, 3.25, false, None),
    ]
}

fn make_encoder_action(code: u16) -> EncoderAction {
    EncoderAction {
        code,
        code_hex: format!("0x{:04X}", code),
        label: keycode_to_human(code),
    }
}

#[tauri::command]
fn probe_keyboard(state: State<DeviceState>) -> Result<ProbeResult, String> {
    with_device(&state, |device| {
        let info = &device.info;
        let layer_count = info.layer_count as usize;
        let buffer_size = layer_count * MICRO_ROWS * MICRO_COLS * 2;
        let dump = device.dump_keymap(buffer_size)?;
        let layout = layout_keys();

        let mut layers = Vec::new();
        for layer in 0..layer_count {
            let mut keys = Vec::new();
            for &(row, col, _x, _y, is_encoder, encoder_id) in &layout {
                let idx = layer * MICRO_ROWS * MICRO_COLS + row * MICRO_COLS + col;
                let code = if idx < dump.keycodes.len() { dump.keycodes[idx] } else { 0 };
                keys.push(KeyInfo {
                    row, col, code,
                    code_hex: format!("0x{:04X}", code),
                    label: keycode_to_human(code),
                    is_encoder, encoder_id,
                });
            }
            let mut encoders = Vec::new();
            for enc_id in 0..NUM_ENCODERS {
                let cw = device.get_encoder(layer as u8, enc_id, 0)?;
                let ccw = device.get_encoder(layer as u8, enc_id, 1)?;
                encoders.push(EncoderInfo { id: enc_id, cw: make_encoder_action(cw), ccw: make_encoder_action(ccw) });
            }
            layers.push(LayerData { index: layer, keys, encoders });
        }

        Ok(ProbeResult {
            product_name: info.product_name.clone(),
            vendor_id: format!("{:04X}", info.vendor_id),
            product_id: format!("{:04X}", info.product_id),
            protocol_version: format!("{}.{}", info.protocol_version >> 8, info.protocol_version & 0xFF),
            layer_count,
            layers,
        })
    })
}

#[tauri::command]
fn set_key(state: State<DeviceState>, layer: u8, row: u8, col: u8, keycode: u16) -> Result<(), String> {
    with_device(&state, |d| d.set_keycode(layer, row, col, keycode))
}

#[tauri::command]
fn set_encoder(state: State<DeviceState>, layer: u8, encoder_id: u8, direction: u8, keycode: u16) -> Result<(), String> {
    with_device(&state, |d| d.set_encoder(layer, encoder_id, direction, keycode))
}

#[derive(Serialize)]
struct PressedKeys { keys: Vec<(u8, u8)> }

#[derive(Serialize)]
struct LightingState {
    underglow_brightness: u8,
    underglow_effect: u8,
    underglow_speed: u8,
    underglow_color: (u8, u8),
    backlight_brightness: u8,
    backlight_effect: u8,
    backlight_speed: u8,
    backlight_color: (u8, u8),
    indicator_brightness: u8,
}

#[tauri::command]
fn get_pressed_keys(state: State<DeviceState>) -> Result<PressedKeys, String> {
    with_device(&state, |d| {
        let keys = d.get_switch_matrix_state(MICRO_ROWS as u8, MICRO_COLS as u8)?;
        Ok(PressedKeys { keys })
    })
}

#[tauri::command]
fn get_lighting(state: State<DeviceState>) -> Result<LightingState, String> {
    with_device(&state, |d| {
        Ok(LightingState {
            underglow_brightness: d.custom_get_value_u8(2, 1)?,
            underglow_effect: d.custom_get_value_u8(2, 2)?,
            underglow_speed: d.custom_get_value_u8(2, 3)?,
            underglow_color: d.custom_get_value_u8x2(2, 4)?,
            backlight_brightness: d.custom_get_value_u8(3, 1)?,
            backlight_effect: d.custom_get_value_u8(3, 2)?,
            backlight_speed: d.custom_get_value_u8(3, 3)?,
            backlight_color: d.custom_get_value_u8x2(3, 4)?,
            indicator_brightness: d.custom_get_value_u8(0, 1)?,
        })
    })
}

#[tauri::command]
fn set_lighting_value(state: State<DeviceState>, channel: u8, value_id: u8, value: Vec<u8>) -> Result<(), String> {
    with_device(&state, |d| {
        if value.len() == 1 {
            d.custom_set_value_u8(channel, value_id, value[0])
        } else if value.len() == 2 {
            d.custom_set_value_u8x2(channel, value_id, value[0], value[1])
        } else {
            Ok(())
        }
    })
}

#[tauri::command]
fn save_lighting(state: State<DeviceState>) -> Result<(), String> {
    with_device(&state, |d| d.custom_save())
}

/// Arm or disarm native keystroke capture.
#[tauri::command]
fn set_capture(flag: State<CaptureFlag>, enabled: bool) {
    flag.0.store(enabled, Ordering::SeqCst);
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let capture_flag = Arc::new(AtomicBool::new(false));
    let monitor_flag = capture_flag.clone();
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(DeviceState(Mutex::new(None)))
        .manage(CaptureFlag(capture_flag))
        .setup(move |_app| {
            #[cfg(target_os = "macos")]
            capture::install(_app.handle().clone(), monitor_flag.clone());
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            probe_keyboard, set_key, set_encoder, get_pressed_keys,
            get_lighting, set_lighting_value, save_lighting, set_capture
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
