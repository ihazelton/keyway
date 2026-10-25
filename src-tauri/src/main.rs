// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    let args: Vec<String> = std::env::args().collect();

    // CLI mode: `keyway light [OPTIONS]`
    // Check for "light" as first real argument (args[0] is the binary path).
    if args.len() > 1 && args[1] == "light" {
        let cli_args = &args[2..];
        match keyway_lib::cli::run(cli_args) {
            Ok(()) => std::process::exit(0),
            Err(e) => {
                eprintln!("Error: {e}");
                std::process::exit(1);
            }
        }
    }

    keyway_lib::run()
}
