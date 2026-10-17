# 006_V1B_EDITING — YAMDV Phase V1-B: Editing (F3 split + block-level F2)

> **Prompt 006.** Phase **V1-B**. Branch `phase/v1b-editing`, tag `phase-v1b`, report `docs/reports/v1b_REPORT.md` (A-4: ≤ 120 lines). Store this file as received in `docs/prompts/`.
> **Precedence:** `docs/PROCESS.md` (incl. A-4 delivery mode) > §1 > §2 > `docs/*`. Deferred features are in `docs/FUTURE.md` — do not build them.

---

## 1. Review of V1-A — accepted. Fixes first.

V1-A is **accepted**: every item is done, the 12 driver scripts pass, and the T4 regression was fixed inside the phase.

| ID | Fix |
|---|---|
| **F-20** | **M1 must include images.** `typical.md` references `typical-assets/one.png` and `two.png` (Phase 003). Make memprobe open the document **in place**, from its corpus folder, so the images resolve. Report M1 with the image cache bytes. This is a path fix, not new tooling. |
| **F-21** | **T4 gate de-flake.** `measure.ps1` fails T4 only when **two consecutive** measured attempts both exceed 16 ms warm p99, with one automatic retry inside the same job. Report both attempts. The 16 ms target itself is unchanged. |

Not fixed now (deferred to the post-v1 optimization phase, FUT-50): T-DLG 583 ms, and a cheaper typical scroll path. Exception: if editing pushes typical warm T4 over the gate, fix the scroll path in this phase.

---

## 2. V1-B tasks

The goal: **edit any Markdown file safely in YAMDV.**
- **F3** shows the source on one side and a live preview on the other.
- **F2** lets you click any block of the rendered page; it turns into its Markdown source in place, and leaving it renders it again. This is the v1 model; the inline marker reveal is FUT-43.

Both modes share **one editor component**.

### 2.1 Editor core (`yamdv-editor`, pure; `ropey` 1.6 is pre-approved)

**Buffer and history:**
- Rope buffer.
- Single caret + selection.
- Undo/redo (`Ctrl+Z`, `Ctrl+Y`, `Ctrl+Shift+Z`): typing coalesced per word or per 1 s pause; selection restored; history capped at 4 MiB; shared between F2 and F3 for the same document.

**Movement and selection:**
- Caret movement is grapheme-aware: `←/→`, `Ctrl+←/→` (Unicode word bounds), `Home`/`End` on wrapped lines, `↑/↓` keeping the column, `PgUp`/`PgDn`, `Ctrl+Home`/`Ctrl+End`.
- `Shift` extends the selection.
- Mouse: click, drag with auto-scroll, double-click selects a word, triple-click a line.

**Clipboard and IME:**
- Clipboard as plain text: cut, copy, paste; `Ctrl+Shift+V` behaves the same (HTML → Markdown is FUT-15).
- IME (R-EDIT-09): inline composition with an underline; the candidate window positioned at the caret (`ImmSetCompositionWindow` / `ImmSetCandidateWindow`); AltGr and dead keys work.

