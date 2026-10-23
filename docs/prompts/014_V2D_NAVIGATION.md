# 014_V2D_NAVIGATION — YAMDV Phase V2-D: Files tab, floating sidebar, heading palette, mind map, keyboard reach → v1.2.0

> **Prompt 014.** Phase **V2-D** (`docs/ROADMAP_V2.md`).
> - Branch `phase/v2d-navigation`, tags `phase-v2d` and **`v1.2.0`**
> - Report `docs/reports/v2d_REPORT.md` (≤ 80 lines)
> - Store this file as received in `docs/prompts/`
>
> **Precedence:** `docs/PROCESS.md` (A-1…A-9) > this prompt > `docs/UI_SPEC.md` / `docs/SPEC.md` > the rest of `docs/*`.
> **Order:** §1 → §2 → §3 → §4. **Resumable:** if the quota runs low, follow §6.

---

## 1. Review of V2-C — accepted
V2-C is **accepted**:
- **CI:** fix run 39919219757 is green; 35/35 driver scripts pass.
- **Gates:** M0 2.36 MiB, M1 4.61 MiB (−0.23), T7 0. The exe grew by 68 KB.
- **v1.1.0:** the draft release exists; the owner publishes it.

**Accepted deviations:**
- the tooltip is drawn inside the main window;
- sup/sub are always synthesized;
- the conflict notice is now a banner.

### 1.1 Fix first
| ID | Fix |
|---|---|
| **F-23** | **Footnote tooltip shows markup.** The preview must be the footnote's *rendered plain text*: no emphasis markers, link text without the URL, code-span text without backticks, math as its LaTeX source. Add a unit test, and update golden `top-v2c-footnote-light` in this phase's run. |
| **F-24** | **Tagging on `[skip ci]` merges.** A version or phase tag may go on a squash-merge commit only when its tree equals a PR head with a green CI run (`git diff --quiet <pr-head> <merge>`). Otherwise run CI on `main` first. Add this as one line to A-9 in `docs/PROCESS.md`. |

### 1.2 Owner notes (only if attached)
If the owner pasted notes from his checks below this prompt, each one becomes an item F-25, F-26, …, fixed right after §1.1.

---

## 2. Setup (by script; keep it cheap)
**Spec patches** (`docs/spec-patches/v2d.txt`, applied with `xtask spec-patch`):

**P-43** — `docs/SECURITY.md`, S-FILE-08. OLD:
```text
File watching covers only the open document's folder (non-recursive) and the config folder; buffer overflow → full reload of the document.
```
NEW:
```text
File watching covers only the open document's folder (non-recursive), the config folder, and the folders currently expanded in the Files tab (each non-recursive, at most 32; a collapsed folder stops being watched); buffer overflow → full reload of the document, or a re-list of that folder.
```

**P-44** — `docs/UI_SPEC.md`, B.10.7. INSERT AFTER the anchor line:
```text
Bottom-center, above the toast stack; width 360, radius 8, `menu_bg`, `shadow`; shown only when a job runs longer than 300 ms. Line 1: 13 px 600 "Exporting PDF…" / "Printing…" / "Exporting HTML…" plus the file name (middle ellipsis). Bar: 4 px, `accent` on a `border` track; indeterminate while preparing (static with reduced motion), determinate after. Line 2: 12 px `text_muted` "Page 37 of 1093 · 3 % · about 3 min left" (HTML counts blocks; the estimate appears after 5 pages or 5 %). [Cancel] right-aligned; `Esc` cancels while the card has focus. Success replaces it with a toast "PDF saved — <name>" with [Open] · [Show in folder]; failure replaces it with an error banner (B.10.5). The taskbar button mirrors the progress (`ITaskbarList3`).
```
NEW lines:
```text
### B.10.8 Heading jump palette
Top-center, 12 px below the title bar; width 520 (clamped to window − 48), radius 8, `menu_bg`, `shadow`. A single-line input (placeholder "Go to heading…") above a list of at most 12 visible rows (32 px, 13 px text, indent 12 px per level, H1 weight 600, matched characters in `accent`). `↑/↓` move, `Enter` jumps (unfolding a folded section), `Esc` or an outside click closes. Filter: substring and word-start matches with the find bar's case folding (Turkish equivalence).
```

**P-45** — `docs/SHORTCUTS.md`. INSERT AFTER the anchor line:
```text
| `toggle_sidebar` | `Ctrl+\` | all |
```
NEW line:
```text
| `focus_next_region` / `focus_prev_region` | `F6` / `Shift+F6` | all |
```

---

## 3. V2-D tasks
Specifications:
- `docs/SPEC.md` R-NAV-01…03, R-NAV-09…11, R-OPEN-04, R-READ-12, R-MD-19, R-MAP-01/02, R-A11Y-01
- `docs/UI_SPEC.md` §B.5, §B.10.8, §B.11 (mind map), §B.16
- `docs/SECURITY.md` S-FILE-08 (P-43), S-RES-02

Issues: FUT-33, FUT-02, FUT-03, FUT-08, FUT-37, HARD-04.

