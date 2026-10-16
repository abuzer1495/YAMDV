//! INI dialect parser and round-trip-safe writer (ARCHITECTURE §E.12, S-FILE-07).
//!
//! UTF-8 (BOM tolerated), `[section]`, `key = value`, `#`/`;` comments, quoted strings with
//! `\" \\ \n \t \u{…}` escapes, repeated keys for lists. Parsing never fails: malformed lines are
//! counted and skipped. The writer keeps comments, unknown keys and ordering of the input.

/// Maximum accepted file size (S-FILE-07).
pub const MAX_FILE: usize = 4 * 1024 * 1024;
/// Maximum accepted line length (S-FILE-07).
pub const MAX_LINE: usize = 64 * 1024;

/// One line of an INI document, kept for round-trip writing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Line {
    /// Blank line or comment, kept verbatim.
    Other(String),
    /// `[section]` header.
    Section(String),
    /// `key = value` (value already unescaped).
    Entry {
        /// Key as written (comparisons are ASCII case-insensitive, R-I18N-04).
        key: String,
        /// Unescaped value.
        value: String,
    },
}

/// A parsed INI document.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Ini {
    /// Lines in file order.
    pub lines: Vec<Line>,
    /// Number of malformed lines that were skipped.
    pub malformed: usize,
    /// `true` when the input exceeded [`MAX_FILE`] (the document is then empty).
    pub too_large: bool,
}

fn unquote(v: &str) -> Option<String> {
    let inner = v.strip_prefix('"')?.strip_suffix('"')?;
    let mut out = String::with_capacity(inner.len());
    let mut it = inner.chars();
    while let Some(c) = it.next() {
        if c != '\\' {
            out.push(c);
            continue;
        }
        match it.next()? {
            '"' => out.push('"'),
            '\\' => out.push('\\'),
            'n' => out.push('\n'),
            't' => out.push('\t'),
            'u' => {
                if it.next()? != '{' {
                    return None;
                }
                let mut hex = String::new();
                for h in it.by_ref() {
                    if h == '}' {
                        break;
                    }
                    if hex.len() >= 6 {
                        return None;
                    }
                    hex.push(h);
                }
                out.push(char::from_u32(u32::from_str_radix(&hex, 16).ok()?)?);
            }
            _ => return None,
        }
    }
    Some(out)
}

/// Quotes a value when it needs it (leading/trailing space, quotes, comment chars, control chars).
pub fn quote(v: &str) -> String {
    let plain = !v.is_empty()
        && v.trim() == v
        && !v.starts_with('"')
        && !v.contains(['#', ';', '\\'])
        && !v.chars().any(char::is_control);
    if plain {
        return v.to_string();
    }
    let mut s = String::from("\"");
    for c in v.chars() {
        match c {
            '"' => s.push_str("\\\""),
            '\\' => s.push_str("\\\\"),
            '\n' => s.push_str("\\n"),
            '\t' => s.push_str("\\t"),
            c if c.is_control() => s.push_str(&format!("\\u{{{:x}}}", c as u32)),
            c => s.push(c),
        }
    }
    s.push('"');
    s
}

impl Ini {
    /// Parses bytes defensively. Invalid UTF-8 is replaced (U+FFFD); oversize input yields an
    /// empty document with `too_large` set.
    pub fn parse(bytes: &[u8]) -> Self {
        if bytes.len() > MAX_FILE {
            return Self {
                too_large: true,
                ..Self::default()
            };
        }
        let bytes = bytes.strip_prefix(&[0xEF, 0xBB, 0xBF]).unwrap_or(bytes);
        let text = String::from_utf8_lossy(bytes);
        let mut ini = Self::default();
        for raw in text.split('\n') {
            let raw = raw.strip_suffix('\r').unwrap_or(raw);
            if raw.len() > MAX_LINE {
                ini.malformed += 1;
                continue;
            }
            let t = raw.trim();
            if t.is_empty() || t.starts_with('#') || t.starts_with(';') {
                ini.lines.push(Line::Other(raw.to_string()));
            } else if let Some(name) = t.strip_prefix('[').and_then(|s| s.strip_suffix(']')) {
                ini.lines.push(Line::Section(name.trim().to_string()));
            } else if let Some((k, v)) = t.split_once('=') {
                let (k, v) = (k.trim(), v.trim());
                let value = if v.starts_with('"') {
                    unquote(v)
                } else {
                    Some(v.to_string())
                };
                match value {
                    Some(value) if !k.is_empty() => ini.lines.push(Line::Entry {
                        key: k.to_string(),
                        value,
                    }),
                    _ => ini.malformed += 1,
                }
            } else {
                ini.malformed += 1;
            }
        }
        // `split` yields a trailing empty piece for a final newline; drop it for round-trips.
        if matches!(ini.lines.last(), Some(Line::Other(s)) if s.is_empty()) {
            ini.lines.pop();
        }
        ini
    }

