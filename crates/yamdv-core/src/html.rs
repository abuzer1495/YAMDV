//! Safe HTML (R-MD-14 + P-24, S-HTML-01…03): a small bounded tokenizer and an allowlist
//! sanitizer. There is no HTML engine; the output is a flat token list the renderer maps to the
//! document model. Linear in the input; nesting depth 64, 100 000 tags per document, 8 KiB per
//! attribute value.

// Byte indices are bounded by the loop conditions and land on ASCII bytes / char boundaries.
#![allow(clippy::indexing_slicing)]

use std::ops::Range;

/// Nesting depth cap (S-HTML-03).
pub const MAX_DEPTH: usize = 64;
/// Tags per document (S-HTML-03).
pub const MAX_TAGS: usize = 100_000;
/// Bytes per attribute value (S-HTML-03).
pub const MAX_ATTR: usize = 8 * 1024;

/// Inline tags of the allowlist.
pub const INLINE: &[&str] = &[
    "b", "strong", "i", "em", "u", "ins", "s", "del", "strike", "mark", "sub", "sup", "small",
    "kbd", "code", "var", "samp", "abbr", "br", "span", "a", "img",
];
/// Block tags of the allowlist.
pub const BLOCK: &[&str] = &[
    "p",
    "div",
    "center",
    "h1",
    "h2",
    "h3",
    "h4",
    "h5",
    "h6",
    "blockquote",
    "ul",
    "ol",
    "li",
    "dl",
    "dt",
    "dd",
    "pre",
    "hr",
    "table",
    "thead",
    "tbody",
    "tfoot",
    "tr",
    "th",
    "td",
    "figure",
    "figcaption",
    "picture",
    "source",
    "details",
    "summary",
];
/// Containers dropped together with their content (S-HTML-01).
pub const DROP: &[&str] = &[
    "script", "style", "iframe", "frame", "frameset", "object", "embed", "applet", "form", "input",
    "button", "select", "textarea", "template", "noscript", "link", "meta", "base", "title",
    "head", "svg", "math",
];
const VOID: &[&str] = &[
    "br", "hr", "img", "source", "input", "meta", "link", "base", "embed",
];

/// A sanitized token.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Tok {
    /// Allowed start tag with its surviving attributes (`self_closing` for void / `/>`).
    Open {
        /// Lower-case tag name.
        tag: String,
        /// Surviving (name, value) pairs, names lower-case.
        attrs: Vec<(String, String)>,
        /// Void element or `/>`.
        self_closing: bool,
    },
    /// Allowed end tag.
    Close(String),
    /// Text (entities decoded) and its source byte range (relative to the input).
    Text {
        /// Decoded text.
        text: String,
        /// Source range.
        range: Range<usize>,
    },
    /// Inline SVG / MathML placeholder (S-HTML-01).
    Unsupported(String),
}

/// Raw tokenizer output.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Raw {
    Start(String, Vec<(String, String)>, bool),
    End(String),
    Text(Range<usize>),
}

fn tokenize(s: &str, budget: &mut usize) -> Vec<Raw> {
    let b = s.as_bytes();
    let mut out = Vec::new();
    let mut i = 0;
    let mut text_start = 0;
    let flush = |out: &mut Vec<Raw>, a: usize, z: usize| {
        if z > a {
            out.push(Raw::Text(a..z));
        }
    };
    while i < b.len() {
        if b[i] != b'<' {
            i += 1;
            continue;
        }
        let rest = &s[i..];
        if let Some(body) = rest.strip_prefix("<!--") {
            flush(&mut out, text_start, i);
            // Comments are hidden, including multi-line and unterminated ones.
            i = body.find("-->").map_or(b.len(), |p| i + 4 + p + 3);
            text_start = i;
            continue;
        }
        if rest.starts_with("<!") || rest.starts_with("<?") {
            flush(&mut out, text_start, i);
            i = rest.find('>').map_or(b.len(), |p| i + p + 1);
            text_start = i;
            continue;
        }
        if let Some((tok, len)) = tag(rest) {
            flush(&mut out, text_start, i);
            if *budget > 0 {
                *budget -= 1;
                out.push(tok);
            }
            i += len;
            text_start = i;
            continue;
        }
        i += 1;
    }
    flush(&mut out, text_start, b.len());
    out
}

fn name_len(s: &[u8]) -> usize {
    let mut j = 0;
    while j < s.len() && (s[j].is_ascii_alphanumeric() || s[j] == b'-') {
        j += 1;
    }
    j
}

