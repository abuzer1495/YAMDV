# Spec changes

Every change to the specification appendices is applied by script (`cargo xtask spec-patch` from `docs/spec-patches/*.txt`). `cargo xtask spec-check` verifies the result as "original (from `docs/prompts/001_…md`) + patches".

| ID | date | doc | section | summary | reason |
|---|---|---|---|---|---|
| P-01 | 2026-10-16 | SPEC.md | R-LINK-05 | allowlist reference `S-EXEC-02` → `S-LINK-03` | wrong cross-reference (001 report §12.1) |
| P-02 | 2026-10-16 | SPEC.md | R-MMD-03 | palette reference B.6.12 → B.3.6 | wrong cross-reference (001 report §12.2) |
| P-03a | 2026-10-16 | SPEC.md | R-THEME-06 | defines `border` alias = `code_border` + `table_border` | undefined token (001 report §12.3) |
| P-03b | 2026-10-16 | UI_SPEC.md | §B.3.1 | notes the `border` alias mapping | same |
| P-04 | 2026-10-16 | PERFORMANCE.md | §D.4 rule 8 | allocator rule clarified; heap type is a measured choice (ADR-0002) | SegmentHeap ambiguity (001 report §12.4) |
| P-05 | 2026-10-16 | PERFORMANCE.md | §D.3 | MB = MiB (2^20), tools print bytes + MiB | unit ambiguity (001 report §12.5) |
| P-06 | 2026-10-16 | ARCHITECTURE.md | §E.9 | spikes may use `unsafe` (A-1) | E.9 vs H14 (001 report §12.6) |
| P-07 | 2026-10-16 | ARCHITECTURE.md | §E.4 | two-level document model (block index + lazy detail trees) | reviewer design decision |
| P-08a | 2026-10-16 | SPEC.md | R-MD-06 | single-tilde subscript off by default (`reading.subscript_tilde`) | conflicts with GFM strikethrough |
| P-08b | 2026-10-16 | UI_SPEC.md | §B.8.2 | adds setting `reading.subscript_tilde` | same |
| P-09 | 2026-10-16 | PERFORMANCE.md | §D.2 | interim scenarios M1b, T1b, T1L | Phase 002 before the full corpus |
| P-10 | 2026-10-16 | PERFORMANCE.md | §D.2 M6 | M6 after host dialog, no trim, PWS+commit | 002 report §8.4 |
| P-11 | 2026-10-16 | PERFORMANCE.md | §D.4 r10 | dialogs out-of-process; trims only after print/export | same |
| P-12 | 2026-10-16 | PERFORMANCE.md | §D.4 r3 | band buffer cap 512 KiB | reviewer finding |
| P-13 | 2026-10-16 | PERFORMANCE.md | §D.4 r4 | image decode/keep/LRU rules, storage ADR | precision |
| P-14 | 2026-10-16 | SECURITY.md | S-PROC-07 | dialog host child process allowed | F-09 |
| P-15 | 2026-10-16 | SECURITY.md | S-IPC-05 | WM_COPYDATA host protocol rules | F-09 |
| P-16 | 2026-10-16 | SECURITY.md | S-FILE-01 | opt-in read-mode task toggle exception | spec conflict |
| P-17 | 2026-10-16 | SPEC.md | R-REN-04 | task toggle wording | same |
| P-18 | 2026-10-16 | ARCHITECTURE.md | §E.10 | re-entrancy rule | 002 bug |
| P-19 | 2026-10-16 | ARCHITECTURE.md | §E.6 | true clipping required | reviewer finding |
| P-20 | 2026-10-16 | SHORTCUTS.md | §F.4 | internal --dialog-host flag | F-09 |
| P-21 | 2026-10-16 | UI_SPEC.md | §B.6.3 | inline code padding via character spacing | 002 deviation 7 |
| P-22 | 2026-10-16 | ROADMAP.md | row 010 | SVG images in Phase 010 | timing |
| P-23 | 2026-10-16 | TESTING.md | §H.4 | golden tile policy | 002 report §8.5 |
| P-24 | 2026-10-16 | SPEC.md | R-MD-14 | id/name anchor attributes | README anchors |
| P-25 | 2026-10-16 | SPEC.md | R-IMG-06 | compact inline image chip | README badges |
| P-26 | 2026-10-17 | PERFORMANCE.md | §D.4 rule 3 | band cap wording (>4096 px uses 32-row floor) | 003 report §8 Q1 |
| P-27 | 2026-10-17 | SPEC.md | R-IMG-06 | *invalid path* reason (device names, ADS, NUL) | 003 report §8 Q3 |
| P-28 | 2026-10-17 | TESTING.md | §H.4 | golden set ≤ 12 MB policy | 24 MB goldens in 003 |
| P-29 | 2026-10-17 | ROADMAP.md | phase table | insert Phase 003b row | phase split |
