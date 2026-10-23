//! Mind-map outline of a document (FUT-37): headings become branches and list items leaves.

use pulldown_cmark::{Event, Parser, Tag, TagEnd};

/// Longest leaf label (chars; longer ones end with "…").
pub const LEAF_CHARS: usize = 80;

/// One outline item in document order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MindItem {
    /// Heading level 1–6, or 6 + list nesting (7, 8, …) for list items.
    pub level: u8,
    /// Label.
    pub text: String,
    /// Source byte offset (jump target).
    pub offset: usize,
    /// List item (leaf style).
    pub leaf: bool,
}

fn clip(s: &str) -> String {
    let s = s.split_whitespace().collect::<Vec<_>>().join(" ");
    if s.chars().count() > LEAF_CHARS {
        let mut t: String = s.chars().take(LEAF_CHARS - 1).collect();
        t.push('…');
        t
    } else {
        s
    }
}

/// Root label (first H1, if any) and the items; the first H1 itself is not an item.
pub fn mind_items(src: &str) -> (Option<String>, Vec<MindItem>) {
    let opts = super::parser_options(super::ReadOpts::default());
    let mut items: Vec<MindItem> = Vec::new();
    let mut root: Option<String> = None;
    let mut cur: Option<MindItem> = None;
    let mut list_depth = 0u8;
    // Item text is collected until its first nested block (nested list) starts.
    let mut item_open = false;
    let mut heading_is_root = false;
    for (ev, r) in Parser::new_ext(src, opts).into_offset_iter() {
        match ev {
            Event::Start(Tag::Heading { level, .. }) => {
                let l = level as u8;
                heading_is_root = l == 1 && root.is_none();
                cur = Some(MindItem {
                    level: l,
                    text: String::new(),
                    offset: r.start,
                    leaf: false,
                });
            }
            Event::End(TagEnd::Heading(_)) => {
                if let Some(mut c) = cur.take() {
                    c.text = clip(&c.text);
                    if heading_is_root {
                        root = Some(c.text);
                    } else {
                        items.push(c);
                    }
                }
                heading_is_root = false;
            }
            Event::Start(Tag::List(_)) => {
                if let Some(mut c) = cur.take() {
                    c.text = clip(&c.text);
                    items.push(c);
                }
                item_open = false;
                list_depth = list_depth.saturating_add(1);
            }
            Event::End(TagEnd::List(_)) => list_depth = list_depth.saturating_sub(1),
            Event::Start(Tag::Item) => {
                cur = Some(MindItem {
                    level: 6u8.saturating_add(list_depth),
                    text: String::new(),
                    offset: r.start,
                    leaf: true,
                });
                item_open = true;
            }
            Event::End(TagEnd::Item) => {
                if item_open && let Some(mut c) = cur.take() {
                    c.text = clip(&c.text);
                    items.push(c);
                }
                item_open = false;
            }
            Event::Text(t) | Event::Code(t) | Event::InlineMath(t) => {
                if let Some(c) = cur.as_mut() {
                    c.text.push_str(&t);
                }
            }
            Event::SoftBreak | Event::HardBreak => {
                if let Some(c) = cur.as_mut() {
                    c.text.push(' ');
                }
            }
            _ => {}
        }
    }
    (root, items)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn headings_and_items() {
        let src = "# Title\n\nIntro\n\n## A\n\n- one *x*\n  - deep\n- two\n\n## B\n\n### B1\n";
        let (root, items) = mind_items(src);
        assert_eq!(root.as_deref(), Some("Title"));
        let got: Vec<(u8, &str)> = items.iter().map(|i| (i.level, i.text.as_str())).collect();
        assert_eq!(
            got,
            [
                (2, "A"),
                (7, "one x"),
                (8, "deep"),
                (7, "two"),
                (2, "B"),
                (3, "B1")
            ]
        );
        let long = format!("- {}\n", "w".repeat(200));
        let (r, it) = mind_items(&long);
        assert_eq!(r, None);
        assert_eq!(it[0].text.chars().count(), LEAF_CHARS);
    }
}
