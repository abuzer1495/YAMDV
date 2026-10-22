# Phase V2-B plan (prompt 012)
Issues: BUG-10 #77, BUG-11 #78, FUT-31 #30, FUT-04 #11, FUT-49 #47, FUT-05 #12, FUT-26 #25 (comment only).
Branch `phase/v2b-settings`; A-8: no CI, every commit `[skip ci]`, local gates before each push.

- [x] §2 prompt stored, ROADMAP_V2 + colors.md extracted, A-8, P-34…P-39, spec-check green, issues #77/#78, R-EXP-07 row
- [ ] §3.1 BUG-10 export palette (pure resolution in core/export + unit tests; wire PDF/Print/HTML)
- [ ] §3.2 BUG-11 non-blocking export (worker progress codec + ETA pure; card; taskbar; cancel)
- [ ] §4.1 settings panel, §4.2 search, §4.3 font pickers, §4.4 color editor, §4.5 shortcuts, §4.6 v1.1.0
- [ ] §5 test pack (run-tests.cmd, 4 driver scripts, screens, OWNER_TEST.md, baseline, zip, test-v2b)
- [ ] §6 PR, merge, tag phase-v2b, report

## CHECKPOINT
- 21:45 §2 done and pushed; next: §3.1 survey palette use in yamdv-export/html.rs, print.rs, cli.rs.
- §3.1 BUG-10 done (yamdv_core::theme::export_palette, colors::apply, html/pdf/print per-job setting).
- §3.2 BUG-11 done: yamdv_core::progress (codec/throttle/ETA, tests), crate::job (worker spawn with
  handle-list inheritance, Link), window/jobui.rs (card, taskbar, Cancel, close confirm, toast
  [Open]/[Show in folder], error banner), driver verbs `ui export-html-bg|export-pdf-bg|job-cancel`.
  Next: §4.1 settings panel (pure model in yamdv-ui::settings_panel, then window/settings.rs).
- 22:25 §4.1–4.5 code done and pushed: yamdv-core::keymap (presets, conflicts, translate),
  yamdv-ui::settings_panel (rows, search, geometry), window/setpanel.rs (panel, fields, font
  list via DirectWrite on demand, colour popover, shortcut capture), keymap layer in wndproc,
  F1 bindings table, diagnostic log wired, ChooseFontW path removed from the app (host kind kept).
  Driver verbs `ui settings-*`, state `settings*`, `key.*`, `pal.*`, `ini.*`, `theme.*`, PWS.
  Scripts settings-panel / color-editor / shortcuts written. Next: export-progress (ps1 +
  yds), titlebar.yds update, §4.6 version 1.1.0 + CHANGELOG, §5 pack, §6.
- 22:50 export-progress.yds (verbs `ui job-wait|job-done|job-reset|cpu-mark`, state job_seen_*,
  job_monotonic, worker, cpu_delta_ms, out_body_bg; fixture pages-30.md), titlebar.yds asserts the
  panel, xtask root falls back to the cwd (pack). §4.6 done: 1.1.0 + CHANGELOG (no tag).
  Next: §5 run-tests.cmd/.ps1, OWNER_TEST.md, baseline build, zip, draft release test-v2b; §6.
- 23:00 §5 pack tooling: tools/testpack/run-tests.cmd + run-tests.ps1 (driver loop with temp
  profiles, export-progress WM_NULL poll, goldens report-only, headless export, opt-in install,
  M0/M1 release budget + dev vs baseline delta gate, T7, T4), OWNER_TEST.md (EN+TR, 12 items),
  build-pack.sh (baseline = dev-tools build of 9f6906e). Next: upload draft test-v2b, then §6.
