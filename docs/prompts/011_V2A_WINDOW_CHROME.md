# 011_V2A_WINDOW_CHROME — YAMDV Phase V2-A: Custom title bar, menus, scheme cards

> **Prompt 011.** This is the first phase of the **v2 plan**: turn v1.0.0 into the originally specified product, **UI first**, staying as fast and light as v1.
> - Branch `phase/v2a-chrome`, tag `phase-v2a`
> - Report `docs/reports/v2a_REPORT.md` (≤ 80 lines)
> - Store this file as received in `docs/prompts/`
>
> **Precedence:** `docs/PROCESS.md` (A-1…A-6 + the new A-7) > this prompt > `docs/UI_SPEC.md` / `docs/SPEC.md` > the rest of `docs/*`.

---

## 1. Review of V1-E
v1.0.0 is **released as a draft and accepted.** The owner tested it on a real PC: everything works and no bugs were found.

The owner's feedback is that the app looks too bare: there is no visible chrome at the top or at the bottom, and there is no settings UI. v2 therefore restores the original UI design first.

## 2. Setup (by script; keep it cheap)
1. **A-7.** Add A-7 to `docs/PROCESS.md` through the amendment mechanism:
```text
A-7 v2 process (owner decision, 2026-10-22) — applies from Phase V2-A on; where it differs from A-4/A-5, A-7 wins.
1. Goal: implement docs/ROADMAP_V2.md phase by phase — the originally specified product — while staying as fast and light as v1.
2. "Light" gates in every phase: M0, M1, T4 typical warm and T7 idle CPU are CI gates (two-attempt retry rule). Per phase, M0 and M1 may grow by at most 0.3 MiB versus the previous phase unless the prompt allows more. Report the exe-size delta. Idle CPU must stay at 0: timers run only while something animates.
3. Tests: unit tests for new pure logic; at most 3 new driver scripts and 4 new goldens per phase (P-28 total ≤ 12 MB still holds).
4. For every implemented item: update TRACEABILITY by script (deferred → DONE); close the GitHub issue through the merged PR ("Closes #n" in the PR body; find the numbers with `gh issue list --search "<ID> in:title"`).
5. Report ≤ 80 lines. One full CI run at the end, plus at most one fix push. If the quota runs low: commit with [skip ci], write the report with "Next action", stop.
6. Releases at the versions in docs/ROADMAP_V2.md, as drafts; the owner publishes them.
```
2. **Roadmap file.** Extract the embedded `docs/ROADMAP_V2.md` (end of this prompt) with the extraction command from prompt 009 step 3, pointed at this prompt file. Do not re-type it.
3. **Spec patches** (`docs/spec-patches/v2a.txt`, applied with `xtask spec-patch`):

**P-32** — `docs/ROADMAP.md`. INSERT AFTER the anchor line:
```text
> **Superseded on 2026-10-17 by the owner's v1 plan:** the phases after 003b in the table below are replaced by **V1-A** (app shell & daily use), **V1-B** (editing: F3 split + block-level F2), **V1-C** (math core + Mermaid flowchart/sequence), **V1-D** (HTML export, print/PDF, packaging, v1 release) and **V1-FIX** (reserve). Security hardening (old 012) and optimization (old 013) follow after v1. Deferred features: `docs/FUTURE.md`.
```
NEW line:
```text
> **v2 plan (2026-10-22):** see `docs/ROADMAP_V2.md` (phases V2-A…V2-J).
```

**P-33** — `docs/SPEC.md`, §A.25. OLD:
```text
`docs/TRACEABILITY.md` marks it `deferred (FUT-xx)`.
```
NEW:
```text
`docs/TRACEABILITY.md` marks it `deferred (FUT-xx)`. From v2 on, `docs/ROADMAP_V2.md` schedules these items; an item is back in scope from the phase that implements it.
```

---
## 3. V2-A tasks
Specifications: `docs/UI_SPEC.md` §B.4, §B.7, §B.10.2, §B.10.3, §B.15 and §B.16. GitHub issues: FUT-30, FUT-32, FUT-35, FUT-46, BUG-09.

