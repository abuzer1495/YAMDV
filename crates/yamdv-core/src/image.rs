//! Image source resolution, network-location policy, header probe and sizing
//! (R-IMG-02…04, R-IMG-06, S-RES-01…04). Pure: every file-system question goes
//! through [`Fs`] so tests can prove that blocked forms never touch the disk.

use crate::link::percent_decode;

/// S-RES-04: maximum image file size in bytes.
pub const MAX_FILE: u64 = 50_000_000;
/// S-RES-04: maximum decoded `data:` URI payload in bytes.
pub const MAX_DATA: usize = 10_000_000;
/// S-RES-04: maximum width or height.
pub const MAX_DIM: u32 = 16_384;
/// S-RES-04: maximum pixel count.
pub const MAX_PIXELS: u64 = 64_000_000;
/// P-13: LRU budget for decoded images outside visible ± 1 screen.
pub const LRU_BYTES: usize = 1_572_864;

/// Result of `GetDriveTypeW` (only the classes the policy needs).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DriveKind {
    /// Fixed, removable, RAM disk or CD.
    Local,
    /// `DRIVE_REMOTE` (mapped network drive).
    Remote,
    /// Unknown / no root.
    Unknown,
}

/// The file-system layer used by the policy. The Windows implementation lives in
/// `yamdv-platform-win`; tests use a recording mock.
pub trait Fs {
    /// `GetDriveTypeW` for a root such as `C:\`.
    fn drive_kind(&mut self, root: &str) -> DriveKind;
    /// Opens the final component with `FILE_FLAG_OPEN_REPARSE_POINT`; returns the
    /// reparse target when it is a symlink/junction, `Some("")` for other
    /// reparse points, `None` when it is not a reparse point.
    fn reparse_target(&mut self, path: &str) -> Option<String>;
    /// File size, `None` if missing.
    fn size(&mut self, path: &str) -> Option<u64>;
}

/// A file system that answers "local, not a reparse point, missing" without any I/O
/// (fuzzing and pure layout).
#[derive(Debug, Default)]
pub struct NoFs;

impl Fs for NoFs {
    fn drive_kind(&mut self, _: &str) -> DriveKind {
        DriveKind::Local
    }
    fn reparse_target(&mut self, _: &str) -> Option<String> {
        None
    }
    fn size(&mut self, _: &str) -> Option<u64> {
        None
    }
}

/// Why an image is shown as a placeholder (R-IMG-06).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Placeholder {
    /// File not found.
    Missing,
    /// `http`/`https`/other remote scheme: never fetched (S-RES-03).
    Remote,
    /// UNC/WebDAV/mapped drive/`file://host`/device path or reparse to network (S-RES-02).
    Network,
    /// File, payload or dimensions over the S-RES-04 limits.
    TooLarge,
    /// Format not decodable (SVG until Phase 010, unknown bytes, missing codec).
    Unsupported,
    /// Malformed `data:` URI or empty source.
    Invalid,
    /// P-27: device name, alternate data stream or NUL — the file system is never touched.
    InvalidPath,
}

impl Placeholder {
    /// English message for the placeholder or chip tooltip.
    pub fn message(&self) -> &'static str {
        match self {
            Self::Missing => "Image not found",
            Self::Remote => "Remote image not loaded — YAMDV works offline",
            Self::Network => "Network image blocked",
            Self::TooLarge => "Image too large",
            Self::Unsupported => "Unsupported image format",
            Self::Invalid => "Invalid image source",
            Self::InvalidPath => "Invalid image path",
        }
    }
}

/// A resolved, policy-checked image source.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Source {
    /// Absolute local Windows path (backslashes), safe to open.
    File(String),
    /// Decoded `data:` payload.
    Data(Vec<u8>),
}

fn is_sep(c: char) -> bool {
    c == '\\' || c == '/'
}

/// `\\server\share` prefix of a UNC path (lower-cased), if any.
fn unc_share(p: &str) -> Option<String> {
    let rest = p.strip_prefix("\\\\")?;
    let mut it = rest.split('\\');
    let host = it.next()?;
    let share = it.next()?;
    if host.is_empty() || share.is_empty() || host == "?" || host == "." {
        return None;
    }
    Some(format!("\\\\{}\\{}", host, share).to_lowercase())
}

/// Lexically normalizes `.`/`..` in an absolute Windows path with a root of `root_len` bytes.
fn normalize(p: &str, root_len: usize) -> String {
    let (root, rest) = p.split_at(root_len.min(p.len()));
    let mut parts: Vec<&str> = Vec::new();
    for c in rest.split('\\') {
        match c {
            "" | "." => {}
            ".." => {
                parts.pop();
            }
            c => parts.push(c),
        }
    }
    let mut out = root.trim_end_matches('\\').to_string();
    for c in parts {
        out.push('\\');
        out.push_str(c);
    }
    out
}

