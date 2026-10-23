//! Fenced block source in F2 (prompt 016 §3 item 2): fence / info / body ranges of a code,
//! math or Mermaid block's text, and the language-picker autocomplete.

use std::ops::Range;

/// What the block source is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FenceKind {
    /// ```` ``` ```` / `~~~` code (any info string other than below).
    Code,
    /// `$$ … $$` or ```` ```math ````.
    Math,
    /// ```` ```mermaid ````.
    Mermaid,
}

/// A fenced block's parts (byte ranges in the block text).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Fence {
    /// Kind.
    pub kind: FenceKind,
    /// Info string (language) on the opening line, trimmed; empty range after the fence when none.
    pub info: Range<usize>,
    /// Body (between the opening line break and the closing line, or the text end).
    pub body: Range<usize>,
}

/// Parses `text` (one block) as a fenced block.
pub fn parse(text: &str) -> Option<Fence> {
    let first_end = text.find('\n').unwrap_or(text.len());
    let first = &text[..first_end];
    let lead = first.len() - first.trim_start().len();
    if lead > 3 {
        return None;
    }
    let t = &first[lead..];
    let ch = t.chars().next()?;
    let n = t.chars().take_while(|&c| c == ch).count();
    let ok = match ch {
        '`' | '~' => n >= 3,
        '$' => n == 2,
        _ => false,
    };
    if !ok {
        return None;
    }
    let after = lead + n;
    let rest = &first[after..];
    let i0 = after + (rest.len() - rest.trim_start().len());
    let i1 = after + rest.trim_end().len();
    let info = i0..i1.max(i0);
    let lang = text[info.clone()]
        .split_whitespace()
        .next()
        .unwrap_or("")
        .to_ascii_lowercase();
    let kind = match (ch, lang.as_str()) {
        ('$', _) | (_, "math") => FenceKind::Math,
        (_, "mermaid") => FenceKind::Mermaid,
        _ => FenceKind::Code,
    };
    let b0 = (first_end + 1).min(text.len());
    let tail = text[b0..].trim_end_matches(['\n', '\r']);
    let mut b1 = b0 + tail.len();
    if let Some(p) = tail.rfind('\n').map(|p| p + 1).or(Some(0)) {
        let last = tail[p..].trim();
        let closes = !last.is_empty()
            && last.chars().all(|c| c == ch)
            && last.chars().count() >= n
            && (b0 + p > b0 || !tail.is_empty());
        if closes {
            b1 = (b0 + p).saturating_sub(1).max(b0);
        }
    }
    Some(Fence {
        kind,
        info,
        body: b0..b1,
    })
}

/// Language-picker suggestions: names starting with `prefix` (ASCII case-insensitive), then
/// names containing it; at most `max`, no duplicates.
pub fn suggest<'a>(names: &[&'a str], prefix: &str, max: usize) -> Vec<&'a str> {
    let p = prefix.trim().to_ascii_lowercase();
    let mut out: Vec<&str> = names
        .iter()
        .copied()
        .filter(|n| n.to_ascii_lowercase().starts_with(&p))
        .collect();
    out.extend(names.iter().copied().filter(|n| {
        !n.to_ascii_lowercase().starts_with(&p) && n.to_ascii_lowercase().contains(&p)
    }));
    let mut seen = std::collections::HashSet::new();
    out.retain(|n| seen.insert(*n));
    out.truncate(max);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fences_and_suggestions() {
        let t = "```rust\nfn a() {}\n```";
        let f = parse(t).unwrap();
        assert_eq!(
            (f.kind, &t[f.info.clone()], &t[f.body.clone()]),
            (FenceKind::Code, "rust", "fn a() {}")
        );
        let t = "```\nopen";
        let f = parse(t).unwrap();
        assert_eq!((&t[f.info.clone()], &t[f.body.clone()]), ("", "open"));
        let t = "$$\na+b\n$$\n";
        let f = parse(t).unwrap();
        assert_eq!((f.kind, &t[f.body.clone()]), (FenceKind::Math, "a+b"));
        let t = "```mermaid\ngraph TD\nA-->B\n```";
        let f = parse(t).unwrap();
        assert_eq!(
            (f.kind, &t[f.body.clone()]),
            (FenceKind::Mermaid, "graph TD\nA-->B")
        );
        assert!(parse("plain text").is_none());
        assert!(parse("``x").is_none());
        let names = ["rust", "ruby", "python", "typescript", "Rust"];
        assert_eq!(suggest(&names, "ru", 8), vec!["rust", "ruby", "Rust"]);
        assert_eq!(suggest(&names, "script", 8), vec!["typescript"]);
        assert_eq!(suggest(&names, "", 2).len(), 2);
    }
}
