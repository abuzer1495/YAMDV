//! Per-user registration plan (R-SYS-01, R-SYS-03, S-REG-01…03): every key and value YAMDV
//! writes, listed in one place, so `--unregister` removes exactly these. Pure; the platform layer
//! writes the plan only on an explicit user action.

/// Markdown extensions (R-SYS-01).
pub const EXTS: [&str; 8] = [
    ".md",
    ".markdown",
    ".mdown",
    ".mkd",
    ".mkdn",
    ".mdwn",
    ".mdtxt",
    ".mdtext",
];
/// ProgID.
pub const PROGID: &str = "YAMDV.Markdown";

/// One registry value under HKCU (`name` empty = default value).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Value {
    /// Key path relative to HKCU.
    pub key: String,
    /// Value name.
    pub name: String,
    /// String data (`REG_SZ`).
    pub data: String,
}

fn v(key: &str, name: &str, data: &str) -> Value {
    Value {
        key: key.to_string(),
        name: name.to_string(),
        data: data.to_string(),
    }
}

/// Values written by `--register` / "Set YAMDV as default app…" (R-SYS-01).
pub fn register_plan(exe: &str) -> Vec<Value> {
    let open = format!("\"{exe}\" \"%1\"");
    let icon = format!("\"{exe}\",0");
    let mut out = vec![
        v("Software\\Classes\\YAMDV.Markdown", "", "Markdown document"),
        v("Software\\Classes\\YAMDV.Markdown\\DefaultIcon", "", &icon),
        v(
            "Software\\Classes\\YAMDV.Markdown\\shell\\open\\command",
            "",
            &open,
        ),
        v(
            "Software\\Classes\\Applications\\yamdv.exe",
            "FriendlyAppName",
            "YAMDV",
        ),
        v(
            "Software\\Classes\\Applications\\yamdv.exe\\shell\\open\\command",
            "",
            &open,
        ),
        v("Software\\YAMDV\\Capabilities", "ApplicationName", "YAMDV"),
        v(
            "Software\\YAMDV\\Capabilities",
            "ApplicationDescription",
            "Markdown viewer",
        ),
        v(
            "Software\\RegisteredApplications",
            "YAMDV",
            "Software\\YAMDV\\Capabilities",
        ),
    ];
    for e in EXTS {
        out.push(v(
            &format!("Software\\Classes\\{e}\\OpenWithProgids"),
            PROGID,
            "",
        ));
        out.push(v(
            "Software\\YAMDV\\Capabilities\\FileAssociations",
            e,
            PROGID,
        ));
        out.push(v(
            "Software\\Classes\\Applications\\yamdv.exe\\SupportedTypes",
            e,
            "",
        ));
    }
    out
}

/// Values of the Explorer context-menu verb (R-SYS-03).
pub fn context_menu_plan(exe: &str, label: &str) -> Vec<Value> {
    let open = format!("\"{exe}\" \"%1\"");
    let mut out = Vec::new();
    for e in EXTS {
        let k = format!("Software\\Classes\\SystemFileAssociations\\{e}\\shell\\YAMDV");
        out.push(v(&k, "", label));
        out.push(v(&k, "Icon", &format!("\"{exe}\",0")));
        out.push(v(&format!("{k}\\command"), "", &open));
    }
    out
}

/// Uninstall entry key (R-SYS-05, "Apps & features").
pub const UNINSTALL_KEY: &str = "Software\\Microsoft\\Windows\\CurrentVersion\\Uninstall\\YAMDV";

/// Values of the uninstall entry written by `--install` (R-SYS-05).
/// (`EstimatedSize` is a `REG_DWORD` and written separately by the platform layer.)
pub fn uninstall_entry_plan(exe: &str, dir: &str, version: &str) -> Vec<Value> {
    vec![
        v(UNINSTALL_KEY, "DisplayName", "YAMDV"),
        v(UNINSTALL_KEY, "DisplayIcon", &format!("\"{exe}\",0")),
        v(UNINSTALL_KEY, "DisplayVersion", version),
        v(UNINSTALL_KEY, "Publisher", "YAMDV"),
        v(UNINSTALL_KEY, "InstallLocation", dir),
        v(
            UNINSTALL_KEY,
            "UninstallString",
            &format!("\"{exe}\" --uninstall"),
        ),
        v(UNINSTALL_KEY, "NoModify", "1"),
        v(UNINSTALL_KEY, "NoRepair", "1"),
    ]
}

/// Install folder below `%LOCALAPPDATA%` (R-SYS-05).
pub fn install_dir(local_app_data: &str) -> String {
    format!("{}\\Programs\\YAMDV", local_app_data.trim_end_matches('\\'))
}

/// `true` when `exe` lives in the install folder (case-insensitive path compare).
pub fn is_installed_path(exe: &str, local_app_data: &str) -> bool {
    let want = format!("{}\\yamdv.exe", install_dir(local_app_data)).to_lowercase();
    !local_app_data.is_empty() && exe.to_lowercase() == want
}

