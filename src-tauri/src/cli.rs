use crate::via::ViaDevice;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::thread;
use std::time::Duration;

/// Lighting snapshot — all 9 readable values across both channels + indicators.
#[derive(Serialize, Deserialize, Debug)]
struct LightingSnapshot {
    underglow_brightness: u8,
    underglow_effect: u8,
    underglow_speed: u8,
    underglow_hue: u8,
    underglow_sat: u8,
    backlight_brightness: u8,
    backlight_effect: u8,
    backlight_speed: u8,
    backlight_hue: u8,
    backlight_sat: u8,
    indicator_brightness: u8,
}

fn snapshot_path() -> PathBuf {
    let mut p = dirs_home().join(".keyway");
    p.push("lighting-snapshot.json");
    p
}

/// Marker file used to interrupt a running flash from another process.
/// A flash loop checks for this file each cycle; if present, it stops,
/// restores the snapshot, and cleans up.
fn stop_flag_path() -> PathBuf {
    let mut p = dirs_home().join(".keyway");
    p.push("flash-stop");
    p
}

fn dirs_home() -> PathBuf {
    std::env::var("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("/tmp"))
}

fn read_snapshot(device: &ViaDevice) -> Result<LightingSnapshot, String> {
    let (uh, us) = device.custom_get_value_u8x2(2, 4)?;
    let (bh, bs) = device.custom_get_value_u8x2(3, 4)?;
    Ok(LightingSnapshot {
        underglow_brightness: device.custom_get_value_u8(2, 1)?,
        underglow_effect: device.custom_get_value_u8(2, 2)?,
        underglow_speed: device.custom_get_value_u8(2, 3)?,
        underglow_hue: uh,
        underglow_sat: us,
        backlight_brightness: device.custom_get_value_u8(3, 1)?,
        backlight_effect: device.custom_get_value_u8(3, 2)?,
        backlight_speed: device.custom_get_value_u8(3, 3)?,
        backlight_hue: bh,
        backlight_sat: bs,
        indicator_brightness: device.custom_get_value_u8(0, 1)?,
    })
}

fn ensure_snapshot(device: &ViaDevice) -> Result<(), String> {
    let path = snapshot_path();
    if path.exists() {
        return Ok(());
    }
    let snap = read_snapshot(device)?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| format!("Failed to create ~/.keyway: {e}"))?;
    }
    let json = serde_json::to_string_pretty(&snap)
        .map_err(|e| format!("JSON serialise failed: {e}"))?;
    fs::write(&path, json).map_err(|e| format!("Failed to write snapshot: {e}"))?;
    eprintln!("Snapshot saved to {}", path.display());
    Ok(())
}

fn apply_snapshot(device: &ViaDevice, snap: &LightingSnapshot) -> Result<(), String> {
    device.custom_set_value_u8(2, 1, snap.underglow_brightness)?;
    device.custom_set_value_u8(2, 2, snap.underglow_effect)?;
    device.custom_set_value_u8(2, 3, snap.underglow_speed)?;
    device.custom_set_value_u8x2(2, 4, snap.underglow_hue, snap.underglow_sat)?;
    device.custom_set_value_u8(3, 1, snap.backlight_brightness)?;
    device.custom_set_value_u8(3, 2, snap.backlight_effect)?;
    device.custom_set_value_u8(3, 3, snap.backlight_speed)?;
    device.custom_set_value_u8x2(3, 4, snap.backlight_hue, snap.backlight_sat)?;
    device.custom_set_value_u8(0, 1, snap.indicator_brightness)?;
    Ok(())
}