fn root_len(p: &str) -> Option<usize> {
    let b = p.as_bytes();
    match b {
        [d, b':', b'\\', ..] if d.is_ascii_alphabetic() => return Some(3),
        [d, b':'] if d.is_ascii_alphabetic() => return Some(2),
        _ => {}
    }
    if let Some(s) = unc_share(p) {
        return Some(s.len());
    }
    None
}

/// Lexical part of S-RES-01/02/03: classifies `src` relative to the document path.
/// No file-system access. Returns an absolute path or a data payload.
pub fn resolve(src: &str, doc_path: &str) -> Result<Source, Placeholder> {
    let src = src.trim();
    if src.is_empty() {
        return Err(Placeholder::Invalid);
    }
    let lower = src.to_ascii_lowercase();
    if lower.starts_with("data:") {
        return data_uri(src).map(Source::Data);
    }
    let path = if let Some(rest) = lower.strip_prefix("file:") {
        let rest = &src[src.len() - rest.len()..];
        // file:///C:/x → C:/x ; file://host/x → network ; file:/C:/x tolerated.
        if let Some(r) = rest.strip_prefix("///") {
            if r.starts_with(['/', '\\']) {
                return Err(Placeholder::Network);
            }
            percent_decode(r)
        } else if let Some(r) = rest.strip_prefix("//") {
            if r.to_ascii_lowercase().starts_with("localhost/") {
                percent_decode(&r[10..])
            } else {
                return Err(Placeholder::Network);
            }
        } else {
            percent_decode(rest.trim_start_matches('/'))
        }
    } else if let Some(i) = src.find(':') {
        let scheme = &src[..i];
        if scheme.len() > 1
            && scheme
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || "+-.".contains(c))
        {
            // `name.ext:stream` is an alternate data stream, not a scheme (P-27).
            if scheme.contains('.') {
                return Err(Placeholder::InvalidPath);
            }
            return Err(Placeholder::Remote);
        }
        percent_decode(src)
    } else {
        percent_decode(src)
    };
    if path.contains('\0') {
        return Err(Placeholder::InvalidPath);
    }
    let path: String = path
        .chars()
        .map(|c| if c == '/' { '\\' } else { c })
        .collect();
    // Device / extended paths: \\?\ \\.\ \??\ — \\?\UNC\ is network, the rest device.
    let pl = path.to_ascii_lowercase();
    if pl.starts_with("\\\\?\\unc\\") || pl.starts_with("\\??\\unc\\") {
        return Err(Placeholder::Network);
    }
    if pl.starts_with("\\\\?\\") || pl.starts_with("\\\\.\\") || pl.starts_with("\\??\\") {
        return Err(Placeholder::InvalidPath);
    }
    if path.starts_with("\\\\") {
        return check_unc(&path, doc_path);
    }
    let abs = if root_len(&path).is_some() {
        if path.len() == 2 {
            return Err(Placeholder::Invalid);
        }
        path
    } else if path.starts_with('\\') {
        // Drive-relative root: use the document's drive.
        let d = doc_path.get(..2).unwrap_or("C:");
        format!("{d}{path}")
    } else {
        let dir = match doc_path.rfind(is_sep) {
            Some(i) => &doc_path[..i],
            None => ".",
        };
        let dir: String = dir
            .chars()
            .map(|c| if c == '/' { '\\' } else { c })
            .collect();
        format!("{dir}\\{path}")
    };
    let rl = root_len(&abs).ok_or(Placeholder::Invalid)?;
    let n = normalize(&abs, rl);
    // ADS: a colon after the drive letter (file.png:stream).
    if n[2..].contains(':') {
        return Err(Placeholder::InvalidPath);
    }
    if n.split('\\').any(is_device_name) {
        return Err(Placeholder::InvalidPath);
    }
    if n.starts_with("\\\\") {
        return check_unc(&n, doc_path);
    }
    Ok(Source::File(n))
}

/// DOS device names (`CON`, `NUL`, `COM1`, `lpt9.png`, …) in any path component.
fn is_device_name(c: &str) -> bool {
    let stem = c
        .split('.')
        .next()
        .unwrap_or("")
        .trim_end()
        .to_ascii_uppercase();
    matches!(
        stem.as_str(),
        "CON" | "PRN" | "AUX" | "NUL" | "CONIN$" | "CONOUT$"
    ) || (stem.len() == 4
        && (stem.starts_with("COM") || stem.starts_with("LPT"))
        && stem.as_bytes()[3].is_ascii_digit()
        && stem.as_bytes()[3] != b'0')
}

