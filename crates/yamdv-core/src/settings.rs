//! Settings model (Appendix B §B.8.2, including patch P-08b): every key typed, defaulted and
//! clamped; unknown keys ignored. Phase 002 only **reads** `settings.ini` (writer exists for 004).

use crate::ini::Ini;
use crate::settings_table::DEFS;

/// Value type and range of a setting.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Kind {
    /// `true` / `false`.
    Bool,
    /// Integer clamped to `min..=max`.
    Int {
        /// Lower bound.
        min: i64,
        /// Upper bound.
        max: i64,
    },
    /// Float clamped to `min..=max`.
    Float {
        /// Lower bound.
        min: f64,
        /// Upper bound.
        max: f64,
    },
    /// One of the listed values (ASCII case-insensitive).
    Enum(&'static [&'static str]),
    /// Free string.
    Str,
}

/// Definition of one key.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Def {
    /// `section.key`.
    pub key: &'static str,
    /// Type and range.
    pub kind: Kind,
    /// Default value in INI syntax.
    pub default: &'static str,
}

/// A typed value.
#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    /// Boolean.
    Bool(bool),
    /// Integer.
    Int(i64),
    /// Float.
    Float(f64),
    /// Enum value (canonical spelling from the definition) or string.
    Str(String),
}

/// Definition of `key` (`section.key`).
pub fn def(key: &str) -> Option<&'static Def> {
    DEFS.iter().find(|d| d.key == key)
}

/// Normalizes `raw` for `def`: `None` when the value cannot be interpreted (→ default).
pub fn coerce(def: &Def, raw: &str) -> Option<Value> {
    let raw = raw.trim();
    match def.kind {
        Kind::Bool => match raw.to_ascii_lowercase().as_str() {
            "true" | "1" | "yes" | "on" => Some(Value::Bool(true)),
            "false" | "0" | "no" | "off" => Some(Value::Bool(false)),
            _ => None,
        },
        Kind::Int { min, max } => raw
            .parse::<i64>()
            .ok()
            .map(|v| Value::Int(v.clamp(min, max))),
        Kind::Float { min, max } => raw
            .parse::<f64>()
            .ok()
            .filter(|v| v.is_finite())
            .map(|v| Value::Float(v.clamp(min, max))),
        Kind::Enum(vals) => vals
            .iter()
            .find(|v| v.eq_ignore_ascii_case(raw))
            .map(|v| Value::Str((*v).to_string())),
        Kind::Str => Some(Value::Str(raw.to_string())),
    }
}

/// Resolved settings: one value per definition.
#[derive(Debug, Clone, PartialEq)]
pub struct Settings {
    values: Vec<Value>,
}

impl Default for Settings {
    fn default() -> Self {
        Self::from_ini(&Ini::default())
    }
}

impl Settings {
    /// Resolves every definition from `ini` (defaults for missing/invalid values).
    pub fn from_ini(ini: &Ini) -> Self {
        let values = DEFS
            .iter()
            .map(|d| {
                let (sec, key) = d.key.split_once('.').unwrap_or(("", d.key));
                ini.get(sec, key)
                    .and_then(|raw| coerce(d, raw))
                    .or_else(|| coerce(d, d.default))
                    .unwrap_or(Value::Str(d.default.to_string()))
            })
            .collect();
        Self { values }
    }

    /// Value of `key` (`section.key`).
    pub fn get(&self, key: &str) -> Option<&Value> {
        DEFS.iter()
            .position(|d| d.key == key)
            .and_then(|i| self.values.get(i))
    }

    /// Boolean setting (false when unknown).
    pub fn bool(&self, key: &str) -> bool {
        matches!(self.get(key), Some(Value::Bool(true)))
    }

    /// Integer setting (0 when unknown).
    pub fn int(&self, key: &str) -> i64 {
        match self.get(key) {
            Some(Value::Int(v)) => *v,
            _ => 0,
        }
    }

    /// Float setting (0.0 when unknown).
    pub fn float(&self, key: &str) -> f64 {
        match self.get(key) {
            Some(Value::Float(v)) => *v,
            _ => 0.0,
        }
    }

    /// String/enum setting ("" when unknown).
    pub fn str(&self, key: &str) -> &str {
        match self.get(key) {
            Some(Value::Str(v)) => v,
            _ => "",
        }
    }

