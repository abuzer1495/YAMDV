# Phase V1-A plan — App shell & daily use (prompt 005)

Goal: a viewer someone can install as their default `.md` app and use every day — popup/context menu with live settings, sidebar (Contents/History/Favorites), themes everywhere, single instance + one window per file, live reload, reading position, find, zoom, external editor, system integration, welcome page, toast. Mode: A-4 (simplest implementation that meets the ACs; unit tests for new pure logic; one driver script per user-visible feature; CI push once at the end + one fix push).

Order: §2 process/scope (script) → pure logic (menu model, find, history grouping, geometry validation, registry plan, editor command) → platform (popup, toast, sidebar, find bar, watcher, IPC, registration) → driver scripts → goldens (once) → measurements → report → PR/merge/tag.

Risks: window.rs is already 2.9 kLOC (split new UI into modules); T4 p99 at 14.2 ms of 16 (keep the sidebar out of the scroll blit path); registry writes must stay explicit-action-only (S-REG-03); CI driver cannot exercise ChooseFontW/default-app UI (NOT VERIFIED there).

## CHECKPOINT
- [x] prompt stored `docs/prompts/005_V1A_SHELL_DAILY_USE.md` (SHA-256 6c6c8e482e5a213268fafd7bbfef410ce8d51f516772c0b01ad5196d5b46c04b)
- [x] §2.1 A-4 (PROCESS_AMENDMENTS.md → PROCESS.md, extracted by script)
- [x] §2.2 docs/FUTURE.md by `cargo xtask future` (spec-check verifies); README section
- [x] §2.3 P-30, P-31 (`docs/spec-patches/v1a.txt`), spec-check green
- [x] §2.4 TRACEABILITY deferrals by `cargo xtask future` (70 rows; 11 section refs are not rows)
- [x] pure core: `state.rs` (state.ini, history groups, favorites, positions, geometry validation), `find.rs` (Turkish i), `cmdline.rs` (S-EXEC-03 quoting, editor templates), `reg.rs` (S-REG plan), `ipc` Forward kind (S-IPC-01), `Settings::set`
- [x] platform services `src/shell.rs` (mutex+forward, watcher, CreateProcessW editor, HKCU reg plan writer, AssocQueryStringW, atomic write); i18n strings for menu/sidebar/find/welcome added
- [x] DONE (compiles, not yet run on Windows): `window/app.rs` settings/state persistence + live restyle, owner-draw context menu (all §3.2 entries incl. ChooseFontW via dialog host `font` kind), zoom keys/wheel + toast, F1 help (assets/help_*.md), F4/Ctrl+E editor, Ctrl+D favorites, About, find bar `window/findbar.rs`, single instance + forward + one window per file, watcher → debounced reload / dimmed + toast, reading positions, history recording, geometry save/restore, CLI --register/--unregister/--set-default/--context-menu/--line
- [x] sidebar `window/sidebar.rs` (tabs, TOC flatten/spy/collapse persisted, history groups/remove/clear, favorites, chevron, drag-resize, left/right)
- [ ] NEXT: welcome page, custom-theme.ini hot reload watcher, driver commands+scripts, goldens, measurements(T3), report
- [ ] platform: popup menu, context menu, toast/zoom, find bar, sidebar, themes live, single instance, watcher, positions, editor, registration + CLI, welcome
- [ ] driver scripts, goldens, measurements, report, PR/merge/tag

- [x] welcome page integrated (default-app status via UserChoice ProgId, no shell assoc engine)
- [x] driver commands (command/type/side-click/write-file/open/launch/sleep/snapshot/resize) + 6 scripts; golden-shell xtask + CI step; driver profiles isolated per script
- [x] custom-theme.ini hot reload (watcher only while custom colours on), T3 in measure.ps1, TRACEABILITY statuses
- [x] PR #5, CI run 1 (39015816643): all driver asserts pass; exit 0xc0000409 at process end (watcher join in TLS dtor), measure.ps1 parse error ($r:), theme.yds default scheme assert
- [x] fix push: explicit teardown on WM_QUIT, ps1 fix, script fix, 5 window goldens from run-1 artifact
- NEXT: CI run 2 -> report with numbers -> merge -> tag phase-v1a
- [x] CI run 2 (39016409650) green, BUT T4 gate was string-compared: typical warm p99 14.36/19.19/18.58 (median 18.58 > 16). Fix push 2 (029b66e): ScrollWindowEx limited to the doc rect (sidebar/chevron/scrollbar not blitted), sidebar repainted only on spy change, overlay rects repainted, measure.ps1 numeric T4 gate. (Third CI push: A-4 rule 6 deviation, justified by AC-3 + prompt §1 watch item.)
- NEXT: CI run 3 (39017680216) -> report -> merge -> tag phase-v1a
- [x] CI run 3 (39017680216) FAILED: runner ~3x slower (M0 first paint 154 ms on untouched code), typical warm p99 36 ms, large T4 hit the 180 s timeout. Root cause: Renderer::paint bands over rcPaint (bounding box), so the full-height scrollbar strip (old code) and the sidebar spy column (fix 2) made every scroll step a full-window repaint. Fix 3: paint the update region's vertical spans, invalidate old/new thumb only, sidebar spy via a 120 ms one-shot timer.
- [x] fix 3 pushed (67f99d5)
- [x] CI run 4 (39019374434) green: T4 typical warm 14.17 ms; report written
- [x] report push run 39020574893: attempt 1 T4 typical 26.4 ms (slow runner, same code), attempt 2 green 14.68 ms; flakiness noted in report
- [x] CI on PR #5 green (39022792455) -> merged (5fdf661, 2026-10-17 13:06 UTC+3) -> tagged phase-v1a (CI 39023641871 green). Phase closed.
