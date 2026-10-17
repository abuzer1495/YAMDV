# 005_V1A_SHELL_DAILY_USE — YAMDV Phase V1-A: App shell & daily use

> **Prompt 005.** Phase **V1-A**. Branch `phase/v1a-shell`, tag `phase-v1a`, report `docs/reports/v1a_REPORT.md`. Store this file as received in `docs/prompts/`.
> **New owner plan:** finish a complete, working v1 in **four phases** (V1-A shell & daily use → V1-B editing → V1-C math + Mermaid → V1-D export, packaging, release) plus one reserve phase. Security hardening, optimization and the deferred features (`docs/FUTURE.md`) come **after** v1. Old phase numbers 004–014 no longer apply.
> **Precedence:** `docs/PROCESS.md` (H1–H15, A-1…A-3, and the new A-4 below) > §2 > §3 > §4 > `docs/*`.

---

## 1. Review of Phase 003b

Phase 003b is **accepted.** The highlights:
- images end to end
- the in-window test driver, with 6 of 6 scripts passing on CI
- Tab focus navigation
- goldens pruned from 24 MB to 3.4 MB

**Answers:**
1. The remaining 003 items move to `docs/FUTURE.md` (FUT-40…42). There is no 003c.
2. WebP and ICO failures under ACG show the "unsupported format / codec not installed" placeholder (R-IMG-06). They are deferred as FUT-48, with no further work in v1.

**Watch item:** typical warm T4 p99 rose to 14.2 ms. The 16 ms CI gate stays. If new content pushes it over, fix the scroll/blit path (cached scaled bitmaps, blit only the intersecting rect) inside this phase.

---

## 2. Process and scope changes (do first, by script — keep it cheap)

1. **Amendment A-4** — add it to `docs/PROCESS.md` through the amendment mechanism:
```text
A-4 Delivery mode (owner decision; from Phase V1-A until the v1 release):
1. The priority is a complete, working v1 within the remaining budget. Choose the simplest implementation that meets the phase's acceptance criteria.
2. Tests: unit tests for new pure logic and one driver script per user-visible feature. No new fuzz targets except for new parsers (run them 5 minutes in the background). Goldens only where the prompt lists them.
3. Measurements: run the existing automated table only; build no new measurement tooling.
4. Docs: update TRACEABILITY by script. ADRs only for new dependencies or irreversible design choices, ≤ 15 lines each.
5. Report ≤ 120 lines: §0, an AC table, the measurements table, deviations, and what is left. Omit sections that have nothing to say (this overrides the template rule "never omit").
6. Push to CI once at the end of the phase, plus at most one fix push.
```

2. **`docs/FUTURE.md`:** write the fenced `FUTURE.md` block at the end of this prompt into the file, **extracted by script** (do not re-type it). Add a README section "Possible future developments" that links to it.

3. **Spec patches** (`docs/spec-patches/v1a.txt`, applied by `xtask spec-patch`, checked by `spec-check`):

**P-30** — `docs/ROADMAP.md`. INSERT AFTER the anchor line:
```text
# APPENDIX G — Roadmap (`docs/ROADMAP.md`)
```
NEW line:
```text
> **Superseded on 2026-10-17 by the owner's v1 plan:** the phases after 003b in the table below are replaced by **V1-A** (app shell & daily use), **V1-B** (editing: F3 split + block-level F2), **V1-C** (math core + Mermaid flowchart/sequence), **V1-D** (HTML export, print/PDF, packaging, v1 release) and **V1-FIX** (reserve). Security hardening (old 012) and optimization (old 013) follow after v1. Deferred features: `docs/FUTURE.md`.
```

**P-31** — `docs/SPEC.md`. INSERT AFTER the anchor line:
```text
- **R-REL-04 MUST** Works without admin rights, on locked-down machines, from read-only media (portable mode then falls back to `%APPDATA%`).
```
NEW lines (a blank line, the heading, a blank line, the paragraph):
```text

## A.25 v1 scope
The owner's v1 scope decision (2026-10-17) defers the features listed in `docs/FUTURE.md` (FUT-01…FUT-50) to after v1. For v1, any requirement or part of a requirement covered by a FUT item is not required, whatever its MUST/SHOULD keyword; `docs/TRACEABILITY.md` marks it `deferred (FUT-xx)`.
```

