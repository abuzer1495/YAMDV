//! Bounded XML tokenizer (S-RES-07): elements, attributes, text, comments, CDATA. DTDs and entity
//! declarations are errors; only the five predefined and numeric character references expand.

use crate::Error;

/// Input cap (10 MB).
pub const MAX_INPUT: usize = 10 * 1024 * 1024;
/// Element cap.
pub const MAX_ELEMENTS: usize = 50_000;
/// Nesting cap.
pub const MAX_DEPTH: usize = 64;

/// Child of an element.
#[derive(Debug, Clone, PartialEq)]
pub enum Kid {
    /// Element index into [`Doc::els`].
    El(usize),
    /// Character data (references expanded).
    Text(String),
}

/// One element (namespace prefix removed; `xlink:href` becomes `href`).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct El {
    /// Local name.
    pub name: String,
    /// Attributes in source order.
    pub attrs: Vec<(String, String)>,
    /// Children.
    pub kids: Vec<Kid>,
}

impl El {
    /// Attribute value.
    pub fn attr(&self, k: &str) -> Option<&str> {
        self.attrs
            .iter()
            .find(|(n, _)| n == k)
            .map(|(_, v)| v.as_str())
    }
}

/// Parsed document; element 0 is the root.
#[derive(Debug, Clone, Default)]
pub struct Doc {
    /// Elements.
    pub els: Vec<El>,
}

impl Doc {
    /// Element by index.
    pub fn el(&self, i: usize) -> Option<&El> {
        self.els.get(i)
    }
    /// First element with `id == id`.
    pub fn by_id(&self, id: &str) -> Option<usize> {
        self.els.iter().position(|e| e.attr("id") == Some(id))
    }
}

fn local(name: &str) -> &str {
    name.rsplit_once(':').map_or(name, |(_, l)| l)
}

/// Expands the predefined and numeric references; anything else stays literal.
pub fn unescape(s: &str) -> String {
    if !s.contains('&') {
        return s.to_string();
    }
    let mut out = String::with_capacity(s.len());
    let mut rest = s;
    while let Some(i) = rest.find('&') {
        out.push_str(rest.get(..i).unwrap_or(""));
        rest = rest.get(i..).unwrap_or("");
        let Some(end) = rest.get(..rest.len().min(12)).and_then(|h| h.find(';')) else {
            out.push('&');
            rest = rest.get(1..).unwrap_or("");
            continue;
        };
        let name = rest.get(1..end).unwrap_or("");
        let ch = match name {
            "lt" => Some('<'),
            "gt" => Some('>'),
            "amp" => Some('&'),
            "quot" => Some('"'),
            "apos" => Some('\''),
            _ => name
                .strip_prefix("#x")
                .or_else(|| name.strip_prefix("#X"))
                .map(|h| u32::from_str_radix(h, 16).ok())
                .or_else(|| name.strip_prefix('#').map(|d| d.parse::<u32>().ok()))
                .flatten()
                .and_then(char::from_u32),
        };
        match ch {
            Some(c) => {
                out.push(c);
                rest = rest.get(end + 1..).unwrap_or("");
            }
            None => {
                out.push('&');
                rest = rest.get(1..).unwrap_or("");
            }
        }
    }
    out.push_str(rest);
    out
}

