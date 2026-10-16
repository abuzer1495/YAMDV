//! Inline extensions over text events (Phase 003 §4.7): `==highlight==` (R-MD-05), GFM autolink
//! literals (R-MD-03) and emoji shortcodes (R-MD-13). Applied only to text events, so never inside
//! code spans, code blocks, link destinations or HTML. Linear in the input.

// Byte indices are bounded by the loop conditions and only ever land on ASCII bytes or on
// `char` boundaries (advanced by `len_utf8`).
#![allow(clippy::indexing_slicing)]

use std::ops::Range;

use super::detail::style;

/// One output piece: byte range in the input text, replacement text, extra style flags.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Piece {
    /// Byte range of the source text this piece stands for.
    pub range: Range<usize>,
    /// Display text when it differs from the range (emoji).
    pub display: Option<&'static str>,
    /// Extra style flags ([`style::HIGHLIGHT`], [`style::LINK`] | [`style::AUTOLINK`]).
    pub extra: u16,
}

/// Emoji for a shortcode name (without colons).
pub fn emoji(name: &str) -> Option<&'static str> {
    let t = &crate::emoji_table::EMOJI;
    t.binary_search_by(|(k, _)| k.as_bytes().cmp(name.as_bytes()))
        .ok()
        .and_then(|i| t.get(i))
        .map(|(_, e)| *e)
}

/// Splits `text` into pieces. `in_link`: the text is already a link label (no autolinks).
pub fn pieces(text: &str, in_link: bool) -> Vec<Piece> {
    let mut out = Vec::new();
    split(text, 0..text.len(), 0, in_link, &mut out);
    out
}

fn push(out: &mut Vec<Piece>, range: Range<usize>, display: Option<&'static str>, extra: u16) {
    if range.is_empty() {
        return;
    }
    if display.is_none()
        && let Some(last) = out.last_mut()
        && last.display.is_none()
        && last.extra == extra
        && last.range.end == range.start
    {
        last.range.end = range.end;
        return;
    }
    out.push(Piece {
        range,
        display,
        extra,
    });
}

fn is_boundary(prev: Option<char>) -> bool {
    prev.is_none_or(|c| c.is_whitespace() || matches!(c, '(' | '*' | '_' | '~' | '"' | '\''))
}

fn split(text: &str, r: Range<usize>, extra: u16, in_link: bool, out: &mut Vec<Piece>) {
    let s = &text[r.clone()];
    let base = r.start;
    let mut plain = base;
    let mut i = 0;
    let bytes = s.as_bytes();
    while i < s.len() {
        let rest = &s[i..];
        let prev = s[..i].chars().next_back();
        // ==highlight==: content non-empty, not starting or ending with whitespace.
        if extra & style::HIGHLIGHT == 0
            && rest.starts_with("==")
            && let Some(end) = highlight_end(rest)
        {
            push(out, plain..base + i, None, extra);
            split(
                text,
                base + i + 2..base + i + end,
                extra | style::HIGHLIGHT,
                in_link,
                out,
            );
            i += end + 2;
            plain = base + i;
            continue;
        }
        if bytes[i] == b':'
            && let Some(len) = shortcode(rest)
            && let Some(e) = emoji(&rest[1..len - 1])
        {
            push(out, plain..base + i, None, extra);
            push(out, base + i..base + i + len, Some(e), extra);
            i += len;
            plain = base + i;
            continue;
        }
        if !in_link
            && is_boundary(prev)
            && let Some(len) = autolink(rest)
        {
            push(out, plain..base + i, None, extra);
            push(
                out,
                base + i..base + i + len,
                None,
                extra | style::LINK | style::AUTOLINK,
            );
            i += len;
            plain = base + i;
            continue;
        }
        i += rest.chars().next().map_or(1, char::len_utf8);
    }
    push(out, plain..r.end, None, extra);
}

/// Length up to (not including) the closing `==` of a highlight starting at `s` (`s` starts with
/// `==`), or `None`.
fn highlight_end(s: &str) -> Option<usize> {
    let inner = s.get(2..)?;
    let first = inner.chars().next()?;
    if first.is_whitespace() || first == '=' {
        return None;
    }
    let close = inner.find("==")?;
    let content = &inner[..close];
    if content.is_empty() || content.ends_with(char::is_whitespace) {
        return None;
    }
    Some(close + 2)
}

/// `:name:` length (including both colons) if `s` starts with a well-formed shortcode.
fn shortcode(s: &str) -> Option<usize> {
    let b = s.as_bytes();
    let mut j = 1;
    while j < b.len() && j <= 65 {
        let c = b[j];
        if c == b':' {
            return (j > 1).then_some(j + 1);
        }
        if !(c.is_ascii_lowercase() || c.is_ascii_digit() || matches!(c, b'_' | b'+' | b'-')) {
            return None;
        }
        j += 1;
    }
    None
}

/// Length of a GFM autolink literal (extended www / url / email autolink) at the start of `s`.
pub fn autolink(s: &str) -> Option<usize> {
    let lower = |p: &str| s.get(..p.len()).is_some_and(|h| h.eq_ignore_ascii_case(p));
    if lower("https://") || lower("http://") || lower("www.") {
        let start = if lower("www.") {
            0
        } else if lower("https://") {
            8
        } else {
            7
        };
        let end = s
            .find(|c: char| c.is_whitespace() || c == '<')
            .unwrap_or(s.len());
        let cand = &s[..end];
        // Domain: at least one char, valid characters, no underscore in the last two labels.
        let dom_end = cand[start..]
            .find(['/', '?', '#'])
            .map_or(cand.len(), |p| p + start);
        let domain = &cand[start..dom_end];
        if domain.is_empty()
            || !domain
                .chars()
                .all(|c| c.is_alphanumeric() || matches!(c, '-' | '_' | '.'))
        {
            return None;
        }
        if lower("www.") && domain.len() <= 4 {
            return None;
        }
        let labels: Vec<&str> = domain.trim_end_matches('.').rsplit('.').take(2).collect();
        if labels.iter().any(|l| l.contains('_')) {
            return None;
        }
        let n = trim_trailing(cand);
        return (n > start.max(4)).then_some(n);
    }
    email(s)
}

