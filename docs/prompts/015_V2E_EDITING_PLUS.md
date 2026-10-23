# 015_V2E_EDITING_PLUS — YAMDV Phase V2-E: F2 toolbar, find & replace, snippets, paste, recovery

> **Prompt 015.** Phase **V2-E** (`docs/ROADMAP_V2.md`).
> - Branch `phase/v2e-editing`, tag `phase-v2e` (no release in this phase)
> - Report `docs/reports/v2e_REPORT.md` (≤ 80 lines)
> - Store this file as received in `docs/prompts/`
>
> **Precedence:** `docs/PROCESS.md` (A-1…A-10) > this prompt > `docs/UI_SPEC.md` / `docs/SPEC.md` > the rest of `docs/*`.
> **Order:** §1 → §2 → §3 → §4. **Resumable:** if the quota runs low, follow §6.

---

## 1. Review of V2-D — accepted
V2-D is **accepted**:
- **CI:** run 39987570191 is green; 37 driver scripts pass.
- **Gates:** M0 2.36 MiB, M1 4.72 MiB (+0.11), T4 7.27 ms, T7 0. The exe grew by 95 KB.
- **A-10:** the owner's own decision (fewer tests, merge anyway) is accepted and applies to this phase.

### 1.1 Fix first
| ID | Fix |
|---|---|
| **F-25** | **Release tags.** If the `v1.2.0` draft release does not exist yet, do this first: on `main`, make an empty commit `chore(release): v1.2.0` without `[skip ci]`, move `v1.2.0` and `phase-v2d` to it, and check the draft (zip + SHA-256). Then add this line to A-9 in `docs/PROCESS.md`: "A version tag always goes on a `chore(release): vX.Y.Z` commit without [skip ci] whose tree equals a green CI head; phase tags may stay on the squash merge." |
| **F-26** | **One watcher thread.** The Files tab uses one thread per expanded folder, which breaks ARCHITECTURE `watch` (a single watcher thread with overlapped I/O). Move every watch onto the existing watcher thread: overlapped `ReadDirectoryChangesW` on up to 32 + 2 handles, with clean cancellation. Report the process thread count with 10 expanded folders: it must be the same as with 0. |
| **F-27** | **Rebindable keys.** `Ctrl+Shift+O`, `Ctrl+Shift+M`, `Ctrl+Shift+1…4` and `F6` / `Shift+F6` are hard-wired. Make them keymap actions: rebindable, listed in the Shortcuts section, and covered by the presets. |
| **F-28** | **Toast actions.** The B.10.7 success toast must offer [Open] · [Show in folder] as B.10.1 action links. The V2-D report says toasts have no action buttons, so check and add them where missing. The actions join the Tab order (HARD-04). |

### 1.2 Owner notes (only if attached)
If the owner pasted notes from his checks below this prompt, each one becomes an item F-29, F-30, …, fixed right after §1.1.

---

## 2. Setup
**Spec patch** (`docs/spec-patches/v2e.txt`, applied with `xtask spec-patch`):

**P-46** — `docs/SPEC.md`, R-EDIT-10. OLD:
```text
Find & Replace `Ctrl+H` (replace, replace all — one undo step); case / whole word.
```
NEW:
```text
Find & Replace `Ctrl+H` (replace, replace all — one undo step); case / whole word; in read mode `Ctrl+H` first switches to Split Edit (F3), because replacing edits the source.
```

**Regex ADR.** Write a new ADR before using regex:
- **Choice:** `regex-lite` or `regex` with minimal features.
- **Rules:** the engine must be linear-time (no backtracking), and it may add at most **+150 KB** to the exe; prefer Unicode case-insensitive matching.
- **Case folding:** the Turkish `i/İ/ı/I` equivalence applies to plain (non-regex) search only; document this.
- **Fallback:** if no engine meets the budget, ship without regex and record the decision in the ADR.

---

## 3. V2-E tasks
Specifications:
- `docs/SPEC.md` R-EDIT-05…08, R-EDIT-10, R-EDIT-13, R-F2-04, R-F3-07
- `docs/UI_SPEC.md` §B.9 (replace row), §B.10.5 (recovery banner), §B.13.2 (format toolbar), B.8.2 `editing.*`
- `docs/SECURITY.md` S-FILE-01, S-FILE-05…07, S-HTML-01…03, S-RES-03/04

Issues: FUT-45, FUT-12, FUT-15.

