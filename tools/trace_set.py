#!/usr/bin/env python3
"""Sets TRACEABILITY statuses from a TSV (`ID<TAB>status`), keeping the phase column.
Usage: python3 tools/trace_set.py <rows.tsv>   (idempotent; fails on an unknown ID)."""
import sys

rows = {}
for line in open(sys.argv[1], encoding="utf-8"):
    if line.strip() and not line.startswith("#"):
        k, v = line.rstrip("\n").split("\t", 1)
        rows[k] = v
path = "docs/TRACEABILITY.md"
out, seen = [], set()
for line in open(path, encoding="utf-8"):
    cells = line.split(" | ")
    rid = cells[0][2:] if line.startswith("| ") else None
    if rid in rows and len(cells) >= 3:
        line = f"| {rid} | {cells[1]} | {rows[rid]} |\n"
        seen.add(rid)
    out.append(line)
missing = set(rows) - seen
if missing:
    sys.exit(f"unknown IDs: {sorted(missing)}")
open(path, "w", encoding="utf-8", newline="\n").write("".join(out))
print(f"updated {len(seen)} rows")