fn check_unc(path: &str, doc_path: &str) -> Result<Source, Placeholder> {
    let doc: String = doc_path
        .chars()
        .map(|c| if c == '/' { '\\' } else { c })
        .collect();
    let rl = root_len(path).ok_or(Placeholder::Network)?;
    let n = normalize(path, rl);
    let host = n[2..].split('\\').next().unwrap_or("");
    if host.contains('@') {
        // WebDAV forms are never allowed, not even for the same share.
        return Err(Placeholder::Network);
    }
    match (unc_share(&n), unc_share(&doc)) {
        (Some(a), Some(b))
            if a == b && !doc[2..].split('\\').next().unwrap_or("").contains('@') =>
        {
            if n[2..].contains(':') {
                return Err(Placeholder::Network);
            }
            Ok(Source::File(n))
        }
        _ => Err(Placeholder::Network),
    }
}

/// Full S-RES-02 policy: lexical checks first, then (only for local paths)
/// `GetDriveTypeW` and the reparse-point check, then the size limit.
pub fn resolve_policy(src: &str, doc_path: &str, fs: &mut dyn Fs) -> Result<Source, Placeholder> {
    let s = resolve(src, doc_path)?;
    let Source::File(p) = &s else {
        return Ok(s);
    };
    let same_share = p.starts_with("\\\\");
    if !same_share {
        let root = format!("{}\\", p.get(..2).unwrap_or(""));
        match fs.drive_kind(&root) {
            DriveKind::Remote => return Err(Placeholder::Network),
            DriveKind::Unknown => return Err(Placeholder::Missing),
            DriveKind::Local => {}
        }
    }
    if let Some(t) = fs.reparse_target(p) {
        let t: String = t.chars().map(|c| if c == '/' { '\\' } else { c }).collect();
        let tl = t.to_ascii_lowercase();
        if t.is_empty() || t.starts_with("\\\\") || tl.starts_with("\\??\\unc\\") {
            return Err(Placeholder::Network);
        }
        if !same_share && t.len() >= 2 {
            let root = format!("{}\\", t.get(..2).unwrap_or(""));
            if fs.drive_kind(&root) != DriveKind::Local {
                return Err(Placeholder::Network);
            }
        }
    }
    match fs.size(p) {
        None => Err(Placeholder::Missing),
        Some(n) if n > MAX_FILE => Err(Placeholder::TooLarge),
        Some(_) => Ok(s),
    }
}

const IMAGE_MIMES: &[&str] = &[
    "image/png",
    "image/jpeg",
    "image/jpg",
    "image/gif",
    "image/bmp",
    "image/webp",
    "image/x-icon",
    "image/vnd.microsoft.icon",
    "image/tiff",
];

/// Decodes `data:image/…;base64,…` within S-RES-04.
pub fn data_uri(src: &str) -> Result<Vec<u8>, Placeholder> {
    let rest = src.get(5..).ok_or(Placeholder::Invalid)?;
    let (meta, payload) = rest.split_once(',').ok_or(Placeholder::Invalid)?;
    let meta = meta.to_ascii_lowercase();
    let mut parts = meta.split(';');
    let mime = parts.next().unwrap_or("").trim();
    if mime == "image/svg+xml" {
        return Err(Placeholder::Unsupported);
    }
    if !IMAGE_MIMES.contains(&mime) {
        return Err(Placeholder::Invalid);
    }
    if !parts.any(|p| p.trim() == "base64") {
        return Err(Placeholder::Invalid);
    }
    if payload.len() / 4 * 3 > MAX_DATA + 3 {
        return Err(Placeholder::TooLarge);
    }
    let out = base64(payload).ok_or(Placeholder::Invalid)?;
    if out.len() > MAX_DATA {
        return Err(Placeholder::TooLarge);
    }
    Ok(out)
}

/// Standard base64 decoder tolerant of whitespace and missing padding.
pub fn base64(s: &str) -> Option<Vec<u8>> {
    let mut out = Vec::with_capacity(s.len() / 4 * 3);
    let mut acc = 0u32;
    let mut n = 0u8;
    let mut pad = false;
    for c in s.bytes() {
        let v = match c {
            b'A'..=b'Z' => c - b'A',
            b'a'..=b'z' => c - b'a' + 26,
            b'0'..=b'9' => c - b'0' + 52,
            b'+' | b'-' => 62,
            b'/' | b'_' => 63,
            b'=' => {
                pad = true;
                continue;
            }
            b' ' | b'\n' | b'\r' | b'\t' => continue,
            _ => return None,
        };
        if pad {
            return None;
        }
        acc = (acc << 6) | u32::from(v);
        n += 1;
        if n == 4 {
            let [_, a, b, c] = acc.to_be_bytes();
            out.extend_from_slice(&[a, b, c]);
            acc = 0;
            n = 0;
        }
    }
    match n {
        0 => {}
        2 => out.push((acc >> 4).to_be_bytes()[3]),
        3 => {
            let [_, _, a, b] = (acc >> 2).to_be_bytes();
            out.extend_from_slice(&[a, b]);
        }
        _ => return None,
    }
    Some(out)
}