    /// All values of `section.key` (ASCII case-insensitive), in file order.
    pub fn get_all(&self, section: &str, key: &str) -> Vec<&str> {
        let mut cur = "";
        let mut out = Vec::new();
        for l in &self.lines {
            match l {
                Line::Section(s) => cur = s,
                Line::Entry { key: k, value }
                    if cur.eq_ignore_ascii_case(section) && k.eq_ignore_ascii_case(key) =>
                {
                    out.push(value.as_str());
                }
                _ => {}
            }
        }
        out
    }

    /// Last value of `section.key`.
    pub fn get(&self, section: &str, key: &str) -> Option<&str> {
        self.get_all(section, key).last().copied()
    }

    /// Sets `section.key` (replaces the last occurrence, or appends to the section, creating it).
    pub fn set(&mut self, section: &str, key: &str, value: &str) {
        let mut cur = String::new();
        let mut last_hit = None;
        let mut section_end = None;
        for (i, l) in self.lines.iter().enumerate() {
            match l {
                Line::Section(s) => cur.clone_from(s),
                Line::Entry { key: k, .. }
                    if cur.eq_ignore_ascii_case(section) && k.eq_ignore_ascii_case(key) =>
                {
                    last_hit = Some(i)
                }
                _ => {}
            }
            if cur.eq_ignore_ascii_case(section) {
                section_end = Some(i + 1);
            }
        }
        let entry = Line::Entry {
            key: key.to_string(),
            value: value.to_string(),
        };
        if let Some(i) = last_hit {
            if let Some(slot) = self.lines.get_mut(i) {
                *slot = entry;
            }
        } else if let Some(end) = section_end {
            self.lines.insert(end, entry);
        } else {
            self.lines.push(Line::Section(section.to_string()));
            self.lines.push(entry);
        }
    }

    /// Serializes with LF line endings and a final newline.
    pub fn write(&self) -> String {
        let mut s = String::new();
        for l in &self.lines {
            match l {
                Line::Other(t) => s.push_str(t),
                Line::Section(n) => {
                    s.push('[');
                    s.push_str(n);
                    s.push(']');
                }
                Line::Entry { key, value } => {
                    s.push_str(key);
                    s.push_str(" = ");
                    s.push_str(&quote(value));
                }
            }
            s.push('\n');
        }
        s
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn s_file_07_parse_basics() {
        let ini = Ini::parse(b"\xEF\xBB\xBF# c\n[General]\nlanguage = tr\n bad line\n[reading]\nx=\"a\\\"b\\n\\u{1F600}\"\nlist = 1\nlist = 2\n");
        assert_eq!(ini.get("general", "LANGUAGE"), Some("tr"));
        assert_eq!(ini.get("reading", "x"), Some("a\"b\n😀"));
        assert_eq!(ini.get_all("reading", "list"), vec!["1", "2"]);
        assert_eq!(ini.malformed, 1);
    }

    #[test]
    fn s_file_07_limits() {
        assert!(Ini::parse(&vec![b'a'; MAX_FILE + 1]).too_large);
        let long = format!("[a]\nk = {}\n", "x".repeat(MAX_LINE + 1));
        let ini = Ini::parse(long.as_bytes());
        assert_eq!(ini.get("a", "k"), None);
        assert_eq!(ini.malformed, 1);
        assert_eq!(Ini::parse(b"k = \"\\q\"\n").malformed, 1);
    }

    #[test]
    fn round_trip_keeps_comments_and_unknown_keys() {
        let src =
            "# top\n[general]\nlanguage = en\nunknown = 1\n\n[reading]\nsmooth_scrolling = true\n";
        let mut ini = Ini::parse(src.as_bytes());
        assert_eq!(ini.write(), src);
        ini.set("general", "language", "tr");
        ini.set("reading", "new", " spaced ");
        ini.set("privacy", "diagnostic_log", "false");
        let out = ini.write();
        assert!(out.contains("language = tr\nunknown = 1"));
        assert!(out.contains("new = \" spaced \""));
        assert!(out.ends_with("[privacy]\ndiagnostic_log = false\n"));
        let again = Ini::parse(out.as_bytes());
        assert_eq!(again.get("reading", "new"), Some(" spaced "));
    }

    #[test]
    fn quote_round_trip() {
        for v in ["a#b", "x\\y", "\"q\"", "tab\tz", "\u{1}", ""] {
            let line = format!("[s]\nk = {}\n", quote(v));
            assert_eq!(Ini::parse(line.as_bytes()).get("s", "k"), Some(v), "{v:?}");
        }
    }
}
