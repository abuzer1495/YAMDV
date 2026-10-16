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
