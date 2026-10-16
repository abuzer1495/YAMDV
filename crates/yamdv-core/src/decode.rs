//! Decoding and file facts (R-OPEN-06/07/08/10, S-PARSE-02/08).

use encoding_rs::{Encoding, UTF_8};

/// Hard refusal limit (R-OPEN-08, S-PARSE-02): 256 MiB.
pub const HARD_LIMIT: u64 = 256 * 1024 * 1024;
/// Soft prompt limit: 20 MiB.
pub const SOFT_LIMIT: u64 = 20 * 1024 * 1024;
/// Bytes inspected for NULs.
pub const BINARY_PROBE: usize = 8 * 1024;

/// Result of the size/binary guards.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Guard {
    /// Open normally.
    Ok,
    /// Larger than 20 MiB: ask "plain text / render anyway / cancel".
    AskLarge,
    /// NUL bytes in the first 8 KiB: ask "open anyway / cancel".
    AskBinary,
    /// Larger than 256 MiB: refuse.
    Refuse,
}

/// Size guard (needs only the file size).
pub fn size_guard(size: u64) -> Guard {
    if size > HARD_LIMIT {
        Guard::Refuse
    } else if size > SOFT_LIMIT {
        Guard::AskLarge
    } else {
        Guard::Ok
    }
}

/// Binary guard on the first bytes. UTF-16 files (BOM) legitimately contain NULs.
pub fn binary_guard(head: &[u8]) -> Guard {
    if head.starts_with(&[0xFF, 0xFE]) || head.starts_with(&[0xFE, 0xFF]) {
        return Guard::Ok;
    }
    if head.iter().take(BINARY_PROBE).any(|b| *b == 0) {
        Guard::AskBinary
    } else {
        Guard::Ok
    }
}

/// Line-ending style (R-OPEN-07).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LineEndings {
    /// Only LF (or no line breaks).
    Lf,
    /// Only CRLF.
    CrLf,
    /// Both; `dominant_crlf` tells which is more frequent.
    Mixed {
        /// CRLF is the dominant ending.
        dominant_crlf: bool,
    },
}

/// Decoded text and facts about the original bytes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Decoded {
    /// UTF-8 text (invalid sequences replaced by U+FFFD, S-PARSE-08).
    pub text: String,
    /// Encoding name (WHATWG label).
    pub encoding: &'static str,
    /// Input started with a BOM.
    pub had_bom: bool,
    /// Decoding replaced invalid sequences (editing disabled later, R-EDIT-12).
    pub lossy: bool,
    /// Line endings.
    pub line_endings: LineEndings,
    /// Text ends with a line break.
    pub final_newline: bool,
}

/// Detects line endings of decoded text.
pub fn line_endings(text: &str) -> LineEndings {
    let b = text.as_bytes();
    let (mut lf, mut crlf) = (0usize, 0usize);
    for (i, c) in b.iter().enumerate() {
        if *c == b'\n' {
            if i > 0 && b.get(i - 1) == Some(&b'\r') {
                crlf += 1
            } else {
                lf += 1
            }
        }
    }
    match (lf, crlf) {
        (_, 0) => LineEndings::Lf,
        (0, _) => LineEndings::CrLf,
        (l, c) => LineEndings::Mixed {
            dominant_crlf: c > l,
        },
    }
}

/// "Damaged UTF-8" heuristic: at least as many valid multi-byte UTF-8 characters as invalid
/// sequences. Such files are decoded as lossy UTF-8 (U+FFFD, S-PARSE-08) instead of being handed
/// to `chardetng`, which would mis-detect them as a legacy code page (ADR-0007 decision).
fn mostly_utf8(bytes: &[u8]) -> bool {
    let (mut good, mut bad) = (0usize, 0usize);
    for chunk in bytes.utf8_chunks() {
        good += chunk.valid().chars().filter(|c| !c.is_ascii()).count();
        if !chunk.invalid().is_empty() {
            bad += 1;
        }
    }
    good > 0 && good >= bad
}

/// Decodes bytes per R-OPEN-06: BOM → that encoding; valid UTF-8 → UTF-8; else chardetng.
pub fn decode(bytes: &[u8]) -> Decoded {
    let (enc, bom_len): (&'static Encoding, usize) = match Encoding::for_bom(bytes) {
        Some((e, n)) => (e, n),
        None if std::str::from_utf8(bytes).is_ok() || mostly_utf8(bytes) => (UTF_8, 0),
        None => {
            let mut det = chardetng::EncodingDetector::new(chardetng::Iso2022JpDetection::Deny);
            det.feed(bytes, true);
            (det.guess(None, chardetng::Utf8Detection::Deny), 0)
        }
    };
    let body = bytes.get(bom_len..).unwrap_or(&[]);
    let (t, lossy) = enc.decode_without_bom_handling(body);
    let text = t.into_owned();
    let le = line_endings(&text);
    let final_newline = text.ends_with('\n');
    Decoded {
        text,
        encoding: enc.name(),
        had_bom: bom_len > 0,
        lossy,
        line_endings: le,
        final_newline,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn r_open_08_guards() {
        assert_eq!(size_guard(HARD_LIMIT + 1), Guard::Refuse);
        assert_eq!(size_guard(HARD_LIMIT), Guard::AskLarge);
        assert_eq!(size_guard(SOFT_LIMIT), Guard::Ok);
        assert_eq!(binary_guard(b"abc\0"), Guard::AskBinary);
        assert_eq!(binary_guard(b"\xFF\xFEa\0"), Guard::Ok);
        let mut late = vec![b'a'; BINARY_PROBE];
        late.push(0);
        assert_eq!(binary_guard(&late), Guard::Ok);
    }

    #[test]
    fn r_open_06_boms_and_utf8() {
        let d = decode(b"\xEF\xBB\xBFh\xC3\xA9");
        assert_eq!(
            (d.text.as_str(), d.encoding, d.had_bom, d.lossy),
            ("hé", "UTF-8", true, false)
        );
        let d = decode(b"\xFF\xFEh\x00i\x00");
        assert_eq!((d.text.as_str(), d.encoding), ("hi", "UTF-16LE"));
        let d = decode(b"\xFE\xFF\x00h\x00i");
        assert_eq!((d.text.as_str(), d.encoding), ("hi", "UTF-16BE"));
        let d = decode("çğüşİı".as_bytes());
        assert_eq!(
            (d.text.as_str(), d.encoding, d.had_bom),
            ("çğüşİı", "UTF-8", false)
        );
    }

    #[test]
    fn s_parse_08_lossy_utf16() {
        let d = decode(b"\xFF\xFE\x00\xD8a\x00");
        assert!(d.lossy);
        assert!(d.text.contains('\u{FFFD}'));
    }

    #[test]
    fn r_open_07_line_endings() {
        assert_eq!(line_endings("a\nb\n"), LineEndings::Lf);
        assert_eq!(line_endings("a\r\nb\r\n"), LineEndings::CrLf);
        assert_eq!(
            line_endings("a\r\nb\r\nc\n"),
            LineEndings::Mixed {
                dominant_crlf: true
            }
        );
        assert!(decode(b"x\n").final_newline);
        assert!(!decode(b"x").final_newline);
    }
}
