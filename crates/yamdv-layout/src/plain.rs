//! Plain-text mode (R-OPEN-09, "Open as plain text"): body font, preserved line breaks and spaces,
//! wrapping at the column width, virtualized in chunks of ≤ 200 lines.

use std::ops::Range;

/// Lines per chunk.
pub const CHUNK_LINES: usize = 200;

/// Chunk source ranges of `text` (each ≤ 200 lines, ending after a line break).
pub fn chunks(text: &str) -> Vec<Range<u32>> {
    let mut out = Vec::new();
    let mut start = 0usize;
    let mut lines = 0usize;
    for (i, b) in text.bytes().enumerate() {
        if b == b'\n' {
            lines += 1;
            if lines == CHUNK_LINES {
                out.push(u(start)..u(i + 1));
                start = i + 1;
                lines = 0;
            }
        }
    }
    if start < text.len() || out.is_empty() {
        out.push(u(start)..u(text.len()));
    }
    out
}

fn u(n: usize) -> u32 {
    u32::try_from(n).unwrap_or(u32::MAX)
}

/// Copy text of a plain-text selection: verbatim with CRLF line breaks.
pub fn copy_text(text: &str, sel: Range<u32>) -> String {
    let s = text.get(sel.start as usize..sel.end as usize).unwrap_or("");
    let mut out = String::with_capacity(s.len() + s.len() / 32);
    let mut prev_cr = false;
    for c in s.chars() {
        if c == '\n' && !prev_cr {
            out.push('\r');
        }
        out.push(c);
        prev_cr = c == '\r';
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chunking() {
        let t: String = (0..450).map(|i| format!("line {i}\n")).collect();
        let c = chunks(&t);
        assert_eq!(c.len(), 3);
        assert_eq!(c[0].start, 0);
        assert_eq!(c[2].end as usize, t.len());
        assert!(c.windows(2).all(|w| w[0].end == w[1].start));
        assert_eq!(chunks("").len(), 1);
        assert_eq!(copy_text("a\nb\r\nc", 0..6), "a\r\nb\r\nc");
    }
}
