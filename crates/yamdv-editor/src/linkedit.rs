//! Link editor model (prompt 016 §3 item 4): find the inline link `[text](url "title")` at a
//! position and write one back.

use std::ops::Range;

/// An inline link's parts.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Link {
    /// Source range of the whole link (empty when inserting a new one).
    pub range: Range<usize>,
    /// Link text.
    pub text: String,
    /// Destination.
    pub url: String,
    /// Title (may be empty).
    pub title: String,
}

/// The inline link of `line` (starting at source offset `base`) that contains `pos`.
pub fn link_at(line: &str, base: usize, pos: usize) -> Option<Link> {
    let b = line.as_bytes();
    let mut i = 0;
    while i < b.len() {
        let prev = i.checked_sub(1).and_then(|p| b.get(p)).copied();
        if b.get(i) == Some(&b'[')
            && !matches!(prev, Some(b'\\' | b'!'))
            && let Some(l) = parse_at(line, i)
        {
            let r = base + l.range.start..base + l.range.end;
            if r.start <= pos && pos <= r.end {
                return Some(Link { range: r, ..l });
            }
            i = l.range.end;
            continue;
        }
        i += 1;
    }
    None
}

/// Parses `[text](dest "title")` starting at byte `i` of `s` (`s[i] == '['`).
fn parse_at(s: &str, i: usize) -> Option<Link> {
    let b = s.as_bytes();
    let mut depth = 0usize;
    let mut j = i;
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
    if b.get(close + 1) != Some(&b'(') {
        return None;
    }
    let mut k = close + 2;
    let mut paren = 0usize;
    let mut in_q: Option<u8> = None;
    let end = loop {
        let c = *b.get(k)?;
        match (in_q, c) {
            (_, b'\\') => k += 1,
            (Some(q), c) if c == q => in_q = None,
            (Some(_), _) => {}
            (None, b'"' | b'\'') => in_q = Some(c),
            (None, b'(') => paren += 1,
            (None, b')') if paren == 0 => break k,
            (None, b')') => paren -= 1,
            _ => {}
        }
        k += 1;
    };
    let inner = s.get(close + 2..end)?.trim();
    let (url, title) = split_dest(inner);
    Some(Link {
        range: i..end + 1,
        text: s.get(i + 1..close)?.to_string(),
        url,
        title,
    })
}

fn split_dest(inner: &str) -> (String, String) {
    let (url, rest) = if let Some(r) = inner.strip_prefix('<') {
        let e = r.find('>').unwrap_or(r.len());
        (r[..e].to_string(), r.get(e + 1..).unwrap_or(""))
    } else {
        let e = inner.find(char::is_whitespace).unwrap_or(inner.len());
        (inner[..e].to_string(), &inner[e..])
    };
    let t = rest.trim();
    let title = if t.len() >= 2
        && ((t.starts_with('"') && t.ends_with('"')) || (t.starts_with('\'') && t.ends_with('\'')))
    {
        t[1..t.len() - 1].replace("\\\"", "\"")
    } else {
        String::new()
    };
    (url, title)
}

/// Markdown source of `l` (`<…>` around a destination with spaces; title quoted when set).
pub fn to_markdown(l: &Link) -> String {
    let text = l.text.replace(']', "\\]").replace('[', "\\[");
    let url = if l.url.contains(char::is_whitespace) || l.url.contains(['(', ')']) {
        format!("<{}>", l.url)
    } else {
        l.url.clone()
    };
    if l.title.is_empty() {
        format!("[{text}]({url})")
    } else {
        format!("[{text}]({url} \"{}\")", l.title.replace('"', "\\\""))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn find_and_write_links() {
        let line = "see [the *docs*](http://x.y/a_(b) \"T 1\") and ![i](p.png) [z](u)";
        let l = link_at(line, 100, 106).unwrap();
        assert_eq!(l.range, 104..140);
        assert_eq!(
            (l.text.as_str(), l.url.as_str(), l.title.as_str()),
            ("the *docs*", "http://x.y/a_(b)", "T 1")
        );
        assert!(link_at(line, 0, 50).is_none(), "image is not a link");
        assert_eq!(link_at(line, 0, line.len()).unwrap().url, "u");
        assert!(link_at("no link", 0, 2).is_none());
        let w = Link {
            text: "a]b".into(),
            url: "my file.md".into(),
            title: "say \"hi\"".into(),
            ..Link::default()
        };
        let md = to_markdown(&w);
        assert_eq!(md, "[a\\]b](<my file.md> \"say \\\"hi\\\"\")");
        let back = link_at(&md, 0, 1).unwrap();
        assert_eq!(
            (back.url.as_str(), back.title.as_str()),
            ("my file.md", "say \"hi\"")
        );
        assert_eq!(
            to_markdown(&Link {
                text: "t".into(),
                url: "u".into(),
                ..Link::default()
            }),
            "[t](u)"
        );
    }
}
