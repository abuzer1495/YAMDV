//! Dialog-host protocol (S-IPC-05, F-09): the versioned `WM_COPYDATA` payload sent by
//! `yamdv.exe --dialog-host open <owner-hwnd> <nonce>` back to the main window.
//!
//! Layout (little-endian): magic `b"YMDH"`, version `u16`, kind `u16`, nonce `u64`,
//! status `u16`, count `u16`, then `count` × (`u32` length in UTF-16 units, UTF-16 units).
//! `COPYDATASTRUCT::dwData` carries [`MAGIC`] as well.

/// Magic (`"YMDH"` as little-endian `u32`), also used as `dwData`.
pub const MAGIC: u32 = u32::from_le_bytes(*b"YMDH");
/// Protocol version.
pub const VERSION: u16 = 1;
/// Payload size limit.
pub const MAX_BYTES: usize = 4 << 20;
/// Path count limit.
pub const MAX_PATHS: usize = 1000;
/// Path length limit in UTF-16 units.
pub const MAX_PATH_UNITS: usize = 32_767;
const HEADER: usize = 4 + 2 + 2 + 8 + 2 + 2;

/// Dialog kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    /// File-open dialog.
    Open = 1,
    /// Second launch forwarding its paths to the primary instance (S-IPC-01); `nonce` carries
    /// the `--line` value (0 = none) and the payload is limited to [`FORWARD_MAX_BYTES`].
    Forward = 2,
    // 3 was the font dialog (removed in V2-C: the settings panel has its own font pickers).
    /// Save-as dialog (`IFileSaveDialog`); the answer is one path.
    Save = 4,
    /// Print dialog (`PrintDlgExW`, prompt 008 §3.1); the answer is serializable data only:
    /// printer name, DEVMODE bytes (hex), page range (`from-to` or `all`), copies.
    Print = 5,
}

/// S-IPC-01 payload limit for [`Kind::Forward`].
pub const FORWARD_MAX_BYTES: usize = 64 * 1024;

/// Encodes a forward message; paths that would exceed 64 KB are dropped.
pub fn encode_forward(paths: &[String], line: u64) -> Vec<u8> {
    let mut keep = Vec::new();
    let mut size = HEADER;
    for p in paths {
        let n = 4 + 2 * p.encode_utf16().count();
        if size + n > FORWARD_MAX_BYTES {
            break;
        }
        size += n;
        keep.push(p.clone());
    }
    encode(&Reply {
        kind: Kind::Forward,
        nonce: line,
        outcome: Outcome::Paths(keep),
    })
}

/// Dialog outcome.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Outcome {
    /// User cancelled.
    Cancel,
    /// Selected paths.
    Paths(Vec<String>),
    /// The dialog could not be shown.
    Error,
}

/// A decoded result message.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Reply {
    /// Dialog kind.
    pub kind: Kind,
    /// Per-launch nonce.
    pub nonce: u64,
    /// Outcome.
    pub outcome: Outcome,
}

/// Encodes a reply; paths beyond the limits are dropped.
pub fn encode(r: &Reply) -> Vec<u8> {
    let mut v = Vec::with_capacity(HEADER + 64);
    v.extend_from_slice(&MAGIC.to_le_bytes());
    v.extend_from_slice(&VERSION.to_le_bytes());
    v.extend_from_slice(&(r.kind as u16).to_le_bytes());
    v.extend_from_slice(&r.nonce.to_le_bytes());
    let (status, paths): (u16, &[String]) = match &r.outcome {
        Outcome::Cancel => (0, &[]),
        Outcome::Paths(p) => (1, p),
        Outcome::Error => (2, &[]),
    };
    v.extend_from_slice(&status.to_le_bytes());
    let count_at = v.len();
    v.extend_from_slice(&0u16.to_le_bytes());
    let mut n = 0u16;
    for p in paths.iter().take(MAX_PATHS) {
        let u: Vec<u16> = p.encode_utf16().collect();
        if u.is_empty() || u.len() > MAX_PATH_UNITS || v.len() + 4 + 2 * u.len() > MAX_BYTES {
            continue;
        }
        v.extend_from_slice(&u32::try_from(u.len()).unwrap_or(0).to_le_bytes());
        for c in u {
            v.extend_from_slice(&c.to_le_bytes());
        }
        n += 1;
    }
    if let Some(c) = v.get_mut(count_at..count_at + 2) {
        c.copy_from_slice(&n.to_le_bytes());
    }
    v
}

/// Why a payload was rejected.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DecodeError {
    /// Too short, too long, trailing bytes or bad length fields.
    Malformed,
    /// Wrong magic.
    Magic,
    /// Unsupported version.
    Version,
    /// Unknown kind or status.
    Unknown,
    /// A path is not valid UTF-16.
    Encoding,
}

fn rd<const N: usize>(b: &[u8], at: &mut usize) -> Result<[u8; N], DecodeError> {
    let s = b.get(*at..*at + N).ok_or(DecodeError::Malformed)?;
    *at += N;
    s.try_into().map_err(|_| DecodeError::Malformed)
}