fn resolve_colour(name: &str) -> Result<u8, String> {
    match name.to_lowercase().as_str() {
        "red" => Ok(0),
        "orange" => Ok(21),
        "amber" => Ok(28),
        "yellow" => Ok(43),
        "green" => Ok(85),
        "cyan" => Ok(128),
        "blue" => Ok(170),
        "purple" => Ok(191),
        "magenta" | "pink" => Ok(213),
        "white" => Ok(0), // hue irrelevant for white; sat will be 0
        _ => name
            .parse::<u8>()
            .map_err(|_| format!("Unknown colour '{}'. Use a name (red/green/blue/amber) or a hue 0-255.", name)),
    }
}

fn set_solid_colour(device: &ViaDevice, hue: u8, sat: u8, brightness: u8) -> Result<(), String> {
    // Effect 1 = Solid Color on both channels
    for channel in [2u8, 3u8] {
        device.custom_set_value_u8(channel, 1, brightness)?; // brightness
        device.custom_set_value_u8(channel, 2, 1)?;          // effect = Solid Color
        device.custom_set_value_u8x2(channel, 4, hue, sat)?; // colour
    }
    Ok(())
}

fn print_usage() {
    eprintln!("Usage: keyway light [OPTIONS]");
    eprintln!();
    eprintln!("Options:");
    eprintln!("  --color <name|hue>   Set solid colour (red, green, blue, amber, or 0-255)");
    eprintln!("  --flash [--color X]  Flash attention pattern (default 10s or until Ctrl-C)");
    eprintln!("  --duration <secs>    Flash duration in seconds (with --flash)");
    eprintln!("  --restore            Restore lighting from saved snapshot");
    eprintln!();
    eprintln!("Colour names: red, orange, amber, yellow, green, cyan, blue, purple, magenta, white");
}

pub fn run(args: &[String]) -> Result<(), String> {
    if args.is_empty() {
        print_usage();
        return Err("No subcommand given.".to_string());
    }

    // Parse arguments
    let mut colour: Option<String> = None;
    let mut flash = false;
    let mut restore = false;
    let mut duration_secs: u64 = 10;
    let mut i = 0;

    while i < args.len() {
        match args[i].as_str() {
            "--color" | "--colour" => {
                i += 1;
                if i >= args.len() {
                    return Err("--color requires a value".to_string());
                }
                colour = Some(args[i].clone());
            }
            "--duration" => {
                i += 1;
                if i >= args.len() {
                    return Err("--duration requires a value in seconds".to_string());
                }
                duration_secs = args[i]
                    .parse()
                    .map_err(|_| format!("Invalid duration '{}'. Use seconds, e.g. --duration 60", args[i]))?;
            }
            "--flash" => {
                flash = true;
            }
            "--restore" => {
                restore = true;
            }
            "--help" | "-h" => {
                print_usage();
                return Ok(());
            }
            other => {
                return Err(format!("Unknown option: {other}"));
            }
        }
        i += 1;
    }

    if !flash && !restore && colour.is_none() {
        print_usage();
        return Err("No action specified.".to_string());
    }

    // Validate colour name early, before touching the device
    if let Some(ref c) = colour {
        resolve_colour(c)?;
    }

    if restore {
        let path = snapshot_path();
        if !path.exists() {
            return Err("No snapshot found. Nothing to restore.".to_string());
        }

        // Signal any running flash to stop, restore, and clean up itself.
        let stop = stop_flag_path();
        if let Some(parent) = stop.parent() {
            fs::create_dir_all(parent).ok();
        }
        fs::write(&stop, b"").ok();

        // If a flash holds the device, open() fails — the flash will handle
        // the restore when it sees the stop flag. That counts as success.
        match ViaDevice::open() {
            Ok(device) => {
                let data = fs::read_to_string(&path)
                    .map_err(|e| format!("Failed to read snapshot: {e}"))?;
                let snap: LightingSnapshot = serde_json::from_str(&data)
                    .map_err(|e| format!("Failed to parse snapshot: {e}"))?;
                apply_snapshot(&device, &snap)?;
                fs::remove_file(&path).ok();
                fs::remove_file(&stop).ok();
                eprintln!("Lighting restored from snapshot.");
            }
            Err(_) => {
                eprintln!("Device busy — a running flash will restore lighting shortly.");
            }
        }
        return Ok(());
    }

    let device = ViaDevice::open()?;

    // Snapshot current state before making changes (only if no snapshot exists yet)
    ensure_snapshot(&device)?;

    if flash {
        let hue = match &colour {
            Some(c) => resolve_colour(c)?,
            None => 28, // amber default
        };
        let sat = if colour.as_deref() == Some("white") { 0 } else { 255 };

        // Set to solid colour first
        set_solid_colour(&device, hue, sat, 255)?;

        // Install Ctrl-C handler
        let running = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(true));
        let r = running.clone();
        ctrlc_flag(&r);

        // Clear any stale stop flag from a previous run
        let stop = stop_flag_path();
        fs::remove_file(&stop).ok();

        // Flash loop: toggle brightness every 400ms for --duration seconds (default 10).
        // Interruptible by Ctrl-C or by another process writing the stop flag
        // (e.g. `keyway light --restore` while this flash holds the device).
        let cycles = duration_secs * 1000 / 400;
        for n in 0..cycles {
            if !running.load(std::sync::atomic::Ordering::Relaxed) || stop.exists() {
                break;
            }
            let brightness = if n % 2 == 0 { 40 } else { 255 };
            for channel in [2u8, 3u8] {
                device.custom_set_value_u8(channel, 1, brightness)?;
            }
            thread::sleep(Duration::from_millis(400));
        }
        fs::remove_file(&stop).ok();

        // Restore after flash
        let path = snapshot_path();
        if path.exists() {
            let data = fs::read_to_string(&path)
                .map_err(|e| format!("Failed to read snapshot: {e}"))?;
            let snap: LightingSnapshot = serde_json::from_str(&data)
                .map_err(|e| format!("Failed to parse snapshot: {e}"))?;
            apply_snapshot(&device, &snap)?;
            fs::remove_file(&path).ok();
            eprintln!("Flash complete. Lighting restored.");
        }

        return Ok(());
    }

    // --color only
    if let Some(ref c) = colour {
        let hue = resolve_colour(c)?;
        let sat = if c.to_lowercase() == "white" { 0 } else { 255 };
        set_solid_colour(&device, hue, sat, 255)?;
        eprintln!("Colour set to {} (hue={}, sat={}).", c, hue, sat);
    }

    Ok(())
}

