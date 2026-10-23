//! Markdown text helpers for the HTML converter: escaping, safe link targets, table rows.

/// Escapes Markdown-active characters in plain text.
pub fn escape(s: &str, in_table: bool) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        if matches!(
            c,
            '\\' | '*' | '_' | '`' | '[' | ']' | '<' | '>' | '~' | '='
        ) || (in_table && c == '|')
        {
            out.push('\\');
        }
        out.push(c);
    }
    out
}

/// Link target that survives Markdown: spaces, parentheses and angle brackets percent-encoded.
pub fn encode_target(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for (i, c) in s.char_indices() {
        match c {
            // An entity-looking `&name;` / `&#…;` would be decoded by the Markdown parser.
            '&' if entity_at(s, i) => out.push_str("%26"),
            '\\' => out.push_str("%5C"),
            ' ' => out.push_str("%20"),
            '(' => out.push_str("%28"),
            ')' => out.push_str("%29"),
            '<' => out.push_str("%3C"),
            '>' => out.push_str("%3E"),
            '"' => out.push_str("%22"),
            c if c.is_control() => {}
            c => out.push(c),
        }
    }
    out
}

fn entity_at(s: &str, i: usize) -> bool {
    let rest = s.get(i + 1..).unwrap_or("");
    let n = rest
        .chars()
        .take_while(|c| c.is_ascii_alphanumeric() || *c == '#')
        .count();
    n > 0 && rest.get(n..).is_some_and(|r| r.starts_with(';'))
}

/// Safe `href` for pasted links: `http`, `https`, `mailto`, or relative (no scheme). Every other
/// scheme (javascript, vbscript, data, file, ms-*, …) is dropped and only the text stays.
pub fn safe_href(v: &str) -> Option<String> {
    let t: String = v
        .chars()
        .filter(|c| !c.is_whitespace() && !c.is_control())
        .collect();
    if t.is_empty() {
        return None;
    }
    let lower = t.to_ascii_lowercase();
    let end = lower.find(['/', '?', '#']).unwrap_or(lower.len());
    if let Some(colon) = lower.get(..end).and_then(|h| h.find(':')) {
        let scheme = lower.get(..colon).unwrap_or("");
        if !matches!(scheme, "http" | "https" | "mailto") {
            return None;
        }
    }
    if lower.starts_with("//") {
        return None; // protocol-relative: no implicit scheme
    }
    Some(encode_target(&t))
}

/// One GFM table row.
pub fn table_row(cells: &[String], cols: usize) -> String {
    let mut out = String::from("|");
    for i in 0..cols {
        out.push(' ');
        out.push_str(cells.get(i).map_or("", |c| c.trim()));
        out.push_str(" |");
    }
    out
}

/// Collapses runs of whitespace to one space (HTML text outside `<pre>`).
pub fn collapse_ws(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut sp = false;
    for c in s.chars() {
        if c.is_whitespace() {
            if !sp {
                out.push(' ');
            }
            sp = true;
        } else {
            out.push(c);
            sp = false;
        }
    }
    out
}

/// Tidies block spacing: at most one blank line, no trailing spaces, trimmed ends.
pub fn tidy(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut blank = 0;
    for line in s.lines() {
        let l = line.trim_end_matches(' ');
        let l = if l.ends_with('\\') && line.len() > l.len() {
            line
        } else {
            l
        };
        if l.trim().is_empty() {
            blank += 1;
            if blank > 1 {
                continue;
            }
            out.push('\n');
        } else {
            blank = 0;
            out.push_str(l);
            out.push('\n');
        }
    }
    out.trim_matches('\n').to_string()
}
