//! In-window test driver scripts (prompt 004 §4.5; dev-tools builds only use this).
//!
//! One command per line, `#` comments. Commands: `click x y`, `click-link "<text>" [ctrl|middle]`,
//! `key <chord>` (e.g. `Tab`, `shift+Tab`, `alt+Left`, `ctrl+W`), `wheel <delta>`, `wait-idle`,
//! `assert <field> <op> <value>` (`==`, `!=`, `>`, `<`, `>=`, `<=`, `contains`), `dump`.

/// Key chord.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Chord {
    /// Virtual-key code.
    pub vk: u16,
    /// Ctrl held.
    pub ctrl: bool,
    /// Shift held.
    pub shift: bool,
    /// Alt held.
    pub alt: bool,
}

/// Comparison operator.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Op {
    /// `==`
    Eq,
    /// `!=`
    Ne,
    /// `>`
    Gt,
    /// `<`
    Lt,
    /// `>=`
    Ge,
    /// `<=`
    Le,
    /// substring
    Contains,
    /// no such substring (`!contains`)
    Lacks,
}

/// One script command.
#[derive(Debug, Clone, PartialEq)]
pub enum Cmd {
    /// Left click at client (logical) coordinates.
    Click(f32, f32),
    /// Click the first link whose text contains the string; `ctrl` / middle-click opens a window.
    ClickLink(String, bool),
    /// Key press.
    Key(Chord),
    /// Wheel delta (120 = one notch).
    Wheel(i16),
    /// Finish animations and pending layout.
    WaitIdle,
    /// Compare a state field.
    Assert(String, Op, String),
    /// Write the state dump to the log.
    Dump,
    /// Run a context-menu command by name (`mode dark`, `scheme glacier`, `favorite`, …).
    Command(String),
    /// Type text (WM_CHAR): the find bar, or the editor in F2/F3 (one character per step, the
    /// key → caret-painted latency is recorded, T5).
    Type(String),
    /// Click the middle of the first rendered occurrence of the text (F2 block entry).
    ClickText(String),
    /// Click the task checkbox of the list item containing the text.
    ClickTask(String),
    /// Click the "Render anyway" button of the diagram block containing the text.
    ClickRender(String),
    /// Click the first sidebar row whose text contains the string.
    SideClick(String),
    /// Write a file relative to the script folder (`\n` = LF, `\r` = CR).
    WriteFile(String, String),
    /// Open a file relative to the script folder in this window.
    Open(String),
    /// Start a second `yamdv.exe` with a path relative to the script folder.
    Launch(String),
    /// Wait for the message loop (watchers, IPC) for `ms` milliseconds.
    Sleep(u32),
    /// Render the whole client area to a PNG (`YAMDV_SNAPSHOT_DIR` or the script folder).
    Snapshot(String),
    /// Resize the client area to `w × h` DIPs.
    Resize(f32, f32),
    /// Export the document as HTML to a file relative to the script folder (R-EXP-02); the
    /// `out_*` state fields describe the written file.
    ExportHtml(String),
    /// Export the first table of the document as CSV to a file (R-EXP-04); `out_*` fields.
    ExportCsv(String),
    /// Window chrome / popup interaction (V2-A): `ui <verb> [arg]` — `ui click more`,
    /// `ui dblclick-caption`, `ui hover 2`, `ui popup-key down`, `ui popup-hover 3`,
    /// `ui popup-click 3`, `ui popup-shot menu.png`, `ui shot-window light.png`.
    Ui(String, String),
}

