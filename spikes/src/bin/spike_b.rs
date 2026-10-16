//! Spike B (§4.7). See `spikes/src/backend_b.rs`.
#![cfg_attr(windows, windows_subsystem = "windows")]

#[cfg(windows)]
fn main() {
    spikes::app::run::<spikes::backend_b::BackendB>();
}

#[cfg(not(windows))]
fn main() {
    std::process::exit(1);
}
