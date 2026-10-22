//! Custom-drawn UI components (pure state + geometry). Phase 002: the in-window confirm dialog
//! (UI_SPEC §B.10.6, focus ring per §B.16). Rendering is done by the platform layer from the
//! geometry computed here.
#![forbid(unsafe_code)]

pub mod dialog;
pub mod settings_panel;
