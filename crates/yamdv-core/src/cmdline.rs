//! Command-line building for external processes (R-EXT-01…03, S-EXEC-03): MSVC /
//! `CommandLineToArgvW` quoting, a reference parser for round-trip tests, and editor
//! argument templates with `{file}`, `{line}`, `{col}`. Pure; never involves a shell.

/// Quotes one argument per the MSVC rules.
pub fn quote_arg(a: &str) -> String {
    if !a.is_empty() && !a.contains([' ', '\t', '\n', '\u{b}', '"']) {
        return a.to_string();
    }
    let mut out = String::from('"');
    let mut bs = 0usize;
    for c in a.chars() {
        match c {
            '\\' => bs += 1,
            '"' => {
                out.extend(std::iter::repeat_n('\\', 2 * bs + 1));
                out.push('"');
                bs = 0;
            }
            _ => {
                out.extend(std::iter::repeat_n('\\', bs));
                out.push(c);
                bs = 0;
            }
        }
    }
    out.extend(std::iter::repeat_n('\\', 2 * bs));
    out.push('"');
    out
}

/// Joins `exe` and `args` into a command line (`exe` quoted as program name).
pub fn command_line(exe: &str, args: &[String]) -> String {
    let mut s = format!("\"{exe}\"");
    for a in args {
        s.push(' ');
        s.push_str(&quote_arg(a));
    }
    s
}

/// Splits arguments like `CommandLineToArgvW` (post-2008 rules) — used for the custom template
/// and for round-trip tests.
pub fn split_args(s: &str) -> Vec<String> {
    let mut out = Vec::new();
    let cs: Vec<char> = s.chars().collect();
    let mut i = 0;
    while i < cs.len() {
        while cs.get(i).is_some_and(|c| *c == ' ' || *c == '\t') {
            i += 1;
        }
        if i >= cs.len() {
            break;
        }
        let mut cur = String::new();
        let mut q = false;
        while let Some(&c) = cs.get(i) {
            if !q && (c == ' ' || c == '\t') {
                break;
            }
            if c == '\\' {
                let mut n = 0;
                while cs.get(i) == Some(&'\\') {
                    n += 1;
                    i += 1;
                }
                if cs.get(i) == Some(&'"') {
                    cur.extend(std::iter::repeat_n('\\', n / 2));
                    if n % 2 == 1 {
                        cur.push('"');
                        i += 1;
                    }
                } else {
                    cur.extend(std::iter::repeat_n('\\', n));
                }
                continue;
            }
            if c == '"' {
                if q && cs.get(i + 1) == Some(&'"') {
                    cur.push('"');
                    i += 2;
                    continue;
                }
                q = !q;
                i += 1;
                continue;
            }
            cur.push(c);
            i += 1;
        }
        out.push(cur);
    }
    out
}

/// Known editors (R-EXT-02, FUT-10 scope).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Editor {
    /// `Code.exe --goto "<file>:<line>:<col>"`.
    VsCode,
    /// `notepad++.exe -n<line> -c<col> "<file>"`.
    NotepadPp,
    /// `notepad.exe "<file>"`.
    Notepad,
}

/// Arguments for a known editor.
pub fn editor_args(e: Editor, file: &str, line: u32, col: u32) -> Vec<String> {
    match e {
        Editor::VsCode => vec!["--goto".into(), format!("{file}:{line}:{col}")],
        Editor::NotepadPp => vec![format!("-n{line}"), format!("-c{col}"), file.to_string()],
        Editor::Notepad => vec![file.to_string()],
    }
}

/// Expands a custom argument template (R-EXT-03): tokenized first, placeholders replaced per
/// token, so a file name can never inject extra arguments.
pub fn expand_template(tpl: &str, file: &str, line: u32, col: u32) -> Vec<String> {
    let t = if tpl.trim().is_empty() { "{file}" } else { tpl };
    split_args(t)
        .into_iter()
        .map(|a| {
            a.replace("{file}", file)
                .replace("{line}", &line.to_string())
                .replace("{col}", &col.to_string())
        })
        .collect()
}

/// `true` for targets S-EXEC-03 forbids (shell interpreters and script files) or non-`.exe`.
pub fn forbidden_target(exe: &str) -> bool {
    let l = exe.to_ascii_lowercase().replace('/', "\\");
    let name = l.rsplit('\\').next().unwrap_or(&l);
    !name.ends_with(".exe")
        || matches!(
            name,
            "cmd.exe" | "powershell.exe" | "pwsh.exe" | "wscript.exe" | "cscript.exe" | "mshta.exe"
        )
        || l.starts_with("\\\\")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn s_exec_03_quote_round_trip() {
        let cases = [
            "plain",
            "with space",
            "q\"uote",
            "trail\\",
            "both \\\"x\\",
            "",
            "C:\\a b\\",
            "\\\\\"",
            "tab\there",
        ];
        for c in cases {
            let line = format!("x {}", quote_arg(c));
            let back = split_args(&line);
            assert_eq!(back.get(1).map(String::as_str), Some(c), "{line}");
        }
        let cl = command_line("C:\\e.exe", &["a b".into(), "c".into()]);
        assert_eq!(split_args(&cl), vec!["C:\\e.exe", "a b", "c"]);
    }

    #[test]
    fn r_ext_03_template_no_injection() {
        let a = expand_template("--goto {file}:{line} -x", "C:\\a \" -evil.md", 7, 1);
        assert_eq!(a, vec!["--goto", "C:\\a \" -evil.md:7", "-x"]);
        assert_eq!(expand_template("", "f.md", 1, 1), vec!["f.md"]);
        assert_eq!(
            editor_args(Editor::NotepadPp, "f", 3, 2),
            vec!["-n3", "-c2", "f"]
        );
        assert!(forbidden_target("C:\\Windows\\System32\\cmd.exe"));
        assert!(forbidden_target("C:\\x.bat"));
        assert!(forbidden_target("\\\\srv\\share\\e.exe"));
        assert!(!forbidden_target("C:\\Tools\\ed.exe"));
    }
}
