//! Find & replace engine (R-EDIT-10, P-46): plain search with the find bar's folding (Turkish
//! `i/İ/ı/I` equivalence), whole word, optional regex (`regex-lite`, ADR-0019) with `$1` /
//! `${name}` templates and a time guard. "Replace all" is ONE undo step. Pure.

use std::ops::Range;
use std::time::{Duration, Instant};

use crate::buffer::Editor;

/// Regex time guard (prompt 015 §3.2): a search stops after this long.
pub const REGEX_GUARD: Duration = Duration::from_millis(100);
/// Compiled-pattern cap (ADR-0019).
const SIZE_LIMIT: usize = 1 << 20;
/// Match cap per search (keeps highlights and replace-all bounded).
pub const MAX_MATCHES: usize = 100_000;

/// What to look for.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Query {
    /// Needle (or pattern when `regex`).
    pub needle: String,
    /// Match case.
    pub case: bool,
    /// Whole words only.
    pub whole_word: bool,
    /// `[.*]`: the needle is a regex.
    pub regex: bool,
}

/// Why a search produced no result list.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FindError {
    /// The pattern does not compile (message from the engine, first line).
    BadRegex(String),
    /// The time guard stopped the search.
    TooSlow,
}

/// Folds one char for plain matching (same rule as the read-mode find bar).
pub fn fold(c: char, case: bool) -> char {
    if case {
        return c;
    }
    match c {
        'I' | 'İ' | 'ı' | 'i' => 'i',
        _ => c.to_lowercase().next().unwrap_or(c),
    }
}

fn is_word(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

fn word_ok(hay: &str, r: &Range<usize>) -> bool {
    let before = hay.get(..r.start).and_then(|s| s.chars().next_back());
    let after = hay.get(r.end..).and_then(|s| s.chars().next());
    !before.is_some_and(is_word) && !after.is_some_and(is_word)
}

fn plain(hay: &str, q: &Query) -> Vec<Range<usize>> {
    let n: Vec<char> = q.needle.chars().map(|c| fold(c, q.case)).collect();
    if n.is_empty() {
        return Vec::new();
    }
    let h: Vec<(usize, char)> = hay
        .char_indices()
        .map(|(i, c)| (i, fold(c, q.case)))
        .collect();
    let mut out = Vec::new();
    let mut i = 0;
    while i + n.len() <= h.len() && out.len() < MAX_MATCHES {
        let hit = n
            .iter()
            .enumerate()
            .all(|(k, c)| h.get(i + k).is_some_and(|x| x.1 == *c));
        if hit {
            let start = h.get(i).map_or(0, |x| x.0);
            let end = h.get(i + n.len()).map_or(hay.len(), |x| x.0);
            let r = start..end;
            if !q.whole_word || word_ok(hay, &r) {
                out.push(r);
                i += n.len();
                continue;
            }
        }
        i += 1;
    }
    out
}

/// Compiles the regex of `q` (whole word wraps it in `\b(?:…)\b`).
pub fn compile(q: &Query) -> Result<regex_lite::Regex, FindError> {
    let pat = if q.whole_word {
        format!(r"\b(?:{})\b", q.needle)
    } else {
        q.needle.clone()
    };
    regex_lite::RegexBuilder::new(&pat)
        .case_insensitive(!q.case)
        .size_limit(SIZE_LIMIT)
        .build()
        .map_err(|e| FindError::BadRegex(e.to_string().lines().next().unwrap_or("").to_string()))
}

/// Every non-overlapping match of `q` in `hay`; regex searches stop at `guard`.
pub fn find_all(hay: &str, q: &Query, guard: Duration) -> Result<Vec<Range<usize>>, FindError> {
    if q.needle.is_empty() {
        return Ok(Vec::new());
    }
    if !q.regex {
        return Ok(plain(hay, q));
    }
    let re = compile(q)?;
    let t0 = Instant::now();
    let mut out = Vec::new();
    for m in re.find_iter(hay) {
        if t0.elapsed() > guard {
            return Err(FindError::TooSlow);
        }
        if m.start() == m.end() {
            continue; // empty matches are not highlighted or replaced
        }
        out.push(m.range());
        if out.len() >= MAX_MATCHES {
            break;
        }
    }
    if t0.elapsed() > guard {
        return Err(FindError::TooSlow);
    }
    Ok(out)
}

/// Replacement text for match `r`: literal for plain search, `$1` / `${name}` for regex.
pub fn expand(hay: &str, r: &Range<usize>, q: &Query, with: &str) -> String {
    if !q.regex {
        return with.to_string();
    }
    let Ok(re) = compile(q) else {
        return with.to_string();
    };
    let mut out = String::new();
    match re.captures_at(hay, r.start) {
        Some(c) if c.get(0).is_some_and(|m| m.range() == *r) => c.expand(with, &mut out),
        _ => out.push_str(with),
    }
    out
}

/// Replace all: ONE undo step. Returns the number of replacements.
pub fn replace_all(ed: &mut Editor, q: &Query, with: &str) -> Result<usize, FindError> {
    let hay = ed.text();
    let ms = find_all(&hay, q, REGEX_GUARD)?;
    if ms.is_empty() {
        return Ok(0);
    }
    let edits: Vec<(Range<usize>, String)> = if q.regex {
        let re = compile(q)?;
        ms.iter()
            .map(|r| {
                let mut out = String::new();
                match re.captures_at(&hay, r.start) {
                    Some(c) => c.expand(with, &mut out),
                    None => out.push_str(with),
                }
                (r.clone(), out)
            })
            .collect()
    } else {
        ms.iter().map(|r| (r.clone(), with.to_string())).collect()
    };
    let caret = ed.caret().min(ed.len());
    let shift: isize = edits
        .iter()
        .filter(|(r, _)| r.end <= caret)
        .map(|(r, t)| t.len().cast_signed() - (r.end - r.start).cast_signed())
        .sum();
    let c = caret
        .cast_signed()
        .saturating_add(shift)
        .max(0)
        .cast_unsigned();
    ed.replace_many(&edits, (c, c));
    Ok(edits.len())
}

/// [Replace]: replaces the selection when it is a match, then selects the next match.
/// Returns whether a replacement happened.
pub fn replace_next(ed: &mut Editor, q: &Query, with: &str) -> Result<bool, FindError> {
    let hay = ed.text();
    let ms = find_all(&hay, q, REGEX_GUARD)?;
    let sel = ed.selection();
    let mut done = false;
    let mut from = sel.end;
    if let Some(r) = ms.iter().find(|r| **r == sel) {
        let t = expand(&hay, r, q, with);
        let end = r.start + t.len();
        ed.replace(r.clone(), &t, end);
        done = true;
        from = end;
    }
    let hay = ed.text();
    let ms = find_all(&hay, q, REGEX_GUARD)?;
    if let Some(r) = ms.iter().find(|r| r.start >= from).or_else(|| ms.first()) {
        ed.set_selection(r.start, r.end);
    }
    Ok(done)
}