/// Decodes and validates a payload (strict: exact length, limits, valid UTF-16).
pub fn decode(b: &[u8]) -> Result<Reply, DecodeError> {
    if b.len() < HEADER || b.len() > MAX_BYTES {
        return Err(DecodeError::Malformed);
    }
    let mut at = 0;
    if u32::from_le_bytes(rd(b, &mut at)?) != MAGIC {
        return Err(DecodeError::Magic);
    }
    if u16::from_le_bytes(rd(b, &mut at)?) != VERSION {
        return Err(DecodeError::Version);
    }
    let kind = match u16::from_le_bytes(rd(b, &mut at)?) {
        1 => Kind::Open,
        2 if b.len() <= FORWARD_MAX_BYTES => Kind::Forward,
        4 => Kind::Save,
        5 => Kind::Print,
        2 => return Err(DecodeError::Malformed),
        _ => return Err(DecodeError::Unknown),
    };
    let nonce = u64::from_le_bytes(rd(b, &mut at)?);
    let status = u16::from_le_bytes(rd(b, &mut at)?);
    let count = usize::from(u16::from_le_bytes(rd(b, &mut at)?));
    if count > MAX_PATHS || (status != 1 && count != 0) {
        return Err(DecodeError::Malformed);
    }
    let outcome = match status {
        0 => Outcome::Cancel,
        2 => Outcome::Error,
        1 => {
            let mut paths = Vec::with_capacity(count);
            for _ in 0..count {
                let n = u32::from_le_bytes(rd(b, &mut at)?) as usize;
                if n == 0 || n > MAX_PATH_UNITS {
                    return Err(DecodeError::Malformed);
                }
                let raw = b.get(at..at + 2 * n).ok_or(DecodeError::Malformed)?;
                at += 2 * n;
                let u: Vec<u16> = raw
                    .as_chunks::<2>()
                    .0
                    .iter()
                    .map(|c| u16::from_le_bytes(*c))
                    .collect();
                paths.push(String::from_utf16(&u).map_err(|_| DecodeError::Encoding)?);
            }
            Outcome::Paths(paths)
        }
        _ => return Err(DecodeError::Unknown),
    };
    if at != b.len() {
        return Err(DecodeError::Malformed);
    }
    Ok(Reply {
        kind,
        nonce,
        outcome,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rt(o: Outcome) {
        let r = Reply {
            kind: Kind::Open,
            nonce: 0x0123_4567_89ab_cdef,
            outcome: o,
        };
        assert_eq!(decode(&encode(&r)), Ok(r));
    }

    #[test]
    fn round_trips() {
        rt(Outcome::Cancel);
        rt(Outcome::Error);
        rt(Outcome::Paths(vec![
            "C:\\a\\b.md".into(),
            "D:\\çğü 漢字 😀.txt".into(),
        ]));
    }

    #[test]
    fn rejects_bad_payloads() {
        let ok = encode(&Reply {
            kind: Kind::Open,
            nonce: 7,
            outcome: Outcome::Paths(vec!["x".into()]),
        });
        assert_eq!(decode(&ok[..10]), Err(DecodeError::Malformed));
        let mut b = ok.clone();
        b[0] ^= 1;
        assert_eq!(decode(&b), Err(DecodeError::Magic));
        let mut b = ok.clone();
        b[4] = 9;
        assert_eq!(decode(&b), Err(DecodeError::Version));
        let mut b = ok.clone();
        b.push(0);
        assert_eq!(decode(&b), Err(DecodeError::Malformed));
        let mut b = ok.clone();
        let n = b.len();
        b[n - 2..].copy_from_slice(&0xD800u16.to_le_bytes());
        assert_eq!(decode(&b), Err(DecodeError::Encoding));
        let mut b = ok;
        b[20..24].copy_from_slice(&u32::MAX.to_le_bytes());
        assert_eq!(decode(&b), Err(DecodeError::Malformed));
    }

    #[test]
    fn decode_never_panics_on_prefixes() {
        let b = encode(&Reply {
            kind: Kind::Open,
            nonce: 1,
            outcome: Outcome::Paths(vec!["abc".into(), "def".into()]),
        });
        for i in 0..b.len() {
            let _ = decode(&b[..i]);
        }
    }

    #[test]
    fn s_ipc_01_forward_round_trip_and_limit() {
        let b = encode_forward(&["C:\\a.md".to_string()], 42);
        let r = decode(&b).unwrap();
        assert_eq!(r.kind, Kind::Forward);
        assert_eq!(r.nonce, 42);
        let many: Vec<String> = (0..100).map(|i| format!("C:\\{i:0>500}.md")).collect();
        let b = encode_forward(&many, 0);
        assert!(b.len() <= FORWARD_MAX_BYTES);
        assert!(decode(&b).is_ok());
        let mut big = encode(&Reply {
            kind: Kind::Forward,
            nonce: 0,
            outcome: Outcome::Paths(many),
        });
        assert!(big.len() > FORWARD_MAX_BYTES);
        assert_eq!(decode(&big), Err(DecodeError::Malformed));
        big.truncate(10);
        assert!(decode(&big).is_err());
    }
}
