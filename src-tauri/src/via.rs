use hidapi::HidApi;
use serde::Serialize;

const VIA_USAGE_PAGE: u16 = 0xFF60;
const VIA_USAGE: u16 = 0x61;
const MSG_LEN: usize = 32;

// VIA protocol commands
const CMD_GET_PROTOCOL_VERSION: u8 = 0x01;
const CMD_DYNAMIC_KEYMAP_GET_LAYER_COUNT: u8 = 0x11;
const CMD_DYNAMIC_KEYMAP_GET_BUFFER: u8 = 0x12;
const CMD_DYNAMIC_KEYMAP_SET_KEYCODE: u8 = 0x05;
const CMD_DYNAMIC_KEYMAP_GET_ENCODER: u8 = 0x14;
const CMD_DYNAMIC_KEYMAP_SET_ENCODER: u8 = 0x15;
const CMD_GET_KEYBOARD_VALUE: u8 = 0x02;
const KEYBOARD_VALUE_SWITCH_MATRIX_STATE: u8 = 0x03;
const CMD_CUSTOM_SET_VALUE: u8 = 0x07;
const CMD_CUSTOM_GET_VALUE: u8 = 0x08;
const CMD_CUSTOM_SAVE: u8 = 0x09;

#[derive(Debug, Serialize, Clone)]
pub struct KeyboardInfo {
    pub vendor_id: u16,
    pub product_id: u16,
    pub product_name: String,
    pub protocol_version: u16,
    pub layer_count: u8,
}

#[derive(Debug, Serialize)]
pub struct KeymapDump {
    pub info: KeyboardInfo,
    pub raw_bytes: Vec<u8>,
    pub keycodes: Vec<u16>,
}

pub struct ViaDevice {
    device: hidapi::HidDevice,
    pub info: KeyboardInfo,
}

impl ViaDevice {
    /// Find and open the first VIA-compatible device.
    pub fn open() -> Result<Self, String> {
        let api = HidApi::new().map_err(|e| format!("HID init failed: {e}"))?;

        for dev_info in api.device_list() {
            if dev_info.usage_page() == VIA_USAGE_PAGE && dev_info.usage() == VIA_USAGE {
                let device = dev_info
                    .open_device(&api)
                    .map_err(|e| format!("Failed to open device: {e}"))?;

                let mut via = ViaDevice {
                    device,
                    info: KeyboardInfo {
                        vendor_id: dev_info.vendor_id(),
                        product_id: dev_info.product_id(),
                        product_name: dev_info
                            .product_string()
                            .unwrap_or("Unknown")
                            .to_string(),
                        protocol_version: 0,
                        layer_count: 0,
                    },
                };

                via.info.protocol_version = via.get_protocol_version()?;
                via.info.layer_count = via.get_layer_count()?;

                return Ok(via);
            }
        }

        Err("No VIA-compatible device found. Is the Micro plugged in?".to_string())
    }

    /// Send a raw VIA message and read the response.
    fn send(&self, data: &[u8]) -> Result<[u8; MSG_LEN], String> {
        let mut msg = [0u8; MSG_LEN + 1]; // +1 for HID report ID prefix
        msg[1..1 + data.len().min(MSG_LEN)].copy_from_slice(&data[..data.len().min(MSG_LEN)]);

        self.device
            .write(&msg)
            .map_err(|e| format!("HID write failed: {e}"))?;

        let mut buf = [0u8; MSG_LEN];
        self.device
            .read_timeout(&mut buf, 1000)
            .map_err(|e| format!("HID read failed: {e}"))?;

        Ok(buf)
    }

    fn get_protocol_version(&self) -> Result<u16, String> {
        let resp = self.send(&[CMD_GET_PROTOCOL_VERSION])?;
        Ok(u16::from_be_bytes([resp[1], resp[2]]))
    }

    fn get_layer_count(&self) -> Result<u8, String> {
        let resp = self.send(&[CMD_DYNAMIC_KEYMAP_GET_LAYER_COUNT])?;
        Ok(resp[1])
    }

    /// Read the entire dynamic keymap buffer.
    /// Keycodes are stored as big-endian u16 values.
    /// Buffer size = layers × rows × cols × 2.
    /// We don't know matrix size yet, so we read until we get empty data.
    pub fn read_keymap_buffer(&self, total_bytes: usize) -> Result<Vec<u8>, String> {
        let mut buffer = Vec::with_capacity(total_bytes);
        let mut offset: u16 = 0;
        let chunk = (MSG_LEN - 4) as u16; // response has 4-byte header

        while (offset as usize) < total_bytes {
            let remaining = (total_bytes - offset as usize).min(chunk as usize) as u8;
            let resp = self.send(&[
                CMD_DYNAMIC_KEYMAP_GET_BUFFER,
                (offset >> 8) as u8,
                (offset & 0xFF) as u8,
                remaining,
            ])?;

            // Response: [cmd, offset_hi, offset_lo, size, ...data]
            buffer.extend_from_slice(&resp[4..4 + remaining as usize]);
            offset += remaining as u16;
        }

        Ok(buffer)
    }