/// Parses a tag at the start of `s` (`s` starts with `<`).
fn tag(s: &str) -> Option<(Raw, usize)> {
    let b = s.as_bytes();
    let mut i = 1;
    let end = b.get(1) == Some(&b'/');
    if end {
        i += 1;
    }
    if !b.get(i)?.is_ascii_alphabetic() {
        return None;
    }
    let n = name_len(&b[i..]);
    let name = s[i..i + n].to_ascii_lowercase();
    i += n;
    let mut attrs = Vec::new();
    let mut self_closing = false;
    loop {
        while i < b.len() && b[i].is_ascii_whitespace() {
            i += 1;
        }
        match b.get(i)? {
            b'>' => {
                i += 1;
                break;
            }
            b'/' if b.get(i + 1) == Some(&b'>') => {
                self_closing = true;
                i += 2;
                break;
            }
            b'/' => {
                i += 1;
                continue;
            }
            _ => {}
        }
        let a0 = i;
        while i < b.len() && !b[i].is_ascii_whitespace() && !matches!(b[i], b'=' | b'>' | b'/') {
            i += 1;
        }
        let an = s[a0..i].to_ascii_lowercase();
        while i < b.len() && b[i].is_ascii_whitespace() {
            i += 1;
        }
        let mut val = String::new();
        if b.get(i) == Some(&b'=') {
            i += 1;
            while i < b.len() && b[i].is_ascii_whitespace() {
                i += 1;
            }
            match b.get(i)? {
                q @ (b'"' | b'\'') => {
                    let q = char::from(*q);
                    let close = s[i + 1..].find(q)?;
                    val = s[i + 1..i + 1 + close].to_string();
                    i += close + 2;
                }
                _ => {
                    let v0 = i;
                    while i < b.len() && !b[i].is_ascii_whitespace() && b[i] != b'>' {
                        i += 1;
                    }
                    val = s[v0..i].to_string();
                }
            }
        }
        if !an.is_empty() && val.len() <= MAX_ATTR {
            attrs.push((an, decode_entities(&val)));
        }
    }
    let tok = if end {
        Raw::End(name)
    } else {
        Raw::Start(name, attrs, self_closing)
    };
    Some((tok, i))
}

/// Decodes numeric and the common named character references; unknown ones stay literal.
pub fn decode_entities(s: &str) -> String {
    if !s.contains('&') {
        return s.to_string();
    }
    let mut out = String::with_capacity(s.len());
    let mut rest = s;
    while let Some(p) = rest.find('&') {
        out.push_str(&rest[..p]);
        rest = &rest[p..];
        let semi = rest[1..].find(';').filter(|&q| q <= 32).map(|q| q + 1);
        let decoded = semi.and_then(|q| entity(&rest[1..q]).map(|c| (c, q + 1)));
        match decoded {
            Some((c, len)) => {
                out.push(c);
                rest = &rest[len..];
            }
            None => {
                out.push('&');
                rest = &rest[1..];
            }
        }
    }
    out.push_str(rest);
    out
}

fn entity(name: &str) -> Option<char> {
    if let Some(num) = name.strip_prefix('#') {
        let v = match num.strip_prefix(['x', 'X']) {
            Some(h) => u32::from_str_radix(h, 16).ok()?,
            None => num.parse::<u32>().ok()?,
        };
        return Some(
            char::from_u32(v)
                .filter(|&c| c != '\0')
                .unwrap_or('\u{fffd}'),
        );
    }
    Some(match name {
        "amp" => '&',
        "lt" => '<',
        "gt" => '>',
        "quot" => '"',
        "apos" => '\'',
        "nbsp" => '\u{a0}',
        "copy" => '©',
        "reg" => '®',
        "trade" => '™',
        "hellip" => '…',
        "mdash" => '—',
        "ndash" => '–',
        "laquo" => '«',
        "raquo" => '»',
        "lsquo" => '‘',
        "rsquo" => '’',
        "ldquo" => '“',
        "rdquo" => '”',
        "middot" => '·',
        "bull" => '•',
        "deg" => '°',
        "times" => '×',
        "divide" => '÷',
        "euro" => '€',
        "pound" => '£',
        "yen" => '¥',
        "cent" => '¢',
        "sect" => '§',
        "para" => '¶',
        "plusmn" => '±',
        "larr" => '←',
        "rarr" => '→',
        "uarr" => '↑',
        "darr" => '↓',
        "check" => '✓',
        "ensp" => '\u{2002}',
        "emsp" => '\u{2003}',
        "thinsp" => '\u{2009}',
        "zwj" => '\u{200d}',
        "zwnj" => '\u{200c}',
        _ => return None,
    })
}

fn is_anchor(v: &str) -> bool {
    (1..=128).contains(&v.len())
        && v.bytes()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, b'_' | b'.' | b':' | b'-'))
}

