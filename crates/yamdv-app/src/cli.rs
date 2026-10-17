//! Command-line parsing and the only module allowed to produce CLI text (APPENDIX F §F.4).
//! `--help`, `--version` and file paths; builds with the `dev-tools` feature additionally accept the
//! developer flags (`--render-png`, `--width`, `--scale`, `--theme`, `--autoscroll`, `--perf-log`,
//! `--mem-stats`, `--script`). Every other flag is a usage error (exit code 2).

/// Product version.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// What the process should do.
#[derive(Debug, Clone, PartialEq)]
pub enum Command {
    /// Open one window per path (R-OPEN-02), or one empty window.
    Run {
        /// Positional arguments (file paths).
        paths: Vec<String>,
        /// Developer flags (empty unless built with `dev-tools`).
        dev: Dev,
    },
    /// Internal: `--dialog-host open <owner-hwnd> <nonce>` (S-PROC-07; not shown in `--help`).
    DialogHost {
        /// `open` or `font`.
        kind: String,
        /// Owner window handle (decimal).
        owner: isize,
        /// Per-launch nonce (decimal).
        nonce: u64,
    },
    /// `--register` (R-SYS-01; explicit action, S-REG-03).
    Register,
    /// `--unregister` (S-REG-02).
    Unregister,
    /// `--set-default` (R-SYS-02).
    SetDefault,
    /// `--context-menu on|off` (R-SYS-03).
    ContextMenu(bool),
    /// Print help and exit 0.
    Help,
    /// Print version and exit 0.
    Version,
    /// Print the message to stderr and exit 2.
    Usage(String),
}

/// Developer flags (feature `dev-tools`).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Dev {
    /// `--render-png <out.png>`.
    pub render_png: Option<String>,
    /// `--width <dip>` (default 1000).
    pub width: Option<f32>,
    /// `--scale <f>` (default 1.0).
    pub scale: Option<f32>,
    /// `--theme light|dark`.
    pub dark: bool,
    /// `--autoscroll`.
    pub autoscroll: bool,
    /// `--perf-log <file>`.
    pub perf_log: Option<String>,
    /// `--mem-stats <file>`.
    pub mem_stats: Option<String>,
    /// `--script <file>` (in-window test driver).
    pub script: Option<String>,
    /// `--block <n>` with `--render-png`: render only top-level block `n` (prompt 007 §4).
    pub block: Option<usize>,
    /// `--line <n>`: scroll the (first) document to source line `n` (not a dev flag).
    pub line: Option<u32>,
}

/// Whether developer flags are accepted (compile-time).
pub const DEV_TOOLS: bool = cfg!(feature = "dev-tools");

fn usage(s: &str) -> Command {
    Command::Usage(format!(
        "yamdv: unknown option '{}'\nTry 'yamdv --help' for more information.\n",
        s.chars().take(64).collect::<String>()
    ))
}

/// Exit code for usage errors.
pub const EXIT_USAGE: i32 = 2;

/// Parse arguments (without the program name).
pub fn parse<I: IntoIterator<Item = String>>(args: I) -> Command {
    parse_with(args, DEV_TOOLS)
}

/// Parse with developer flags enabled or not (testable on every build).
pub fn parse_with<I: IntoIterator<Item = String>>(args: I, dev_tools: bool) -> Command {
    let args: Vec<String> = args.into_iter().collect();
    if args.first().map(String::as_str) == Some("--dialog-host") {
        // Exactly `--dialog-host open <owner> <nonce>`; anything else is a usage error (exit 2).
        return match args.as_slice() {
            [_, k, o, n] if k == "open" || k == "font" || k == "save" => {
                match (o.parse::<isize>(), n.parse::<u64>()) {
                    (Ok(owner), Ok(nonce)) if owner != 0 => Command::DialogHost {
                        kind: k.clone(),
                        owner,
                        nonce,
                    },
                    _ => usage("--dialog-host"),
                }
            }
            _ => usage("--dialog-host"),
        };
    }
    let mut paths = Vec::new();
    let mut only_paths = false;
    let mut dev = Dev::default();
    let mut it = args.into_iter();
    while let Some(a) = it.next() {
        if only_paths {
            paths.push(a);
            continue;
        }
        match a.as_str() {
            "--help" | "-h" | "/?" => return Command::Help,
            "--version" | "-V" => return Command::Version,
            "--" => only_paths = true,
            "--register" => return Command::Register,
            "--unregister" => return Command::Unregister,
            "--set-default" => return Command::SetDefault,
            "--context-menu" => {
                return match it.next().as_deref() {
                    Some("on") => Command::ContextMenu(true),
                    Some("off") => Command::ContextMenu(false),
                    _ => usage(&a),
                };
            }
            "--line" => match it.next().and_then(|v| v.parse::<u32>().ok()) {
                Some(n) if n > 0 => dev.line = Some(n),
                _ => return usage(&a),
            },
            "--autoscroll" if dev_tools => dev.autoscroll = true,
            "--block" if dev_tools => match it.next().and_then(|v| v.parse::<usize>().ok()) {
                Some(n) => dev.block = Some(n),
                None => return usage(&a),
            },
            "--render-png" | "--perf-log" | "--mem-stats" | "--script" | "--width" | "--scale"
            | "--theme"
                if dev_tools =>
            {
                let Some(v) = it.next() else {
                    return Command::Usage(format!("yamdv: {a} needs a value\n"));
                };
                match a.as_str() {
                    "--render-png" => dev.render_png = Some(v),
                    "--perf-log" => dev.perf_log = Some(v),
                    "--mem-stats" => dev.mem_stats = Some(v),
                    "--script" => dev.script = Some(v),
                    "--width" => match v.parse::<f32>() {
                        Ok(w) if (100.0..=8000.0).contains(&w) => dev.width = Some(w),
                        _ => return usage(&a),
                    },
                    "--scale" => match v.parse::<f32>() {
                        Ok(s) if (0.5..=4.0).contains(&s) => dev.scale = Some(s),
                        _ => return usage(&a),
                    },
                    _ => match v.as_str() {
                        "light" => dev.dark = false,
                        "dark" => dev.dark = true,
                        _ => return usage(&a),
                    },
                }
            }
            s if s.starts_with('-') && s.len() > 1 => return usage(s),
            _ => paths.push(a),
        }
    }
    Command::Run { paths, dev }
}