1. **Files tab** (R-NAV-10, R-NAV-11; FUT-33)
   - **Tree:** a lazy explorer tree of the Markdown-family files under the current document's folder.
     - Folders come first, sorted naturally (`file2` < `file10`), with Turkish-aware case folding. The current file is shown in `accent`.
     - Each folder is enumerated **only when it expands**, on a worker thread, never on the UI thread.
     - Limits: 5 000 entries and depth 8. When a limit is reached, show a muted "…more" row.
   - **Skipped:** hidden and system items; `.git`, `node_modules`, `target`, `.venv` and `__pycache__`; and every reparse point (junctions, symlinks), which also prevents loops.
   - **Network:** the tree never crosses into a network location (S-RES-02 rules), except within the document's own share.
   - **Opening a file:** a click or `Enter` switches the document **in place without flicker**, through the existing open-in-place path (history, scroll restore).
   - **Watching:** the root folder and the expanded folders are watched per P-43; changes update the tree within 1 s. Watching uses the existing watcher thread, and idle CPU stays 0.
   - **Keyboard** (R-NAV-11): `↑/↓`, `Enter`, `←/→` (collapse/expand), `Home/End`, and `Ctrl+Shift+1…4` for the tabs.
   - **Dropped folder** (R-OPEN-04 SHOULD): dropping a folder on the window opens the Files tab rooted at that folder.
   - **Settings:** `sidebar.show_files` in the settings panel. If History, Favorites and Files are all hidden, the tab row disappears (R-NAV-01).
