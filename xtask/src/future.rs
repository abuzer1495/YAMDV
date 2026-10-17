//! `cargo xtask future`: extracts the `FUTURE.md` block from prompt 005 into `docs/FUTURE.md`
//! and marks the deferred IDs in `docs/TRACEABILITY.md` (prompt 005 §2.2, §2.4). Idempotent.

use std::path::Path;

/// Prompt that carries the owner's v1 scope decision.
pub const PROMPT_005: &str = "docs/prompts/005_V1A_SHELL_DAILY_USE.md";

/// Returns the body of the first ````` ````markdown ````` fence after the `## 5. FUTURE.md` heading.
pub fn extract(prompt: &str) -> Option<String> {
    let t = prompt.replace("\r\n", "\n");
    let start = t.find("\n## 5. FUTURE.md")?;
    let rest = t.get(start..)?;
    let open = rest.find("````markdown\n")? + "````markdown\n".len();
    let body = rest.get(open..)?;
    let close = body.find("\n````")?;
    Some(format!("{}\n", body.get(..close)?))
}

/// Expands one "Spec IDs" token (`R-SET-01…04`, `R-REN-10 (tooltip)`, `F.6`) to IDs.
pub fn expand(token: &str) -> Vec<String> {
    let tok = token.split(" (").next().unwrap_or("").trim();
    if let Some((a, b)) = tok.split_once('…') {
        let Some((prefix, from)) = a.rsplit_once('-') else {
            return vec![tok.to_string()];
        };
        let (Ok(f), Ok(l)) = (from.parse::<u32>(), b.parse::<u32>()) else {
            return vec![tok.to_string()];
        };
        let w = from.len();
        return (f..=l).map(|n| format!("{prefix}-{n:0w$}")).collect();
    }
    vec![tok.to_string()]
}

/// `(FUT id, spec IDs)` rows of the FUTURE.md table.
pub fn rows(future: &str) -> Vec<(String, Vec<String>)> {
    let mut out = Vec::new();
    for line in future.lines() {
        let cells: Vec<&str> = line.split('|').map(str::trim).collect();
        let (Some(id), Some(ids)) = (cells.get(1), cells.get(4)) else {
            continue;
        };
        if !id.starts_with("FUT-") {
            continue;
        }
        let v = ids.split(", ").flat_map(expand).collect();
        out.push(((*id).to_string(), v));
    }
    out
}

/// Applies the deferrals to the traceability table text; returns the new text and unmatched refs.
pub fn apply(trace: &str, rows: &[(String, Vec<String>)]) -> (String, Vec<String>) {
    let mut unmatched = Vec::new();
    let mut lines: Vec<String> = trace.lines().map(str::to_string).collect();
    for (fut, ids) in rows {
        for id in ids {
            let key = format!("| {id} |");
            let Some(line) = lines.iter_mut().find(|l| l.starts_with(&key)) else {
                unmatched.push(format!("{fut}: {id}"));
                continue;
            };
            if line.contains(fut.as_str()) {
                continue;
            }
            let Some(body) = line.strip_suffix(" |") else {
                continue;
            };
            let Some((head, status)) = body.rsplit_once(" | ") else {
                continue;
            };
            let new_status = if let Some(inner) = status
                .strip_prefix("deferred (")
                .and_then(|s| s.strip_suffix(')'))
            {
                format!("deferred ({inner}, {fut})")
            } else if status.starts_with("planned") {
                format!("deferred ({fut})")
            } else if status.contains("part deferred: ") {
                format!("{status}, {fut}")
            } else {
                format!("{status}; part deferred: {fut}")
            };
            *line = format!("{head} | {new_status} |");
        }
    }
    let mut s = lines.join("\n");
    s.push('\n');
    (s, unmatched)
}

/// Runs the extraction and the deferral pass.
pub fn run(root: &Path) -> Result<(), String> {
    let prompt = std::fs::read_to_string(root.join(PROMPT_005)).map_err(|e| e.to_string())?;
    let fut = extract(&prompt).ok_or("FUTURE.md block not found in prompt 005")?;
    std::fs::write(root.join("docs/FUTURE.md"), &fut).map_err(|e| e.to_string())?;
    println!("wrote docs/FUTURE.md");
    let tp = root.join("docs/TRACEABILITY.md");
    let trace = std::fs::read_to_string(&tp).map_err(|e| e.to_string())?;
    let (out, unmatched) = apply(&trace, &rows(&fut));
    std::fs::write(&tp, out).map_err(|e| e.to_string())?;
    println!("updated docs/TRACEABILITY.md");
    for u in unmatched {
        println!("  not a traceability row (section reference): {u}");
    }
    Ok(())
}

/// `spec-check` part: `docs/FUTURE.md` equals the block extracted from prompt 005.
pub fn check(root: &Path) -> Result<bool, String> {
    let Ok(prompt) = std::fs::read_to_string(root.join(PROMPT_005)) else {
        return Ok(true);
    };
    let fut = extract(&prompt).ok_or("FUTURE.md block not found in prompt 005")?;
    let cur = std::fs::read_to_string(root.join("docs/FUTURE.md")).unwrap_or_default();
    Ok(cur == fut)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ranges_expand() {
        assert_eq!(expand("R-SET-01…04").len(), 4);
        assert_eq!(expand("R-SET-01…04")[3], "R-SET-04");
        assert_eq!(expand("R-REN-10 (tooltip)"), vec!["R-REN-10"]);
    }

    #[test]
    fn deferral_rules() {
        let t = "| A-1 | 004 | planned |\n| A-2 | 003 | implemented (003) |\n";
        let r = vec![
            (
                "FUT-01".to_string(),
                vec!["A-1".into(), "A-2".into(), "F.6".into()],
            ),
            ("FUT-02".to_string(), vec!["A-1".into(), "A-2".into()]),
        ];
        let (o, un) = apply(t, &r);
        assert!(o.contains("| A-1 | 004 | deferred (FUT-01, FUT-02) |"));
        assert!(o.contains("| A-2 | 003 | implemented (003); part deferred: FUT-01, FUT-02 |"));
        assert_eq!(un, vec!["FUT-01: F.6"]);
        let (o2, _) = apply(&o, &r);
        assert_eq!(o, o2, "idempotent");
    }

    #[test]
    fn extracts_block() {
        let p = "x\n## 5. FUTURE.md (y)\n````markdown\n# T\n| FUT-01 | a | b | R-X-01 |\n````\n";
        let f = extract(p).unwrap_or_default();
        assert_eq!(f, "# T\n| FUT-01 | a | b | R-X-01 |\n");
        assert_eq!(rows(&f)[0].1, vec!["R-X-01"]);
    }
}