**Markdown-aware keys** (R-EDIT-07 minus FUT-12):
- `Enter` continues list, task and quote prefixes; on an empty item it ends the list.
- `Tab`/`Shift+Tab` indent and outdent list items (`editing.list_indent`); elsewhere `Tab` inserts 4 spaces.
- Formatting commands: `Ctrl+B`, `Ctrl+I`, ``Ctrl+` ``, `Ctrl+Shift+X`, `Ctrl+K`, `Ctrl+1…6`, `Ctrl+Shift+0`, `Ctrl+Shift+Q`, `Ctrl+Shift+8/7/9`, `Ctrl+Enter` (toggle task).
  - They toggle on the selection or on the word at the caret.
  - `Ctrl+K` wraps the selection as `[sel](url)` and puts the caret on `url`.
- Invariant tests with `proptest`: random edit / undo / redo sequences must restore the exact bytes.

### 2.2 Saving (R-EDIT-01…04, R-EDIT-11, R-EDIT-12; S-FILE-01/03/10)

**When it saves:**
- **Autosave** (`editing.autosave`, default on): 800 ms after the last edit, on mode exit, on window deactivation, and on close.
- `Ctrl+S` saves immediately.
- A dirty `●` appears in the title: `<file> ● — Live Edit — YAMDV`.

**How it writes — bytes outside edited ranges never change:**
- Atomic save per S-FILE-03.
- Keep the original encoding and BOM, the line endings (new lines use the dominant ending) and the final-newline state.
- Test it: a CRLF + BOM + windows-1254 fixture with one word edited must be byte-identical outside the edit.

**Conflicts** (R-EDIT-03, R-READ-01):
- Before writing, compare the file's size, mtime and hash with the last known state.
- If it changed on disk, show the in-window dialog: [Overwrite] [Reload (discard mine)] [Save mine as…].
- *Save as* uses the **dialog host** with a new `save` kind; S-PROC-07 already allows it.
- While the buffer is dirty, live reload does **not** reload silently; it opens the same dialog.

**Editing is disabled** (with the reason shown in a dialog) for:
- read-only files — offer [Save a copy as…];
- lossily decoded files — offer [Convert to UTF-8 and edit];
- files opened in the plain-text fast mode.

### 2.3 Incremental reparse
Wire the existing splice API (ADR-0007): after each edit, re-parse only the affected top-level blocks, re-lay out only changed blocks, and keep the scroll anchor. The preview updates within 50 ms after a typing pause (about 30 ms debounce).

### 2.4 F3 Split Edit (R-F3-01…06 minus FUT-45)
- **Layout:** `F3` toggles. Editor on the left, preview on the right. Swap via the menu, remembered in `editing.split_swapped`.
- **Splitter:** draggable 6 px, 20–80 %, remembered (`editing.split_ratio`), scales with the window.
- **Editor pane** (B.13.1):
  - plain source text in the `editing.editor_font` (body font by default), same size and line height as the body;
  - line-number gutter (`editing.editor_line_numbers`);
  - soft wrap (`editing.editor_soft_wrap`);
  - subtle current-line highlight;
  - caret blinking stops after 5 s idle (T7).
- **Scroll sync** by block in both directions. Clicking in the preview puts the caret at the matching source position.
- **"?" button** (bottom-right of the editor pane) opens the built-in help document (F1).
- **Find (`Ctrl+F`)** searches the editor buffer in F3 and rendered text in F2 and read mode.
- The sidebar is hidden while in F3.

### 2.5 F2 Live Edit, block level (R-F2-01, R-F2-07, R-F2-08, R-F2-10; model per FUT-43)

**Entering and leaving a block:**
- `F2` toggles. The title shows `— Live Edit —`, and the reading position is kept.
- Clicking a top-level block turns **that block** into an in-place editor. It uses the shared editor component, shows the block's exact source text in the `editing.editor_font`, and has a subtle `code_bg` background. The blocks below move as the source grows or shrinks.
- The source block is re-rendered (with incremental reparse) when you click another block, press `Esc`, or move the caret past its first or last line. `↑`/`↓` past the edge opens the previous or next block with the caret on its last or first line.
- Blank lines typed inside a block may split it into several blocks after re-rendering; that is expected.

**Rendered blocks:**
- Clicking a task checkbox in a rendered block toggles `[ ]` ↔ `[x]` with a single source edit.
- `Ctrl+click` follows links (R-LINK); a plain click on a link enters the block.
- With the setting `reading.task_toggle_in_read_mode` on, checkbox clicks work in read mode too (P-16/P-17).

**Unchanged bytes:** untouched blocks are never re-serialized (R-F2-10).

**Mode switching** (R-EDIT-11): `F2` ↔ `F3` switch directly. `Esc` leaves the active source block first, then the mode. Caret and scroll stay on the same content.

### 2.6 Menu additions
- **Edit ▸** Live Edit `F2` · Split Edit `F3` · Swap panes.
- In edit modes the menu starts with: Undo · Redo · Cut · Copy · Paste · ─ · **Format ▸** (Bold, Italic, Strikethrough, Inline code, Link, Heading 1–3, Paragraph, Quote, Bullet list, Numbered list, Task list).
- **Settings ▸** gains: ✓ Autosave · Editor font ◉ (Body / Code) · ✓ Editor line numbers · ✓ Editor soft wrap.

### 2.7 Tests, goldens, measurements
**Driver scripts (CI, one per feature):**
- F3 type → preview updates
- undo/redo restores bytes
- autosave writes the file with untouched bytes identical
- external change while dirty → conflict dialog
- F2 click paragraph → edit → `Esc` → rendered
- F2 `↑` into the previous block
- task checkbox toggle
- `Enter` continues a list
- `Ctrl+B` on the word at the caret

Add a driver command `type "<text>"` that records key → caret-painted latency into the JSON dump (needed for T5).

**Goldens (2 new):** F3 split (light); F2 with an active source block (light).

**Measurements** — the existing table plus:
| ID | Scenario | Soft budget |
|---|---|---|
| M5 | F3 and F2 after 200 edits on `typical.md` | ≤ 9.6 MiB |
| T5 | typing latency p99 in F3 on `large-1mb.md` / in F2 on `typical.md` | ≤ 25 / 40 ms |

### 2.8 Out of scope
Snippets, image paste, the recovery journal, replace, regex, the F2 format toolbar, table cell editing and table operations, inline marker reveal, HTML paste (FUT-15, -43, -44, -45); math, Mermaid, export, packaging.

---

## 3. Acceptance criteria
| ID | Criterion |
|---|---|
| AC-1 | F-20, F-21 done; M1 reported with images. |
| AC-2 | §2.1–2.6 work; all §2.7 driver scripts pass on CI; proptest invariants pass. |
| AC-3 | Byte-identity test (CRLF + BOM + windows-1254) passes; no write ever happens in read mode except the opt-in task toggle. |
| AC-4 | Gates green incl. memprobe, T4 (with F-21) and the new M5/T5 soft budgets; PR merged; tag `phase-v1b`. |
| AC-5 | Report ≤ 120 lines with a 6-step owner QA list: F3 typing, F2 click-edit-Esc, autosave + reopen, external edit conflict, Turkish keyboard input incl. `ğüşıöçİ`, and IME (if available). |

**END OF PROMPT 006.** Start with §1 now.