/// Virtual-key code for a key name.
pub fn vk(name: &str) -> Option<u16> {
    let n = name.to_ascii_lowercase();
    Some(match n.as_str() {
        "backspace" => 0x08,
        "delete" | "del" => 0x2E,
        "`" | "backquote" => 0xC0,
        "tab" => 0x09,
        "enter" | "return" => 0x0D,
        "esc" | "escape" => 0x1B,
        "space" => 0x20,
        "pageup" => 0x21,
        "pagedown" => 0x22,
        "end" => 0x23,
        "home" => 0x24,
        "left" => 0x25,
        "up" => 0x26,
        "right" => 0x27,
        "down" => 0x28,
        "," | "comma" => 0xBC,
        "." | "period" => 0xBE,
        "/" | "slash" => 0xBF,
        "-" | "minus" => 0xBD,
        "=" | "equals" => 0xBB,
        f if f.len() >= 2
            && f.starts_with('f')
            && f[1..].parse::<u16>().is_ok_and(|k| (1..=12).contains(&k)) =>
        {
            0x6F + f[1..].parse::<u16>().unwrap_or(1)
        }
        _ => match n.as_bytes() {
            [c] if c.is_ascii_alphanumeric() => u16::from(c.to_ascii_uppercase()),
            _ => return None,
        },
    })
}

/// Parse a chord such as `ctrl+shift+C`.
pub fn chord(s: &str) -> Option<Chord> {
    let mut c = Chord {
        vk: 0,
        ctrl: false,
        shift: false,
        alt: false,
    };
    let parts: Vec<&str> = s.split('+').collect();
    let (key, mods) = parts.split_last()?;
    for m in mods {
        match m.to_ascii_lowercase().as_str() {
            "ctrl" => c.ctrl = true,
            "shift" => c.shift = true,
            "alt" => c.alt = true,
            _ => return None,
        }
    }
    c.vk = vk(key)?;
    Some(c)
}

fn unquote(s: &str) -> String {
    let s = s.trim();
    s.strip_prefix('"')
        .and_then(|r| r.strip_suffix('"'))
        .unwrap_or(s)
        .to_string()
}

/// Parse one line; `Ok(None)` for blank/comment lines.
pub fn parse_line(line: &str) -> Result<Option<Cmd>, String> {
    let l = line.trim();
    if l.is_empty() || l.starts_with('#') {
        return Ok(None);
    }
    let (cmd, rest) = l.split_once(' ').unwrap_or((l, ""));
    let rest = rest.trim();
    let bad = || format!("bad command: {l}");
    Ok(Some(match cmd {
        "click" => {
            let mut it = rest.split_whitespace().map(str::parse::<f32>);
            match (it.next(), it.next()) {
                (Some(Ok(x)), Some(Ok(y))) => Cmd::Click(x, y),
                _ => return Err(bad()),
            }
        }
        "click-link" => {
            let (text, modi) = if let Some(r) = rest.strip_prefix('"') {
                let (t, m) = r.split_once('"').ok_or_else(bad)?;
                (t.to_string(), m.trim())
            } else {
                (rest.to_string(), "")
            };
            let new = match modi {
                "" => false,
                "ctrl" | "middle" => true,
                _ => return Err(bad()),
            };
            Cmd::ClickLink(text, new)
        }
        "key" => Cmd::Key(chord(rest).ok_or_else(bad)?),
        "wheel" => Cmd::Wheel(rest.parse().map_err(|_| bad())?),
        "wait-idle" => Cmd::WaitIdle,
        "command" if !rest.is_empty() => Cmd::Command(rest.to_string()),
        "type" => Cmd::Type(unquote(rest)),
        "side-click" => Cmd::SideClick(unquote(rest)),
        "click-text" if !rest.is_empty() => Cmd::ClickText(unquote(rest)),
        "click-task" if !rest.is_empty() => Cmd::ClickTask(unquote(rest)),
        "click-render" if !rest.is_empty() => Cmd::ClickRender(unquote(rest)),
        "write-file" => {
            let (f, t) = rest.split_once(' ').ok_or_else(bad)?;
            Cmd::WriteFile(
                f.to_string(),
                unquote(t)
                    .replace("\\n", "\n")
                    .replace("\\r", "\r")
                    .replace("\\ufeff", "\u{feff}"),
            )
        }
        "open" if !rest.is_empty() => Cmd::Open(unquote(rest)),
        "launch" if !rest.is_empty() => Cmd::Launch(unquote(rest)),
        "snapshot" if !rest.is_empty() => Cmd::Snapshot(unquote(rest)),
        "export-html" if !rest.is_empty() => Cmd::ExportHtml(unquote(rest)),
        "export-csv" if !rest.is_empty() => Cmd::ExportCsv(unquote(rest)),
        "resize" => {
            let mut it = rest.split_whitespace().map(str::parse::<f32>);
            match (it.next(), it.next()) {
                (Some(Ok(w)), Some(Ok(h))) => Cmd::Resize(w, h),
                _ => return Err(bad()),
            }
        }
        "ui" if !rest.is_empty() => {
            let (verb, arg) = rest.split_once(' ').unwrap_or((rest, ""));
            Cmd::Ui(verb.to_string(), unquote(arg))
        }
        "sleep" => Cmd::Sleep(rest.parse().map_err(|_| bad())?),
        "dump" => Cmd::Dump,
        "assert" => {
            let mut it = rest.splitn(3, ' ');
            let (f, o, v) = (it.next(), it.next(), it.next());
            let (Some(f), Some(o), Some(v)) = (f, o, v) else {
                return Err(bad());
            };
            let op = match o {
                "==" => Op::Eq,
                "!=" => Op::Ne,
                ">" => Op::Gt,
                "<" => Op::Lt,
                ">=" => Op::Ge,
                "<=" => Op::Le,
                "contains" => Op::Contains,
                "!contains" => Op::Lacks,
                _ => return Err(bad()),
            };
            Cmd::Assert(f.to_string(), op, unquote(v))
        }
        _ => return Err(bad()),
    }))
}

