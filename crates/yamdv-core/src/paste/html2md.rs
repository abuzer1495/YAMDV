//! Clipboard HTML → Markdown (prompt 015 §3.5). The input goes through the bounded tokenizer
//! and allowlist sanitizer ([`crate::html::sanitize`]): scripts, styles, frames and forms are
//! dropped with their content, event attributes never survive. Links keep only `http`,
//! `https`, `mailto` or relative targets; remote images are never fetched (the URL stays text in
//! the Markdown); `data:` images are handed to `save` and become asset files.

use super::text::{collapse_ws, encode_target, escape, safe_href, table_row, tidy};
use crate::html::{self, Tok};
use crate::image;

/// Most table columns kept.
const MAX_COLS: usize = 64;

#[derive(Debug, Default)]
struct Node {
    tag: String,
    attrs: Vec<(String, String)>,
    kids: Vec<Item>,
}

#[derive(Debug)]
enum Item {
    El(Node),
    Text(String),
}

fn tree(toks: Vec<Tok>) -> Node {
    let mut stack = vec![Node::default()];
    for t in toks {
        match t {
            Tok::Text { text, .. } => {
                if let Some(top) = stack.last_mut() {
                    top.kids.push(Item::Text(text));
                }
            }
            Tok::Open {
                tag,
                attrs,
                self_closing,
            } => {
                let n = Node {
                    tag,
                    attrs,
                    kids: Vec::new(),
                };
                if self_closing {
                    if let Some(top) = stack.last_mut() {
                        top.kids.push(Item::El(n));
                    }
                } else {
                    stack.push(n);
                }
            }
            Tok::Close(tag) => {
                if stack.iter().skip(1).any(|n| n.tag == tag) {
                    while stack.len() > 1 {
                        let Some(n) = stack.pop() else { break };
                        let done = n.tag == tag;
                        if let Some(top) = stack.last_mut() {
                            top.kids.push(Item::El(n));
                        }
                        if done {
                            break;
                        }
                    }
                }
            }
            Tok::Unsupported(_) => {}
        }
    }
    while stack.len() > 1 {
        if let Some(n) = stack.pop()
            && let Some(top) = stack.last_mut()
        {
            top.kids.push(Item::El(n));
        }
    }
    stack.pop().unwrap_or_default()
}

/// Where the content goes.
#[derive(Clone, Copy, Default)]
struct Ctx {
    pre: bool,
    table: bool,
}

struct Conv<'a> {
    save: &'a mut dyn FnMut(&[u8], &'static str) -> Option<String>,
}

fn raw_text(n: &Node, out: &mut String) {
    for k in &n.kids {
        match k {
            Item::Text(t) => out.push_str(t),
            Item::El(e) if e.tag == "br" => out.push('\n'),
            Item::El(e) => raw_text(e, out),
        }
    }
}

fn wrap(m: &str, inner: &str) -> String {
    let t = inner.trim();
    if t.is_empty() {
        return inner.to_string();
    }
    // Keep surrounding spaces outside the markers (`** x**` is not bold).
    let lead = if inner.starts_with(' ') { " " } else { "" };
    let trail = if inner.ends_with(' ') { " " } else { "" };
    format!("{lead}{m}{t}{m}{trail}")
}

fn block(s: &str) -> String {
    format!("\n\n{}\n\n", s.trim_matches('\n'))
}

