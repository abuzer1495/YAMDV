# V2-D plan (prompt 014): Files tab, floating sidebar, favorites reorder, heading palette, [TOC], mind map, keyboard reach → v1.2.0

## Goals (restated)
- §1.1: F-23 footnote tooltip = rendered plain text (`yamdv_core::doc::plain`); F-24 tagging rule in A-9. §1.2 skipped (no owner notes).
- §2: P-43…P-45 via `docs/spec-patches/v2d.txt` + `xtask spec-patch`; `spec-check` green.
- §3: items 1–7 (pure logic in platform-independent crates with unit tests first, then the window layer), item 8 v1.2.0.
- Tests: driver scripts files-favorites, palette-mindmap-toc, keyboard-reach; 4 goldens; unit tests.

## Tagging (F-24)
Merge #PR with a clean squash title; if the merge tree equals the green PR head (`git diff --quiet`), tag `phase-v2d`
and `v1.2.0` on it (the v1.2.0 tag run builds the draft). Otherwise let the main push run go green first.

## Risks
- Scope is large; order: pure crates + tests → window layer split into modules → driver verbs → scripts → goldens.
- Idle CPU 0 with the Files tab and the mind map: no timers; watcher on the existing thread.

## CHECKPOINT
- 11:17: prompt 014 stored (14,0 KB, 227 lines); F-23 plain_text + unit test, wired into the footnote tooltip; F-24 line A-9 §8; gate OK. Next: P-43…P-45, then item 1 pure parts (natural sort, skip rules).
- 11:17: P-43…P-45 applied via docs/spec-patches/v2d.txt; F-24 line lives in PROCESS_AMENDMENTS.md (PROCESS.md is generated); spec-check match. Next: item 1 pure parts.
- 11:25 §3 pure layer: core files.rs (natural sort/skip/tree), outline.rs (palette filter, [TOC]), doc/mind.rs; ui mindmap.rs (tidy layout, hues); layout focus.rs. Next: window nav modules.
