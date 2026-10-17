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
- [ ] platform: popup menu, context menu, toast/zoom, find bar, sidebar, themes live, single instance, watcher, positions, editor, registration + CLI, welcome
- [ ] driver scripts, goldens, measurements, report, PR/merge/tag