impl Conv<'_> {
    fn kids(&mut self, n: &Node, c: Ctx) -> String {
        let mut out = String::new();
        for k in &n.kids {
            match k {
                Item::Text(t) if c.pre => out.push_str(t),
                Item::Text(t) => out.push_str(&escape(&collapse_ws(t), c.table)),
                Item::El(e) => out.push_str(&self.el(e, c)),
            }
        }
        out
    }

    fn line(&mut self, n: &Node, c: Ctx) -> String {
        collapse_ws(&self.kids(n, c).replace("\\\n", " ").replace('\n', " "))
            .trim()
            .to_string()
    }

    fn img(&mut self, n: &Node) -> String {
        let alt = escape(html::attr(&n.attrs, "alt").unwrap_or(""), false);
        let src = html::attr(&n.attrs, "src").unwrap_or("").trim();
        let lower = src.to_ascii_lowercase();
        if lower.starts_with("data:") {
            let Ok(bytes) = image::data_uri(src) else {
                return alt;
            };
            let ext = match image::probe(&bytes).map(|p| p.format) {
                Some(image::Format::Png) => "png",
                Some(image::Format::Jpeg) => "jpg",
                Some(image::Format::Gif) => "gif",
                Some(image::Format::Webp) => "webp",
                Some(image::Format::Bmp) => "bmp",
                _ => return alt,
            };
            return match (self.save)(&bytes, ext) {
                Some(link) => format!("![{alt}]({link})"),
                None => alt,
            };
        }
        if lower.starts_with("http://") || lower.starts_with("https://") {
            // Kept as a reference only; the viewer never fetches remote images (S-NET).
            return format!("![{alt}]({})", encode_target(src));
        }
        alt
    }

    fn list(&mut self, n: &Node, ordered: bool) -> String {
        let mut out = String::new();
        let mut i = 1usize;
        for k in &n.kids {
            let Item::El(li) = k else { continue };
            let marker = if ordered {
                format!("{i}. ")
            } else {
                "- ".to_string()
            };
            i += 1;
            let body = if li.tag == "li" {
                tidy(&self.kids(li, Ctx::default()))
            } else {
                tidy(&self.el(li, Ctx::default()))
            };
            let pad = " ".repeat(marker.len());
            for (j, l) in body.lines().enumerate() {
                if j == 0 {
                    out.push_str(&marker);
                } else if !l.is_empty() {
                    out.push_str(&pad);
                }
                out.push_str(l);
                out.push('\n');
            }
            if body.is_empty() {
                out.push_str(marker.trim_end());
                out.push('\n');
            }
        }
        block(&out)
    }

    fn rows(&mut self, n: &Node, rows: &mut Vec<Vec<String>>) {
        for k in &n.kids {
            let Item::El(e) = k else { continue };
            match e.tag.as_str() {
                "tr" => {
                    let mut cells = Vec::new();
                    for c in &e.kids {
                        if let Item::El(td) = c
                            && matches!(td.tag.as_str(), "td" | "th")
                            && cells.len() < MAX_COLS
                        {
                            let ctx = Ctx {
                                pre: false,
                                table: true,
                            };
                            cells.push(self.line(td, ctx));
                        }
                    }
                    rows.push(cells);
                }
                "thead" | "tbody" | "tfoot" => self.rows(e, rows),
                _ => {}
            }
        }
    }

    fn table(&mut self, n: &Node, c: Ctx) -> String {
        if c.table {
            return self.line(n, c); // nested table: its text only
        }
        let mut rows = Vec::new();
        self.rows(n, &mut rows);
        let cols = rows.iter().map(Vec::len).max().unwrap_or(0);
        if cols == 0 {
            return String::new();
        }
        let mut out = String::new();
        for (i, r) in rows.iter().enumerate() {
            out.push_str(&table_row(r, cols));
            out.push('\n');
            if i == 0 {
                out.push('|');
                out.push_str(&"---|".repeat(cols));
                out.push('\n');
            }
        }
        block(&out)
    }

    fn el(&mut self, n: &Node, c: Ctx) -> String {
        match n.tag.as_str() {
            "b" | "strong" => wrap("**", &self.kids(n, c)),
            "i" | "em" => wrap("*", &self.kids(n, c)),
            "s" | "del" | "strike" => wrap("~~", &self.kids(n, c)),
            "mark" => wrap("==", &self.kids(n, c)),
            "code" if !c.pre => {
                let mut t = String::new();
                raw_text(n, &mut t);
                let t = collapse_ws(&t);
                if t.trim().is_empty() {
                    t
                } else if t.contains('`') {
                    format!("`` {t} ``")
                } else {
                    format!("`{t}`")
                }
            }
            "br" if c.table => " ".into(),
            "br" => "\\\n".into(),
            "a" => {
                let inner = self.kids(n, c);
                match html::attr(&n.attrs, "href").and_then(safe_href) {
                    Some(h) if !inner.trim().is_empty() => format!("[{}]({h})", inner.trim()),
                    _ => inner,
                }
            }
            "img" => self.img(n),
            h @ ("h1" | "h2" | "h3" | "h4" | "h5" | "h6") if !c.table => {
                let lvl = h
                    .get(1..)
                    .and_then(|d| d.parse::<usize>().ok())
                    .unwrap_or(1);
                block(&format!("{} {}", "#".repeat(lvl), self.line(n, c)))
            }
            "blockquote" if !c.table => {
                let inner = tidy(&self.kids(n, c));
                let q: Vec<String> = inner
                    .lines()
                    .map(|l| {
                        if l.is_empty() {
                            ">".into()
                        } else {
                            format!("> {l}")
                        }
                    })
                    .collect();
                block(&q.join("\n"))
            }
            "ul" if !c.table => self.list(n, false),
            "ol" if !c.table => self.list(n, true),
            "pre" if !c.table => {
                let mut t = String::new();
                raw_text(n, &mut t);
                let fence = if t.contains("```") { "````" } else { "```" };
                block(&format!("{fence}\n{}\n{fence}", t.trim_matches('\n')))
            }
            "hr" if !c.table => block("---"),
            "table" => self.table(n, c),
            "p" | "div" | "center" | "figure" | "figcaption" | "details" | "summary" | "dl"
            | "dt" | "dd" | "li" | "tr"
                if !c.table =>
            {
                block(&self.kids(n, c))
            }
            _ => self.kids(n, c),
        }
    }
}

/// Converts clipboard HTML to Markdown; `None` when it is larger than [`super::MAX_HTML`]
/// (paste plain text instead). `save(bytes, ext)` stores a `data:` image and returns its link.
pub fn convert(
    html_src: &str,
    save: &mut dyn FnMut(&[u8], &'static str) -> Option<String>,
) -> Option<String> {
    if html_src.len() > super::MAX_HTML {
        return None;
    }
    let mut budget = html::MAX_TAGS;
    let root = tree(html::sanitize(html_src, &mut budget));
    let mut c = Conv { save };
    Some(tidy(&c.kids(&root, Ctx::default())))
}