2. **Floating sidebar** (R-NAV-02, B.5.1; FUT-02)
   - **Positions:** `sidebar.position` gains `left-floating` and `right-floating`.
   - **Card:** a floating sidebar is a card that **overlays** the page: 12 px from the window edges, 8 px below the title bar, radius 12, `shadow`, 1 px `chrome_border`.
     - The page layout keeps its full width, so opening it causes no reflow.
   - **Controls:** the edge chevron and `Ctrl+\` still toggle it (R-NAV-03). Resizing works as when docked.
   - **Settings:** add the position control to the panel's Sidebar section, plus a quick toggle under *Appearance ▸ Sidebar position*.
3. **Favorites reorder** (R-NAV-09; FUT-03)
   - **Drag:** drag a row to reorder; a 2 px `accent` line shows the drop point.
   - **Keyboard:** while a Favorites row has focus, `Alt+↑/↓` moves it. That context wins over `column_wider` / `column_narrower`.
   - **Saving:** the order persists atomically and survives a restart.
4. **Heading jump palette** (R-READ-12, B.10.8 per P-44; FUT-08)
   - **Opening:** `Ctrl+Shift+O`, and *Go to heading…* in the ⋯ menu.
   - **Built from:** the V2-B single-line input and the V2-C popover base.
   - **Content:** every heading of the document. The filter is pure code, covered by unit tests.
   - **Memory:** the palette is built on demand and freed on close.
5. **`[TOC]` placeholder** (R-MD-19; FUT-08)
   - **Trigger:** a paragraph that is exactly `[TOC]` or `[[_TOC_]]`.
   - **Rendering:** an inline table of contents titled "Contents" (localized). It is a nested list of heading links, down to `sidebar.toc_max_depth`, and a click jumps to the heading.
   - **Updates:** the list follows heading edits (F2 / F3, live reload).
   - **F2:** the source stays `[TOC]`.
   - **Exports:** HTML has real anchor links; PDF and Print show the list.
6. **Mind-map view** (R-MAP-01/02, B.11; FUT-37)
   - **Opening:** `Ctrl+Shift+M` and *Export ▸ View as mind map*.
   - **Overlay:** the B.11 viewer frame with an opaque `bg`, the same toolbar plus [Right | Balanced].
   - **Tree:**
     - The root is the first H1, or the file name.
     - Headings become branches by level, and list items become leaves, truncated to 80 characters.
   - **Layout:** a pure, deterministic O(n) tidy-tree layout (right-expanding and balanced) in a platform-independent crate, with unit tests: no overlaps, stable output.
   - **Connectors:** cubic curves.
   - **Branch colors:** six hues derived from the scheme `accent` by 40° hue steps, at the accent's lightness, adjusted per mode (theme-aware).
   - **Interactions:**
     - wheel zoom at the cursor, and drag to pan;
     - a click collapses or expands a node;
     - a double-click closes the map and jumps to that heading;
     - `Esc` closes it;
     - [copy] copies a PNG of the whole map at 1×, capped at 8 192 px per side.
   - **Big documents:**
     - Above 2 000 nodes, the list-item leaves start collapsed under their heading and show a count badge.
     - Only the visible nodes are drawn.
   - **Memory:** the map is built on demand and freed on close.
7. **Keyboard reachability** (R-A11Y-01, B.16; HARD-04)
   - **Tab order:** every interactive element joins the `Tab` order in reading order, with the 2 px focus ring (shown after keyboard use only):
     - links
     - code-toolbar buttons (Copy, Line numbers, Wrap)
     - task boxes (when toggling is enabled)
     - callout fold chevrons
     - the front-matter card toggle
     - `<details>` summaries
     - heading fold toggles
     - ⛶ badges and the table pill
     - banner buttons and toast actions
   - **Activation:** `Enter` / `Space` activate the focused element.
   - **Regions:** `F6` / `Shift+F6` (P-45) cycle the focus through title bar → banners → sidebar → document. The sidebar tabs and rows are reachable with `Tab` and the arrow keys.
   - **Off-screen focus:** a focused element that is off screen scrolls into view.
   - **Data:** the focus-order builder is pure code, covered by unit tests.
   - **Out of scope:** UI Automation and High Contrast stay with FUT-47 (V2-I).
8. **Release v1.2.0**
   - Set version 1.2.0 in the workspace, the version resource and About.
   - Add a `CHANGELOG.md` entry for V2-C and V2-D.
   - After the merge, tag `v1.2.0` per F-24. The tag job creates the **draft** release (zip + SHA-256).

**Out of scope:** everything in V2-E and later (F2 toolbar, find & replace, snippets, paste, recovery, …).

**Tests (A-7/A-9):**
- **Driver scripts (3):**
  - `files-favorites`. The script builds a temp folder tree with subfolders, `file2.md` / `file10.md`, a `node_modules` folder, a hidden folder and a junction made with `mklink /J`. Then:
    - the tree order and the skipped items are correct;
    - `↓ → Enter` opens a file in place (same window, new title);
    - a new `.md` in an expanded folder appears within 1 s;
    - 3 favorites, with the third dragged to the top, keep that order after a restart;
    - `Alt+↑` on a focused favorite moves it.
  - `palette-mindmap-toc`:
    - `Ctrl+Shift+O`, type "inst", `Enter`: the view scrolls to "Installation" and unfolds it if it was folded;
    - a `[TOC]` fixture renders the expected number of links, and clicking one jumps;
    - `Ctrl+Shift+M` shows a node count of headings + list items;
    - double-clicking a node closes the map and jumps;
    - after closing, PWS is back within +0.2 MiB, and T7 is 0 while the map is open and idle.
  - `keyboard-reach`, on the fixture `interactive.md` with every element type from item 7:
    - `Tab` visits them in reading order with a focus ring;
    - `Enter` on Copy fills the clipboard, and on a `<details>` it opens it;
    - `F6` reaches the sidebar, and `Ctrl+Shift+2` shows History;
    - `left-floating` leaves the page width unchanged, and the card sits 12 px from the edges.
- **Goldens (4):**
  - the Files tab (light)
  - the floating sidebar (dark)
  - the heading palette (light)
  - the mind map, balanced (light)
- **Unit tests:**
  - natural sort and skip rules
  - the heading filter with Turkish folding
  - the mind-map layout
  - `[TOC]` detection
  - the focus order
  - F-23 plain text

**Gates (A-7/A-9):**
- M0 and M1 at most +0.3 MiB vs V2-C (2.36 / 4.61 MiB).
- T4 typical warm.
- T7, also with the Files tab visible and with the mind map open and idle.
- Report the exe-size delta. Report-only: mind-map-open PWS on `large-1mb.md`, and its build time.

---

## 4. Merge and report
- **PR and CI:** one CI run at the end, plus at most one fix push.
- **Issues:** the PR body closes FUT-33, FUT-02, FUT-03, FUT-08, FUT-37 and HARD-04 with "Closes #n".
- **After CI is green:** merge, then tag `phase-v2d` and `v1.2.0` per F-24.
- **Report (≤ 80 lines):**
  - the AC table
  - the gates
  - the exe delta
  - deviations
  - the `v1.2.0` draft link and its SHA-256
  - the owner check list (≤ 8 steps, EN + TR), which must cover:
    - the Files tab with a real project folder;
    - dropping a folder;
    - the floating sidebar;
    - reordering favorites;
    - `Ctrl+Shift+O`;
    - `[TOC]`;
    - the mind map;
    - Tab / F6 through a document.

## 5. Acceptance criteria
| ID | Criterion |
|---|---|
| AC-1 | F-23 and F-24 are done (and §1.2 if notes were attached); P-43…P-45 are applied, with `spec-check` green. |
| AC-2 | §3 items 1–7 work per the listed specs; TRACEABILITY is updated by script; S-FILE-08 holds as patched. |
| AC-3 | The 3 driver scripts pass on CI; the 4 goldens are committed and self-reviewed; the unit tests pass; the PR artifact `yamdv-exe` exists. |
| AC-4 | The gates are green within the growth limits; the PR is merged with the issues closed; `phase-v2d` and `v1.2.0` are tagged per F-24; the draft release `v1.2.0` has the zip + SHA-256; the report follows §4. |

## 6. Stop rule (quota)
If the Grok quota or the Actions minutes run low at any point:
1. commit and push with `[skip ci]`;
2. write `docs/reports/v2d_REPORT.md` with a done / partial / not-started table for §1–§4 and **"Next action: …"**;
3. stop.

The owner will give this same prompt again.

**END OF PROMPT 014.** Start with §1.1 now.