/// GFM extended path validation: trailing punctuation and unbalanced `)` are excluded.
fn trim_trailing(s: &str) -> usize {
    let mut end = s.len();
    loop {
        let t = &s[..end];
        let Some(c) = t.chars().next_back() else {
            return 0;
        };
        if matches!(
            c,
            '?' | '!' | '.' | ',' | ':' | '*' | '_' | '~' | '\'' | '"'
        ) {
            end -= c.len_utf8();
            continue;
        }
        if c == ')' {
            let open = t.matches('(').count();
            let close = t.matches(')').count();
            if close > open {
                end -= 1;
                continue;
            }
        }
        if c == ';'
            && let Some(amp) = t.rfind('&')
            && t[amp + 1..end - 1]
                .chars()
                .all(|c| c.is_ascii_alphanumeric())
        {
            end = amp;
            continue;
        }
        return end;
    }
}

fn email(s: &str) -> Option<usize> {
    let b = s.as_bytes();
    let mut i = 0;
    while i < b.len() && (b[i].is_ascii_alphanumeric() || matches!(b[i], b'.' | b'-' | b'_' | b'+'))
    {
        i += 1;
    }
    if i == 0 || b.get(i) != Some(&b'@') {
        return None;
    }
    let at = i;
    i += 1;
    let mut dots = 0;
    while i < b.len() && (b[i].is_ascii_alphanumeric() || matches!(b[i], b'.' | b'-' | b'_')) {
        if b[i] == b'.' {
            dots += 1;
        }
        i += 1;
    }
    // Trailing `.` is not part of the address; last char must not be `-` or `_`.
    while i > at + 1 && b[i - 1] == b'.' {
        i -= 1;
        dots -= 1;
    }
    if dots == 0 || i <= at + 1 || matches!(b[i - 1], b'-' | b'_') {
        return None;
    }
    Some(i)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn show(t: &str) -> Vec<(String, u16)> {
        pieces(t, false)
            .into_iter()
            .map(|p| {
                (
                    p.display
                        .map_or_else(|| t[p.range].to_string(), str::to_string),
                    p.extra,
                )
            })
            .collect()
    }

    const L: u16 = style::LINK | style::AUTOLINK;

    #[test]
    fn r_md_05_highlight() {
        assert_eq!(
            show("a ==b c== d"),
            vec![
                ("a ".into(), 0),
                ("b c".into(), style::HIGHLIGHT),
                (" d".into(), 0)
            ]
        );
        assert_eq!(show("a == b == c"), vec![("a == b == c".into(), 0)]);
        assert_eq!(show("x ==== y"), vec![("x ==== y".into(), 0)]);
    }

    #[test]
    fn r_md_13_emoji() {
        assert_eq!(emoji("smile"), Some("\u{1f604}"));
        assert_eq!(emoji("+1"), Some("\u{1f44d}"));
        assert_eq!(
            show("hi :smile: :nope: 10:30:00"),
            vec![
                ("hi ".into(), 0),
                ("\u{1f604}".into(), 0),
                (" :nope: 10:30:00".into(), 0)
            ]
        );
    }

    #[test]
    fn r_md_03_autolinks() {
        assert_eq!(
            show("see https://example.com/a?b=1."),
            vec![
                ("see ".into(), 0),
                ("https://example.com/a?b=1".into(), L),
                (".".into(), 0)
            ]
        );
        assert_eq!(
            show("(www.commonmark.org/he<lp)"),
            vec![
                ("(".into(), 0),
                ("www.commonmark.org/he".into(), L),
                ("<lp)".into(), 0)
            ]
        );
        assert_eq!(
            show("www.google.com/search?q=(business))+ok"),
            vec![("www.google.com/search?q=(business))+ok".into(), L)]
        );
        assert_eq!(
            show("www.google.com/search?q=Markup+(business))"),
            vec![
                ("www.google.com/search?q=Markup+(business)".into(), L),
                (")".into(), 0)
            ]
        );
        assert_eq!(
            show("foo@bar.baz."),
            vec![("foo@bar.baz".into(), L), (".".into(), 0)]
        );
        assert_eq!(show("a.b-c_d@a.b_"), vec![("a.b-c_d@a.b_".into(), 0)]);
        assert_eq!(
            show("http://a.b.c_d/x"),
            vec![("http://a.b.c_d/x".into(), 0)]
        );
        assert_eq!(show("xhttps://a.com"), vec![("xhttps://a.com".into(), 0)]);
        assert_eq!(
            show("www.commonmark.org/a.b&amp;"),
            vec![("www.commonmark.org/a.b".into(), L), ("&amp;".into(), 0)]
        );
        assert!(pieces("https://x.org", true).iter().all(|p| p.extra == 0));
    }

    #[test]
    fn nested_highlight_with_link() {
        assert_eq!(
            show("==go https://a.io now=="),
            vec![
                ("go ".into(), style::HIGHLIGHT),
                ("https://a.io".into(), style::HIGHLIGHT | L),
                (" now".into(), style::HIGHLIGHT)
            ]
        );
    }
}