1. **Custom title bar** (B.4, R-WIN-05; FUT-30)
   - **Layout:**
     - `WM_NCCALCSIZE` extends the client area over the caption: a 32 px bar in `chrome_bg`, seamless with the page.
     - When the document is scrolled, a 1 px `border_subtle` line appears under the bar.
     - **Left:** icon, then the title (`general.title_format`: file name or full path, middle-ellipsized), then the mode pill (Live Edit / Split Edit / Read-only), then the dirty `●`.
     - **Right:** ⚙ and ⋯ (32 × 32), then Minimize / Maximize-Restore / Close (46 × 32). Glyphs come from Segoe Fluent Icons, then Segoe MDL2 Assets, then a vector fallback. Hover and pressed overlays per B.3.5; Close hover is `#c42b1c` with a white glyph.
   - **Behavior:**
     - drag to move; double-click toggles maximize; dragging a maximized window restores it under the cursor;
     - right-click or `Alt+Space` opens the system menu;
     - `WM_NCHITTEST` returns `HTMAXBUTTON` over Maximize, so **Windows 11 Snap Layouts** appear;
     - 6 px resize borders; DWM shadow and Windows 11 rounded corners;
     - inactive window: the title at 55 % opacity;
     - correct per-monitor DPI;
     - live theme and scheme switching.
   - **Buttons and keys:**
     - **⋯** opens the full context menu anchored under the button.
     - **⚙** opens the *Settings* submenu as a popup under the button. V2-B replaces this with the settings panel; `Ctrl+,` does the same.
     - `Alt` alone or `F10` opens the ⋯ menu (the Windows menu-bar convention).
2. **Layered popup menu component** (B.7.1; BUG-09)
   - Replace `TrackPopupMenuEx` owner-draw with our own layered popup windows, built from the same menu model and actions.
   - **Look:**
     - radius 8 (0 with `corner_style = sharp`), 1 px `menu_border`, shadow;
     - 80 ms fade (instant with reduced motion);
     - 32 px items with icon column, shortcut column, check/radio marks and separators.
   - **Keyboard and mouse:**
     - submenus open after 150 ms of hover or with `→`/`Enter`; `←`/`Esc` close one level;
     - `↑`/`↓`/`Home`/`End` move; typing a letter jumps to the next item starting with it;
     - the menu flips to stay on the current monitor;
     - it closes on an outside click or on window deactivation.
   - Used everywhere: right-click, `Shift+F10` / Menu key, ⋯ and ⚙.
3. **Color-scheme cards** (B.7.3; FUT-32)
   - *Appearance ▸ Color scheme…* opens the card flyout: 11 cards in 3 columns, each 112 × 84, with a preview drawn from that scheme's tokens and its name.
   - **Hover** = live preview of the whole window. **Click** = apply and persist. **Esc** or leaving the flyout = revert.
   - The selected card has a 2 px accent ring and a check badge.
   - This replaces the radio list.
4. **F11 distraction-free fullscreen** (B.4, R-WIN-06; FUT-46)
   - Hides the title bar and sidebar and fills the monitor.
   - With the mouse in the top 4 px, the title bar shows temporarily.
   - `F11` or `Esc` exits.
5. **Word-count badge** (B.10.3, R-READ-07; FUT-35)
   - A bottom-right pill: words and characters, plus the selection count; CJK ideographs count as one word each.
   - Toggled with *Appearance ▸ ✓ Word count* (`reading.word_count_badge`, default off per spec).
   - The detailed tooltip comes with the tooltip component in V2-C.
6. **Zoom indicator** (B.10.2): a 140 × 48 pill in the window center, shown for 900 ms. It replaces the zoom toast.

**Tests (A-7):**
- Driver scripts (3):
  - `titlebar`: ⋯ opens the menu with the full item count; ⚙ popup; maximize and restore by button and by double-click.
  - `scheme-cards`: hover preview → `Esc` reverts → click applies and persists across a restart.
  - `fullscreen-wordcount`: `F11` in and out; badge toggled on and off.
- Goldens (4): window top light, window top dark, menu open (now capturable, since it is our own window), scheme cards.

**Gates (A-7):** M0, M1, T4 typical warm, T7. Growth vs v1.0.0 at most +0.3 MiB on each of M0 (2.30) and M1 (4.71). Report the exe-size delta.

**Out of scope:** the settings panel (V2-B), tooltips/popovers/banners (V2-C), and everything else in `docs/ROADMAP_V2.md`.

---