/// URL scheme check for `href` / `src` (S-HTML-02). Further policy is S-LINK / S-RES.
pub fn url_allowed(v: &str, img: bool) -> bool {
    let t: String = v
        .chars()
        .filter(|c| !c.is_whitespace() && !c.is_control())
        .collect::<String>()
        .to_ascii_lowercase();
    // Any active scheme anywhere (also behind quotes/backticks the tokenizer kept).
    if t.contains("javascript:") || t.contains("vbscript:") || t.contains("livescript:") {
        return false;
    }
    if t.starts_with("data:") {
        return img && t.starts_with("data:image/") && !t.starts_with("data:image/svg");
    }
    true
}

const NAMED_COLORS: &[&str] = &[
    "black",
    "white",
    "red",
    "green",
    "blue",
    "yellow",
    "orange",
    "purple",
    "gray",
    "grey",
    "silver",
    "maroon",
    "olive",
    "lime",
    "aqua",
    "teal",
    "navy",
    "fuchsia",
    "pink",
    "brown",
    "gold",
    "crimson",
    "darkred",
    "darkgreen",
    "darkblue",
    "darkorange",
    "transparent",
];

fn color_ok(v: &str) -> bool {
    if let Some(h) = v.strip_prefix('#') {
        return matches!(h.len(), 3 | 4 | 6 | 8) && h.bytes().all(|c| c.is_ascii_hexdigit());
    }
    if let Some(inner) = v.strip_prefix("rgb(").and_then(|r| r.strip_suffix(')')) {
        let parts: Vec<&str> = inner.split(',').map(str::trim).collect();
        return parts.len() == 3 && parts.iter().all(|p| p.parse::<u8>().is_ok());
    }
    NAMED_COLORS.contains(&v)
}

/// Restricted `style` grammar (S-HTML-02): only `color`, `background-color`, `text-align`.
pub fn sanitize_style(v: &str) -> String {
    let mut out = Vec::new();
    for decl in v.split(';') {
        let Some((p, val)) = decl.split_once(':') else {
            continue;
        };
        let p = p.trim().to_ascii_lowercase();
        let val = val.trim().to_ascii_lowercase();
        let ok = match p.as_str() {
            "color" | "background-color" => color_ok(&val),
            "text-align" => matches!(val.as_str(), "left" | "center" | "right" | "justify"),
            _ => false,
        };
        if ok {
            out.push(format!("{p}: {val}"));
        }
    }
    out.join("; ")
}

fn keep_attr(tag: &str, name: &str, val: &str) -> Option<String> {
    let heading = matches!(tag, "h1" | "h2" | "h3" | "h4" | "h5" | "h6");
    let ok = match name {
        "id" | "name" => is_anchor(val),
        "align" => matches!(tag, "p" | "div" | "img" | "td" | "th") || heading,
        "width" | "height" => tag == "img" && val.trim_end_matches("px").parse::<u32>().is_ok(),
        "src" => tag == "img" && url_allowed(val, true),
        "alt" => tag == "img",
        "title" => matches!(tag, "img" | "a" | "abbr"),
        "href" => tag == "a" && url_allowed(val, false),
        "open" => tag == "details",
        "colspan" | "rowspan" => {
            matches!(tag, "td" | "th") && val.parse::<u16>().is_ok_and(|n| (1..=1000).contains(&n))
        }
        "style" => {
            let s = sanitize_style(val);
            return (!s.is_empty()).then_some(s);
        }
        _ => false,
    };
    ok.then(|| val.to_string())
}

/// Tokenizes and sanitizes `html`. `budget` is the remaining per-document tag allowance
/// (S-HTML-03); it is decremented for every tag seen.
pub fn sanitize(html: &str, budget: &mut usize) -> Vec<Tok> {
    let raw = tokenize(html, budget);
    let mut out = Vec::new();
    let mut drop: Option<(String, usize)> = None;
    let mut depth = 0usize;
    for t in raw {
        if let Some((name, n)) = drop.as_mut() {
            match &t {
                Raw::Start(s, _, sc) if s == name && !sc => *n += 1,
                Raw::End(s) if s == name => {
                    *n -= 1;
                    if *n == 0 {
                        drop = None;
                    }
                }
                _ => {}
            }
            continue;
        }
        match t {
            Raw::Text(r) => out.push(Tok::Text {
                text: decode_entities(&html[r.clone()]),
                range: r,
            }),
            Raw::Start(name, attrs, sc) => {
                if DROP.contains(&name.as_str()) {
                    if matches!(name.as_str(), "svg" | "math") {
                        out.push(Tok::Unsupported(name.clone()));
                    }
                    if !sc && !VOID.contains(&name.as_str()) {
                        drop = Some((name, 1));
                    }
                    continue;
                }
                if !INLINE.contains(&name.as_str()) && !BLOCK.contains(&name.as_str()) {
                    continue;
                }
                let void = sc || VOID.contains(&name.as_str());
                if !void {
                    if depth >= MAX_DEPTH {
                        continue;
                    }
                    depth += 1;
                }
                let attrs = attrs
                    .into_iter()
                    .filter_map(|(k, v)| keep_attr(&name, &k, &v).map(|v| (k, v)))
                    .collect();
                out.push(Tok::Open {
                    tag: name,
                    attrs,
                    self_closing: void,
                });
            }
            Raw::End(name) => {
                if (INLINE.contains(&name.as_str()) || BLOCK.contains(&name.as_str()))
                    && !VOID.contains(&name.as_str())
                {
                    depth = depth.saturating_sub(1);
                    out.push(Tok::Close(name));
                }
            }
        }
    }
    out
}

