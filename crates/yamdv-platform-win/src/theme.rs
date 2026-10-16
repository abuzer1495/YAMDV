//! Windows app mode (light/dark) detection (R-THEME-01, temporary background only in Phase 001).
#![allow(unsafe_code)]

use windows::Win32::System::Registry::{HKEY_CURRENT_USER, RRF_RT_REG_DWORD, RegGetValueW};
use windows::core::w;

/// Theme mode of the app.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    /// Light app mode.
    Light,
    /// Dark app mode.
    Dark,
}

impl Mode {
    /// Theme token `bg` as a GDI `COLORREF` value (0x00BBGGRR): light `#ffffff`, dark `#1e1e1e`.
    pub fn bg_colorref(self) -> u32 {
        match self {
            Self::Light => 0x00ff_ffff,
            Self::Dark => 0x001e_1e1e,
        }
    }
}

/// Read `HKCU\…\Themes\Personalize\AppsUseLightTheme`; missing value → light (Windows default).
pub fn app_mode() -> Mode {
    let mut value = 1u32;
    let mut size = u32::try_from(size_of::<u32>()).unwrap_or(4);
    // SAFETY: value/size are valid out-pointers sized for a REG_DWORD; strings are static.
    let r = unsafe {
        RegGetValueW(
            HKEY_CURRENT_USER,
            w!("Software\\Microsoft\\Windows\\CurrentVersion\\Themes\\Personalize"),
            w!("AppsUseLightTheme"),
            RRF_RT_REG_DWORD,
            None,
            Some((&raw mut value).cast()),
            Some(&raw mut size),
        )
    };
    if r.is_ok() && value == 0 {
        Mode::Dark
    } else {
        Mode::Light
    }
}
