//! Spike C (§4.7). See `spikes/src/backend_c.rs`.
#![cfg_attr(windows, windows_subsystem = "windows")]

#[cfg(windows)]
fn main() {
    spikes::app::run::<spikes::backend_c::BackendC>();
}

#[cfg(not(windows))]
fn main() {
    std::process::exit(1);
}
