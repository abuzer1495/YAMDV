//! GitHub-style heading slugs (R-MD-10).

use std::collections::HashMap;

/// GitHub slug: lower-case, keep letters/digits/`-`/`_` (Unicode-aware), spaces → `-`, drop the rest.
pub fn github_slug(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for c in text.trim().chars() {
        if c.is_alphanumeric() || c == '_' || c == '-' {
            out.extend(c.to_lowercase());
        } else if c == ' ' {
            out.push('-');
        }
    }
    out
}

/// De-duplicates slugs like GitHub (`a`, `a-1`, `a-2`, …).
#[derive(Debug, Default)]
pub struct Slugger {
    seen: HashMap<String, u32>,
}

impl Slugger {
    /// Returns the unique slug for `base`.
    pub fn unique(&mut self, base: &str) -> String {
        let mut candidate = base.to_string();
        if let Some(n) = self.seen.get(base).copied() {
            let mut k = n;
            loop {
                candidate = format!("{base}-{k}");
                k += 1;
                if !self.seen.contains_key(&candidate) {
                    break;
                }
            }
            self.seen.insert(base.to_string(), k);
        } else {
            self.seen.insert(base.to_string(), 1);
        }
        self.seen.entry(candidate.clone()).or_insert(1);
        candidate
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn r_md_10_slugs() {
        assert_eq!(github_slug("Hello, World!"), "hello-world");
        assert_eq!(github_slug("Çalışma Notları"), "çalışma-notları");
        assert_eq!(github_slug("a_b - c"), "a_b---c");
        assert_eq!(github_slug("日本語 テスト"), "日本語-テスト");
        let mut s = Slugger::default();
        assert_eq!(s.unique("a"), "a");
        assert_eq!(s.unique("a"), "a-1");
        assert_eq!(s.unique("a-1"), "a-1-1");
        assert_eq!(s.unique("a"), "a-2");
    }
}