/// Parse a whole script.
pub fn parse(src: &str) -> Result<Vec<Cmd>, String> {
    let mut out = Vec::new();
    for (i, l) in src.lines().enumerate() {
        if let Some(c) = parse_line(l).map_err(|e| format!("line {}: {e}", i + 1))? {
            out.push(c);
        }
    }
    Ok(out)
}

/// Evaluate `actual op expected`; numeric when both parse as numbers.
pub fn check(actual: &str, op: Op, expected: &str) -> bool {
    if op == Op::Contains {
        return actual.contains(expected);
    }
    if op == Op::Lacks {
        return !actual.contains(expected);
    }
    match (actual.parse::<f64>(), expected.parse::<f64>()) {
        (Ok(a), Ok(e)) => match op {
            Op::Eq => (a - e).abs() < 0.5,
            Op::Ne => (a - e).abs() >= 0.5,
            Op::Gt => a > e,
            Op::Lt => a < e,
            Op::Ge => a >= e - 0.5,
            Op::Le => a <= e + 0.5,
            Op::Contains | Op::Lacks => false,
        },
        _ => match op {
            Op::Eq => actual == expected,
            Op::Ne => actual != expected,
            _ => false,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn export_commands_parse() {
        assert_eq!(
            parse_line("export-html out.tmp.html"),
            Ok(Some(Cmd::ExportHtml("out.tmp.html".into())))
        );
        assert_eq!(
            parse_line("export-csv \"t.tmp.csv\""),
            Ok(Some(Cmd::ExportCsv("t.tmp.csv".into())))
        );
        assert!(parse_line("export-html").is_err());
    }

    #[test]
    fn parses_commands() {
        let s = "# c\nclick 10 20\nclick-link \"Next page\" ctrl\nkey shift+Tab\nwheel -120\nwait-idle\nassert scroll_y > 100\nassert document == \"b.md\"\ndump\n";
        let c = parse(s).unwrap_or_default();
        assert_eq!(c.len(), 8);
        assert_eq!(c[0], Cmd::Click(10.0, 20.0));
        assert_eq!(c[1], Cmd::ClickLink("Next page".into(), true));
        assert_eq!(
            c[2],
            Cmd::Key(Chord {
                vk: 0x09,
                ctrl: false,
                shift: true,
                alt: false
            })
        );
        assert_eq!(c[5], Cmd::Assert("scroll_y".into(), Op::Gt, "100".into()));
        assert_eq!(c[6], Cmd::Assert("document".into(), Op::Eq, "b.md".into()));
    }

    #[test]
    fn parses_ui_commands() {
        assert_eq!(
            parse_line("ui chrome more"),
            Ok(Some(Cmd::Ui("chrome".into(), "more".into())))
        );
        assert_eq!(
            parse_line("ui menu-close"),
            Ok(Some(Cmd::Ui("menu-close".into(), String::new())))
        );
        assert_eq!(
            parse_line("ui snapshot-top \"top light.png\""),
            Ok(Some(Cmd::Ui("snapshot-top".into(), "top light.png".into())))
        );
        assert!(parse_line("ui").is_err());
    }

    #[test]
    fn rejects_bad_lines() {
        assert!(parse("frobnicate").is_err());
        assert!(parse("key hyper+X").is_err());
        assert!(parse("assert a ~ b").is_err());
        assert!(parse("click 1").is_err());
    }

    #[test]
    fn checks() {
        assert!(check("120.2", Op::Gt, "100"));
        assert!(check("16", Op::Eq, "16.3"));
        assert!(check("a.md", Op::Eq, "a.md"));
        assert!(check("ask-url", Op::Contains, "ask"));
        assert!(check("ask-url", Op::Lacks, "script"));
        assert!(!check("x", Op::Gt, "y"));
    }

    #[test]
    fn v1a_shell_commands_parse() {
        assert_eq!(
            parse_line("command scheme glacier").unwrap(),
            Some(Cmd::Command("scheme glacier".into()))
        );
        assert_eq!(
            parse_line("type \"tar get\"").unwrap(),
            Some(Cmd::Type("tar get".into()))
        );
        assert_eq!(
            parse_line("write-file live.md \"# A\\nB\"").unwrap(),
            Some(Cmd::WriteFile("live.md".into(), "# A\nB".into()))
        );
        assert_eq!(parse_line("sleep 300").unwrap(), Some(Cmd::Sleep(300)));
        assert!(parse_line("sleep x").is_err());
        assert!(parse_line("command").is_err());
        assert_eq!(
            parse_line("resize 1000 700").unwrap(),
            Some(Cmd::Resize(1000.0, 700.0))
        );
        assert_eq!(
            parse_line("snapshot a.png").unwrap(),
            Some(Cmd::Snapshot("a.png".into()))
        );
    }

    #[test]
    fn editing_commands_parse() {
        assert_eq!(vk("F2"), Some(0x71));
        assert_eq!(vk("f12"), Some(0x7B));
        assert_eq!(vk("backspace"), Some(0x08));
        assert_eq!(vk("`"), Some(0xC0));
        assert_eq!(
            parse_line("click-text \"two words\"").ok().flatten(),
            Some(Cmd::ClickText("two words".into()))
        );
        assert_eq!(
            parse_line("click-task todo").ok().flatten(),
            Some(Cmd::ClickTask("todo".into()))
        );
        assert_eq!(
            parse_line("click-render graph").ok().flatten(),
            Some(Cmd::ClickRender("graph".into()))
        );
        assert_eq!(
            parse_line("write-file a.md \"x\\r\\ny\"").ok().flatten(),
            Some(Cmd::WriteFile("a.md".into(), "x\r\ny".into()))
        );
    }

    #[test]
    fn committed_scripts_parse() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests");
        let mut n = 0;
        for dir in ["driver", "golden/v1a", "perf"] {
            for e in std::fs::read_dir(root.join(dir))
                .into_iter()
                .flatten()
                .flatten()
            {
                let p = e.path();
                if p.extension().is_some_and(|x| x == "yds") {
                    let src = std::fs::read_to_string(&p).unwrap_or_default();
                    assert!(
                        parse(&src).is_ok(),
                        "{}: {:?}",
                        p.display(),
                        parse(&src).err()
                    );
                    n += 1;
                }
            }
        }
        assert!(n >= 24, "found {n} scripts");
    }
}
