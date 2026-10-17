//! Save-side encoding (R-EDIT-02, S-FILE-10, prompt 006 §2.2): the text goes back in the
//! encoding it was read with, with the same BOM; line endings are never normalised (the editor
//! inserts the dominant ending for new lines). Conflict stamps (R-EDIT-03).

use encoding_rs::Encoding;

use crate::decode::LineEndings;

/// Why a text cannot be written in its original encoding.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EncodeError {
    /// A character has no mapping in the legacy code page (would become `&#…;`).
    Unmappable,
    /// The encoding name is unknown.
    Unknown,
}

/// Encodes `text` as `encoding` (a WHATWG name from [`crate::decode::decode`]), with a BOM when
/// the original had one.
pub fn encode(text: &str, encoding: &str, bom: bool) -> Result<Vec<u8>, EncodeError> {
    match encoding {
        "UTF-8" => {
            let mut out = Vec::with_capacity(text.len() + 3);
            if bom {
                out.extend_from_slice(b"\xEF\xBB\xBF");
            }
            out.extend_from_slice(text.as_bytes());
            Ok(out)
        }
        "UTF-16LE" | "UTF-16BE" => {
            let le = encoding == "UTF-16LE";
            let mut out = Vec::with_capacity(text.len() * 2 + 2);
            let mut push = |u: u16| {
                out.extend_from_slice(&if le { u.to_le_bytes() } else { u.to_be_bytes() });
            };
            if bom {
                push(0xFEFF);
            }
            for u in text.encode_utf16() {
                push(u);
            }
            Ok(out)
        }
        name => {
            let enc = Encoding::for_label(name.as_bytes()).ok_or(EncodeError::Unknown)?;
            let (bytes, used, unmappable) = enc.encode(text);
            if unmappable || used != enc {
                return Err(EncodeError::Unmappable);
            }
            Ok(bytes.into_owned())
        }
    }
}

/// Line break for new lines: the dominant ending of the file (LF for files without breaks).
pub fn dominant_eol(le: LineEndings) -> &'static str {
    match le {
        LineEndings::CrLf
        | LineEndings::Mixed {
            dominant_crlf: true,
        } => "\r\n",
        _ => "\n",
    }
}

/// FNV-1a 64-bit hash of the file bytes (conflict detection only, not security).
pub fn hash(bytes: &[u8]) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for b in bytes {
        h ^= u64::from(*b);
        h = h.wrapping_mul(0x0100_0000_01b3);
    }
    h
}

/// Last known on-disk state of the document (size, mtime, hash).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Stamp {
    /// Size in bytes.
    pub size: u64,
    /// Last write time (FILETIME ticks).
    pub mtime: u64,
    /// [`hash`] of the content.
    pub hash: u64,
}

impl Stamp {
    /// Stamp of bytes just read or written.
    pub fn of(bytes: &[u8], mtime: u64) -> Self {
        Self {
            size: bytes.len() as u64,
            mtime,
            hash: hash(bytes),
        }
    }

    /// R-EDIT-03: did the file change on disk? Size and mtime are compared first; when they
    /// differ, the content hash decides (`read` returns the current bytes, `None` = gone).
    pub fn changed(&self, size: u64, mtime: u64, read: impl FnOnce() -> Option<Vec<u8>>) -> bool {
        if size == self.size && mtime == self.mtime {
            return false;
        }
        match read() {
            Some(b) => hash(&b) != self.hash || b.len() as u64 != self.size,
            None => true,
        }
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    use super::*;
    use crate::decode::decode;

    #[test]
    fn roundtrip_keeps_bom_and_encoding() {
        for bytes in [
            b"\xEF\xBB\xBFa\r\nb".to_vec(),
            b"\xFF\xFEa\x00\r\x00\n\x00".to_vec(),
            b"\xFE\xFF\x00a".to_vec(),
            b"plain\n".to_vec(),
        ] {
            let d = decode(&bytes);
            assert_eq!(encode(&d.text, d.encoding, d.had_bom).unwrap(), bytes);
        }
    }

    #[test]
    fn unmappable_is_an_error() {
        assert_eq!(
            encode("emoji 🎉", "windows-1254", false),
            Err(EncodeError::Unmappable)
        );
        assert_eq!(
            encode("ğüşıöçİ", "windows-1254", false).unwrap(),
            b"\xF0\xFC\xFE\xFD\xF6\xE7\xDD"
        );
    }

    #[test]
    fn stamps_detect_changes_by_hash() {
        let s = Stamp::of(b"abc", 5);
        assert!(!s.changed(3, 5, || None));
        assert!(!s.changed(3, 9, || Some(b"abc".to_vec())));
        assert!(s.changed(3, 9, || Some(b"abd".to_vec())));
        assert!(s.changed(4, 9, || None));
        assert_eq!(
            dominant_eol(LineEndings::Mixed {
                dominant_crlf: true
            }),
            "\r\n"
        );
        assert_eq!(dominant_eol(LineEndings::Lf), "\n");
    }
}