/// What removal does with a key.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Removal {
    /// Delete the whole key tree (owned by YAMDV).
    Tree(String),
    /// Delete one value from a shared key.
    Value(String, String),
}

/// Exactly the removals for `--unregister` (S-REG-02); shared keys only lose our values.
pub fn unregister_plan() -> Vec<Removal> {
    let mut out = vec![
        Removal::Tree("Software\\Classes\\YAMDV.Markdown".into()),
        Removal::Tree("Software\\Classes\\Applications\\yamdv.exe".into()),
        Removal::Tree("Software\\YAMDV\\Capabilities".into()),
        Removal::Value("Software\\RegisteredApplications".into(), "YAMDV".into()),
    ];
    for e in EXTS {
        out.push(Removal::Value(
            format!("Software\\Classes\\{e}\\OpenWithProgids"),
            PROGID.into(),
        ));
    }
    out.extend(context_menu_removal());
    out
}

/// `--uninstall` removals: the registration plus the uninstall entry (S-REG-02).
pub fn uninstall_plan() -> Vec<Removal> {
    let mut out = unregister_plan();
    out.push(Removal::Tree(UNINSTALL_KEY.into()));
    out
}

/// Removal of the context-menu verb only.
pub fn context_menu_removal() -> Vec<Removal> {
    EXTS.iter()
        .map(|e| {
            Removal::Tree(format!(
                "Software\\Classes\\SystemFileAssociations\\{e}\\shell\\YAMDV"
            ))
        })
        .collect()
}

/// S-REG-01 allowlist check for a key path.
pub fn allowed_key(key: &str) -> bool {
    let k = key.to_ascii_lowercase();
    let ok = [
        "software\\classes\\yamdv.markdown",
        "software\\classes\\applications\\yamdv.exe",
        "software\\yamdv\\capabilities",
        "software\\registeredapplications",
        "software\\microsoft\\windows\\currentversion\\uninstall\\yamdv",
    ];
    ok.iter()
        .any(|p| k == *p || k.starts_with(&format!("{p}\\")))
        || EXTS.iter().any(|e| {
            k == format!("software\\classes\\{e}\\openwithprogids")
                || k.starts_with(&format!(
                    "software\\classes\\systemfileassociations\\{e}\\shell\\yamdv"
                ))
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn r_sys_05_uninstall_entry_and_install_dir() {
        let dir = install_dir("C:\\Users\\u\\AppData\\Local\\");
        assert_eq!(dir, "C:\\Users\\u\\AppData\\Local\\Programs\\YAMDV");
        let exe = format!("{dir}\\yamdv.exe");
        assert!(is_installed_path(
            &exe.to_uppercase(),
            "C:\\Users\\u\\AppData\\Local"
        ));
        assert!(!is_installed_path("D:\\tools\\yamdv.exe", "C:\\x"));
        assert!(!is_installed_path(&exe, ""));
        let plan = uninstall_entry_plan(&exe, &dir, "1.0.0");
        assert!(plan.iter().all(|v| allowed_key(&v.key)));
        let un = plan
            .iter()
            .find(|v| v.name == "UninstallString")
            .map(|v| v.data.clone());
        assert_eq!(un, Some(format!("\"{exe}\" --uninstall")));
        assert!(uninstall_plan().contains(&Removal::Tree(UNINSTALL_KEY.into())));
        assert!(!allowed_key(
            "Software\\Microsoft\\Windows\\CurrentVersion\\Uninstall\\Other"
        ));
    }

    #[test]
    fn s_reg_01_every_write_is_allowlisted() {
        let exe = "C:\\Users\\u\\AppData\\Local\\Programs\\YAMDV\\yamdv.exe";
        for x in register_plan(exe)
            .iter()
            .chain(&context_menu_plan(exe, "Open with YAMDV"))
        {
            assert!(allowed_key(&x.key), "{}", x.key);
            assert!(!x.key.to_ascii_lowercase().contains("userchoice"));
        }
        assert!(!allowed_key("Software\\Classes\\.md"));
        assert!(!allowed_key(
            "Software\\Microsoft\\Windows\\CurrentVersion\\Explorer\\FileExts\\.md\\UserChoice"
        ));
    }

    #[test]
    fn s_reg_02_unregister_covers_every_written_key() {
        let exe = "C:\\y\\yamdv.exe";
        let rm = unregister_plan();
        for x in register_plan(exe)
            .iter()
            .chain(&context_menu_plan(exe, "x"))
        {
            let covered = rm.iter().any(|r| match r {
                Removal::Tree(t) => x.key == *t || x.key.starts_with(&format!("{t}\\")),
                Removal::Value(k, n) => x.key == *k && x.name == *n,
            });
            assert!(covered, "{} / {}", x.key, x.name);
        }
    }
}
