//! `cargo xtask spec-check` / `spec-patch` / PROCESS.md generation (Phase 002 F-01, F-03, §3).
//!
//! The appendices are extracted from the byte-exact prompt copy `docs/prompts/001_…md`, normalized,
//! verified against the reviewer's SHA-256 values, patched with `docs/spec-patches/*.txt` and
//! compared with `docs/*.md`.

use std::path::Path;

use crate::sha256;

/// Prompt 001 copy (byte-exact, F-02).
pub const PROMPT_001: &str = "docs/prompts/001_FOUNDATION_AND_SPIKE.md";

/// Appendix letter, target doc, expected SHA-256 of the normalized original.
pub const APPENDICES: [(char, &str, &str); 8] = [
    (
        'A',
        "docs/SPEC.md",
        "6d5ab534cff27b29bce64af68ae2d82b513f504190f8ba1db7644f445d3aec86",
    ),
    (
        'B',
        "docs/UI_SPEC.md",
        "26dfa16e1cc7ab4560f932b5b4d9236bde80e2091b31e7706bd190c2d0d5c2a3",
    ),
    (
        'C',
        "docs/SECURITY.md",
        "60fcc8d845dc93cfec157825cf5398957397517bf74d12938bb00715708c5711",
    ),
    (
        'D',
        "docs/PERFORMANCE.md",
        "57bd7bfb50faa18638a2dac4a2dd18d9a059a69e73804460fa7e6f0972a7a0f7",
    ),
    (
        'E',
        "docs/ARCHITECTURE.md",
        "12b8443a899a99edc1da6f12ba7206b588c077ccf41b34401ef0536a8d3fbd18",
    ),
    (
        'F',
        "docs/SHORTCUTS.md",
        "49abff061d0aceca9b6322a50aa7c111fa8ee204f6a1ddfb2303ee7033ecb012",
    ),
    (
        'G',
        "docs/ROADMAP.md",
        "9a0778360d59064318c5f7a630d43c2e8eb2fa146db6fb3e96ca8e61cc004d14",
    ),
    (
        'H',
        "docs/TESTING.md",
        "1a2c529a77513dda8d65dd985a3315572f5c895bace97034e2c4636f03b6ad23",
    ),
];

/// Normalization of F-03: CRLF→LF, strip trailing whitespace, drop leading/trailing blank lines
/// and trailing `---` lines, end with exactly one `\n`.
pub fn normalize(text: &str) -> String {
    let t = text.replace("\r\n", "\n");
    let mut lines: Vec<&str> = t.split('\n').map(str::trim_end).collect();
    while lines.first().is_some_and(|l| l.is_empty()) {
        lines.remove(0);
    }
    while lines.last().is_some_and(|l| l.is_empty() || *l == "---") {
        lines.pop();
    }
    let mut s = lines.join("\n");
    s.push('\n');
    s
}

/// Extracts appendix `letter` from the prompt text (normalized).
pub fn extract_appendix(prompt: &str, letter: char) -> Option<String> {
    let t = prompt.replace("\r\n", "\n");
    let lines: Vec<&str> = t.split('\n').collect();
    let head = format!("# APPENDIX {letter} ");
    let start = lines.iter().position(|l| l.starts_with(&head))?;
    let end = lines
        .iter()
        .enumerate()
        .skip(start + 1)
        .find(|(_, l)| l.starts_with("# APPENDIX") || l.starts_with("**END OF PROMPT 001.**"))
        .map_or(lines.len(), |(i, _)| i);
    Some(normalize(&lines.get(start..end)?.join("\n")))
}

/// Extracts the section starting at the line beginning with `start` up to (excluding) the first
/// following line beginning with any of `ends`.
pub fn extract_section(prompt: &str, start: &str, ends: &[&str]) -> Option<String> {
    let t = prompt.replace("\r\n", "\n");
    let lines: Vec<&str> = t.split('\n').collect();
    let s = lines.iter().position(|l| l.starts_with(start))?;
    let e = lines
        .iter()
        .enumerate()
        .skip(s + 1)
        .find(|(_, l)| ends.iter().any(|x| l.starts_with(x)))
        .map_or(lines.len(), |(i, _)| i);
    Some(normalize(&lines.get(s..e)?.join("\n")))
}

/// One spec patch.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Patch {
    /// Patch id, e.g. `P-01`.
    pub id: String,
    /// Target doc path.
    pub doc: String,
    /// `REPLACE` (old→new) or `INSERT_AFTER` (anchor line + new lines).
    pub kind: PatchKind,
    /// Old text or anchor.
    pub old: String,
    /// New text.
    pub new: String,
}

/// Patch kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PatchKind {
    /// Replace a unique substring.
    Replace,
    /// Insert lines after the unique line containing the anchor.
    InsertAfter,
}

