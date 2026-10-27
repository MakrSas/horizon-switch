//! Horizon: native Rust client for Minecraft Java Edition 26.3 gameplay.
#![forbid(unsafe_code)]
mod game;

use horizon_client::{crash, logging};

/// The mod API level this game serves, kept in the program for the launcher
/// to read without running it (`horizon_mods::api_level::MARK`). `main`
/// reads it, so the linker keeps it.
static MOD_API_LEVEL: [u8; 27] = horizon_mods::api_level::MARK;

/// The Switch: newlib's malloc behind a lock-free cache of small blocks (horizon_nx::alloc).
#[cfg(target_os = "horizon")]
#[global_allocator]
static ALLOCATOR: horizon_nx::alloc::Allocator = horizon_nx::alloc::Allocator;

fn main() {
    std::hint::black_box(&MOD_API_LEVEL);
    horizon_client::telemetry::mark_start();
    // The Switch starts in no folder: the game's files live in its SD card folder.
    #[cfg(target_os = "horizon")]
    {
        let _ = std::fs::create_dir_all(logging::SWITCH_HOME);
        if let Err(e) = std::env::set_current_dir(logging::SWITCH_HOME) {
            eprintln!("Horizon: could not enter {}: {e}", logging::SWITCH_HOME);
        }
    }
    let args: Vec<String> = std::env::args().skip(1).collect();
    let game_args = args.iter().take_while(|a| *a != "--dedicated");
    let help = game_args.clone().any(|a| a == "--help" || a == "-h");
    // The dedicated server logs in its own folder, as vanilla's does; the game in its home.
    let (home, side, thread) = if args.iter().any(|a| a == "--dedicated") {
        (std::path::PathBuf::from("."), "server", "Server thread")
    } else {
        (logging::game_home(), "client", "Render thread")
    };
    if !help {
        logging::init(&home, thread);
        #[cfg(target_os = "horizon")]
        log::info!("Horizon: Switch build {}, folder {:?}", env!("NX_BUILD"), std::env::current_dir());
        #[cfg(target_os = "horizon")]
        {
            let (total, used) = horizon_nx::memory::usage();
            log::info!("Horizon: memory {} MB, {} MB used", total >> 20, used >> 20);
            log::info!("Horizon: {} CPU cores", std::thread::available_parallelism().map_or(0, |n| n.get()));
        }
        crash::install(home.clone(), side);
    }
    let result = game::run();
    #[cfg(target_os = "horizon")]
    log::info!("Horizon: the game loop returned ({})", if result.is_ok() { "ok" } else { "error" });
    if let Err(error) = result {
        log::error!("Horizon: {error}");
        if !help {
            crash::report_error(&home, side, &*error);
        }
        // Started from Explorer, the console window closes with the game:
        // keep the message on screen until it is read.
        #[cfg(windows)]
        if std::io::IsTerminal::is_terminal(&std::io::stdin()) {
            eprintln!("Press Enter to close.");
            let _ = std::io::stdin().read_line(&mut String::new());
        }
        std::process::exit(1);
    }
    logging::flush();
    #[cfg(target_os = "horizon")]
    horizon_nx::exit_now();
}
