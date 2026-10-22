# Phase V2-A plan (prompt 011)
Issues: FUT-30 #29, FUT-32 #31, FUT-35 #34, FUT-46 #44, BUG-09 #69.

- [x] §2 prompt stored (sha256 d0364143…), ROADMAP_V2.md extracted, A-7 amendment, P-32/P-33, spec-check green
- [x] §3.2 layered popup menu component (replaces TrackPopupMenuEx)
- [x] §3.1 custom title bar (NCCALCSIZE/NCHITTEST, buttons, ⚙/⋯, Alt/F10)
- [x] §3.3 scheme cards flyout
- [x] §3.4 F11 fullscreen
- [x] §3.5 word-count badge (pure counter in core + unit tests)
- [x] §3.6 zoom indicator pill
- [x] tests: titlebar.yds, scheme-cards.yds, fullscreen-wordcount.yds; 4 goldens from CI artifact
- [ ] TRACEABILITY by script; local gates; CI run; fix push; merge; tag phase-v2a
- [ ] report docs/reports/v2a_REPORT.md

## CHECKPOINT
(see latest entry below)
- 13:50 setup committed; next: survey menu/window code in yamdv-app / yamdv-platform-win.
- 14:05 design: menu = non-modal thread-local popup (WS_EX_LAYERED alpha fade + NOACTIVATE + capture on root popup, DWM corner pref); title bar drawn in main client with a top offset (TOP) applied centrally (renderer origin + lparam_xy).
- 13:51 core chrome.rs + devscript ui + i18n pushed; next: render Band origin, popup.rs
- 13:57 popup.rs + app model written (not yet wired/compiling); next: chrome.rs + window.rs integration
- 14:10 all features wired; local gates green (fmt, clippy native+xwin, tests, deny, spec-check, check-imports, size 2,826,752 B); TRACEABILITY updated; next: full CI run, goldens from artifact
