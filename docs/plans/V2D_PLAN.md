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
- CHECKPOINT 11:36 nav module: Files tab, floating card, tab list, favorites drag/Alt+arrows, keyboard focus + F6, driver verbs; palette/mind map stubs. Next: palette, mind map, TOC, menus/keymap/settings rows, blit fix.
- CHECKPOINT 11:37 heading palette done (Ctrl+Shift+O, unfold on jump). Next: mind map, TOC, menus/keymap/settings, blit fix.
- CHECKPOINT 11:39 mind map viewer done. Next: [TOC] render + export, menus/keymap/settings rows, blit fix, version, tests.
- CHECKPOINT 11:42 [TOC] (core toc.rs, refresh on edit, export/print depth) done. Next: menus/keymap/settings rows, blit fix, version, tests.
- CHECKPOINT 11:44 menus (Floating, Go to heading, View as mind map), settings rows, floating blit fix. Next: version 1.2.0, tests/goldens, docs.
- CHECKPOINT 11:48 driver scripts files-favorites, palette-mindmap-toc, keyboard-reach + fixtures. Next: goldens, TRACEABILITY, docs, PR.
- CHECKPOINT 11:49 4 goldens wired (shots.yds/SHELL_SHOTS), TRACEABILITY by script. Next: open PR, single CI run.
- CHECKPOINT 11:50 PR opened; full CI run 1 of 1 (+ at most one fix push).
- CHECKPOINT 12:21 CI run 39927529395 red (tab-order needs heading stops, keyboard-reach walk wrapped, files-favorites timeout; menu + footnote goldens). Fix push: scripts, bounded nav-wait-files, partial driver log, watcher drop cancel loop, 6 goldens, mind-map-on-large report step.
- CHECKPOINT 12:49 fix run 39931340336 red only on files-favorites last step (drop-folder path '.'); probe fix pushed [skip ci], not CI-verified. No merge/tags (F-24). Report written. Next: owner approves one more CI run.
- CHECKPOINT 19:01 owner approved one more CI run; triggering it.
- CHECKPOINT 19:27 run 3 39983230028 red: only export-progress cpu_delta_ms 16>15 (timer tick); no fixes pushed; waiting for owner decision (rerun or relax to <32).
- CHECKPOINT 19:32 A-10 owner override: export-progress CPU assert report-only, anchor.yds removed (covered by navigation.yds); final CI run then merge regardless.
- CHECKPOINT 19:55 run 4 39987570191 green; merged aaac914 (tree = db674fb); tags phase-v2d, v1.2.0 pushed; tag workflow skipped by [skip ci] in merge subject → no draft release; owner decides (move tags or manual draft).
