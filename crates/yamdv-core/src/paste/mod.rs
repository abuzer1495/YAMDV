//! Paste and drop helpers (prompt 015 §3.5): clipboard HTML (`CF_HTML`) → Markdown through the
//! bounded tokenizer and allowlist of [`crate::html`], asset names and links for pasted images
//! and dropped files, and the sandbox rule for asset folders (S-FILE-05). Pure.

pub mod assets;
pub mod dib;
pub mod html2md;
mod text;

/// HTML above this size is pasted as plain text instead (prompt 015 §3.5).
pub const MAX_HTML: usize = 2 * 1024 * 1024;

/// The fragment of a `CF_HTML` clipboard payload (between `StartFragment` / `EndFragment`,
/// falling back to `<!--StartFragment-->` markers or the whole HTML part).
pub fn cf_html_fragment(raw: &str) -> &str {
    let num = |key: &str| -> Option<usize> {
        let i = raw.find(key)? + key.len();
        let digits: String = raw
            .get(i..)?
            .chars()
            .take_while(char::is_ascii_digit)
            .collect();
        digits.parse().ok()
    };
    if let (Some(a), Some(b)) = (num("StartFragment:"), num("EndFragment:"))
        && a <= b
        && let Some(f) = raw.get(a..b)
    {
        return f;
    }
    if let (Some(a), Some(b)) = (
        raw.find("<!--StartFragment-->"),
        raw.find("<!--EndFragment-->"),
    ) && let Some(f) = raw.get(a + 20..b)
    {
        return f;
    }
    match num("StartHTML:") {
        Some(a) => raw.get(a..).unwrap_or(raw),
        None => raw,
    }
}

#[cfg(test)]
mod tests;
