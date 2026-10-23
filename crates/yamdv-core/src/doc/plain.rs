//! Rendered plain text of a Markdown snippet (F-23: footnote tooltip preview).
//!
//! Emphasis markers vanish, links keep their text (no URL), code spans keep their text without
//! backticks, math keeps its LaTeX source, and HTML tags are dropped.

use pulldown_cmark::{Event, Parser, Tag, TagEnd};

/// Plain text of `src` as it renders; whitespace runs collapse to one space.
#[must_use]
pub fn plain_text(src: &str) -> String {
    let opts = super::parser_options(super::ReadOpts::default());
    let mut out = String::new();
    for ev in Parser::new_ext(src, opts) {
        match ev {
            Event::Text(t) | Event::Code(t) | Event::InlineMath(t) | Event::DisplayMath(t) => {
                out.push_str(&t);
            }
            Event::SoftBreak
            | Event::HardBreak
            | Event::Rule
            | Event::Start(Tag::Item)
            | Event::End(TagEnd::Paragraph | TagEnd::Heading(_) | TagEnd::Item) => out.push(' '),
            Event::FootnoteReference(l) => {
                out.push('[');
                out.push_str(&l);
                out.push(']');
            }
            Event::TaskListMarker(done) => out.push_str(if done { "☑ " } else { "☐ " }),
            _ => {}
        }
    }
    out.split_whitespace().collect::<Vec<_>>().join(" ")
}

#[cfg(test)]
mod tests {
    use super::plain_text;

    #[test]
    fn f23_markup_is_removed() {
        assert_eq!(
            plain_text("with *emphasis* and **strong** and ~~gone~~ removed."),
            "with emphasis and strong and gone removed."
        );
        assert_eq!(
            plain_text("see [the docs](https://example.com/x \"T\") and <https://a.b>"),
            "see the docs and https://a.b"
        );
        assert_eq!(plain_text("run `cargo test` now"), "run cargo test now");
        assert_eq!(plain_text("area $\\pi r^2$ here"), "area \\pi r^2 here");
        assert_eq!(plain_text("a <b>bold</b>\nline"), "a bold line");
        assert_eq!(plain_text("![alt text](i.png)"), "alt text");
    }
}
