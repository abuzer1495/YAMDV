#!/usr/bin/env python3
"""Create the YAMDV post-v1 backlog as GitHub issues (idempotent).

Usage (from the repository root, with an authenticated gh CLI):
    python3 tools/backlog/create_issues.py tools/backlog/issues.jsonl
An issue is skipped when an existing issue (open or closed) has the same ID prefix
before the first ':' (e.g. "FUT-07", "PERF-02"), so the script can be re-run safely.
"""
import json
import subprocess
import sys
import time

LABELS = {
    "future": ("0e8a16", "Deferred from v1 (docs/FUTURE.md)"),
    "hardening": ("b60205", "Security hardening after v1"),
    "performance": ("fbca04", "Memory / speed optimization after v1"),
    "bug": ("d73a4a", "Known deviation or defect"),
    "qa": ("5319e7", "Needs verification on real hardware"),
    "decision": ("0052cc", "Needs an owner decision"),
    "idea": ("c5def5", "Improvement not in the original spec"),
}
MILESTONE = "post-v1"


def gh(*args, check=True):
    r = subprocess.run(["gh", *args], capture_output=True, text=True)
    if check and r.returncode != 0:
        sys.exit(f"gh {' '.join(args[:3])} ... failed: {r.stderr.strip()}")
    return r


def main(path):
    for name, (color, desc) in LABELS.items():
        gh("label", "create", name, "--color", color, "--description", desc, "--force")
    titles = gh("api", "repos/{owner}/{repo}/milestones?state=all&per_page=100",
                "--jq", ".[].title").stdout.split("\n")
    if MILESTONE not in titles:
        gh("api", "repos/{owner}/{repo}/milestones", "-f", f"title={MILESTONE}",
           "-f", "description=Work planned after YAMDV v1.0.0")
    existing = json.loads(gh("issue", "list", "--state", "all", "--limit", "1000",
                             "--json", "title").stdout)
    have = {t["title"].split(":")[0].strip() for t in existing}
    created = skipped = 0
    with open(path, encoding="utf-8") as f:
        for line in f:
            if not line.strip():
                continue
            item = json.loads(line)
            key = item["title"].split(":")[0].strip()
            if key in have:
                skipped += 1
                continue
            gh("issue", "create", "--title", item["title"], "--body", item["body"],
               "--label", ",".join(item["labels"]), "--milestone", MILESTONE)
            have.add(key)
            created += 1
            time.sleep(2)  # stay well below GitHub's content-creation rate limits
    print(f"created {created}, skipped {skipped} (already present)")


if __name__ == "__main__":
    main(sys.argv[1])