/// Parses `src` into a [`Doc`] within the limits.
pub fn parse(src: &str) -> Result<Doc, Error> {
    if src.len() > MAX_INPUT {
        return Err(Error::TooLarge);
    }
    let b = src.as_bytes();
    let mut doc = Doc::default();
    let mut stack: Vec<usize> = Vec::new();
    let mut i = 0;
    let push_text = |doc: &mut Doc, stack: &[usize], t: String| {
        if let Some(e) = stack.last().and_then(|&p| doc.els.get_mut(p))
            && !t.is_empty()
        {
            e.kids.push(Kid::Text(t));
        }
    };
    while i < b.len() {
        let Some(lt) = src.get(i..).and_then(|r| r.find('<')).map(|k| k + i) else {
            push_text(&mut doc, &stack, unescape(src.get(i..).unwrap_or("")));
            break;
        };
        if lt > i {
            push_text(&mut doc, &stack, unescape(src.get(i..lt).unwrap_or("")));
        }
        let rest = src.get(lt..).unwrap_or("");
        if rest.starts_with("<!--") {
            i = rest.find("-->").map_or(b.len(), |k| lt + k + 3);
        } else if let Some(r) = rest.strip_prefix("<![CDATA[") {
            let k = r.find("]]>").ok_or(Error::Malformed)?;
            push_text(&mut doc, &stack, r.get(..k).unwrap_or("").to_string());
            i = lt + 9 + k + 3;
        } else if rest.starts_with("<!") {
            // `<!DOCTYPE`, `<!ENTITY`, `<!ELEMENT`…: rejected (no DTD processing at all).
            return Err(Error::Dtd);
        } else if rest.starts_with("<?") {
            i = rest.find("?>").map_or(b.len(), |k| lt + k + 2);
        } else if let Some(r) = rest.strip_prefix("</") {
            let k = r.find('>').ok_or(Error::Malformed)?;
            let name = local(r.get(..k).unwrap_or("").trim());
            let open = stack.pop().ok_or(Error::Malformed)?;
            if doc.els.get(open).map(|e| e.name.as_str()) != Some(name) {
                return Err(Error::Malformed);
            }
            i = lt + 2 + k + 1;
        } else {
            let (el, len, closed) = tag(rest)?;
            if doc.els.len() >= MAX_ELEMENTS {
                return Err(Error::TooMany);
            }
            if doc.els.is_empty() && el.name != "svg" {
                return Err(Error::NoSvg);
            }
            if stack.is_empty() && !doc.els.is_empty() {
                return Err(Error::Malformed);
            }
            let id = doc.els.len();
            if let Some(p) = stack.last().and_then(|&p| doc.els.get_mut(p)) {
                p.kids.push(Kid::El(id));
            }
            doc.els.push(el);
            if !closed {
                if stack.len() >= MAX_DEPTH {
                    return Err(Error::TooDeep);
                }
                stack.push(id);
            }
            i = lt + len;
        }
    }
    if doc.els.is_empty() {
        return Err(Error::NoSvg);
    }
    Ok(doc)
}

/// Parses one start tag at `s` (`<name a="v" …>` or `/>`): (element, byte length, self-closed).
fn tag(s: &str) -> Result<(El, usize, bool), Error> {
    let b = s.as_bytes();
    let name_end = s
        .get(1..)
        .and_then(|r| r.find(|c: char| c.is_whitespace() || c == '>' || c == '/'))
        .map(|k| k + 1)
        .ok_or(Error::Malformed)?;
    let name = local(s.get(1..name_end).unwrap_or(""));
    if name.is_empty() {
        return Err(Error::Malformed);
    }
    let mut el = El {
        name: name.to_string(),
        ..El::default()
    };
    let mut i = name_end;
    loop {
        while b.get(i).is_some_and(u8::is_ascii_whitespace) {
            i += 1;
        }
        match b.get(i) {
            None => return Err(Error::Malformed),
            Some(b'>') => return Ok((el, i + 1, false)),
            Some(b'/') => {
                return if b.get(i + 1) == Some(&b'>') {
                    Ok((el, i + 2, true))
                } else {
                    Err(Error::Malformed)
                };
            }
            Some(_) => {}
        }
        let eq = s
            .get(i..)
            .and_then(|r| r.find('='))
            .ok_or(Error::Malformed)?
            + i;
        let key = s.get(i..eq).unwrap_or("").trim();
        let mut j = eq + 1;
        while b.get(j).is_some_and(u8::is_ascii_whitespace) {
            j += 1;
        }
        let q = *b.get(j).ok_or(Error::Malformed)?;
        if q != b'"' && q != b'\'' {
            return Err(Error::Malformed);
        }
        let close = s
            .get(j + 1..)
            .and_then(|r| r.find(q as char))
            .ok_or(Error::Malformed)?
            + j
            + 1;
        let val = unescape(s.get(j + 1..close).unwrap_or(""));
        if key.is_empty() || key.contains(|c: char| c.is_whitespace() || c == '<') {
            return Err(Error::Malformed);
        }
        let key = if key == "xlink:href" { "href" } else { key };
        el.attrs.push((key.to_string(), val));
        i = close + 1;
    }
}
