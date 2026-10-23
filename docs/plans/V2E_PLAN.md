# V2-E plan (prompt 015)

Order: §1.1 F-25 → F-26 → F-27 → F-28 → §2 P-46 + regex ADR → §3 items 1–6 → tests → gates → PR, one CI run (+1 fix) → merge → tag phase-v2e → report.

## CHECKPOINT
- 20:13 F-25: chore(release): v1.2.0 = 2534afb on main (tree = f99904c; f99904c vs aaac914 changes docs only: v2d_REPORT, V2D_PLAN). Tags v1.2.0 + phase-v2d moved to 2534afb and pushed; tag runs 39993219747 (v1.2.0), 39993219341 (phase-v2d), main 39993213895. F-25 line added to A-9 (PROCESS_AMENDMENTS, spec-patch). Prompt stored. Next: wait for draft, then F-26.
- 20:17 F-26 code pushed (watch.rs; probe nav-watch-threads in files-favorites). Next: wine check, then F-27.
- F-26 wine: watch_threads 2>2 (delta 0, live 14). F-27 done: nav keys are keymap actions (+ V2-E edit actions), scope-aware conflicts, v2d_key uses action_of. Next: F-28 toast links.
- F-26 wine: watch_threads 2>2 (delta 0, live 14). F-27 done: nav keys are keymap actions (+ V2-E edit actions), scope-aware conflicts, v2d_key uses action_of. Next: F-28 toast links.
- F-28 done: toast links drawn as B.10.1 links, stops in F6 banner region. Next: P-46 + regex ADR.
- P-46 + ADR-0019 (regex-lite +59 KB probe; regex minimal +330 KB). Next: task 2 find/replace engine (editor crate), then 3 snippets, 4 line ops, 1 toolbar, 5 paste, 6 journal.
- Pure: replace/snippets/lines/pairs (editor), paste html2md/assets (core) + XSS paste context done. Next: journal (core), then platform wiring.
- CHECKPOINT: core journal.rs done (format/name/schedule). Next: platform wiring (find bar replace row first).
- CHECKPOINT 20:42 +03: task 5 paste/drop wired (paste_rich, paste_plain, drop_into_doc, sandboxed asset save); next recovery journal integration.
- CHECKPOINT 20:46 +03: task 6 recovery journal wired (worker, timer, banner, hooks). Next: driver verbs v2e-*, 2 scripts, 2 goldens.
- CHECKPOINT 2026-10-23 20:56 task tests: edit-plus.yds green under Wine (v2e-live-select verb: Wine DirectWrite has no hit test, click-text unusable locally); goldens v2e-toolbar-light.png, v2e-replace-dark.png. Next: paste-recovery.yds.
- CHECKPOINT 2026-10-23 20:59 paste-recovery.yds green (Ctrl+V/Shift+Ins now rich paste; !contains op; v2e-banner verb). Next: full driver regression, TRACEABILITY, gates.
