# V2-G plan (prompt 017: content engines)

Order: §1.1 F-31 → F-32 → §2 ADR-0021 (before SVG code), ADR-0022, P-48, IDEA-05 → §3 items 1–7 → tests/goldens → gates → PR (one CI run, +1 fix) → merge → phase-v2g (no release) → report.

## CHECKPOINT 1 (§1.1 done)
- Prompt stored as docs/prompts/017_V2G_CONTENT_ENGINES.md (rewritten from the Read output; sha256 2433cad3… differs from the attachment name bac86ab5…, content identical as read).
- F-31: ci.yml driver loop reruns a script once when its only FAIL lines are pws_delta_kib / cpu_delta_ms asserts.
- F-32: push tags `['v*', '!phase-*']`; pure/windows skip on tags and on `chore(release):` pushes; release job has no `needs`. One line (A-9 item 10) in PROCESS.md + PROCESS_AMENDMENTS.md.
- Next: §2 ADR-0021 SVG (check D2D SVG device needs, resvg size), ADR-0022, P-48, IDEA-05.
