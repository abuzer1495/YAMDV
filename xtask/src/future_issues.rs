//! `cargo xtask future-issues` (prompt 008 §5.2): one GitHub issue per `docs/FUTURE.md` row in
//! THIS private repository, label `future`, milestone `post-v1`. Idempotent: existing titles are
//! skipped, the label and milestone are created only when missing. Uses the `gh` CLI and its
//! existing login; refuses to run unless the repository is private (H2: nothing is published).

use std::path::Path;
use std::process::Command;

/// Label and milestone names (owner decision, prompt 005 §2.2).
pub const LABEL: &str = "future";
/// Milestone title.
pub const MILESTONE: &str = "post-v1";

/// `(title, body)` of every FUT row: title `FUT-xx: <feature>`, body = "v1 instead" + spec IDs.
pub fn issues(future: &str) -> Vec<(String, String)> {
    let mut out = Vec::new();
    for line in future.lines() {
        let cells: Vec<&str> = line.split('|').map(str::trim).collect();
        let (Some(id), Some(feature), Some(instead), Some(ids)) =
            (cells.get(1), cells.get(2), cells.get(3), cells.get(4))
        else {
            continue;
        };
        if !id.starts_with("FUT-") {
            continue;
        }
        let title = format!("{id}: {feature}");
        let body = format!(
            "**v1 instead:** {instead}\n\n**Spec IDs:** {ids}\n\nDeferred by the owner's v1 scope decision (2026-10-17); see `docs/FUTURE.md`."
        );
        out.push((title, body));
    }
    out
}

fn gh(args: &[&str]) -> Result<String, String> {
    let o = Command::new("gh")
        .args(args)
        .output()
        .map_err(|e| format!("gh: {e}"))?;
    if !o.status.success() {
        return Err(format!(
            "gh {}: {}",
            args.first().copied().unwrap_or(""),
            String::from_utf8_lossy(&o.stderr).trim()
        ));
    }
    Ok(String::from_utf8_lossy(&o.stdout).into_owned())
}

/// Runs the issue creation once.
pub fn run(root: &Path) -> Result<(), String> {
    let future = std::fs::read_to_string(root.join("docs/FUTURE.md")).map_err(|e| e.to_string())?;
    let want = issues(&future);
    if want.is_empty() {
        return Err("no FUT rows in docs/FUTURE.md".into());
    }
    let repo = gh(&[
        "repo",
        "view",
        "--json",
        "nameWithOwner",
        "-q",
        ".nameWithOwner",
    ])?;
    let repo = repo.trim().to_string();
    let vis = gh(&["repo", "view", "--json", "visibility", "-q", ".visibility"])?;
    if vis.trim() != "PRIVATE" {
        return Err(format!(
            "{repo} is not private ({}); refusing (H2)",
            vis.trim()
        ));
    }
    let labels = gh(&[
        "label", "list", "--limit", "500", "--json", "name", "-q", ".[].name",
    ])?;
    if !labels.lines().any(|l| l.trim() == LABEL) {
        gh(&[
            "label",
            "create",
            LABEL,
            "--color",
            "c5def5",
            "--description",
            "Deferred post-v1 (docs/FUTURE.md)",
        ])?;
        println!("created label {LABEL}");
    }
    let ms = gh(&[
        "api",
        &format!("repos/{repo}/milestones?state=all&per_page=100"),
        "-q",
        ".[].title",
    ])?;
    if !ms.lines().any(|l| l.trim() == MILESTONE) {
        gh(&[
            "api",
            "-X",
            "POST",
            &format!("repos/{repo}/milestones"),
            "-f",
            &format!("title={MILESTONE}"),
            "-f",
            "description=Features deferred after v1.0.0 (docs/FUTURE.md)",
        ])?;
        println!("created milestone {MILESTONE}");
    }
    let have = gh(&[
        "issue",
        "list",
        "--state",
        "all",
        "--limit",
        "1000",
        "--json",
        "title",
        "-q",
        ".[].title",
    ])?;
    let have: Vec<&str> = have.lines().map(str::trim).collect();
    let (mut created, mut skipped) = (0, 0);
    for (title, body) in &want {
        if have.contains(&title.as_str()) {
            skipped += 1;
            continue;
        }
        gh(&[
            "issue",
            "create",
            "--title",
            title,
            "--body",
            body,
            "--label",
            LABEL,
            "--milestone",
            MILESTONE,
        ])?;
        created += 1;
        println!("created: {title}");
    }
    println!(
        "{} FUT rows: {created} created, {skipped} already present ({repo}, private)",
        want.len()
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn one_issue_per_fut_row() {
        let f = "| FUT | Deferred feature | v1 instead | Spec IDs |\n|---|---|---|---|\n| FUT-01 | Lexers | 30 lexers | R-CODE-01 |\n| FUT-39 | Native PDF | print path | R-EXP-01 |\n(FUT-07 merged)\n";
        let v = issues(f);
        assert_eq!(v.len(), 2);
        assert_eq!(v[0].0, "FUT-01: Lexers");
        assert!(v[1].1.contains("**v1 instead:** print path"));
        assert!(v[1].1.contains("R-EXP-01"));
    }

    #[test]
    fn real_future_md_rows() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
        let f = std::fs::read_to_string(root.join("docs/FUTURE.md")).unwrap_or_default();
        let n = f.lines().filter(|l| l.starts_with("| FUT-")).count();
        assert_eq!(issues(&f).len(), n);
        assert!(n > 30);
    }
}
