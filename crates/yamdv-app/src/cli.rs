//! Command-line parsing and the only module allowed to produce CLI text (APPENDIX F §F.4).
//! Phase 001 supports `--help` and `--version`; every other flag is a usage error (exit code 2).

/// Product version.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// What the process should do.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Command {
    /// Open the window. Positional paths are accepted but not opened yet.
    /// PHASE-002: open the given files (R-OPEN-02).
    Run {
        /// Positional arguments (file paths).
        paths: Vec<String>,
    },
    /// Print help and exit 0.
    Help,
    /// Print version and exit 0.
    Version,
    /// Print the message to stderr and exit 2.
    Usage(String),
}

/// Exit code for usage errors.
pub const EXIT_USAGE: i32 = 2;

/// Parse arguments (without the program name).
pub fn parse<I: IntoIterator<Item = String>>(args: I) -> Command {
    let mut paths = Vec::new();
    let mut only_paths = false;
    for a in args {
        if only_paths {
            paths.push(a);
            continue;
        }
        match a.as_str() {
            "--help" | "-h" | "/?" => return Command::Help,
            "--version" | "-V" => return Command::Version,
            "--" => only_paths = true,
            s if s.starts_with('-') && s.len() > 1 => {
                return Command::Usage(format!(
                    "yamdv: unknown option '{}'\nTry 'yamdv --help' for more information.\n",
                    s.chars().take(64).collect::<String>()
                ));
            }
            _ => paths.push(a),
        }
    }
    Command::Run { paths }
}

/// `--version` text.
pub fn version_text() -> String {
    format!("yamdv {VERSION}\n")
}

/// `--help` text.
pub fn help_text() -> String {
    format!(
        "YAMDV {VERSION} - Yet Another Markdown Viewer\n\n\
         Usage:\n  yamdv [<file> ...]   open files (file opening arrives in a later phase)\n  \
         yamdv --help         show this help\n  yamdv --version      show the version\n\n\
         Exit codes: 0 success, 1 runtime error, 2 usage error.\n"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(a: &[&str]) -> Command {
        parse(a.iter().map(|s| (*s).to_string()))
    }

    #[test]
    fn r_sys_04_help_and_version() {
        assert_eq!(p(&["--help"]), Command::Help);
        assert_eq!(p(&["-h"]), Command::Help);
        assert_eq!(p(&["--version"]), Command::Version);
        assert!(version_text().starts_with("yamdv 0."));
        assert!(help_text().contains("--version"));
    }

    #[test]
    fn r_sys_04_unknown_flag_is_usage_error() {
        match p(&["--bogus"]) {
            Command::Usage(m) => assert!(m.contains("--bogus")),
            other => panic!("unexpected {other:?}"),
        }
    }

    #[test]
    fn positional_paths_and_double_dash() {
        assert_eq!(p(&[]), Command::Run { paths: vec![] });
        assert_eq!(
            p(&["a.md", "--", "--weird.md"]),
            Command::Run {
                paths: vec!["a.md".into(), "--weird.md".into()]
            }
        );
        assert_eq!(
            p(&["-"]),
            Command::Run {
                paths: vec!["-".into()]
            }
        );
    }
}