/// Container format detected from the magic bytes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Format {
    /// PNG.
    Png,
    /// JPEG.
    Jpeg,
    /// GIF.
    Gif,
    /// BMP.
    Bmp,
    /// ICO.
    Ico,
    /// TIFF.
    Tiff,
    /// WebP.
    Webp,
    /// SVG (unsupported until Phase 010).
    Svg,
}

/// Header probe result.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Probe {
    /// Format.
    pub format: Format,
    /// Width in pixels (0 if unknown from the header).
    pub w: u32,
    /// Height in pixels.
    pub h: u32,
}

fn be16(b: &[u8], i: usize) -> Option<u32> {
    Some(u32::from(u16::from_be_bytes([*b.get(i)?, *b.get(i + 1)?])))
}
fn le16(b: &[u8], i: usize) -> Option<u32> {
    Some(u32::from(u16::from_le_bytes([*b.get(i)?, *b.get(i + 1)?])))
}
fn be32(b: &[u8], i: usize) -> Option<u32> {
    Some(u32::from_be_bytes([
        *b.get(i)?,
        *b.get(i + 1)?,
        *b.get(i + 2)?,
        *b.get(i + 3)?,
    ]))
}
fn le32(b: &[u8], i: usize) -> Option<u32> {
    Some(u32::from_le_bytes([
        *b.get(i)?,
        *b.get(i + 1)?,
        *b.get(i + 2)?,
        *b.get(i + 3)?,
    ]))
}

/// Reads dimensions from the first bytes of a file (R-IMG-04 layout-time probe).
/// `head` should hold at least 64 KiB for JPEG.
pub fn probe(head: &[u8]) -> Option<Probe> {
    let p = |format, w, h| Some(Probe { format, w, h });
    if head.starts_with(b"\x89PNG\r\n\x1a\n") {
        return p(Format::Png, be32(head, 16)?, be32(head, 20)?);
    }
    if head.starts_with(b"GIF87a") || head.starts_with(b"GIF89a") {
        return p(Format::Gif, le16(head, 6)?, le16(head, 8)?);
    }
    if head.starts_with(b"BM") {
        let h = le32(head, 22)? as i32;
        return p(Format::Bmp, le32(head, 18)?, h.unsigned_abs());
    }
    if head.starts_with(&[0, 0, 1, 0]) {
        let w = u32::from(*head.get(6)?);
        let h = u32::from(*head.get(7)?);
        return p(
            Format::Ico,
            if w == 0 { 256 } else { w },
            if h == 0 { 256 } else { h },
        );
    }
    if head.starts_with(b"RIFF") && head.get(8..12) == Some(b"WEBP") {
        return match head.get(12..16)? {
            b"VP8 " => p(
                Format::Webp,
                le16(head, 26)? & 0x3fff,
                le16(head, 28)? & 0x3fff,
            ),
            b"VP8L" => {
                let v = le32(head, 21)?;
                p(Format::Webp, (v & 0x3fff) + 1, ((v >> 14) & 0x3fff) + 1)
            }
            b"VP8X" => {
                let w = le32(head, 24)? & 0xff_ffff;
                let h = le32(head, 27)? & 0xff_ffff;
                p(Format::Webp, w + 1, h + 1)
            }
            _ => None,
        };
    }
    if head.starts_with(b"II*\0") || head.starts_with(b"MM\0*") {
        return tiff(head);
    }
    if head.starts_with(&[0xff, 0xd8]) {
        return jpeg(head);
    }
    let t = String::from_utf8_lossy(head.get(..head.len().min(512)).unwrap_or(&[]))
        .to_ascii_lowercase();
    if t.trim_start().starts_with("<svg")
        || (t.trim_start().starts_with("<?xml") && t.contains("<svg"))
    {
        return p(Format::Svg, 0, 0);
    }
    None
}

fn jpeg(b: &[u8]) -> Option<Probe> {
    let mut i = 2;
    while i + 9 < b.len() {
        if b.get(i) != Some(&0xff) {
            return None;
        }
        let m = *b.get(i + 1)?;
        if m == 0xff {
            i += 1;
            continue;
        }
        let len = be16(b, i + 2)? as usize;
        if matches!(m, 0xc0..=0xcf) && !matches!(m, 0xc4 | 0xc8 | 0xcc) {
            return Some(Probe {
                format: Format::Jpeg,
                w: be16(b, i + 7)?,
                h: be16(b, i + 5)?,
            });
        }
        i += 2 + len;
    }
    None
}