## 4. Acceptance criteria
| ID | Criterion |
|---|---|
| AC-1 | A-7, `docs/ROADMAP_V2.md`, P-32 and P-33 done; `spec-check` green. |
| AC-2 | Items 1–6 work as in UI_SPEC; the 3 driver scripts pass on CI; the 4 goldens are committed and self-reviewed. |
| AC-3 | Gates green within the growth limits; PR merged (closing FUT-30, FUT-32, FUT-35, FUT-46, BUG-09); tag `phase-v2a`. |
| AC-4 | Report ≤ 80 lines with a 5-step owner QA list: title-bar buttons and Snap Layouts, title bar in dark mode, ⋯ menu with mouse and keyboard, scheme cards, F11. |

**END OF PROMPT 011.** Start with §2 now.

---

## Embedded file (extracted in §2 step 2 — do not re-type it)

<!-- FILE: docs/ROADMAP_V2.md -->
````markdown
# YAMDV — Roadmap v2 (owner plan, 2026-10-22)

**Goal:** turn v1.0.0 into the product originally specified in `docs/SPEC.md` and `docs/UI_SPEC.md` (before the v1 scope cut), phase by phase, while staying as fast and light as v1. **The order is UI first**, then reading/editing features, then content engines and export, then hardening and performance.

Each phase implements the listed items. Their requirement IDs come back into scope, `docs/TRACEABILITY.md` moves them from `deferred (FUT-xx)` to DONE, and the merged PR closes the matching GitHub issues (milestone `post-v1`).

| Phase | Prompt | Content | Items (issue IDs) | Release |
|---|---|---|---|---|
| V2-A | 011 | Window chrome: custom title bar with ⚙ and ⋯ buttons, layered popup menus (B.7), color-scheme cards, F11 fullscreen, word-count badge, zoom indicator | FUT-30, FUT-32, FUT-35, FUT-46, BUG-09 | — |
| V2-B | 012 | Settings panel (B.8, all sections), custom-color editor, code-font picker, shortcut list + key capture + Typora preset, settings search | FUT-31, FUT-04, FUT-49, FUT-05, FUT-26 | v1.1.0 |
| V2-C | 013 | Tooltips, anchored popovers, banners; fullscreen viewers (image, table, diagram) with ⛶ badges; heading folding; inline objects for sup/sub/footnote refs; wide diagrams scroll instead of shrinking | FUT-40, FUT-36, FUT-06, FUT-34, FUT-41, BUG-08 | — |
| V2-D | 014 | Navigation: Files tab, floating sidebar, favorites reorder, heading jump palette + `[TOC]`, mind-map view; keyboard reachability of every control | FUT-33, FUT-02, FUT-03, FUT-08, FUT-37, HARD-04 | v1.2.0 |
| V2-E | 015 | Editing plus: F2 format toolbar, find & replace (+ regex, mirrored in F3), snippets, image/file paste, recovery journal, editor extras, HTML paste → Markdown | FUT-45, FUT-12, FUT-15 | — |
| V2-F | 016 | Tables and full WYSIWYG: table toolbar and operations, drag handles, TSV paste, F2 inline marker reveal, F2 table cell editing | FUT-44, FUT-43 | v1.3.0 |
| V2-G | 017 | Content engines: math beyond the core + macros; Mermaid gantt, pie, state, class, ER + `%%{init}%%` + Copy as PNG; SVG images; animated GIF; ICO/WebP; more highlight languages | FUT-17, FUT-18, FUT-20, FUT-48, FUT-01 | — |
| V2-H | 018 | Export and integration: native PDF writer (vector text, bookmarks, links), interactive HTML export (hash CSP), copy rich HTML / PNG, more external editors, reopen with encoding, jump list, custom icons; known bugs | FUT-39, FUT-21, FUT-23, FUT-10, FUT-09, FUT-25, BUG-01…07 | v1.4.0 |
| V2-I | 019 | Hardening: full security review + THREAT_MODEL, long fuzzing, ACG re-verification, High Contrast + UI Automation, real-hardware QA | HARD-01…03, FUT-47, QA-01 | — |
| V2-J | 020 | Performance and 2.0: final memory budgets (Appendix D final column), dialog latency, scroll paths, Mermaid rasters, binary size, incremental live reload, stable-hardware perf gates, DIB/DDB | PERF-01…07, FUT-42 | v2.0.0 |
| Later | — | Not planned for v2: editor plugins (FUT-27), macOS/Linux (FUT-28), more UI languages (FUT-29), code signing / winget / ARM64 / spell check (IDEA-01…04), license (DEC-01, owner decision) | — | — |

The order may be re-cut by the reviewer from phase reports (a phase that does not finish continues in the next prompt, per A-3 rule 9).
````
