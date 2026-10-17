//! Self-install and uninstall (R-SYS-05, prompt 008 §4): no third-party installer. Everything
//! written is listed in `yamdv_core::reg` (S-REG-02) plus one Start-menu shortcut and the
//! install folder; `--uninstall` removes exactly those.
#![allow(unsafe_code)]

use windows::Win32::System::Com::{
    CLSCTX_INPROC_SERVER, COINIT_APARTMENTTHREADED, CoCreateInstance, CoInitializeEx, IPersistFile,
};
use windows::Win32::System::Registry::{HKEY_CURRENT_USER, REG_DWORD, RegSetKeyValueW};
use windows::Win32::UI::Shell::{IShellLinkW, ShellLink};
use windows::core::{HSTRING, Interface};
use yamdv_core::reg;

/// Version written to the uninstall entry.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

fn env(k: &str) -> String {
    std::env::var(k).unwrap_or_default()
}

/// `%LOCALAPPDATA%\Programs\YAMDV` (empty when the variable is missing).
pub fn install_dir() -> String {
    let l = env("LOCALAPPDATA");
    if l.is_empty() {
        String::new()
    } else {
        reg::install_dir(&l)
    }
}

/// Installed exe path.
pub fn installed_exe() -> String {
    format!("{}\\yamdv.exe", install_dir())
}

/// Start-menu shortcut path (`%APPDATA%\Microsoft\Windows\Start Menu\Programs\YAMDV.lnk`).
pub fn shortcut_path() -> String {
    format!(
        "{}\\Microsoft\\Windows\\Start Menu\\Programs\\YAMDV.lnk",
        env("APPDATA").trim_end_matches('\\')
    )
}

/// `true` when this exe runs from the install folder.
pub fn running_installed() -> bool {
    reg::is_installed_path(&crate::shell::exe_path(), &env("LOCALAPPDATA"))
}

/// `true` when `yamdv.portable` sits beside the exe (R-SET-05).
pub fn running_portable() -> bool {
    std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|d| d.join("yamdv.portable").exists()))
        .unwrap_or(false)
}

/// The welcome page offers "Install YAMDV for this user" (prompt 008 §4).
pub fn offer_install() -> bool {
    !running_installed() && !running_portable() && !install_dir().is_empty()
}

/// Test builds only: `YAMDV_TEST_HKCU=<subkey>` redirects HKCU to `HKCU\<subkey>` so CI checks
/// `--install` / `--uninstall` against a scratch hive.
fn test_hive() {
    #[cfg(feature = "dev-tools")]
    {
        use windows::Win32::System::Registry::{
            HKEY, KEY_ALL_ACCESS, REG_OPTION_NON_VOLATILE, RegCreateKeyExW, RegOverridePredefKey,
        };
        let sub = env("YAMDV_TEST_HKCU");
        if sub.is_empty() || !sub.starts_with("Software\\YAMDV-Test") {
            return;
        }
        let mut hk = HKEY::default();
        // SAFETY: creates/opens a scratch key; the handle stays open for the process lifetime as
        // the override target.
        unsafe {
            if RegCreateKeyExW(
                HKEY_CURRENT_USER,
                &HSTRING::from(sub.as_str()),
                None,
                windows::core::PCWSTR::null(),
                REG_OPTION_NON_VOLATILE,
                KEY_ALL_ACCESS,
                None,
                &mut hk,
                None,
            )
            .is_ok()
            {
                let _ = RegOverridePredefKey(HKEY_CURRENT_USER, Some(hk));
            }
        }
    }
}

fn say(s: &str) {
    crate::console::write(crate::console::Stream::Out, &format!("{s}\n"));
}

fn fail(s: &str) -> i32 {
    crate::console::write(crate::console::Stream::Err, &format!("yamdv: {s}\n"));
    1
}

/// `--install [--no-launch]`: copy, register, shortcut, uninstall entry, start the copy.
/// Idempotent: a second run refreshes the same files and values.
pub fn install(launch: bool) -> i32 {
    test_hive();
    let dir = install_dir();
    if dir.is_empty() {
        return fail("LOCALAPPDATA is not set");
    }
    let me = crate::shell::exe_path();
    let target = installed_exe();
    if std::fs::create_dir_all(&dir).is_err() {
        return fail("cannot create the install folder");
    }
    if !me.eq_ignore_ascii_case(&target) {
        let tmp = format!("{target}.new");
        if std::fs::copy(&me, &tmp).is_err() || std::fs::rename(&tmp, &target).is_err() {
            let _ = std::fs::remove_file(&tmp);
            return fail("cannot copy yamdv.exe (is the installed copy running?)");
        }
    }
    let mut ok = crate::shell::register_exe(&target);
    ok &= crate::shell::set_values(&reg::uninstall_entry_plan(&target, &dir, VERSION));
    let kb = std::fs::metadata(&target).map_or(0, |m| m.len().div_ceil(1024));
    let kb = u32::try_from(kb).unwrap_or(u32::MAX);
    // SAFETY: HKCU value inside the allowlisted uninstall key; 4-byte DWORD buffer.
    let r = unsafe {
        RegSetKeyValueW(
            HKEY_CURRENT_USER,
            &HSTRING::from(reg::UNINSTALL_KEY),
            &HSTRING::from("EstimatedSize"),
            REG_DWORD.0,
            Some((&raw const kb).cast()),
            4,
        )
    };
    ok &= r.is_ok();
    ok &= make_shortcut(&target, &dir);
    if !ok {
        return fail("installation incomplete (registry or shortcut)");
    }
    say(&format!("installed {target}"));
    if launch {
        crate::shell::spawn(&target, &[]);
    }
    0
}