fn tiff(b: &[u8]) -> Option<Probe> {
    let le = b.first() == Some(&b'I');
    let r16 = |i| if le { le16(b, i) } else { be16(b, i) };
    let r32 = |i| if le { le32(b, i) } else { be32(b, i) };
    let ifd = r32(4)? as usize;
    let n = r16(ifd)? as usize;
    let (mut w, mut h) = (0, 0);
    for k in 0..n.min(256) {
        let e = ifd + 2 + k * 12;
        let tag = r16(e)?;
        let ty = r16(e + 2)?;
        let v = if ty == 3 { r16(e + 8)? } else { r32(e + 8)? };
        match tag {
            256 => w = v,
            257 => h = v,
            _ => {}
        }
    }
    Some(Probe {
        format: Format::Tiff,
        w,
        h,
    })
}

/// S-RES-04 header-checked limits.
pub fn within_limits(p: &Probe) -> bool {
    p.w <= MAX_DIM && p.h <= MAX_DIM && u64::from(p.w) * u64::from(p.h) <= MAX_PIXELS
}

/// Requested size from HTML attributes or the Obsidian `|300` / `|300x200` suffix.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Req {
    /// Width in logical px.
    pub w: Option<f32>,
    /// Height in logical px.
    pub h: Option<f32>,
}

/// Parses an HTML `width`/`height` value (`300`, `300px`; percentages ignored).
pub fn html_len(v: &str) -> Option<f32> {
    let v = v.trim().trim_end_matches("px").trim();
    let n: f32 = v.parse().ok()?;
    (n.is_finite() && n > 0.0 && n <= 16_384.0).then_some(n)
}

/// Splits `img.png|300` / `img.png|300x200` (Obsidian embed size).
pub fn obsidian_size(target: &str) -> (&str, Req) {
    let Some((name, s)) = target.rsplit_once('|') else {
        return (target, Req::default());
    };
    let (w, h) = match s.split_once('x') {
        Some((w, h)) => (html_len(w), html_len(h)),
        None => (html_len(s), None),
    };
    if w.is_none() {
        return (target, Req::default());
    }
    (name, Req { w, h })
}

/// R-IMG-03 display size in logical px: 1 image px = 1 logical px, requested
/// size honoured, scaled down to `column`, aspect kept, never upscaled automatically.
pub fn display_size(img_w: u32, img_h: u32, req: Req, column: f32) -> (f32, f32) {
    let (iw, ih) = (img_w.max(1) as f32, img_h.max(1) as f32);
    let (mut w, mut h) = match (req.w, req.h) {
        (Some(w), Some(h)) => (w, h),
        (Some(w), None) => (w, w * ih / iw),
        (None, Some(h)) => (h * iw / ih, h),
        (None, None) => (iw, ih),
    };
    if w > column && column > 0.0 {
        h *= column / w;
        w = column;
    }
    (w.max(1.0), h.max(1.0))
}

/// 4:3 estimate at column width used until the probe answers (§4.5.2).
pub fn estimate(column: f32) -> (f32, f32) {
    (column, column * 0.75)
}

/// A minimal LRU over decoded-bitmap byte sizes (P-13). Keys are image ids.
#[derive(Debug, Default)]
pub struct Lru {
    items: Vec<(u64, usize)>,
    bytes: usize,
}

