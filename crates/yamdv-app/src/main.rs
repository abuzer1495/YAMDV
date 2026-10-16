//! `yamdv.exe` — composition root (ARCHITECTURE §E.2). Phase 001: hardening, CLI, window skeleton.
#![cfg_attr(windows, windows_subsystem = "windows")]

// On non-Windows hosts only the parser is exercised (by unit tests).
#[cfg_attr(not(windows), allow(dead_code))]
mod cli;

#[cfg(windows)]
fn main() {
    // S-PROC-01..04: hardening is the very first thing the process does.
    let _steps = yamdv_platform_win::hardening::apply();
    yamdv_platform_win::crash::install(cli::VERSION);
    use yamdv_platform_win::console::{Stream, write};
    let code = match cli::parse(
        std::env::args_os()
            .skip(1)
            .map(|a| a.to_string_lossy().into_owned()),
    ) {
        cli::Command::Help => {
            write(Stream::Out, &cli::help_text());
            0
        }
        cli::Command::Version => {
            write(Stream::Out, &cli::version_text());
            0
        }
        cli::Command::Usage(msg) => {
            write(Stream::Err, &msg);
            cli::EXIT_USAGE
        }
        cli::Command::Run { paths: _ } => yamdv_platform_win::window::run("YAMDV"),
    };
    std::process::exit(code);
}

/// YAMDV runs only on Windows; on other hosts the binary exists so the workspace builds and the
/// pure CLI logic can be unit-tested.
#[cfg(not(windows))]
fn main() {
    let _ = cli::parse(std::env::args().skip(1));
    std::process::exit(1);
}