1. **F2 format toolbar** (R-F2-04, B.13.2)
   - **When it shows:** above a non-empty selection, 8 px above it (below if there is no room). It appears when the selection settles (mouse up or keyboard), hides while typing, and `Esc` dismisses it.
   - **Look:** height 36, radius 8, `menu_bg`, `shadow`, 28 × 28 buttons in groups with `separator`s, and V2-C tooltips on every button.
   - **Buttons:** Bold · Italic · Strikethrough · Highlight · Inline code · Link | H1 · H2 · H3 | Quote · Bullet · Numbered · Task | Clear formatting.
   - **Actions:** each button calls the same command as its shortcut (R-EDIT-07); *Clear formatting* removes inline markers inside the selection.
   - **Setting:** `editing.format_toolbar`. The toolbar is drawn in-window, like the V2-C tooltip, and built on demand.
2. **Find & replace** (R-EDIT-10, R-F3-07, B.9, P-46)
   - **Replace row:** in edit modes, `Ctrl+H` opens the find bar with the replace row ([▸] toggles it): input · [Replace] · [Replace all].
   - **Undo:** *Replace all* is **one undo step**.
   - **Options:** case and whole word, and Turkish folding as the find bar already does. Regex is a `[.*]` toggle, only if the ADR allows it, and supports `$1` / `${name}` groups in the replacement.
   - **F3 mirroring:** matches in the editor pane are mirrored in the preview (R-F3-07).
   - **Engine:** the search/replace engine is pure code with unit tests. A regex that runs longer than 100 ms is cancelled with a message (time guard).
3. **Tab snippets** (R-EDIT-06; setting `editing.snippets`)
   - **Triggers:** every trigger in R-EDIT-06:
     - `h1`…`h6`, `table`, `tableRxC`
     - `code` and the known language names
     - `link`, `img`, `line` / `hr`, `quote`, `task`
     - the callout names (`red` = caution)
     - `math`, `mermaid`, `flow`, `seq`, `gantt`
     - `date`, `time`, `details`, `footnote`
   - **Expanding:** a trigger expands only when it is the whole word before the caret and `Tab` is pressed.
   - **Placeholders:** the caret goes to the first placeholder, `Tab` jumps to the next one, and `Esc` leaves snippet mode.
   - **Undo:** one expansion is one undo step.
   - **Where:** F2 and F3.
   - **Data:** the snippet table and the placeholder logic are pure code with unit tests.
4. **Editor extras** (R-EDIT-07; FUT-12)
   - **Commands:** `Ctrl+Shift+D` duplicates the line or selection, `Ctrl+L` selects the line (repeat to extend), and `Alt+↑/↓` moves the line or block in edit modes. In read mode `Alt+↑/↓` keeps its column-width meaning, as SHORTCUTS says.
   - **Auto-pairing** (`editing.auto_pair`, default off):
     - pairs: round, square and curly brackets; double and single quotes; the backtick; `**`, `__` and `==`;
     - over-typing a closing character skips it;
     - `Backspace` on an empty pair deletes both;
     - nothing is paired inside code spans or code blocks, except brackets.
   - Implement every other R-EDIT-07 key that is still missing; list them in the report.
5. **Paste and drop** (R-EDIT-05, R-EDIT-08; FUT-15)
   - **HTML paste:** `Ctrl+V` with `CF_HTML` (from browsers) inserts **Markdown**.
     - **Conversion:**
       - It runs through the existing bounded tokenizer and allowlist (S-HTML-01…03), then converts the result.
       - Supported: headings, emphasis, strike, code / pre, links, lists (nested), blockquotes, tables (GFM), `<br>`, and images.
       - Dropped: script, style and the other S-HTML-01 containers, together with their content.
     - **Links:** `href` values keep only safe schemes (S-LINK). Remote image URLs stay as Markdown images; they are never fetched (S-RES-03).
     - **`data:` images:** saved as assets through item 5's image path, within the S-RES-04 limits; otherwise they become their alt text.
     - **Size:** HTML over 2 MB falls back to plain text.
     - `Ctrl+Shift+V` always pastes plain text.
   - **Image paste:** a bitmap on the clipboard (`CF_DIBV5` / `CF_DIB` / PNG) is encoded to PNG with WIC, saved to `<doc folder>/<assets_dir>/<doc stem>/image-YYYYMMDD-HHMMSS[-n].png`, and inserted as `![](assets/<stem>/<name>)`.
     - The asset path follows S-FILE-05: sandbox check after `GetFinalPathNameByHandleW`, sanitized names, `CREATE_NEW`, never overwriting.
   - **File drop in edit modes:** the file is copied to the same folder and inserted as `[name](…)`, with forward slashes and `%20`. A file over 50 MB asks first.
   - **Untitled buffer:** if the document has never been saved, ask to save first.
   - **Read mode:** drops still open files (R-OPEN-04).
   - **Unit tests:**
     - the HTML → Markdown converter, with ≥ 30 cases plus the XSS vectors as a new S-TEST-02 context;
     - asset naming and the sandbox, including a junction escape.