/// `--version` text.
pub fn version_text() -> String {
    format!("yamdv {VERSION}\n")
}

/// `--help` text.
pub fn help_text() -> String {
    format!(
        "YAMDV {VERSION} - Yet Another Markdown Viewer\n\n\
         Usage:\n  yamdv [<file> ...]   open each file in its own window\n  \
         yamdv --line <n> <file>  open the file at source line n\n  \
         yamdv --register     register YAMDV for Markdown files (current user)\n  \
         yamdv --unregister   remove that registration\n  \
         yamdv --set-default  register and open Windows default-app settings\n  \
         yamdv --context-menu on|off  \"Open with YAMDV\" in Explorer\n  \
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
    fn r_sys_01_registration_flags() {
        assert_eq!(p(&["--register"]), Command::Register);
        assert_eq!(p(&["--unregister"]), Command::Unregister);
        assert_eq!(p(&["--set-default"]), Command::SetDefault);
        assert_eq!(p(&["--context-menu", "on"]), Command::ContextMenu(true));
        assert_eq!(p(&["--context-menu", "off"]), Command::ContextMenu(false));
        assert!(matches!(p(&["--context-menu", "x"]), Command::Usage(_)));
        match p(&["--line", "12", "a.md"]) {
            Command::Run { paths, dev } => {
                assert_eq!(paths, vec!["a.md"]);
                assert_eq!(dev.line, Some(12));
            }
            c => panic!("{c:?}"),
        }
        assert!(matches!(p(&["--line", "0", "a.md"]), Command::Usage(_)));
        assert!(help_text().contains("--register"));
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
    fn dialog_host_flag() {
        assert_eq!(
            p(&["--dialog-host", "open", "1234", "99"]),
            Command::DialogHost {
                kind: "open".into(),
                owner: 1234,
                nonce: 99
            }
        );
        for bad in [
            &["--dialog-host"][..],
            &["--dialog-host", "print", "1", "2"],
            &["--dialog-host", "open", "0", "2"],
            &["--dialog-host", "open", "x", "2"],
            &["--dialog-host", "open", "1", "2", "extra"],
            &["a.md", "--dialog-host", "open", "1", "2"],
        ] {
            assert!(matches!(p(bad), Command::Usage(_)), "{bad:?}");
        }
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
        let run = |paths: Vec<String>| Command::Run {
            paths,
            dev: Dev::default(),
        };
        assert_eq!(parse_with(Vec::<String>::new(), false), run(vec![]));
        assert_eq!(
            parse_with(["a.md", "--", "--weird.md"].map(String::from), false),
            run(vec!["a.md".into(), "--weird.md".into()])
        );
        assert_eq!(
            parse_with(["-"].map(String::from), false),
            run(vec!["-".into()])
        );
        assert_eq!(
            parse_with(["a.md", "b.md"].map(String::from), false),
            run(vec!["a.md".into(), "b.md".into()])
        );
    }

    #[test]
    fn dev_flags_only_with_dev_tools() {
        let a = [
            "--render-png",
            "o.png",
            "--width",
            "1000",
            "--scale",
            "1.5",
            "--theme",
            "dark",
            "x.md",
        ]
        .map(String::from);
        assert!(matches!(parse_with(a.clone(), false), Command::Usage(_)));
        match parse_with(a, true) {
            Command::Run { paths, dev } => {
                assert_eq!(paths, vec!["x.md".to_string()]);
                assert_eq!(dev.render_png.as_deref(), Some("o.png"));
                assert_eq!(dev.width, Some(1000.0));
                assert_eq!(dev.scale, Some(1.5));
                assert!(dev.dark);
            }
            other => panic!("unexpected {other:?}"),
        }
        assert!(matches!(
            parse_with(["--scale", "9"].map(String::from), true),
            Command::Usage(_)
        ));
    }
}