/// Attribute value of an `Open` token.
pub fn attr<'a>(attrs: &'a [(String, String)], name: &str) -> Option<&'a str> {
    attrs
        .iter()
        .find(|(k, _)| k == name)
        .map(|(_, v)| v.as_str())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn san(s: &str) -> Vec<Tok> {
        let mut b = MAX_TAGS;
        sanitize(s, &mut b)
    }

    fn text(s: &str) -> String {
        san(s)
            .into_iter()
            .filter_map(|t| match t {
                Tok::Text { text, .. } => Some(text),
                _ => None,
            })
            .collect()
    }

    fn tags(s: &str) -> Vec<String> {
        san(s)
            .into_iter()
            .filter_map(|t| match t {
                Tok::Open { tag, .. } => Some(tag),
                Tok::Close(t) => Some(format!("/{t}")),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn s_html_01_drops_dangerous_containers_with_content() {
        assert_eq!(text("a<script>alert(1)</script>b"), "ab");
        assert_eq!(text("<style>x{}</style><b>ok</b>"), "ok");
        assert_eq!(text("<iframe src=x>in</iframe>t"), "t");
        assert_eq!(text("<script><script>x</script>y</script>z"), "z");
        assert_eq!(text("<unknown>kept</unknown>"), "kept");
        assert_eq!(tags("<unknown>kept</unknown>"), Vec::<String>::new());
        assert!(san("<svg><circle/></svg>").contains(&Tok::Unsupported("svg".into())));
        assert_eq!(text("<!-- hidden\nmulti -->shown<!-- open"), "shown");
    }

    #[test]
    fn s_html_02_attributes() {
        let t = san(r#"<a href="javascript:alert(1)" onclick="x" title="t">l</a>"#);
        assert_eq!(
            t[0],
            Tok::Open {
                tag: "a".into(),
                attrs: vec![("title".into(), "t".into())],
                self_closing: false
            }
        );
        let t = san("<a href=' jav&#x09;ascript:x'>l</a>");
        assert!(
            matches!(&t[0], Tok::Open { attrs, .. } if attrs.is_empty()),
            "{t:?}"
        );
        let t = san(r#"<img src="data:image/png;base64,AA" alt=a><img src="data:text/html,x">"#);
        assert!(matches!(&t[0], Tok::Open { attrs, .. } if attrs.len() == 2));
        assert!(matches!(&t[1], Tok::Open { attrs, .. } if attrs.is_empty()));
        assert_eq!(
            sanitize_style(
                "color: #f00; position: fixed; text-align: center; background-color: url(x)"
            ),
            "color: #f00; text-align: center"
        );
        let t = san(r#"<p id="ok-1" name="bad id">x</p>"#);
        assert!(
            matches!(&t[0], Tok::Open { attrs, .. } if attrs == &[("id".to_string(), "ok-1".to_string())])
        );
        let long = format!("<img alt=\"{}\">", "a".repeat(MAX_ATTR + 1));
        assert!(matches!(&san(&long)[0], Tok::Open { attrs, .. } if attrs.is_empty()));
    }

    #[test]
    fn s_html_03_limits() {
        let deep = "<div>".repeat(200);
        let opens = san(&deep).len();
        assert_eq!(opens, MAX_DEPTH);
        let mut budget = 3;
        let t = sanitize("<b>a</b><i>b</i>", &mut budget);
        assert_eq!(budget, 0);
        assert_eq!(t.len(), 5);
    }

    #[test]
    fn entities_and_malformed() {
        assert_eq!(
            text("a &amp; b &lt;c&gt; &#x1F600; &#0; &bogus; &"),
            "a & b <c> 😀 \u{fffd} &bogus; &"
        );
        assert_eq!(text("1 < 2 and <3"), "1 < 2 and <3");
        assert_eq!(text("<b unclosed"), "<b unclosed");
        assert_eq!(tags("<P Align=center>x</P><br/>"), vec!["p", "/p", "br"]);
    }
}
