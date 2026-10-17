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