/// Parses the patch file format:
/// `=== <ID> <doc> <REPLACE|INSERT_AFTER>`, then `--- old` (or `--- anchor`) and `--- new` blocks.
pub fn parse_patches(text: &str) -> Result<Vec<Patch>, String> {
    let mut out = Vec::new();
    let mut cur: Option<(String, String, PatchKind)> = None;
    let mut part = "";
    let (mut old, mut new) = (Vec::<&str>::new(), Vec::<&str>::new());
    let t = text.replace("\r\n", "\n");
    let flush = |cur: &mut Option<(String, String, PatchKind)>,
                 old: &mut Vec<&str>,
                 new: &mut Vec<&str>,
                 out: &mut Vec<Patch>| {
        if let Some((id, doc, kind)) = cur.take() {
            out.push(Patch {
                id,
                doc,
                kind,
                old: old.join("\n"),
                new: new.join("\n"),
            });
        }
        old.clear();
        new.clear();
    };
    for line in t.split('\n') {
        if line == "=== end" {
            break;
        }
        if let Some(h) = line.strip_prefix("=== ") {
            flush(&mut cur, &mut old, &mut new, &mut out);
            let f: Vec<&str> = h.split_whitespace().collect();
            let [id, doc, kind] = f.as_slice() else {
                return Err(format!("bad patch header: {line}"));
            };
            let kind = match *kind {
                "REPLACE" => PatchKind::Replace,
                "INSERT_AFTER" => PatchKind::InsertAfter,
                k => return Err(format!("bad patch kind {k}")),
            };
            cur = Some(((*id).to_string(), (*doc).to_string(), kind));
            part = "";
        } else if line == "--- old" || line == "--- anchor" {
            part = "old";
        } else if line == "--- new" {
            part = "new";
        } else if line == "=== end" {
            break;
        } else if cur.is_some() {
            match part {
                "old" => old.push(line),
                "new" => new.push(line),
                _ => {}
            }
        }
    }
    flush(&mut cur, &mut old, &mut new, &mut out);
    // Trailing empty line before the next header is a separator, not content.
    for p in &mut out {
        while p.old.ends_with('\n') {
            p.old.pop();
        }
        while p.new.ends_with('\n') {
            p.new.pop();
        }
    }
    Ok(out)
}

/// Applies `p` to `text`; the old text / anchor must occur exactly once.
pub fn apply(text: &str, p: &Patch) -> Result<String, String> {
    match p.kind {
        PatchKind::Replace => {
            let n = text.matches(p.old.as_str()).count();
            if n != 1 {
                return Err(format!("{}: OLD text matches {n} times in {}", p.id, p.doc));
            }
            Ok(text.replacen(p.old.as_str(), &p.new, 1))
        }
        PatchKind::InsertAfter => {
            let lines: Vec<&str> = text.split('\n').collect();
            let hits: Vec<usize> = lines
                .iter()
                .enumerate()
                .filter(|(_, l)| l.contains(p.old.as_str()))
                .map(|(i, _)| i)
                .collect();
            let [i] = hits.as_slice() else {
                return Err(format!(
                    "{}: anchor matches {} lines in {}",
                    p.id,
                    hits.len(),
                    p.doc
                ));
            };
            let mut out: Vec<&str> = Vec::with_capacity(lines.len() + 4);
            out.extend(lines.iter().take(i + 1));
            out.extend(p.new.split('\n'));
            out.extend(lines.iter().skip(i + 1));
            Ok(out.join("\n"))
        }
    }
}

fn read(root: &Path, rel: &str) -> Result<String, String> {
    std::fs::read_to_string(root.join(rel)).map_err(|e| format!("{rel}: {e}"))
}

fn all_patches(root: &Path) -> Result<Vec<Patch>, String> {
    let dir = root.join("docs/spec-patches");
    let mut files: Vec<_> = std::fs::read_dir(&dir)
        .map_err(|e| format!("{}: {e}", dir.display()))?
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|x| x == "txt"))
        .collect();
    files.sort();
    let mut v = Vec::new();
    for f in files {
        let t = std::fs::read_to_string(&f).map_err(|e| format!("{}: {e}", f.display()))?;
        v.extend(parse_patches(&t)?);
    }
    Ok(v)
}

/// Expected doc contents: original appendix + all patches for that doc (normalized).
fn expected(
    root: &Path,
    prompt: &str,
    patches: &[Patch],
) -> Result<Vec<(char, String, String)>, String> {
    let _ = root;
    let mut out = Vec::new();
    for (letter, doc, _) in APPENDICES {
        let mut t =
            extract_appendix(prompt, letter).ok_or(format!("appendix {letter} not found"))?;
        for p in patches.iter().filter(|p| p.doc == doc) {
            t = apply(&t, p)?;
        }
        out.push((letter, doc.to_string(), normalize(&t)));
    }
    Ok(out)
}