4. **TRACEABILITY:** mark every ID covered by FUT-01…FUT-50, using a script driven by the "Spec IDs" column of `FUTURE.md`. Ranges such as `R-TBL-01…08` expand to every ID in the range. IDs that are not done become `deferred (FUT-xx)`; IDs that are already DONE or PARTIAL keep their status and get "part deferred: FUT-xx" appended.

---

## 3. V1-A tasks

The goal: a viewer someone can install as their default `.md` app and use every day. It must look and behave like the reference app within the v1 scope.

1. **Popup menu component.**
   - Layered owner-drawn popup with submenus, check and radio marks, shortcut column and separators, styled per B.7.1.
   - Keyboard: arrows, Enter, Esc, typing a letter jumps to the item.
   - Follows the theme.
   - Opens on right-click and on `Shift+F10` / the Menu key.
2. **Context menu content.** It replaces the settings panel in v1 (FUT-31). Every setting writes `settings.ini` (the writer exists) and applies live. The menu:
   - **Copy** · **Copy as Markdown** · **Copy link address** (on links) · **Select all**
   - **Appearance ▸**
     - Theme mode ◉ (System / Light / Dark)
     - Color scheme ◉ (the 11 schemes as a radio list)
     - Body font… (`ChooseFontW` shown by the **dialog host**, family name only)
     - Font size ▸ (Bigger / Smaller / Reset)
     - Column width ▸ (Wider / Narrower / Reset)
     - ✓ Show outline `Ctrl+\`
     - Outline position ◉ (Left / Right)
     - ✓ Single newline as line break
     - ✓ Code line numbers
     - ✓ Code wrap
   - **Open in external editor** `F4`
   - **☆ Add to / ★ Remove from favorites** `Ctrl+D`
   - **File ▸** — Open… `Ctrl+O` · Reload `F5` · Show in Explorer · Copy file path
   - **Settings ▸**
     - Language ◉ (System / English / Türkçe)
     - Open external links ◉ (Ask / Direct / Never)
     - ✓ Restore reading position
     - Set YAMDV as default app…
     - ✓ "Open with YAMDV" in Explorer
     - Edit settings file
     - Open config folder
   - **Keyboard shortcuts** `F1` — opens a built-in help document, compiled into the exe as Markdown, in a new YAMDV window.
   - **About YAMDV** — in-window dialog with version, exe path and config path.

   Editing and Export entries are added in V1-B and V1-D; do not show them as disabled placeholders now.
3. **Sidebar** (B.5, minus FUT-02/03/33):
   - **Tabs:** Contents · History · Favorites.
   - **Contents:** TOC tree with title flattening, scroll spy, click → smooth scroll, per-node collapse, TOC font size from settings.
   - **History:** grouped Current session / Today / Yesterday / dates, remove per item, Clear per group, click opens in the current window.
   - **Favorites:** add/remove.
   - **Frame:** edge chevron toggle and `Ctrl+\`; drag-resize 180–480 px; position left or right.
   - All state persisted in `state.ini`.
4. **Themes:** mode + 11 schemes live across the whole UI (document, sidebar, menus, dialogs, toasts, scrollbars). `custom-theme.ini` is hot-reloaded (FUT-04 scope).
5. **Windows and instances:**
   - Single instance with argument forwarding (S-IPC-01…03); one window per file (re-activate if already open).
   - Window geometry remembered and validated against the current monitors.
   - System title bar `<file name> — YAMDV` (the custom title bar is FUT-30).
6. **Live reload** (R-READ-01): directory watcher thread; catches atomic-rename saves. Full re-parse is acceptable in v1 if the scroll anchor is kept and there is no flicker. A deleted file shows a toast and a dimmed document.
7. **Reading position memory** (R-READ-02).
8. **Find** `Ctrl+F` (B.9 without the replace row): incremental, highlight all, "n/m" counter, Enter / Shift+Enter, `Ctrl+G` / `Ctrl+Shift+G`, Match case toggle, Turkish i/İ/ı/I equivalence, Esc to close.
9. **Zoom:** font size via `Ctrl+wheel`, `Ctrl+=`/`-`/`0`, `Ctrl+↑/↓`; column width via `Alt+wheel`, `Alt+↑/↓`, `Alt+0`. Indicator toast. Persisted.
10. **External editor** `F4`/`Ctrl+E` (R-EXT, FUT-10 scope): VS Code, Notepad++, Notepad, or the custom command from `settings.ini`; jumps to the line; S-EXEC-03 rules.
11. **System integration:**
    - Per-user registration (R-SYS-01) and the set-default flow (R-SYS-02).
    - Explorer context-menu toggle (R-SYS-03).
    - CLI flags `--register`, `--unregister`, `--set-default`, `--context-menu on|off`, `--line <n>`.
    - Portable mode (R-SET-05).
    - Registration only on explicit action (S-REG-03).
12. **Welcome page** (simplified B.12): icon, title, drop-zone text, [Open file…] button, the last 8 recent files, and a default-app status line with [Set as default].
13. **Toast** component (B.10.1) if it does not exist yet.

**Driver scripts (CI):**
- switch scheme and mode through the menu
- `Ctrl+F` count and navigation
- second launch with a path → the first instance opens it
- live reload after an external write
- sidebar click → scroll
- favorites add/remove

**Goldens (≤ 6 new, P-28):** welcome light; document + sidebar light and dark; menu open; find bar; Glacier light.

**Measurements:** the existing table — M0, M1 (now with decoded images, plus image cache bytes), M2, M6, T1, T2, T4 warm/cold, T7, B-SIZE — plus **T3** (second launch → file visible).

**Out of scope:** editing, math, Mermaid, export, install/uninstall (V1-D), and every FUT item.

---

## 4. Acceptance criteria
| ID | Criterion |
|---|---|
| AC-1 | A-4, FUTURE.md, P-30/P-31 and TRACEABILITY deferrals done by script; `spec-check` green. |
| AC-2 | Items 1–13 work; driver scripts pass on CI. |
| AC-3 | All gates green, including memprobe and T4 ≤ 16 ms; PR merged; tag `phase-v1a`. |
| AC-4 | Report per A-4 (≤ 120 lines), with a 5-step owner QA list: install as the default app, double-click a `.md` file, live reload, theme switch, find. |

---

## 5. FUTURE.md (extract this block by script into `docs/FUTURE.md`)
````markdown
# Possible future developments (post-v1)

Owner decision 2026-10-17: v1 is a complete product; the items below are deferred. Each becomes a GitHub issue (label `future`, milestone `post-v1`) at the v1 release. "Spec IDs" lists the requirements (or parts of them) that are deferred.

| FUT | Deferred feature | v1 instead | Spec IDs |
|---|---|---|---|
| FUT-01 | Highlighting for languages beyond the 30 existing lexers | 30 lexers + aliases | R-CODE-01 |
| FUT-02 | Floating sidebar positions | left / right docked | R-NAV-02 |
| FUT-03 | Favorites drag-to-reorder | add / remove | R-NAV-09 |
| FUT-04 | In-app custom-color editor with picker | `custom-theme.ini` + hot reload | R-THEME-06 |
| FUT-05 | Shortcut presets and in-app rebinding UI | overrides in `settings.ini` | R-SET-06, F.6 |
| FUT-06 | Table fullscreen viewer | — | R-READ-08, R-REN-07 |
| FUT-08 | Heading jump palette; `[TOC]` placeholder | sidebar outline | R-READ-12, R-MD-19 |
| FUT-09 | "Reopen with encoding" menu | automatic detection | R-OPEN-06 |
| FUT-10 | Editor auto-detection beyond VS Code, Notepad++, Notepad | custom command | R-EXT-02 |
| FUT-12 | Duplicate/select/move line, auto-pairing | core Markdown keys | R-EDIT-07 |
| FUT-15 | HTML clipboard → Markdown on paste | plain-text paste | R-EDIT-05 |
| FUT-17 | Math beyond the core subset; macros | core subset (V1-C) | R-MATH-02, R-MATH-07 |
| FUT-18 | Mermaid gantt, pie and all other types; `%%{init}%%`; visual editing; Copy as PNG; extra shapes; `o`/`x` edges; `linkStyle` | flowchart + sequence (V1-C) | R-MMD-01…10, R-F2-11 |
| FUT-20 | SVG images; animated GIF | placeholder; first frame | R-IMG-01 |
| FUT-21 | Interactive HTML export (scripted features) | static HTML, no script | R-EXP-02, S-EXP-02 |
| FUT-23 | Copy document as rich HTML; PNG export of diagrams | — | R-EXP-06 |
| FUT-25 | Jump list; custom app/document icons | — | R-SYS-08, R-SYS-09 |
| FUT-26 | Settings search; F2 image resize handles | — | R-SET-02, R-F2-12 |
| FUT-27 | VS Code / Sublime live-preview plugins | — | A.0 |
| FUT-28 | macOS / Linux ports | Windows only | E.14 |
| FUT-29 | UI languages beyond English and Turkish | EN, TR | R-I18N-01 |
| FUT-30 | Custom-drawn title bar with ⚙/⋯ buttons and Snap-Layouts hit testing | system title bar (dark-mode aware) | B.4, R-WIN-05 |
| FUT-31 | Settings panel UI | settings in the context menu + "Edit settings file" | R-SET-01…04, B.8.1 |
| FUT-32 | Color-scheme cards with hover preview | radio list in the menu | R-THEME-03, B.7.3 |
| FUT-33 | Files tab in the sidebar | — | R-NAV-10 |
| FUT-34 | Heading folding (incl. Alt+click fold-all) | — | R-READ-06 |
| FUT-35 | Word-count badge | — | R-READ-07 |
| FUT-36 | Fullscreen viewers and ⛶ badges (image, table, diagram) | images/diagrams fit the column | R-IMG-05, R-READ-08, R-MMD-02 |
| FUT-37 | Mind-map view | — | R-MAP-01, R-MAP-02 |
| FUT-39 | Native PDF writer with selectable text, bookmarks, links | PDF via the print path to "Microsoft Print to PDF" | R-EXP-01 |
| FUT-40 | Tooltip, anchored popover and banner components (link titles, abbr, footnote preview, MotW/elevation/lossy banners) | in-window dialog for confirmations, toast for notices; MotW still forces *Ask* | B.10.4–B.10.6, R-REN-10 (tooltip), S-UI-03 |
| FUT-41 | Inline objects for sup/sub/footnote refs (baseline shift) | smaller text without shift | R-REN-03 |
| FUT-42 | Table row laziness; code-toolbar fade; DIB vs DDB measurement | — | R-REN-07, B.15, D.4 |
| FUT-43 | Obsidian-style inline marker reveal in F2; table cell editing in F2 | block-level source reveal | R-F2-02, R-F2-03 |
| FUT-44 | Table editing operations and toolbar | edit tables as source | R-TBL-01…08 |
| FUT-45 | Snippets; image/file paste into the document; recovery journal; find & replace; regex; F2 format toolbar | formatting shortcuts | R-EDIT-06, R-EDIT-08, R-EDIT-10, R-EDIT-13, R-F2-04, R-F3-07 |
| FUT-46 | F11 distraction-free fullscreen | — | R-WIN-06 |
| FUT-47 | High Contrast mode and full UI Automation | — | R-THEME-08, R-A11Y-03 |
| FUT-48 | ICO and WebP decoding under ACG | "unsupported format" placeholder | R-IMG-01 |
| FUT-49 | Code-font picker | body font via the system font dialog | R-THEME-04 |
| FUT-50 | Security hardening phase and optimization phase (final budgets) | soft budgets | Appendix C review, Appendix D final |

(FUT-07, -11, -13, -14, -16, -19, -22 and -24 were merged into FUT-34, -45, -45, -44, -45, -37, -39 and -47.)
````

**END OF PROMPT 005.** Start with §2 now.
