//! Export job progress (BUG-11, R-EXP-07, UI_SPEC B.10.7): the worker → window record codec,
//! the 100 ms send throttle, the time estimate and the card's text lines.

use crate::i18n::Lang;

/// `WM_COPYDATA` `dwData` of a progress record ("YDPG").
pub const MAGIC: u32 = 0x5944_5047;
/// Codec version.
pub const VERSION: u16 = 1;
/// Encoded size in bytes.
pub const LEN: usize = 24;
/// Minimum interval between two records (ms).
pub const MIN_INTERVAL_MS: u64 = 100;

/// Job phase.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Phase {
    /// Layout / pagination (indeterminate bar).
    Preparing = 1,
    /// Pages (or blocks) are being produced (determinate bar).
    Printing = 2,
}

/// What a job produces.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JobKind {
    /// Export PDF.
    Pdf,
    /// Print….
    Print,
    /// Export HTML (counts blocks).
    Html,
}

/// One progress record.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Record {
    /// Per-job nonce (S-IPC-05).
    pub nonce: u64,
    /// Phase.
    pub phase: Phase,
    /// Current page (or block), 0..=total.
    pub n: u32,
    /// Total pages (or blocks); 0 = unknown.
    pub total: u32,
}

/// Encodes `r` (little endian: magic, version, phase, nonce, n, total).
pub fn encode(r: &Record) -> [u8; LEN] {
    let mut b = [0u8; LEN];
    b[0..4].copy_from_slice(&MAGIC.to_le_bytes());
    b[4..6].copy_from_slice(&VERSION.to_le_bytes());
    b[6..8].copy_from_slice(&(r.phase as u16).to_le_bytes());
    b[8..16].copy_from_slice(&r.nonce.to_le_bytes());
    b[16..20].copy_from_slice(&r.n.to_le_bytes());
    b[20..24].copy_from_slice(&r.total.to_le_bytes());
    b
}

/// Decodes a record; `None` on a wrong size, magic, version or phase, or `n > total`.
pub fn decode(b: &[u8]) -> Option<Record> {
    if b.len() != LEN {
        return None;
    }
    let u16_at = |i: usize| u16::from_le_bytes([b[i], b[i + 1]]);
    let u32_at = |i: usize| u32::from_le_bytes([b[i], b[i + 1], b[i + 2], b[i + 3]]);
    if u32_at(0) != MAGIC || u16_at(4) != VERSION {
        return None;
    }
    let phase = match u16_at(6) {
        1 => Phase::Preparing,
        2 => Phase::Printing,
        _ => return None,
    };
    let mut nb = [0u8; 8];
    nb.copy_from_slice(&b[8..16]);
    let (n, total) = (u32_at(16), u32_at(20));
    if total != 0 && n > total {
        return None;
    }
    Some(Record {
        nonce: u64::from_le_bytes(nb),
        phase,
        n,
        total,
    })
}

/// Rate limit for records: at most one per [`MIN_INTERVAL_MS`], except phase changes and the
/// final record.
#[derive(Debug, Default, Clone)]
pub struct Throttle {
    last: Option<(u64, Phase)>,
}

impl Throttle {
    /// Whether a record for `phase`, `n` of `total` at `now_ms` should be sent (and marks it).
    pub fn due(&mut self, now_ms: u64, phase: Phase, n: u32, total: u32) -> bool {
        let force = total != 0 && n >= total;
        let ok = match self.last {
            None => true,
            Some((t, p)) => force || p != phase || now_ms.saturating_sub(t) >= MIN_INTERVAL_MS,
        };
        if ok {
            self.last = Some((now_ms, phase));
        }
        ok
    }
}

/// Seconds left, estimated linearly; shown only after 5 units or 5 % (B.10.7).
pub fn eta_secs(elapsed_ms: u64, n: u32, total: u32) -> Option<u64> {
    if n == 0 || total == 0 || n >= total {
        return None;
    }
    if n < 5 && u64::from(n) * 20 < u64::from(total) {
        return None;
    }
    let per = elapsed_ms as f64 / f64::from(n);
    Some(((per * f64::from(total - n)) / 1000.0).round() as u64)
}

/// Whole percent (floor), 0..=100.
pub fn percent(n: u32, total: u32) -> u32 {
    if total == 0 {
        0
    } else {
        ((u64::from(n.min(total)) * 100) / u64::from(total)) as u32
    }
}

/// "about 3 min left" / "about 40 s left" (TR: "yaklaşık 3 dk kaldı").
pub fn eta_text(lang: Lang, secs: u64) -> String {
    let (v, en, tr) = if secs >= 90 {
        (secs.div_ceil(60), "min", "dk")
    } else {
        (secs.max(1), "s", "sn")
    };
    match lang {
        Lang::En => format!("about {v} {en} left"),
        Lang::Tr => format!("yaklaşık {v} {tr} kaldı"),
    }
}

