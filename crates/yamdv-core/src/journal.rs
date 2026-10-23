//! Recovery journal (prompt 015 §3.6): unsaved edits are journaled when autosave is off or a
//! save failed, one-shot 2 s after the last edit and at most every 10 s, to
//! `%LOCALAPPDATA%\YAMDV\recovery\` (the portable folder in portable mode). A journal is a
//! header (path, base mtime + hash, encoding) plus the UTF-8 text. Pure: format, names, timing.

/// Buffers above this are not journaled (one toast instead).
pub const MAX_TEXT: usize = 32 * 1024 * 1024;
/// Delay after the last edit.
pub const DELAY_MS: u64 = 2_000;
/// Minimum spacing of journal writes.
pub const MIN_GAP_MS: u64 = 10_000;
/// File extension of journal files.
pub const EXT: &str = "yjr";
const MAGIC: &str = "YAMDV-RECOVERY 1";
/// Header lines are bounded (paths are ≤ 32 767 UTF-16 units).
const MAX_HEADER: usize = 128 * 1024;

/// Journal header.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Header {
    /// Document path (empty for an untitled buffer).
    pub path: String,
    /// Base file's last-write time (FILETIME ticks) when the buffer was loaded / saved.
    pub mtime: u64,
    /// FNV-1a 64 of the base file's bytes.
    pub hash: u64,
    /// Encoding label of the document (re-applied on restore).
    pub encoding: String,
}

/// FNV-1a 64 (cheap change check of the base file; not a security hash).
pub fn hash(bytes: &[u8]) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for b in bytes {
        h ^= u64::from(*b);
        h = h.wrapping_mul(0x0100_0000_01b3);
    }
    h
}

/// Journal file name for a document path (case-insensitive, like NTFS) or an untitled id.
pub fn file_name(path: &str, untitled_id: u64) -> String {
    if path.is_empty() {
        return format!("untitled-{untitled_id:016x}.{EXT}");
    }
    format!("{:016x}.{EXT}", hash(path.to_lowercase().as_bytes()))
}

/// Serialized journal; `None` when the text is too large ([`MAX_TEXT`]).
pub fn encode(h: &Header, text: &str) -> Option<Vec<u8>> {
    if text.len() > MAX_TEXT {
        return None;
    }
    let clean = |s: &str| s.replace(['\r', '\n'], "");
    let head = format!(
        "{MAGIC}\npath: {}\nmtime: {}\nhash: {:016x}\nencoding: {}\nlen: {}\n\n",
        clean(&h.path),
        h.mtime,
        h.hash,
        clean(&h.encoding),
        text.len()
    );
    let mut out = head.into_bytes();
    out.extend_from_slice(text.as_bytes());
    Some(out)
}

/// Parses a journal; `None` for anything malformed or truncated (a torn write is ignored).
pub fn decode(bytes: &[u8]) -> Option<(Header, String)> {
    let end = bytes
        .windows(2)
        .take(MAX_HEADER)
        .position(|w| w == b"\n\n")?;
    let head = std::str::from_utf8(bytes.get(..end)?).ok()?;
    let body = bytes.get(end + 2..)?;
    let mut lines = head.lines();
    if lines.next()? != MAGIC {
        return None;
    }
    let mut h = Header::default();
    let mut len = None;
    for l in lines {
        let (k, v) = l.split_once(": ").unwrap_or((l.trim_end_matches(':'), ""));
        match k {
            "path" => h.path = v.to_string(),
            "mtime" => h.mtime = v.parse().ok()?,
            "hash" => h.hash = u64::from_str_radix(v, 16).ok()?,
            "encoding" => h.encoding = v.to_string(),
            "len" => len = Some(v.parse::<usize>().ok()?),
            _ => {}
        }
    }
    if len? != body.len() || body.len() > MAX_TEXT {
        return None;
    }
    Some((h, String::from_utf8(body.to_vec()).ok()?))
}

/// Journal only when autosave is off or the last save failed.
pub fn wanted(autosave: bool, save_failed: bool) -> bool {
    !autosave || save_failed
}

/// The base file changed since the journal was written (show the conflict banner too).
pub fn base_changed(h: &Header, mtime_now: u64, hash_now: Option<u64>) -> bool {
    if h.path.is_empty() {
        return false;
    }
    h.mtime != mtime_now || hash_now.is_some_and(|x| x != h.hash)
}

/// Write timing: one-shot [`DELAY_MS`] after the last edit, at most every [`MIN_GAP_MS`].
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Schedule {
    last_edit: Option<u64>,
    last_write: Option<u64>,
}

impl Schedule {
    /// An edit happened at `now` (ms).
    pub fn edit(&mut self, now: u64) {
        self.last_edit = Some(now);
    }
    /// The pending journal write was issued at `now`.
    pub fn written(&mut self, now: u64) {
        self.last_write = Some(now);
        self.last_edit = None;
    }
    /// Saved / discarded: nothing pending.
    pub fn clear(&mut self) {
        self.last_edit = None;
    }
    /// When the next write is due (ms), if one is pending.
    pub fn due(&self) -> Option<u64> {
        let e = self.last_edit? + DELAY_MS;
        Some(match self.last_write {
            Some(w) => e.max(w + MIN_GAP_MS),
            None => e,
        })
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn round_trip_and_rejects_torn_files() {
        let h = Header {
            path: "C:\\d\\a.md".into(),
            mtime: 133_000_000_000_000_000,
            hash: hash(b"base"),
            encoding: "windows-1254".into(),
        };
        let b = encode(&h, "x\r\nğ\n\n# t").unwrap();
        assert_eq!(decode(&b), Some((h.clone(), "x\r\nğ\n\n# t".to_string())));
        assert_eq!(decode(&b[..b.len() - 1]), None);
        assert_eq!(decode(b"garbage"), None);
        let mut evil = h.clone();
        evil.path = "a\nlen: 0".into();
        let (back, _) = decode(&encode(&evil, "t").unwrap()).unwrap();
        assert_eq!(back.path, "alen: 0");
        assert!(encode(&h, &"x".repeat(MAX_TEXT + 1)).is_none());
    }

    #[test]
    fn names_policy_and_conflict() {
        assert_eq!(file_name("C:\\D\\A.md", 0), file_name("c:\\d\\a.MD", 9));
        assert_ne!(file_name("C:\\d\\a.md", 0), file_name("C:\\d\\b.md", 0));
        assert!(file_name("", 7).starts_with("untitled-0000000000000007"));
        assert!(wanted(false, false) && wanted(true, true) && !wanted(true, false));
        let h = Header {
            path: "p".into(),
            mtime: 5,
            hash: 9,
            encoding: String::new(),
        };
        assert!(!base_changed(&h, 5, Some(9)));
        assert!(base_changed(&h, 6, Some(9)) && base_changed(&h, 5, Some(1)));
    }

    #[test]
    fn schedule_one_shot_and_min_gap() {
        let mut s = Schedule::default();
        assert_eq!(s.due(), None);
        s.edit(1_000);
        s.edit(1_500);
        assert_eq!(s.due(), Some(3_500));
        s.written(3_500);
        assert_eq!(s.due(), None);
        s.edit(4_000);
        assert_eq!(s.due(), Some(13_500));
        s.clear();
        assert_eq!(s.due(), None);
    }
}
