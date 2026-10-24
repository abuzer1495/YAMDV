//! F2 image resize writer (R-F2-12): the new display width of the first image in a block's
//! source, written back as Obsidian `|N`, an HTML `<img width="N">`, or by turning a Markdown
//! `![alt](src "title")` into `<img src alt title width>` (allowlisted attributes only).

use std::ops::Range;

/// The edit (range in `text`, replacement) that sets the first image's width to `width`.
/// A height is dropped so the aspect ratio stays locked. `None` when there is no image the
/// writer understands (e.g. a reference image `![a][r]`).
pub fn set_width(text: &str, width: u32) -> Option<(Range<usize>, String)> {
    let b = text.as_bytes();
    let mut i = 0;
    while i < b.len() {
        let esc = i > 0 && b.get(i - 1) == Some(&b'\\');
        let hit = if !esc && text.get(i..).is_some_and(|r| r.starts_with("![[")) {
            wiki(text, i, width)
        } else if !esc && text.get(i..).is_some_and(|r| r.starts_with("![")) {
            markdown(text, i, width)
        } else if b.get(i) == Some(&b'<') && is_img_tag(text, i) {
            html(text, i, width)
        } else {
            None
        };
        if hit.is_some() {
            return hit;
        }
        i += 1;
    }
    None
}

fn is_img_tag(text: &str, i: usize) -> bool {
    let t = text.as_bytes();
    t.get(i + 1..i + 4)
        .is_some_and(|w| w.eq_ignore_ascii_case(b"img"))
        && matches!(t.get(i + 4), Some(b' ' | b'\t' | b'\n' | b'/' | b'>'))
}

/// `![[file|300]]` / `![[file|300x200]]` / `![[file]]` → `![[file|N]]`.
fn wiki(text: &str, i: usize, width: u32) -> Option<(Range<usize>, String)> {
    let inner_start = i + 3;
    let close = inner_start + text.get(inner_start..)?.find("]]")?;
    let inner = text.get(inner_start..close)?;
    if inner.contains('\n') {
        return None;
    }
    let is_size = |s: &str| {
        let mut p = s.splitn(2, 'x');
        let a = p.next().unwrap_or("");
        let ok = |v: &str| !v.is_empty() && v.bytes().all(|b| b.is_ascii_digit());
        ok(a) && p.next().is_none_or(ok)
    };
    let new = match inner.rsplit_once('|') {
        Some((head, size)) if is_size(size.trim()) => format!("{head}|{width}"),
        _ => format!("{inner}|{width}"),
    };
    Some((inner_start..close, new))
}