/// Card line 1 prefix: "Exporting PDF…" / "Printing…" / "Exporting HTML…".
pub fn title(lang: Lang, k: JobKind) -> &'static str {
    match (lang, k) {
        (Lang::En, JobKind::Pdf) => "Exporting PDF…",
        (Lang::En, JobKind::Print) => "Printing…",
        (Lang::En, JobKind::Html) => "Exporting HTML…",
        (Lang::Tr, JobKind::Pdf) => "PDF dışa aktarılıyor…",
        (Lang::Tr, JobKind::Print) => "Yazdırılıyor…",
        (Lang::Tr, JobKind::Html) => "HTML dışa aktarılıyor…",
    }
}

/// Card line 2: "Page 37 of 1093 · 3 % · about 3 min left" (HTML: "Block …"); "Preparing…"
/// while the total is unknown.
pub fn status_line(lang: Lang, k: JobKind, r: &Record, elapsed_ms: u64) -> String {
    if r.phase == Phase::Preparing || r.total == 0 {
        return match lang {
            Lang::En => "Preparing…".into(),
            Lang::Tr => "Hazırlanıyor…".into(),
        };
    }
    let unit = match (lang, k) {
        (Lang::En, JobKind::Html) => "Block",
        (Lang::En, _) => "Page",
        (Lang::Tr, JobKind::Html) => "Blok",
        (Lang::Tr, _) => "Sayfa",
    };
    let mut s = match lang {
        Lang::En => format!("{unit} {} of {}", r.n, r.total),
        Lang::Tr => format!("{unit} {} / {}", r.n, r.total),
    };
    s.push_str(&format!(" · {} %", percent(r.n, r.total)));
    if let Some(e) = eta_secs(elapsed_ms, r.n, r.total) {
        s.push_str(" · ");
        s.push_str(&eta_text(lang, e));
    }
    s
}

/// Shortens `s` to at most `max` characters with a middle ellipsis (file names, B.10.7).
pub fn middle_ellipsis(s: &str, max: usize) -> String {
    let n = s.chars().count();
    if n <= max || max < 3 {
        return s.to_string();
    }
    let keep = max - 1;
    let head = keep.div_ceil(2);
    let tail = keep - head;
    let mut out: String = s.chars().take(head).collect();
    out.push('…');
    out.extend(s.chars().skip(n - tail));
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bug_11_codec_round_trip_and_rejects() {
        let r = Record {
            nonce: 0x0102_0304_0506_0708,
            phase: Phase::Printing,
            n: 37,
            total: 1093,
        };
        let b = encode(&r);
        assert_eq!(decode(&b), Some(r));
        assert_eq!(decode(&b[..23]), None);
        let mut bad = b;
        bad[0] ^= 1;
        assert_eq!(decode(&bad), None);
        let mut bad = b;
        bad[6] = 9;
        assert_eq!(decode(&bad), None);
        let over = encode(&Record {
            n: 5,
            total: 4,
            ..r
        });
        assert_eq!(decode(&over), None);
        let prep = encode(&Record {
            phase: Phase::Preparing,
            n: 0,
            total: 0,
            ..r
        });
        assert_eq!(decode(&prep).map(|x| x.phase), Some(Phase::Preparing));
    }

    #[test]
    fn bug_11_throttle_100_ms() {
        let mut t = Throttle::default();
        assert!(t.due(0, Phase::Preparing, 0, 0));
        assert!(!t.due(50, Phase::Preparing, 0, 0));
        assert!(t.due(60, Phase::Printing, 1, 10), "phase change");
        assert!(!t.due(100, Phase::Printing, 2, 10));
        assert!(t.due(160, Phase::Printing, 3, 10));
        assert!(t.due(170, Phase::Printing, 10, 10), "final record");
    }

    #[test]
    fn bug_11_estimate_and_text() {
        assert_eq!(eta_secs(1000, 4, 1093), None, "needs 5 pages or 5 %");
        assert_eq!(eta_secs(1000, 1, 10), Some(9), "10 % of 10 pages");
        assert_eq!(eta_secs(5_000, 5, 1093), Some(1088));
        assert_eq!(eta_secs(1000, 10, 10), None);
        assert_eq!(percent(37, 1093), 3);
        assert_eq!(percent(5, 0), 0);
        let r = Record {
            nonce: 1,
            phase: Phase::Printing,
            n: 37,
            total: 1093,
        };
        // 190 ms per page → (1093-37)·0.19 s ≈ 201 s ≈ 4 min.
        assert_eq!(
            status_line(Lang::En, JobKind::Pdf, &r, 37 * 190),
            "Page 37 of 1093 · 3 % · about 4 min left"
        );
        assert_eq!(
            status_line(Lang::Tr, JobKind::Html, &r, 37 * 10),
            "Blok 37 / 1093 · 3 % · yaklaşık 11 sn kaldı"
        );
        let p = Record {
            phase: Phase::Preparing,
            ..r
        };
        assert_eq!(status_line(Lang::En, JobKind::Print, &p, 0), "Preparing…");
        assert_eq!(title(Lang::Tr, JobKind::Print), "Yazdırılıyor…");
        assert_eq!(middle_ellipsis("abcdefghij.md", 7), "abc….md");
        assert_eq!(middle_ellipsis("short.md", 20), "short.md");
    }
}