impl Lru {
    /// Inserts or touches `id`; returns ids evicted to stay within `budget`.
    pub fn touch(&mut self, id: u64, size: usize, budget: usize) -> Vec<u64> {
        if let Some(p) = self.items.iter().position(|(k, _)| *k == id) {
            let it = self.items.remove(p);
            self.bytes -= it.1;
        }
        self.items.push((id, size));
        self.bytes += size;
        let mut out = Vec::new();
        while self.bytes > budget && self.items.len() > 1 {
            let (k, s) = self.items.remove(0);
            self.bytes -= s;
            out.push(k);
        }
        out
    }
    /// Removes `id`.
    pub fn remove(&mut self, id: u64) {
        if let Some(p) = self.items.iter().position(|(k, _)| *k == id) {
            let it = self.items.remove(p);
            self.bytes -= it.1;
        }
    }
    /// Total bytes held.
    pub fn bytes(&self) -> usize {
        self.bytes
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Records every file-system call (AC-6).
    #[derive(Default)]
    struct Mock {
        calls: Vec<String>,
        remote_drives: Vec<&'static str>,
        reparse: Vec<(&'static str, &'static str)>,
        files: Vec<(&'static str, u64)>,
    }
    impl Fs for Mock {
        fn drive_kind(&mut self, root: &str) -> DriveKind {
            self.calls.push(format!("drive {root}"));
            if self
                .remote_drives
                .iter()
                .any(|d| root.eq_ignore_ascii_case(d))
            {
                DriveKind::Remote
            } else {
                DriveKind::Local
            }
        }
        fn reparse_target(&mut self, path: &str) -> Option<String> {
            self.calls.push(format!("reparse {path}"));
            self.reparse
                .iter()
                .find(|(p, _)| *p == path)
                .map(|(_, t)| (*t).to_string())
        }
        fn size(&mut self, path: &str) -> Option<u64> {
            self.calls.push(format!("size {path}"));
            self.files
                .iter()
                .find(|(p, _)| p.eq_ignore_ascii_case(path))
                .map(|(_, s)| *s)
        }
    }

    const DOC: &str = "C:\\docs\\notes\\a.md";

    #[test]
    fn r_img_02_path_forms() {
        let f = |s| resolve(s, DOC);
        assert_eq!(
            f("img.png"),
            Ok(Source::File("C:\\docs\\notes\\img.png".into()))
        );
        assert_eq!(
            f("./img.png"),
            Ok(Source::File("C:\\docs\\notes\\img.png".into()))
        );
        assert_eq!(
            f("../assets/x.png"),
            Ok(Source::File("C:\\docs\\assets\\x.png".into()))
        );
        assert_eq!(
            f("..\\assets\\x.png"),
            Ok(Source::File("C:\\docs\\assets\\x.png".into()))
        );
        assert_eq!(
            f("D:/pics/x.png"),
            Ok(Source::File("D:\\pics\\x.png".into()))
        );
        assert_eq!(
            f("file:///D:/my%20pics/x.png"),
            Ok(Source::File("D:\\my pics\\x.png".into()))
        );
        assert_eq!(
            f("file://localhost/D:/x.png"),
            Ok(Source::File("D:\\x.png".into()))
        );
        assert_eq!(
            f("my%20img.png"),
            Ok(Source::File("C:\\docs\\notes\\my img.png".into()))
        );
        assert_eq!(f("/root.png"), Ok(Source::File("C:\\root.png".into())));
        assert_eq!(f("../../../../x.png"), Ok(Source::File("C:\\x.png".into())));
        assert_eq!(f(""), Err(Placeholder::Invalid));
    }

    #[test]
    fn s_res_03_remote_never_fetched() {
        for s in [
            "http://x/a.png",
            "https://img.shields.io/b.svg",
            "ftp://h/a.png",
            "HTTPS://X/Y",
        ] {
            assert_eq!(resolve(s, DOC), Err(Placeholder::Remote), "{s}");
        }
    }

    /// S-TEST-02 image path matrix: every network/device/ADS form is blocked before any FS call.
    #[test]
    fn s_res_02_network_matrix_before_fs() {
        let forms = [
            "\\\\server\\share\\a.png",
            "//server/share/a.png",
            "\\\\?\\UNC\\server\\share\\a.png",
            "\\\\?\\C:\\a.png",
            "\\\\.\\C:\\a.png",
            "\\??\\C:\\a.png",
            "\\\\.\\pipe\\x",
            "file://server/share/a.png",
            "file:////server/share/a.png",
            "\\\\host@SSL\\DavWWWRoot\\a.png",
            "\\\\host@8080\\x\\a.png",
            "\\\\host@SSL@443\\x\\a.png",
            "%5C%5Cserver%5Cshare%5Ca.png",
            "a.png:Zone.Identifier",
            "C:\\x\\a.png:ads",
        ];
        for s in forms {
            let mut m = Mock::default();
            let r = resolve_policy(s, DOC, &mut m);
            assert!(
                matches!(
                    r,
                    Err(Placeholder::Network | Placeholder::Remote | Placeholder::InvalidPath)
                ),
                "{s}: {r:?}"
            );
            assert!(m.calls.is_empty(), "{s}: fs touched {:?}", m.calls);
        }
    }

    /// P-27: device names, ADS and NUL are *invalid path* and never touch the file system.
    #[test]
    fn p27_invalid_path_never_touches_fs() {
        for s in [
            "\\\\?\\C:\\a.png",
            "\\\\.\\C:\\a.png",
            "\\??\\C:\\a.png",
            "\\\\.\\pipe\\x",
            "a.png:Zone.Identifier",
            "C:\\x\\a.png:ads",
            "a%00.png",
            "CON",
            "nul.png",
            "sub/COM1.jpg",
            "LPT9",
        ] {
            let mut m = Mock::default();
            assert_eq!(
                resolve_policy(s, DOC, &mut m),
                Err(Placeholder::InvalidPath),
                "{s}"
            );
            assert!(m.calls.is_empty(), "{s}");
        }
        assert_eq!(Placeholder::InvalidPath.message(), "Invalid image path");
        let mut m = Mock::default();
        assert_eq!(
            resolve_policy("COM0.png", DOC, &mut m),
            Err(Placeholder::Missing)
        );
        assert_eq!(
            resolve_policy("console.png", DOC, &mut m),
            Err(Placeholder::Missing)
        );
    }

    #[test]
    fn s_res_02_mapped_drive_and_reparse() {
        let mut m = Mock {
            remote_drives: vec!["Z:\\"],
            ..Mock::default()
        };
        assert_eq!(
            resolve_policy("Z:/a.png", DOC, &mut m),
            Err(Placeholder::Network)
        );
        assert_eq!(m.calls, vec!["drive Z:\\"]);
        let mut m = Mock {
            reparse: vec![
                ("C:\\docs\\notes\\l.png", "\\\\evil\\s\\x.png"),
                ("C:\\docs\\notes\\j.png", "\\??\\UNC\\evil\\s"),
            ],
            files: vec![("C:\\docs\\notes\\l.png", 10)],
            ..Mock::default()
        };
        assert_eq!(
            resolve_policy("l.png", DOC, &mut m),
            Err(Placeholder::Network)
        );
        assert!(!m.calls.iter().any(|c| c.starts_with("size")));
        assert_eq!(
            resolve_policy("j.png", DOC, &mut m),
            Err(Placeholder::Network)
        );
        let mut m = Mock {
            remote_drives: vec!["Y:\\"],
            reparse: vec![("C:\\docs\\notes\\k.png", "Y:\\x.png")],
            ..Mock::default()
        };
        assert_eq!(
            resolve_policy("k.png", DOC, &mut m),
            Err(Placeholder::Network)
        );
    }

    #[test]
    fn s_res_02_same_share_exception() {
        let doc = "\\\\nas\\team\\docs\\a.md";
        let mut m = Mock {
            files: vec![("\\\\nas\\team\\img\\x.png", 5)],
            ..Mock::default()
        };
        assert_eq!(
            resolve_policy("../img/x.png", doc, &mut m),
            Ok(Source::File("\\\\nas\\team\\img\\x.png".into()))
        );
        assert!(!m.calls.iter().any(|c| c.starts_with("drive")));
        for s in [
            "\\\\nas\\other\\x.png",
            "\\\\evil\\team\\x.png",
            "\\\\nas@SSL\\team\\x.png",
        ] {
            let mut m = Mock::default();
            assert_eq!(
                resolve_policy(s, doc, &mut m),
                Err(Placeholder::Network),
                "{s}"
            );
            assert!(m.calls.is_empty());
        }
        // A WebDAV document never grants the exception.
        let mut m = Mock::default();
        assert_eq!(
            resolve_policy("x.png", "\\\\h@SSL\\d\\a.md", &mut m),
            Err(Placeholder::Network)
        );
    }

    #[test]
    fn s_res_04_limits_and_missing() {
        let mut m = Mock {
            files: vec![
                ("C:\\docs\\notes\\big.png", MAX_FILE + 1),
                ("C:\\docs\\notes\\ok.png", 9),
            ],
            ..Mock::default()
        };
        assert_eq!(
            resolve_policy("big.png", DOC, &mut m),
            Err(Placeholder::TooLarge)
        );
        assert_eq!(
            resolve_policy("nope.png", DOC, &mut m),
            Err(Placeholder::Missing)
        );
        assert!(resolve_policy("ok.png", DOC, &mut m).is_ok());
        assert!(within_limits(&Probe {
            format: Format::Png,
            w: 16_384,
            h: 3_000
        }));
        assert!(!within_limits(&Probe {
            format: Format::Png,
            w: 16_385,
            h: 1
        }));
        assert!(!within_limits(&Probe {
            format: Format::Png,
            w: 10_000,
            h: 10_000
        }));
    }

    #[test]
    fn s_res_01_data_uris() {
        assert_eq!(
            data_uri("data:image/png;base64,iVBORw=="),
            Ok(vec![0x89, b'P', b'N', b'G'])
        );
        assert_eq!(
            data_uri("data:text/html;base64,PHNjcmlwdD4="),
            Err(Placeholder::Invalid)
        );
        assert_eq!(
            data_uri("data:image/svg+xml;base64,PHN2Zz4="),
            Err(Placeholder::Unsupported)
        );
        assert_eq!(data_uri("data:image/png,raw"), Err(Placeholder::Invalid));
        assert_eq!(
            data_uri("data:image/png;base64,***"),
            Err(Placeholder::Invalid)
        );
        let big = format!("data:image/png;base64,{}", "A".repeat(14_000_000));
        assert_eq!(data_uri(&big), Err(Placeholder::TooLarge));
        assert_eq!(base64("aGVsbG8"), Some(b"hello".to_vec()));
    }

    #[test]
    fn r_img_04_probe_headers() {
        let mut png = b"\x89PNG\r\n\x1a\n\0\0\0\rIHDR".to_vec();
        png.extend_from_slice(&800u32.to_be_bytes());
        png.extend_from_slice(&600u32.to_be_bytes());
        assert_eq!(
            probe(&png),
            Some(Probe {
                format: Format::Png,
                w: 800,
                h: 600
            })
        );
        let gif = b"GIF89a\x20\x03\x58\x02";
        assert_eq!(
            probe(gif),
            Some(Probe {
                format: Format::Gif,
                w: 800,
                h: 600
            })
        );
        let mut bmp = vec![0u8; 26];
        bmp[..2].copy_from_slice(b"BM");
        bmp[18..22].copy_from_slice(&10u32.to_le_bytes());
        bmp[22..26].copy_from_slice(&(-20i32).to_le_bytes());
        assert_eq!(
            probe(&bmp),
            Some(Probe {
                format: Format::Bmp,
                w: 10,
                h: 20
            })
        );
        let ico = [0, 0, 1, 0, 1, 0, 0, 32];
        assert_eq!(
            probe(&ico),
            Some(Probe {
                format: Format::Ico,
                w: 256,
                h: 32
            })
        );
        let jpg = [
            0xff, 0xd8, 0xff, 0xe0, 0, 4, 0, 0, 0xff, 0xc0, 0, 11, 8, 0x02, 0x58, 0x03, 0x20, 3, 0,
            0, 0,
        ];
        assert_eq!(
            probe(&jpg),
            Some(Probe {
                format: Format::Jpeg,
                w: 800,
                h: 600
            })
        );
        let mut webp = b"RIFF\0\0\0\0WEBPVP8L\0\0\0\0\x2f".to_vec();
        let v: u32 = 99 | (49 << 14);
        webp.extend_from_slice(&v.to_le_bytes());
        assert_eq!(
            probe(&webp),
            Some(Probe {
                format: Format::Webp,
                w: 100,
                h: 50
            })
        );
        let mut tif = b"II*\0\x08\0\0\0\x02\0".to_vec();
        tif.extend_from_slice(&[0, 1, 3, 0, 1, 0, 0, 0, 7, 0, 0, 0]);
        tif.extend_from_slice(&[1, 1, 4, 0, 1, 0, 0, 0, 9, 0, 0, 0]);
        assert_eq!(
            probe(&tif),
            Some(Probe {
                format: Format::Tiff,
                w: 7,
                h: 9
            })
        );
        assert_eq!(
            probe(b"<svg xmlns='x'>").map(|p| p.format),
            Some(Format::Svg)
        );
        assert_eq!(probe(b"garbage"), None);
        assert_eq!(probe(b"\x89PNG\r\n\x1a\n"), None);
    }

    #[test]
    fn r_img_03_sizing() {
        assert_eq!(
            display_size(400, 300, Req::default(), 700.0),
            (400.0, 300.0)
        );
        assert_eq!(
            display_size(1400, 700, Req::default(), 700.0),
            (700.0, 350.0)
        );
        assert_eq!(
            display_size(
                400,
                200,
                Req {
                    w: Some(300.0),
                    h: None
                },
                700.0
            ),
            (300.0, 150.0)
        );
        assert_eq!(
            display_size(
                100,
                100,
                Req {
                    w: Some(50.0),
                    h: Some(20.0)
                },
                700.0
            ),
            (50.0, 20.0)
        );
        assert_eq!(
            obsidian_size("x.png|300"),
            (
                "x.png",
                Req {
                    w: Some(300.0),
                    h: None
                }
            )
        );
        assert_eq!(
            obsidian_size("x.png|30x20"),
            (
                "x.png",
                Req {
                    w: Some(30.0),
                    h: Some(20.0)
                }
            )
        );
        assert_eq!(obsidian_size("a|b.png").0, "a|b.png");
        assert_eq!(html_len("120px"), Some(120.0));
        assert_eq!(html_len("50%"), None);
        assert_eq!(estimate(800.0), (800.0, 600.0));
    }

    #[test]
    fn p_13_lru_budget() {
        let mut l = Lru::default();
        assert!(l.touch(1, 600_000, LRU_BYTES).is_empty());
        assert!(l.touch(2, 600_000, LRU_BYTES).is_empty());
        l.touch(1, 600_000, LRU_BYTES);
        assert_eq!(l.touch(3, 600_000, LRU_BYTES), vec![2]);
        assert!(l.bytes() <= LRU_BYTES);
        l.remove(1);
        assert_eq!(l.bytes(), 600_000);
    }
}