/// PROCESS.md generated from prompt 001 §2, §3, §7 plus the committed amendments.
pub fn process_doc(root: &Path, prompt: &str) -> Result<String, String> {
    let s2 = extract_section(prompt, "## 2. Hard rules", &["## 3. "]).ok_or("§2 not found")?;
    let s3 = extract_section(prompt, "## 3. Development methodology", &["## 4. "])
        .ok_or("§3 not found")?;
    let s7 = extract_section(prompt, "## 7. Report template", &["# APPENDIX A "])
        .ok_or("§7 not found")?;
    let amend = read(root, "docs/spec-patches/PROCESS_AMENDMENTS.md")?;
    Ok(format!(
        "# YAMDV — Process (hard rules, methodology, report template)\n\n\
         > Sections 2, 3 and 7 below are copied programmatically from `{PROMPT_001}` by\n\
         > `cargo xtask spec-patch`; `cargo xtask spec-check` verifies them. Do not edit them by hand.\n\n\
         {s2}\n{s3}\n{s7}\n{}",
        normalize(&amend)
    ))
}

/// `spec-check`: prints the verification table, fails on any mismatch.
pub fn check(root: &Path) -> Result<(), String> {
    let prompt = read(root, PROMPT_001)?;
    let patches = all_patches(root)?;
    let mut ok = true;
    println!(
        "| Appendix | doc | extracted hash | phase-001 doc | current doc (original + patches) |"
    );
    println!("|---|---|---|---|---|");
    for ((letter, doc, want), (_, _, exp)) in
        APPENDICES.iter().zip(expected(root, &prompt, &patches)?)
    {
        let orig = extract_appendix(&prompt, *letter).unwrap_or_default();
        let h = sha256::hex(orig.as_bytes());
        let hash_ok = h == *want;
        let tagged = std::process::Command::new("git")
            .args(["show", &format!("phase-001:{doc}")])
            .current_dir(root)
            .output()
            .ok()
            .filter(|o| o.status.success())
            .map(|o| normalize(&String::from_utf8_lossy(&o.stdout)) == orig);
        let cur = read(root, doc)
            .map(|t| normalize(&t) == exp)
            .unwrap_or(false);
        ok &= hash_ok && cur && tagged != Some(false);
        println!(
            "| {letter} | {doc} | {} | {} | {} |",
            if hash_ok { "verified" } else { "MISMATCH" },
            match tagged {
                Some(true) => "match",
                Some(false) => "MISMATCH",
                None => "n/a (tag not available)",
            },
            if cur { "match" } else { "MISMATCH" }
        );
    }
    let proc_ok = read(root, "docs/PROCESS.md")
        .map(|t| t == process_doc(root, &prompt).unwrap_or_default())
        .unwrap_or(false);
    println!(
        "PROCESS.md vs prompt 001 §2/§3/§7 + amendments: {}",
        if proc_ok { "match" } else { "MISMATCH" }
    );
    println!(
        "patches: {}",
        patches
            .iter()
            .map(|p| p.id.as_str())
            .collect::<Vec<_>>()
            .join(", ")
    );
    if ok && proc_ok {
        Ok(())
    } else {
        Err("spec-check failed".into())
    }
}

/// `spec-patch`: rewrites `docs/*.md` = original + patches and regenerates `docs/PROCESS.md`.
pub fn patch(root: &Path) -> Result<(), String> {
    let prompt = read(root, PROMPT_001)?;
    let patches = all_patches(root)?;
    for (_, doc, text) in expected(root, &prompt, &patches)? {
        std::fs::write(root.join(&doc), text).map_err(|e| format!("{doc}: {e}"))?;
        println!("wrote {doc}");
    }
    std::fs::write(root.join("docs/PROCESS.md"), process_doc(root, &prompt)?)
        .map_err(|e| e.to_string())?;
    println!("wrote docs/PROCESS.md");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_rules() {
        assert_eq!(normalize("\r\n\nA  \r\nB\n\n---\n\n"), "A\nB\n");
    }

    #[test]
    fn extract_and_patch() {
        let p = "x\n# APPENDIX A — t\nline one\nanchor here\n---\n# APPENDIX B — u\nb\n**END OF PROMPT 001.**\n";
        let a = extract_appendix(p, 'A').unwrap();
        assert_eq!(a, "# APPENDIX A — t\nline one\nanchor here\n");
        assert_eq!(extract_appendix(p, 'B').unwrap(), "# APPENDIX B — u\nb\n");
        let pats = parse_patches("=== P-1 d REPLACE\n--- old\nline one\n--- new\nline 1\n\n=== P-2 d INSERT_AFTER\n--- anchor\nanchor here\n--- new\nadded\n").unwrap();
        assert_eq!(pats.len(), 2);
        let t = apply(&a, &pats[0]).unwrap();
        let t = apply(&t, &pats[1]).unwrap();
        assert_eq!(t, "# APPENDIX A — t\nline 1\nanchor here\nadded\n");
        assert!(apply("none", &pats[0]).is_err());
    }
}
