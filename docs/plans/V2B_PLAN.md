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
