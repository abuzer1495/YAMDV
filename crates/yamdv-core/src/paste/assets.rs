//! Asset names, folders and links for pasted images and dropped files (prompt 015 §3.5,
//! S-FILE-05). The platform creates the folder, resolves it with `GetFinalPathNameByHandleW`,
//! checks [`within`] and creates files with `CREATE_NEW`.

use super::text::encode_target;

/// Dropped files above this size ask first (prompt 015 §3.5).
pub const ASK_ABOVE: u64 = 50 * 1024 * 1024;
/// File-name cap in UTF-16 units (S-FILE-05).
pub const MAX_NAME: usize = 120;

/// Local time parts for a file name.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Stamp {
    /// Year.
    pub y: u16,
    /// Month 1–12.
    pub mo: u16,
    /// Day.
    pub d: u16,
    /// Hour.
    pub h: u16,
    /// Minute.
    pub mi: u16,
    /// Second.
    pub s: u16,
}

/// `image-YYYYMMDD-HHMMSS[-n].<ext>`; `n` = 0 for the first try, then 2, 3, … on collisions.
pub fn image_name(t: Stamp, n: u32, ext: &str) -> String {
    let base = format!(
        "image-{:04}{:02}{:02}-{:02}{:02}{:02}",
        t.y, t.mo, t.d, t.h, t.mi, t.s
    );
    if n == 0 {
        format!("{base}.{ext}")
    } else {
        format!("{base}-{n}.{ext}")
    }
}

/// Sanitized file name (S-FILE-05): no `<>:"/\|?*` or controls, no trailing dots/spaces, not a
/// device name, at most [`MAX_NAME`] UTF-16 units (the extension is kept). `None` if nothing
/// usable is left.
pub fn sanitize_name(name: &str) -> Option<String> {
    let mut s: String = name
        .chars()
        .filter(|c| {
            !c.is_control() && !matches!(c, '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*')
        })
        .collect();
    s = s.trim_end_matches(['.', ' ']).trim_start().to_string();
    if s.is_empty() {
        return None;
    }
    if crate::path::is_reserved(&s) {
        s.insert(0, '_');
    }
    if s.encode_utf16().count() > MAX_NAME {
        let (stem, ext) = match s.rfind('.') {
            Some(i) if s.len() - i <= 10 => (s[..i].to_string(), s[i..].to_string()),
            _ => (s.clone(), String::new()),
        };
        let room = MAX_NAME.saturating_sub(ext.encode_utf16().count());
        let mut out = String::new();
        let mut units = 0;
        for c in stem.chars() {
            units += c.len_utf16();
            if units > room {
                break;
            }
            out.push(c);
        }
        s = format!("{}{ext}", out.trim_end_matches(['.', ' ']));
    }
    Some(s)
}

/// The relative asset folder `<assets_dir>/<stem>` (forward slashes), or `None` when the
/// setting is absolute, has a drive, a stream, or `..` (S-FILE-05).
pub fn asset_dir(assets_dir: &str, doc_stem: &str) -> Option<String> {
    let a = assets_dir.trim().replace('\\', "/");
    let a = a.trim_matches('/');
    if a.is_empty() || a.contains(':') || assets_dir.trim().starts_with(['/', '\\']) {
        return None;
    }
    let mut parts = Vec::new();
    for p in a.split('/') {
        if p.is_empty() || p == "." {
            continue;
        }
        if p == ".." {
            return None;
        }
        parts.push(sanitize_name(p)?);
    }
    parts.push(sanitize_name(doc_stem)?);
    Some(parts.join("/"))
}

/// Markdown image for a pasted image: `![](assets/<stem>/<name>)`.
pub fn image_markdown(rel_dir: &str, name: &str) -> String {
    format!("![]({})", encode_target(&format!("{rel_dir}/{name}")))
}

/// Markdown link for a dropped file: `[name](assets/<stem>/<name>)` (spaces as `%20`).
pub fn file_markdown(rel_dir: &str, name: &str) -> String {
    let text: String = name
        .chars()
        .flat_map(|c| {
            let esc = matches!(c, '[' | ']' | '\\' | '*' | '_' | '`');
            esc.then_some('\\').into_iter().chain(std::iter::once(c))
        })
        .collect();
    format!("[{text}]({})", encode_target(&format!("{rel_dir}/{name}")))
}

/// Normalized comparison form of a Windows path: `\\?\` and `\\?\UNC\` removed, `/` → `\`,
/// no trailing separator, case-folded.
fn norm(p: &str) -> String {
    let p = p.replace('/', "\\");
    let p = if let Some(r) = p.strip_prefix(r"\\?\UNC\") {
        format!(r"\\{r}")
    } else {
        p.strip_prefix(r"\\?\").map_or(p.clone(), str::to_string)
    };
    p.trim_end_matches('\\').to_lowercase()
}

/// S-FILE-05: `final_path` (from `GetFinalPathNameByHandleW`) is `root` or inside it.
pub fn within(root: &str, final_path: &str) -> bool {
    let (r, f) = (norm(root), norm(final_path));
    if r.is_empty() || f.split('\\').any(|c| c == "..") {
        return false;
    }
    f == r
        || f.strip_prefix(&r)
            .is_some_and(|rest| rest.starts_with('\\'))
}