6. **Recovery journal** (R-EDIT-13; B.10.5)
   - **When it runs:** only when autosave is off or a save failed. It journals the unsaved buffer to `%LOCALAPPDATA%\YAMDV\recovery\` (the portable config folder in portable mode).
   - **Writing:**
     - a one-shot timer 2 s after the last edit, at most once every 10 s;
     - the write is atomic and happens on a worker thread;
     - no timer runs when the buffer is clean.
   - **File format:** a header (document path, base file mtime + hash, encoding) followed by the UTF-8 text.
     - Buffers over 32 MB are not journaled (one toast).
     - Restore parses defensively (S-FILE-07).
   - **Deletion:** a journal entry is deleted after a successful save, after *Discard*, or when the document is closed without changes.
   - **Next launch:** show a banner "Unsaved changes from an earlier session: <name>" with [Restore] · [Discard].
     - *Restore* opens the document with the journaled text as a **dirty** buffer; it never saves on its own.
     - If the base file changed since then, the existing conflict banner shows as well.
   - **Data:** the journal format is pure code with unit tests, including a round trip and a corrupt file.

**Out of scope:** table editing and full F2 WYSIWYG (V2-F); everything later.

**Tests (A-7/A-9 as reduced by A-10):**
- **Driver scripts (2):**
  - `edit-plus`, in F3:
    - `table2x3` + `Tab` inserts the table with the caret in the first cell, and `Tab` moves to the next cell;
    - `Ctrl+Shift+D`, `Alt+↑` and `Ctrl+L` work;
    - with `auto_pair` on, typing `(` gives `()`;
    - `Ctrl+H` *Replace all* foo→bar, then one `Ctrl+Z` restores everything;
    - if regex is shipped, `\d+` → `#`;
    - then in F2: select a word → the toolbar shows → Bold gives `**word**` → `Esc` hides the toolbar.
  - `paste-recovery`:
    - with a `CF_HTML` clipboard (h2, bold, link, list, table, `<script>`), `Ctrl+V` inserts the expected Markdown, with no script text;
    - `Ctrl+Shift+V` pastes plain text;
    - a bitmap on the clipboard gives an asset PNG and the image link;
    - with autosave off: edit → `TerminateProcess` → restart → the recovery banner → *Restore* brings the text back as dirty;
    - *Discard* deletes the journal file.
- **Goldens (2):** the F2 toolbar over a selection (light); F3 with the find bar's replace row (dark).
- **Unit tests:**
  - snippets
  - the find / replace engine (one undo group)
  - the regex guard: `(a+)+$` on 100 KB finishes in under 50 ms
  - line operations
  - auto-pair rules
  - the HTML converter
  - assets
  - the journal

**Gates (A-7/A-9):**
- M0 and M1 at most +0.3 MiB vs V2-D (2.36 / 4.72 MiB).
- T4 typical warm.
- T7, also in F3 with autosave off after a journal write.
- Report the exe-size delta, with the regex share separately.

---

## 4. Merge and report
- **PR and CI:** one CI run at the end, plus at most one fix push.
- **Issues:** the PR body closes FUT-45, FUT-12 and FUT-15 with "Closes #n".
- **After CI is green:** merge, then tag `phase-v2e` (a phase tag may stay on the squash merge, per F-25).
- **Report (≤ 80 lines):**
  - the AC table
  - the gates
  - the exe delta
  - the regex decision
  - deviations
  - the F-26 thread count
  - the owner check list (≤ 8 steps, EN + TR), which must cover:
    - the F2 toolbar;
    - `Ctrl+H` in F3, and from read mode;
    - snippets (`table2x3`, `python`, `note`);
    - pasting from a web page;
    - pasting a screenshot;
    - recovery after killing the app with autosave off;
    - auto-pairing on and off.

## 5. Acceptance criteria
| ID | Criterion |
|---|---|
| AC-1 | F-25…F-28 are done (and §1.2 if notes were attached); P-46 is applied with `spec-check` green; the regex ADR is written. |
| AC-2 | §3 items 1–6 work per the listed specs; TRACEABILITY is updated by script; the security rules listed for item 5 hold. |
| AC-3 | The 2 driver scripts pass on CI; the 2 goldens are committed and self-reviewed; the unit tests pass; the PR artifact `yamdv-exe` exists. |
| AC-4 | The gates are green within the growth limits; the PR is merged with the issues closed; `phase-v2e` is tagged; the report follows §4. |

## 6. Stop rule (quota)
If the Grok quota or the Actions minutes run low at any point:
1. commit and push with `[skip ci]`;
2. write `docs/reports/v2e_REPORT.md` with a done / partial / not-started table for §1–§4 and **"Next action: …"**;
3. stop.

The owner will give this same prompt again.

**END OF PROMPT 015.** Start with §1.1 now.