/// Set an atomic bool to false on Ctrl-C. Uses a simple signal handler
/// without adding a dependency — just catches the first SIGINT.
fn ctrlc_flag(flag: &std::sync::Arc<std::sync::atomic::AtomicBool>) {
    let f = flag.clone();
    // We cannot use ctrlc crate (not a dependency), so use libc-free approach:
    // just spawn a thread is not viable. Instead, set a simple signal handler.
    // On macOS we can use the signal() function from libc which is linked by default.
    unsafe {
        // Store the flag pointer in a static so the signal handler can access it.
        CTRLC_FLAG.store(f.as_ref() as *const std::sync::atomic::AtomicBool as *mut std::sync::atomic::AtomicBool, std::sync::atomic::Ordering::SeqCst);
        libc::signal(libc::SIGINT, ctrlc_handler as *const () as libc::sighandler_t);
        libc::signal(libc::SIGTERM, ctrlc_handler as *const () as libc::sighandler_t);
    }
}

static CTRLC_FLAG: std::sync::atomic::AtomicPtr<std::sync::atomic::AtomicBool> =
    std::sync::atomic::AtomicPtr::new(std::ptr::null_mut());

extern "C" fn ctrlc_handler(_sig: libc::c_int) {
    let ptr = CTRLC_FLAG.load(std::sync::atomic::Ordering::SeqCst);
    if !ptr.is_null() {
        unsafe { &*ptr }.store(false, std::sync::atomic::Ordering::SeqCst);
    }
}