/// Attribute value escaping for the generated `<img>`.
fn attr(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('"', "&quot;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

/// Backslash escapes of Markdown text removed (`\*` → `*`).
fn unescape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut it = s.chars().peekable();
    while let Some(c) = it.next() {
        if c == '\\' && it.peek().is_some_and(char::is_ascii_punctuation) {
            if let Some(n) = it.next() {
                out.push(n);
            }
        } else {
            out.push(c);
        }
    }
    out
}

/// `![alt](src "title")` → `<img src="src" alt="alt" title="title" width="N">`.
fn markdown(text: &str, i: usize, width: u32) -> Option<(Range<usize>, String)> {
    let b = text.as_bytes();
    let mut j = i + 1;
    let mut depth = 0usize;
    let close = loop {
        match b.get(j)? {
            b'\\' => j += 1,
            b'[' => depth += 1,
            b']' => {
                depth -= 1;
                if depth == 0 {
                    break j;
                }
            }
            _ => {}
        }
        j += 1;
    };
    let alt = text.get(i + 2..close)?;
    if b.get(close + 1) != Some(&b'(') {
        return None;
    }
    let mut k = close + 2;
    while matches!(b.get(k), Some(b' ' | b'\t')) {
        k += 1;
    }
    let src = if b.get(k) == Some(&b'<') {
        let e = k + 1 + text.get(k + 1..)?.find('>')?;
        let s = text.get(k + 1..e)?;
        k = e + 1;
        s
    } else {
        let st = k;
        let mut par = 0usize;
        while let Some(&c) = b.get(k) {
            match c {
                b'\\' => k += 1,
                b'(' => par += 1,
                b')' if par == 0 => break,
                b')' => par -= 1,
                b' ' | b'\t' | b'\n' => break,
                _ => {}
            }
            k += 1;
        }
        text.get(st..k)?
    };
    while matches!(b.get(k), Some(b' ' | b'\t' | b'\n')) {
        k += 1;
    }
    let mut title = "";
    if let Some(&q) = b.get(k).filter(|q| matches!(q, b'"' | b'\'' | b'(')) {
        let endq = if q == b'(' { b')' } else { q };
        let st = k + 1;
        let mut e = st;
        while let Some(&c) = b.get(e) {
            if c == b'\\' {
                e += 1;
            } else if c == endq {
                break;
            }
            e += 1;
        }
        title = text.get(st..e)?;
        k = e + 1;
        while matches!(b.get(k), Some(b' ' | b'\t' | b'\n')) {
            k += 1;
        }
    }
    if b.get(k) != Some(&b')') {
        return None;
    }
    let mut out = format!(
        "<img src=\"{}\" alt=\"{}\"",
        attr(&unescape(src)),
        attr(&unescape(alt))
    );
    if !title.is_empty() {
        out.push_str(&format!(" title=\"{}\"", attr(&unescape(title))));
    }
    out.push_str(&format!(" width=\"{width}\">"));
    Some((i..k + 1, out))
}

/// `<img … width="old" height="h">` → `<img … width="N">`.
fn html(text: &str, i: usize, width: u32) -> Option<(Range<usize>, String)> {
    let b = text.as_bytes();
    let mut k = i + 4;
    // (whole attribute span including leading space, lower-case name)
    let mut attrs: Vec<(Range<usize>, String)> = Vec::new();
    let end = loop {
        let ws = k;
        while matches!(b.get(k), Some(b' ' | b'\t' | b'\n' | b'\r')) {
            k += 1;
        }
        match b.get(k)? {
            b'>' => break k,
            b'/' if b.get(k + 1) == Some(&b'>') => break k,
            b'/' => {
                k += 1;
                continue;
            }
            _ => {}
        }
        let ns = k;
        while b
            .get(k)
            .is_some_and(|c| !matches!(c, b' ' | b'\t' | b'\n' | b'\r' | b'=' | b'>' | b'/'))
        {
            k += 1;
        }
        if k == ns {
            k += 1;
            continue;
        }
        let name = text.get(ns..k)?.to_ascii_lowercase();
        let mut v = k;
        while matches!(b.get(v), Some(b' ' | b'\t')) {
            v += 1;
        }
        if b.get(v) == Some(&b'=') {
            v += 1;
            while matches!(b.get(v), Some(b' ' | b'\t')) {
                v += 1;
            }
            match b.get(v) {
                Some(&q @ (b'"' | b'\'')) => {
                    v += 1 + text.get(v + 1..)?.find(char::from(q))? + 1;
                }
                _ => {
                    while b
                        .get(v)
                        .is_some_and(|c| !matches!(c, b' ' | b'\t' | b'\n' | b'>'))
                    {
                        v += 1;
                    }
                }
            }
            k = v;
        }
        attrs.push((ws..k, name));
    };
    let mut out = String::new();
    let mut at = i;
    let mut wrote = false;
    for (r, name) in &attrs {
        if name == "width" || name == "height" {
            out.push_str(text.get(at..r.start)?);
            if name == "width" && !wrote {
                out.push_str(&format!(" width=\"{width}\""));
                wrote = true;
            }
            at = r.end;
        }
    }
    out.push_str(text.get(at..end)?);
    if !wrote {
        // Before a `/>` keep its separating space.
        let trimmed = out.trim_end().len();
        out.insert_str(trimmed, &format!(" width=\"{width}\""));
    }
    Some((i..end, out))
}

#[cfg(test)]
mod tests {
    use super::set_width;

    fn apply(s: &str, w: u32) -> String {
        let (r, t) = set_width(s, w).expect("image");
        format!("{}{t}{}", &s[..r.start], &s[r.end..])
    }

    #[test]
    fn resize_writer() {
        assert_eq!(apply("![[x.png|300]]", 120), "![[x.png|120]]");
        assert_eq!(apply("![[x.png|30x20]]", 64), "![[x.png|64]]");
        assert_eq!(apply("![[x.png]]\n", 64), "![[x.png|64]]\n");
        assert_eq!(apply("![[a|b.png|cap]]", 9), "![[a|b.png|cap|9]]");
        assert!(set_width("![a](img/x y.png)", 200).is_none()); // not an image: space in dest
        assert_eq!(
            apply("![a \"b\" \\*](<img/x y.png> 'T&')", 200),
            "<img src=\"img/x y.png\" alt=\"a &quot;b&quot; *\" title=\"T&amp;\" width=\"200\">"
        );
        assert_eq!(
            apply("x ![p](p.png) y", 5),
            "x <img src=\"p.png\" alt=\"p\" width=\"5\"> y"
        );
        assert_eq!(
            apply("<IMG src='a.png' WIDTH=40 height=\"30\" alt=x/>", 80),
            "<IMG src='a.png' width=\"80\" alt=x/>"
        );
        assert_eq!(
            apply("<img src=\"a.png\" />", 80),
            "<img src=\"a.png\" width=\"80\" />"
        );
        assert_eq!(
            apply("<img src=\"a.png\">", 7),
            "<img src=\"a.png\" width=\"7\">"
        );
        assert!(set_width("![a][ref] \\![[x]] <imgx>", 5).is_none());
    }
}