    /// Write a single keycode to the board.
    /// VIA command 0x05: set_keycode(layer, row, col, keycode_hi, keycode_lo)
    pub fn set_keycode(&self, layer: u8, row: u8, col: u8, keycode: u16) -> Result<(), String> {
        self.send(&[
            CMD_DYNAMIC_KEYMAP_SET_KEYCODE,
            layer,
            row,
            col,
            (keycode >> 8) as u8,
            (keycode & 0xFF) as u8,
        ])?;
        Ok(())
    }

    /// Read the switch matrix state — which keys are physically pressed.
    /// Returns a vec of (row, col) for each pressed key.
    /// Matrix state is packed as ceil(cols/8) bytes per row.
    pub fn get_switch_matrix_state(&self, rows: u8, cols: u8) -> Result<Vec<(u8, u8)>, String> {
        let resp = self.send(&[CMD_GET_KEYBOARD_VALUE, KEYBOARD_VALUE_SWITCH_MATRIX_STATE])?;
        let bytes_per_row = ((cols as usize) + 7) / 8;
        let mut pressed = Vec::new();

        // Response header is [cmd, value_id, offset] — matrix bytes start at index 3.
        // Reading from index 2 attributed each physical row's bits to the next row,
        // so pressing a key highlighted the key one row below it.
        for row in 0..rows {
            for col in 0..cols {
                let byte_idx = 3 + (row as usize) * bytes_per_row + (col as usize) / 8;
                let bit_idx = (col as usize) % 8;
                if byte_idx < MSG_LEN && (resp[byte_idx] >> bit_idx) & 1 == 1 {
                    pressed.push((row, col));
                }
            }
        }

        Ok(pressed)
    }

    /// Read an encoder mapping. direction: 0 = CW, 1 = CCW.
    pub fn get_encoder(&self, layer: u8, encoder_id: u8, direction: u8) -> Result<u16, String> {
        let resp = self.send(&[CMD_DYNAMIC_KEYMAP_GET_ENCODER, layer, encoder_id, direction])?;
        Ok(u16::from_be_bytes([resp[4], resp[5]]))
    }

    /// Write an encoder mapping. direction: 0 = CW, 1 = CCW.
    pub fn set_encoder(
        &self,
        layer: u8,
        encoder_id: u8,
        direction: u8,
        keycode: u16,
    ) -> Result<(), String> {
        self.send(&[
            CMD_DYNAMIC_KEYMAP_SET_ENCODER,
            layer,
            encoder_id,
            direction,
            (keycode >> 8) as u8,
            (keycode & 0xFF) as u8,
        ])?;
        Ok(())
    }

    /// Get a custom menu value (1 byte). Used for brightness, effect, speed.
    pub fn custom_get_value_u8(&self, channel: u8, value_id: u8) -> Result<u8, String> {
        let resp = self.send(&[CMD_CUSTOM_GET_VALUE, channel, value_id])?;
        Ok(resp[3])
    }

    /// Get a custom menu value (2 bytes). Used for color (hue, sat).
    pub fn custom_get_value_u8x2(&self, channel: u8, value_id: u8) -> Result<(u8, u8), String> {
        let resp = self.send(&[CMD_CUSTOM_GET_VALUE, channel, value_id])?;
        Ok((resp[3], resp[4]))
    }

    /// Set a custom menu value (1 byte).
    pub fn custom_set_value_u8(&self, channel: u8, value_id: u8, value: u8) -> Result<(), String> {
        self.send(&[CMD_CUSTOM_SET_VALUE, channel, value_id, value])?;
        Ok(())
    }

    /// Set a custom menu value (2 bytes).
    pub fn custom_set_value_u8x2(&self, channel: u8, value_id: u8, v1: u8, v2: u8) -> Result<(), String> {
        self.send(&[CMD_CUSTOM_SET_VALUE, channel, value_id, v1, v2])?;
        Ok(())
    }

    /// Save custom menu values to EEPROM.
    pub fn custom_save(&self) -> Result<(), String> {
        self.send(&[CMD_CUSTOM_SAVE])?;
        Ok(())
    }

    /// Dump keymap as a struct. Caller provides expected buffer size
    /// (layers × rows × cols × 2). For Work Louder Micro: 4 layers × 1 row × 4 cols = 32 bytes.
    pub fn dump_keymap(&self, buffer_size: usize) -> Result<KeymapDump, String> {
        let raw = self.read_keymap_buffer(buffer_size)?;
        let keycodes: Vec<u16> = raw
            .chunks_exact(2)
            .map(|pair| u16::from_be_bytes([pair[0], pair[1]]))
            .collect();

        Ok(KeymapDump {
            info: self.info.clone(),
            raw_bytes: raw,
            keycodes,
        })
    }
}
