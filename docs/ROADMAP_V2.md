# YAMDV — Roadmap v2 (owner plan, 2026-10-22; revised the same day)

**Goal:** turn v1.0.0 into the product originally specified in `docs/SPEC.md` and `docs/UI_SPEC.md` (before the v1 scope cut), phase by phase, while staying as fast and light as v1. **The order is UI first**, then reading/editing features, then content engines and export, then hardening and performance.

Each phase implements the listed items. Their requirement IDs come back into scope, `docs/TRACEABILITY.md` moves them from `deferred (FUT-xx)` to DONE, and the merged PR closes the matching GitHub issues (milestone `post-v1`).

**Rules:**
- Bugs the owner reports from real-PC use go first in the next prompt.
- From V2-C on, CI runs again (PROCESS A-9). The owner checks each release by hand.
- Version tags are set when CI is green; releases stay drafts until the owner publishes them.

| Phase | Prompt | Content | Items (issue IDs) | Release |
|---|---|---|---|---|
| V2-A | 011 | Window chrome: custom title bar with ⚙ and ⋯ buttons, layered popup menus (B.7), color-scheme cards, F11 fullscreen, word-count badge, zoom indicator | FUT-30, FUT-32, FUT-35, FUT-46, BUG-09 | — |
| V2-B | 012 | **First:** A-8 owner test pack; owner bugs — the export palette (PDF, Print, HTML; `colors.md` fixture) and non-blocking export with a progress card + Cancel. **Then:** settings panel (B.8, all sections), custom-color editor, font pickers, shortcut list + key capture + Typora preset, settings search | BUG-10, BUG-11, FUT-31, FUT-04, FUT-49, FUT-05, FUT-26 | v1.1.0 |
| V2-C | 013 | Tooltips, anchored popovers, banners; fullscreen viewers (image, table, diagram) with ⛶ badges; heading folding; inline objects for sup/sub/footnote refs; wide diagrams scroll instead of shrinking | FUT-40, FUT-36, FUT-06, FUT-34, FUT-41, BUG-08 | — |
| V2-D | 014 | Navigation: Files tab, floating sidebar, favorites reorder, heading jump palette + `[TOC]`, mind-map view; keyboard reachability of every control | FUT-33, FUT-02, FUT-03, FUT-08, FUT-37, HARD-04 | v1.2.0 |
| V2-E | 015 | Editing plus: F2 format toolbar, find & replace (+ regex, mirrored in F3), snippets, image/file paste, recovery journal, editor extras, HTML paste → Markdown | FUT-45, FUT-12, FUT-15 | — |
| V2-F | 016 | Tables and full WYSIWYG: table toolbar and operations, drag handles, TSV paste, F2 inline marker reveal, F2 table cell editing | FUT-44, FUT-43 | v1.3.0 |
| V2-G | 017 | Content engines: math beyond the core + macros; Mermaid gantt, pie, state, class, ER + `%%{init}%%` + Copy as PNG; SVG images; animated GIF; ICO/WebP; more highlight languages | FUT-17, FUT-18, FUT-20, FUT-48, FUT-01 | — |
| V2-H | 018 | Export and integration: native PDF writer (vector text, bookmarks, links; also the PDF speed fix), interactive HTML export (hash CSP), copy rich HTML / PNG, more external editors, reopen with encoding, jump list, custom icons; known bugs | FUT-39, FUT-21, FUT-23, FUT-10, FUT-09, FUT-25, BUG-01…07 | v1.4.0 |
| V2-I | 019 | Hardening: full security review + THREAT_MODEL, long fuzzing, ACG re-verification, High Contrast + UI Automation, real-hardware QA | HARD-01…03, FUT-47, QA-01 | — |
| V2-J | 020 | Performance and 2.0: final memory budgets (Appendix D final column), dialog latency, scroll paths, Mermaid rasters, binary size, incremental live reload, stable-hardware perf gates, DIB/DDB | PERF-01…07, FUT-42 | v2.0.0 |
| Later | — | Not planned for v2: editor plugins (FUT-27), macOS/Linux (FUT-28), more UI languages (FUT-29), code signing / winget / ARM64 / spell check (IDEA-01…04), license (DEC-01, owner decision) | — | — |

The reviewer may re-cut the order from the phase reports. A phase that does not finish continues in the next prompt (A-3 rule 9).
