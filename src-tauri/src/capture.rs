//! Native keystroke capture (macOS).
//!
//! WKWebView never receives Cmd/Ctrl key combos — macOS routes them to the
//! menu key-equivalent path before the web content sees them. To capture full
//! combos (the way Electron does for VIA) we install an in-app NSEvent local
//! monitor. It only sees events destined for *our* focused window, so it needs
//! no Input Monitoring permission. While capture mode is armed it swallows the
//! keypress (returns nil) so the combo doesn't also trigger an app action.

use std::ptr::NonNull;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use block2::RcBlock;
use objc2_app_kit::{NSEvent, NSEventMask, NSEventModifierFlags};
use tauri::{AppHandle, Emitter};

use crate::keycodes::keycode_to_human;

#[derive(Clone, serde::Serialize)]
struct CapturedKeystroke {
    keycode: u16,
    label: String,
}

/// macOS virtual key code → QMK/HID base keycode. Layout-independent (the
/// virtual codes are physical positions), so this holds across keyboards.
fn mac_keycode_to_qmk(vk: u16) -> Option<u16> {
    Some(match vk {
        // Letters
        0x00 => 0x04, 0x0B => 0x05, 0x08 => 0x06, 0x02 => 0x07, // A B C D
        0x0E => 0x08, 0x03 => 0x09, 0x05 => 0x0a, 0x04 => 0x0b, // E F G H
        0x22 => 0x0c, 0x26 => 0x0d, 0x28 => 0x0e, 0x25 => 0x0f, // I J K L
        0x2E => 0x10, 0x2D => 0x11, 0x1F => 0x12, 0x23 => 0x13, // M N O P
        0x0C => 0x14, 0x0F => 0x15, 0x01 => 0x16, 0x11 => 0x17, // Q R S T
        0x20 => 0x18, 0x09 => 0x19, 0x0D => 0x1a, 0x07 => 0x1b, // U V W X
        0x10 => 0x1c, 0x06 => 0x1d,                             // Y Z
        // Digits
        0x12 => 0x1e, 0x13 => 0x1f, 0x14 => 0x20, 0x15 => 0x21, // 1 2 3 4
        0x17 => 0x22, 0x16 => 0x23, 0x1A => 0x24, 0x1C => 0x25, // 5 6 7 8
        0x19 => 0x26, 0x1D => 0x27,                             // 9 0
        // Whitespace / control
        0x24 => 0x28, // Return
        0x35 => 0x29, // Escape
        0x33 => 0x2a, // Backspace
        0x30 => 0x2b, // Tab
        0x31 => 0x2c, // Space
        // Symbols
        0x1B => 0x2d, // -
        0x18 => 0x2e, // =
        0x21 => 0x2f, // [
        0x1E => 0x30, // ]
        0x2A => 0x31, // backslash
        0x29 => 0x33, // ;
        0x27 => 0x34, // '
        0x32 => 0x35, // `
        0x2B => 0x36, // ,
        0x2F => 0x37, // .
        0x2C => 0x38, // /
        0x39 => 0x39, // Caps Lock
        // Function keys
        0x7A => 0x3a, 0x78 => 0x3b, 0x63 => 0x3c, 0x76 => 0x3d, // F1-F4
        0x60 => 0x3e, 0x61 => 0x3f, 0x62 => 0x40, 0x64 => 0x41, // F5-F8
        0x65 => 0x42, 0x6D => 0x43, 0x67 => 0x44, 0x6F => 0x45, // F9-F12
        // Navigation
        0x72 => 0x49, // Insert (Help)
        0x73 => 0x4a, // Home
        0x74 => 0x4b, // Page Up
        0x75 => 0x4c, // Forward Delete
        0x77 => 0x4d, // End
        0x79 => 0x4e, // Page Down
        0x7C => 0x4f, // Right
        0x7B => 0x50, // Left
        0x7D => 0x51, // Down
        0x7E => 0x52, // Up
        _ => return None,
    })
}

fn mods_from_flags(flags: NSEventModifierFlags) -> u16 {
    let mut m = 0u16;
    if flags.contains(NSEventModifierFlags::Control) {
        m |= 0x01;
    }
    if flags.contains(NSEventModifierFlags::Shift) {
        m |= 0x02;
    }
    if flags.contains(NSEventModifierFlags::Option) {
        m |= 0x04;
    }
    if flags.contains(NSEventModifierFlags::Command) {
        m |= 0x08;
    }
    m
}

/// Install the local key monitor. Must run on the main thread (call from setup).
pub fn install(app: AppHandle, capturing: Arc<AtomicBool>) {
    let handler = RcBlock::new(move |event: NonNull<NSEvent>| -> *mut NSEvent {
        // Not armed: pass the event through untouched.
        if !capturing.load(Ordering::SeqCst) {
            return event.as_ptr();
        }
        let ev: &NSEvent = unsafe { event.as_ref() };
        let vk = ev.keyCode();
        let flags = ev.modifierFlags();
        if let Some(base) = mac_keycode_to_qmk(vk) {
            let mods = mods_from_flags(flags);
            let keycode = if mods == 0 { base } else { (mods << 8) | base };
            let label = keycode_to_human(keycode);
            let _ = app.emit("captured-keystroke", CapturedKeystroke { keycode, label });
        }
        // Swallow the event so the combo doesn't also trigger an action.
        std::ptr::null_mut()
    });

    unsafe {
        let token = NSEvent::addLocalMonitorForEventsMatchingMask_handler(
            NSEventMask::KeyDown,
            &handler,
        );
        // Keep the monitor token and the block alive for the app's lifetime.
        std::mem::forget(token);
    }
    std::mem::forget(handler);
}