    /// Sets `key` from its textual form (coerced and clamped like a file value); `false` when the
    /// key is unknown or the value invalid.
    pub fn set(&mut self, key: &str, raw: &str) -> bool {
        let Some(i) = DEFS.iter().position(|d| d.key == key) else {
            return false;
        };
        let (Some(d), Some(slot)) = (DEFS.get(i), self.values.get_mut(i)) else {
            return false;
        };
        match coerce(d, raw) {
            Some(v) => {
                *slot = v;
                true
            }
            None => false,
        }
    }

    /// Writes every value into `ini` (round-trip safe; used from Phase 004).
    pub fn write_into(&self, ini: &mut Ini) {
        for (d, v) in DEFS.iter().zip(&self.values) {
            let (sec, key) = d.key.split_once('.').unwrap_or(("", d.key));
            let s = match v {
                Value::Bool(b) => b.to_string(),
                Value::Int(i) => i.to_string(),
                Value::Float(f) => f.to_string(),
                Value::Str(s) => s.clone(),
            };
            ini.set(sec, key, &s);
        }
    }
}

/// Where settings and state live (R-SET-05).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DataDirs {
    /// `%APPDATA%\YAMDV` (config) and `%LOCALAPPDATA%\YAMDV` (logs, recovery).
    Installed {
        /// Roaming config directory.
        config: String,
        /// Local data directory.
        local: String,
    },
    /// `yamdv.portable` beside the exe → everything in `<exe dir>\YAMDV-data`.
    Portable {
        /// The single data directory.
        dir: String,
    },
}

/// Resolves the data directories. `portable_marker_exists` tells whether `yamdv.portable` exists
/// beside the executable in `exe_dir`.
pub fn resolve_dirs(
    exe_dir: &str,
    portable_marker_exists: bool,
    appdata: &str,
    localappdata: &str,
) -> DataDirs {
    let join = |a: &str, b: &str| format!("{}\\{b}", a.trim_end_matches(['\\', '/']));
    if portable_marker_exists {
        DataDirs::Portable {
            dir: join(exe_dir, "YAMDV-data"),
        }
    } else {
        DataDirs::Installed {
            config: join(appdata, "YAMDV"),
            local: join(localappdata, "YAMDV"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_default_is_valid() {
        for d in DEFS {
            assert!(
                coerce(d, d.default).is_some(),
                "{} default {:?}",
                d.key,
                d.default
            );
        }
        assert!(DEFS.iter().any(|d| d.key == "reading.subscript_tilde"));
    }

    #[test]
    fn s_file_07_clamp_and_ignore() {
        let ini = Ini::parse(b"[typography]\nfont_size = 99\nline_height = 0.5\ncolumn_width_pct = abc\n[appearance]\nscheme = GitHub\nmode = purple\n[x]\nunknown = 1\n");
        let s = Settings::from_ini(&ini);
        assert_eq!(s.int("typography.font_size"), 32);
        assert!((s.float("typography.line_height") - 1.2).abs() < 1e-9);
        assert_eq!(s.int("typography.column_width_pct"), 65);
        assert_eq!(s.str("appearance.scheme"), "github");
        assert_eq!(s.str("appearance.mode"), "system");
        assert!(s.get("x.unknown").is_none());
        assert!(!s.bool("reading.subscript_tilde"));
    }

    #[test]
    fn writer_round_trip() {
        let mut ini = Ini::parse(b"# mine\n[general]\nlanguage = tr\n");
        let s = Settings::from_ini(&ini);
        s.write_into(&mut ini);
        let back = Settings::from_ini(&Ini::parse(ini.write().as_bytes()));
        assert_eq!(back, s);
        assert!(
            ini.write()
                .starts_with("# mine\n[general]\nlanguage = tr\n")
        );
    }

    #[test]
    fn r_set_05_portable() {
        assert_eq!(
            resolve_dirs("C:\\Tools\\", true, "A", "L"),
            DataDirs::Portable {
                dir: "C:\\Tools\\YAMDV-data".into()
            }
        );
        assert_eq!(
            resolve_dirs(
                "C:\\Tools",
                false,
                "C:\\Users\\u\\AppData\\Roaming",
                "C:\\L"
            ),
            DataDirs::Installed {
                config: "C:\\Users\\u\\AppData\\Roaming\\YAMDV".into(),
                local: "C:\\L\\YAMDV".into()
            }
        );
    }
}