fn make_shortcut(target: &str, dir: &str) -> bool {
    let lnk = shortcut_path();
    if let Some(d) = std::path::Path::new(&lnk).parent() {
        let _ = std::fs::create_dir_all(d);
    }
    // SAFETY: COM on this thread; the shell link object is used and released here.
    unsafe {
        let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
        let Ok(link) = CoCreateInstance::<_, IShellLinkW>(&ShellLink, None, CLSCTX_INPROC_SERVER)
        else {
            return false;
        };
        let _ = link.SetPath(&HSTRING::from(target));
        let _ = link.SetWorkingDirectory(&HSTRING::from(dir));
        let _ = link.SetDescription(&HSTRING::from("YAMDV Markdown viewer"));
        let _ = link.SetIconLocation(&HSTRING::from(target), 0);
        let Ok(pf) = link.cast::<IPersistFile>() else {
            return false;
        };
        pf.Save(&HSTRING::from(lnk.as_str()), true).is_ok()
    }
}

/// `--uninstall` without a decision: the in-window dialog asks whether to keep settings.
pub fn uninstall(keep: Option<bool>) -> i32 {
    test_hive();
    match keep {
        Some(k) => perform_uninstall(k),
        None => crate::window::run_uninstall(),
    }
}

/// Removes the registration, the uninstall entry, the shortcut, the install folder and, unless
/// `keep_settings`, the data folders. From inside the install folder a temp-copy helper deletes
/// the folder after this process exits.
pub fn perform_uninstall(keep_settings: bool) -> i32 {
    crate::shell::remove(&reg::uninstall_plan());
    crate::shell::notify_assoc_changed();
    let _ = std::fs::remove_file(shortcut_path());
    if !keep_settings {
        for d in [env("APPDATA"), env("LOCALAPPDATA")] {
            if !d.is_empty() {
                let _ = std::fs::remove_dir_all(format!("{}\\YAMDV", d.trim_end_matches('\\')));
            }
        }
    }
    let dir = install_dir();
    if dir.is_empty() {
        return 0;
    }
    if running_installed() {
        let helper = format!(
            "{}\\yamdv-uninstall-{}.exe",
            env("TEMP").trim_end_matches('\\'),
            std::process::id()
        );
        if std::fs::copy(crate::shell::exe_path(), &helper).is_ok()
            && crate::shell::spawn(
                &helper,
                &["--uninstall-helper".into(), std::process::id().to_string()],
            )
        {
            say("uninstalled; the install folder is removed after exit");
            return 0;
        }
        return fail("cannot start the uninstall helper");
    }
    let _ = std::fs::remove_dir_all(&dir);
    say("uninstalled");
    0
}

/// `--uninstall-helper <pid>` (temp copy): waits for the parent to exit, then deletes the
/// install folder. The folder is recomputed here, never taken from the command line.
pub fn uninstall_helper(pid: u32) -> i32 {
    use windows::Win32::Foundation::CloseHandle;
    use windows::Win32::System::Threading::{
        OpenProcess, PROCESS_SYNCHRONIZE, WaitForSingleObject,
    };
    // SAFETY: SYNCHRONIZE-only handle to the parent; closed after the wait.
    unsafe {
        if let Ok(h) = OpenProcess(PROCESS_SYNCHRONIZE, false, pid) {
            let _ = WaitForSingleObject(h, 30_000);
            let _ = CloseHandle(h);
        }
    }
    let dir = install_dir();
    if dir.is_empty() || !dir.to_ascii_lowercase().ends_with("\\programs\\yamdv") {
        return 1;
    }
    for _ in 0..10 {
        if std::fs::remove_dir_all(&dir).is_ok() || !std::path::Path::new(&dir).exists() {
            return 0;
        }
        std::thread::sleep(std::time::Duration::from_millis(300));
    }
    1
}
