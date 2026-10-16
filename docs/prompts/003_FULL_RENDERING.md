# 003_FULL_RENDERING — YAMDV Phase 003: Full rendering, links & images

> **Prompt 003 of the chained series.** You are executing **Phase 003 only**.
> Process, hard rules and the report template are in `docs/PROCESS.md`; the specification is in `docs/`. This prompt adds (1) the review of Phase 002 with answers and fix items, (2) exact spec patches, and (3) the Phase 003 tasks.
> Store this file as received as `docs/prompts/003_FULL_RENDERING.md` and report its SHA-256 (the reviewer's copy is LF-terminated; transport may change line endings — that is fine).

---

## 0. Precedence
`docs/PROCESS.md` (H1–H15 + amendments) > fix items (§2) > spec patches (§3) > Phase 003 tasks (§4) > `docs/*` > your judgment.

---

## 1. Review of Phase 002 — accepted

Phase 002 is **accepted**. Very good work: every AC passed, and the numbers are far inside the budgets (M0 2.05 MiB, M1b 2.50 MiB, M2 5.48 MiB, exe 1.45 MiB, idle CPU 0). The fix items in §2 come first.

**Answers to report section 8:**
1. *`~sub~` (§4.4.3 vs P-08a):* P-08a wins. §4.4.3 was phase-local text, so no doc change is needed.
2. *F-02 hash:* the owner's transport turned the file into CRLF line endings without the final newline. The reviewer verified that this exact transformation of the reviewer's copy gives `39e1f158…`. The content is identical; keep your copy and no diff is needed. (Prompt 002 arrived byte-exact: `43ae3020…` matches.)
3. *RTL alignment:* your implementation (RTL reading direction + `LEADING`) is correct. The error was in prompt 002 §4.4.1. R-REN-19 already says "right-aligned", so no patch is needed.
4. *M6 vs trimming:* agreed, the trim hid a real cost. Fix it structurally with an out-of-process **dialog host** (F-09, P-10, P-11, P-14, P-15, P-20).
5. *Golden tiles:* accepted. The policy is formalized in P-23.

**Reviewer findings beyond your report:**
- The band buffer scales with window width: at 3840 px a 128-row band is 1.9 MiB. It must be capped (F-12, P-12).
- Phase 003 needs **real clipping** in the renderer. Masking overflow with page-colored fills breaks inside colored containers (callouts, quotes, table cells) and for horizontal scrolling (P-19).
- **Spec conflict:** R-REN-04 and B.8.2 allow toggling task checkboxes in read mode, but S-FILE-01 says read mode never writes. Resolved by P-16/P-17: an opt-in exception, implemented in Phase 006 together with the save path.
- **README badges:** remote images are blocked offline. With the current R-IMG-06 a README with 8 badges would show 8 large placeholders. Inline images get a compact chip instead (P-25).
- HTML anchors such as `<a name="top">` / `id="…"` are common in READMEs, but `id`/`name` were missing from the attribute allowlist (P-24).

---

## 2. Fix items (do these first)

| ID | Fix | Done when |
|---|---|---|
| **F-09** | **Dialog host** (P-10, P-11, P-14, P-15, P-20). `Ctrl+O` launches `yamdv.exe --dialog-host open <owner-hwnd> <nonce>` by **absolute path** (`GetModuleFileNameW`, `CreateProcessW` with `lpApplicationName`).<br>• **Host:** applies the startup hardening (S-PROC-01…03); MAY run without ACG (record why in the ADR). It shows `IFileOpenDialog` owned by the main window (filters per R-OPEN-03, multi-select), then sends the result (cancel, or a list of paths) with `WM_COPYDATA` (own magic, versioned payload, the nonce) from a message-only window, and exits.<br>• **Main window:** while waiting it is disabled (`EnableWindow(FALSE)`). It accepts results only from that child's PID and only with the matching nonce. It re-enables itself on the result or when the child process ends (wait on the process handle, integrated into the message loop). If the host crashes, show a toast "The file dialog could not be opened".<br>• **Remove** the in-process dialog and its post-dialog trim. Save dialogs (Phase 011) and the print dialog will reuse the host. The print dialog will return only serializable data (printer name, DEVMODE, page range, copies), never a handle; the main process creates its printer DC itself.<br>• Write an ADR. | new M6 (P-10) met **without trimming**; module count of the main process unchanged after the dialog; dialog latency `T-DLG` (Ctrl+O → dialog visible) ≤ 400 ms soft, reported |
| **F-10** | **Font warm-up.** After the first paint, in idle time, run one small task per posted message so input stays responsive. Each task creates the text formats and lays out a tiny sample for every style the document uses: regular, bold, italic, bold-italic, code, heading weights, plus CJK/emoji fallback only if the document contains such characters. CI reports **cold** T4 (first measured run, no warm-up pass) **and** warm T4. Gate on warm (≤ 16 ms); report cold against a target of ≤ 33 ms. | both numbers in the report; M1 unchanged within 0.2 MiB |
| **F-11** | **Re-entrancy audit** (P-18). Apply the rule to every call site that can dispatch messages. Add a debug-build guard that detects re-entrant state borrows and logs the message id. List the audited call sites in ADR-0007 (amend) or a new ADR. | ADR + guard + test that provokes a synchronous message |
| **F-12** | **Band cap** (P-12): band height = `clamp(524288 / (4 × width_px), 32, 128)` rows. Unit tests at widths 1000, 1920, 3840, 7680 px. Memprobe JSON reports band bytes. | tests + band bytes ≤ 512 KiB at any width |
| **F-13** | **Inline code padding** without overlap (P-21): apply `IDWriteTextLayout1::SetCharacterSpacing` (leading on the first character, trailing on the last) equal to the horizontal padding; hit-testing, selection and copy stay correct. | goldens `code-*` updated; overlap gone |
| **F-14** | Report the commit hash of tag `phase-002` in §0 of this report. From now on, report the PR head commit, plus the previous phase's merge commit in §0. | hash in report |
| **F-15** | Update `docs/OWNER_KIT.md` and the `owner-kit` artifact for the Phase 003 features (links incl. the Ask popover and blocked targets, images, tables, code toolbar, the dialog host). | kit updated |

---

## 3. Spec patches

Apply them exactly like Phase 002, with `cargo xtask spec-patch` from `docs/spec-patches/003.txt`. Each **OLD** text and each **INSERT AFTER** anchor line must match exactly once in the current docs (original + 002 patches). Log the patches in `docs/SPEC_CHANGES.md`, keep `spec-check` green, and update `TRACEABILITY.md`.

**P-10** — `docs/PERFORMANCE.md`, §D.2 M6 row. OLD:
```text
| **M6** | After closing the Open-file dialog — PWS returns to ≤ M1 + 1 MB within 2 s | required | required |
```
NEW:
```text
| **M6** | 2 s after closing the Open-file dialog (shown by the out-of-process dialog host, S-PROC-07), **without any working-set trim** — PWS and commit vs M1 | PWS ≤ M1 + 0.5 MB and commit ≤ M1 commit + 0.5 MB | PWS ≤ M1 + 1 MB and commit ≤ M1 commit + 1.5 MB |
```

**P-11** — `docs/PERFORMANCE.md`, §D.4 rule 10. OLD:
```text
10. **Transient spikes** (file dialogs, printing, PDF export) are allowed; afterwards transient buffers are freed, and a working-set trim (`SetProcessWorkingSetSizeEx(-1, -1)`) MAY be performed **once, right after the transient operation** and on minimize. Periodic or timer-based trimming is **forbidden** — it fakes the metric.
```
NEW:
```text
10. **Transient spikes:** native file and print dialogs run in the out-of-process dialog host (S-PROC-07), so shell components are never loaded into the main process. Printing and PDF export may spike in-process; afterwards transient buffers are freed, and a working-set trim (`SetProcessWorkingSetSizeEx(-1, -1)`) MAY be performed **once, right after that operation** and on minimize. Periodic or timer-based trimming is **forbidden** — it fakes the metric. Budgets measured after a transient operation are checked on both private working set and commit.
```

**P-12** — `docs/PERFORMANCE.md`, §D.4 rule 3. OLD:
```text
Scrolling reuses pixels (`ScrollWindowEx`) and paints only exposed strips.
```
NEW:
```text
Scrolling reuses pixels (`ScrollWindowEx`) and paints only exposed strips. The band buffer is capped at 512 KiB regardless of window width (band height = clamp(524288 / (4 × width_px), 32, 128) rows).
```

**P-13** — `docs/PERFORMANCE.md`, §D.4 rule 4. OLD:
```text
4. **Images** decoded at display size; LRU cache ≤ 1.5 MB decoded pixels; off-screen images evicted; re-decoded on demand.
```
NEW:
```text
4. **Images** are decoded at display size in device pixels, on the worker thread (E.10). Images intersecting the viewport ± one screen stay decoded; all **other** decoded images share an LRU cache ≤ 1.5 MB and are re-decoded on demand. The bitmap storage type (DIB section vs. device-dependent bitmap) is a measured choice recorded in an ADR.
```

**P-14** — `docs/SECURITY.md`, S-PROC-07. OLD:
```text
- **S-PROC-07 MUST** Child processes only for: the external editor (user action), self-install/uninstall helper, nothing else.
```
NEW:
```text
- **S-PROC-07 MUST** Child processes only for: the external editor (user action); the **dialog host** — a second instance of our own `yamdv.exe`, started by absolute path (`GetModuleFileNameW`) with the internal flag `--dialog-host`, which shows exactly one native file-open/save or print dialog and exits; and the self-install/uninstall helper — nothing else.
```

**P-15** — `docs/SECURITY.md`, after S-IPC-04. INSERT AFTER the anchor line:
```text
- **S-IPC-04 MUST** No named pipes, sockets, shared memory sections, COM servers, DDE, or listening endpoints of any kind.
```
NEW line:
```text
- **S-IPC-05 MUST** Dialog-host results return through `WM_COPYDATA` with their own magic value, a versioned payload and the per-launch nonce. The main window accepts them only while it waits for a host it started, only from that host's process ID (checked with `GetWindowThreadProcessId` on the sender window), and treats returned paths exactly like user-opened paths. The host refuses to run if the owner window does not belong to a process of the same user, and it never does anything other than show one dialog.
```

**P-16** — `docs/SECURITY.md`, S-FILE-01 (read-mode task toggle conflict). OLD:
```text
and the install folder during `--install`. Read mode never writes the document.
```
NEW:
```text
and the install folder during `--install`. Read mode never writes the document, with one opt-in exception: when `reading.task_toggle_in_read_mode` is on, clicking a task checkbox performs a single edit of that `[ ]`/`[x]` marker through the editor's save path (atomic save and conflict detection, available from Phase 006).
```

**P-17** — `docs/SPEC.md`, R-REN-04. OLD:
```text
task-list checkboxes (rendered, not interactive in read mode unless the setting allows).
```
NEW:
```text
task-list checkboxes (rendered; in read mode clickable only when `reading.task_toggle_in_read_mode` is on, which edits the file per S-FILE-01 — available from Phase 006).
```

**P-18** — `docs/ARCHITECTURE.md`, §E.10. INSERT AFTER the anchor line:
```text
- Communication via `PostMessageW` + a lock-free or mutex-protected queue. No async runtime.
```
NEW line:
```text
- **Re-entrancy rule:** many Win32 calls dispatch window messages synchronously (`SetWindowPos`, `ShowWindow`, `UpdateWindow`, `RedrawWindow` with `RDW_UPDATENOW`, `DestroyWindow`, `SetFocus`, `EnableWindow`, `SetCursor`-triggering calls, `MessageBoxW`, any modal loop). Never hold a borrow of window or app state across such a call. Mutations requested while handling a message are queued as deferred commands and applied after the handler returns; a re-entrant `WM_PAINT` is skipped and the region re-invalidated. Debug builds detect re-entrant borrows and log the message id.
```

**P-19** — `docs/ARCHITECTURE.md`, §E.6. INSERT AFTER the anchor line:
```text
Backend per ADR-0002. Requirements regardless of backend: dirty-rect invalidation; scrolling by pixel reuse + exposed-strip painting; overlays (find bar, toasts, badges, viewers) drawn as top layers of the same window; menus, popovers and tooltips as separate layered popup windows; correct per-monitor DPI; no work when nothing changed.
```
NEW line:
```text
The renderer supports true rectangular clipping (`PushClip`/`PopClip`) for glyph runs, shapes and images — required for horizontally scrolled blocks, table cells, callouts and viewers. Rounded clips MAY use corner masks painted in the known container color. Painting background over overflow is not a substitute for clipping.
```

**P-20** — `docs/SHORTCUTS.md`, §F.4. INSERT AFTER the anchor line:
```text
Exit codes: `0` success · `1` runtime error · `2` usage error. Console output only when started from a console (`AttachConsole`).
```
NEW line:
```text
Internal flag, not shown in `--help`: `--dialog-host <kind> <owner-hwnd> <nonce> …` — used only by YAMDV itself (S-PROC-07, S-IPC-05); it exits with code 2 when started any other way.
```

**P-21** — `docs/UI_SPEC.md`, §B.6.3. OLD:
```text
padding 0.15 em × 0.4 em, radius 4 (sharp 0).
```
NEW:
```text
padding 0.15 em × 0.4 em (the horizontal padding is realized as extra character spacing on the span's first and last character, so neighbouring glyphs never overlap), radius 4 (sharp 0).
```

**P-22** — `docs/ROADMAP.md`, row 010 (SVG timing). OLD:
```text
fullscreen diagram viewer, mind map view |
```
NEW:
```text
fullscreen diagram viewer, mind map view, local SVG images (R-IMG-01, renderer chosen by ADR) |
```

**P-23** — `docs/TESTING.md`, §H.4 (golden policy). INSERT AFTER the anchor line:
```text
- Each report links the new/changed goldens so the owner can review the look without building.
```
NEW line:
```text
- Goldens are stored as tiles of ≤ 5 000 rows and ≤ 1 MB each, saved with maximum PNG compression; prefer several focused documents over one tall document. Each report lists the number and total bytes of goldens added or changed.
```

**P-24** — `docs/SPEC.md`, R-MD-14 (anchor attributes). OLD:
```text
Allowed attributes: `align` (p, div, h1–h6, img, td, th),
```
NEW:
```text
Allowed attributes: `id` and `name` (any allowed element; used only as anchor targets, restricted to `[A-Za-z0-9_.:-]{1,128}`), `align` (p, div, h1–h6, img, td, th),
```

**P-25** — `docs/SPEC.md`, R-IMG-06 (README badges). OLD:
```text
*blocked network path*, *too large*, *unsupported format*.
```
NEW:
```text
*blocked network path*, *too large*, *unsupported format*. Images that sit inside a line of text (e.g. README badges, usually remote) use a **compact inline chip** instead of the block placeholder: line-height tall, radius 4, `code_bg` fill, a 12 px glyph and the alt text (or file name) in 12 px `text_muted`; the full message and actions are in its tooltip and on click.
```

---

## 4. Phase 003 tasks

Write `docs/plans/003_PLAN.md` first. The goal: everything in a typical README, docs page or AI-generated file renders as specified — except math (Phase 009), Mermaid (Phase 010) and SVG images (Phase 010). Interim placeholders from Phase 002 disappear for everything implemented here.

### 4.1 Rendering and interaction infrastructure
1. **Clipping** (P-19): `PushClip`/`PopClip` in the display list and backend A, for glyph runs, tiny-skia shapes and images. Remove the overflow masking from Phase 002.
2. **Horizontal scroll regions** for code blocks (and later display math): per-block offset (session state), `Shift+Wheel` and `WM_MOUSEHWHEEL` / touchpad horizontal deltas over the block, and the always-visible 6 px scrollbar while overflowing (B.6.7). Only the block repaints.
3. **Inline objects** (`IDWriteInlineObject`) for superscript/subscript, footnote references, inline images and inline remote-image chips. Each is a single caret/selection unit that copies its text form (sup/sub text, `[n]`, image alt).
4. **Hit testing and hover:**
   - Map the mouse position to interactive elements: link, footnote ref/back-link, code toolbar button, callout fold title, front-matter toggle, `<details>` summary, image, abbreviation.
   - Cursors: hand over links and buttons, I-beam over text.
   - Repaint only affected rects on hover changes.
5. **UI components** in `yamdv-ui`, all theme-token driven, keyboard-operable and EN/TR:
   - **toast** (B.10.1)
   - **tooltip** (B.10.6: 500 ms delay)
   - **link status tooltip** (B.10.4)
   - **popover** (B.10.6)
   - **banner** (B.10.5)

   Popovers and tooltips are layered popup windows (E.6). Timers run only during delays and fades.
6. **Keyboard focus** (F.2): `Tab`/`Shift+Tab` moves through links and interactive elements in document order, with the focus ring per B.16 and auto-scroll into view; `Enter` activates.
7. **Worker thread** (E.10, first use): image probing and decoding, plus very large highlight/layout jobs if needed. Use a 256 KiB stack, cancellable jobs, results posted to the UI thread, and an exit after 5 s idle.

### 4.2 Code blocks — R-CODE-01…06, B.6.7, B.3.4
1. `yamdv-highlight`: hand-written lexers with **per-line start state**, so highlighting is lazy and virtualizable. Cover every language and alias in R-CODE-01 and all token classes. Add a unit test per language: original sample → expected token stream. Unknown language → plain.
2. Hover toolbar inside the 28 px header strip:
   - **Copy** — raw code → clipboard; toast "Copied"; check glyph for 1.2 s.
   - **Line numbers** toggle (drawn, never copied).
   - **Wrap** toggle (soft wrap keeps one number per logical line).
   - **Language label.**

   The toolbar is visible on hover with a 120 ms fade, or on keyboard focus. Defaults come from `reading.code_line_numbers` and `reading.code_wrap`; per-block state lasts the session.
3. `diff` blocks (R-CODE-05); blocks over 5 000 lines are virtualized inside the block (R-CODE-06); tabs at width 4, final newline trimmed.
4. ` ```mermaid ` fences stay plain code blocks labeled `mermaid` until Phase 010. ` ```math ` fences and display/inline math keep their Phase 002 interim rendering until Phase 009.

### 4.3 Tables — R-REN-07 (hover toolbar and fullscreen are Phase 005), B.6.8
1. **Auto table layout:** per column, min width = the widest unbreakable token (≥ 48 px) and max width = the unwrapped content; distribute the available width like CSS auto layout; wrap with `DWRITE_WORD_WRAPPING_EMERGENCY_BREAK` so content never overflows the column.
2. Header weight 600 and `table_header_bg`; per-column alignment; inline formatting in cells; `\|` escapes; `appearance.table_zebra`; outer corner radius 6 per `corner_style` (corner masks allowed, P-19).
3. **Row-level laziness:** a 10 000-row table is one top-level block, but only its visible rows are laid out; row heights are estimated otherwise.
4. Limits (S-PARSE-02): more than 1 000 columns or 100 000 cells → render the source in a code card with a "table too large" label.

### 4.4 Lists, task lists, callouts, footnotes, front matter, definition lists
1. **Task checkboxes:** vector-drawn per B.6.4 (replaces ☐/☑). Not clickable in this phase (P-16/P-17).
2. **Callouts** (R-MD-04, R-REN-06, B.6.6, B.3.3):
   - **Recognition:** GFM kinds from pulldown-cmark, and Obsidian syntax (a blockquote whose first line is `[!type]`, `[!type]+` or `[!type]-` with an optional title), using the full type mapping.
   - **Content:** any Markdown inside.
   - **Folding:** the title row toggles (+ expanded / − collapsed by default); session state.
   - **Icons:** per B.14, with vector fallbacks for *important* and *caution*.
3. **Footnotes** (R-REN-10, B.6.9):
   - References are superscript `[n]`, numbered in order of first reference (GitHub behaviour).
   - The notes section is rendered at the end of the document, whatever the source order; definitions disappear from their source position.
   - Clicking a reference smooth-scrolls to its note. `↩` jumps back to the reference; multiple references give `↩ ↩²`.
   - A hover tooltip shows the note's plain text (max 400 px wide, 6 lines).
4. **Front matter card** (R-REN-13): collapsed / expanded / hidden per `reading.front_matter`; click toggles. Minimal key/value extraction: top-level `key: value` (YAML) or `key = value` (TOML); anything complex is shown raw.
5. **Definition lists** (R-MD-11): keep the Phase 002 rendering and add goldens.

### 4.5 Images — R-IMG-01…04, R-IMG-06 (+ P-25), S-RES-01…06; the fullscreen viewer is Phase 005
1. WIC decoding of PNG, JPEG, GIF (first frame; `reading.animate_gifs` stays unimplemented, marked `// PHASE-MAY`), BMP, ICO, TIFF and WebP. **Verify each under ACG** on CI; any failure goes in the ADR.
2. **Layout-time header probe** for dimensions (worker thread). Until it answers, use a 4:3 estimate at column width; scroll anchoring absorbs the correction.
3. **Decode at display size** in device pixels with high-quality scaling, on the worker. Keep visible ± 1 screen; the LRU for the rest is ≤ 1.5 MiB (P-13).
4. **Measure** the bitmap storage choice: DIB section vs. device-dependent bitmap (`CreateDIBitmap` / `CreateCompatibleBitmap`) effect on PWS and commit, with 4 large images. Record it in an ADR.
5. **Resolution and policy, complete** (S-RES-01…06):
   - Path forms: relative, `../`, absolute, `file:///`, percent-decoding, backslashes.
   - `data:` URIs within the limits.
   - **Network-location blocking before any file access:** UNC, `//host`, `\\?\UNC\`, `file://host`, WebDAV `@SSL`/`@port`, `DRIVE_REMOTE` (mockable `GetDriveTypeW`), and reparse-point final components (`FILE_FLAG_OPEN_REPARSE_POINT`).
   - The same-share exception for documents opened from a share.
   - Header-checked dimension limits.
6. **Sizing:** HTML `width`/`height`, `align="center"` and centered containers, and Obsidian `![[img.png|300]]`; never upscale automatically.
7. **Placeholders** per R-IMG-06, plus the compact inline chip (P-25) for in-line images, including all remote badges.
8. SVG files show the "unsupported format" placeholder until Phase 010.

### 4.6 Safe HTML — R-MD-14 (+ P-24), S-HTML-01…03
1. **Tokenizer:** your own, linear-time and bounded (depth 64, 100 000 tags, 8 KiB per attribute value). Entities decoded; comments hidden, including multi-line ones.
2. **Sanitizer → allowlist model:**
   - **Inline tags:** `b strong i em u ins s del strike mark sub sup small kbd code var samp abbr br span a img`. `abbr` gets a `title` tooltip; `span` allows only the restricted `style` grammar.
   - **Block tags:** `p div center h1–h6 blockquote ul ol li dl dt dd pre hr table thead tbody tfoot tr th td figure figcaption picture source details summary`. For tables, `colspan` is honored and `rowspan` MAY be treated as 1 (document it).
   - **Unknown elements** are dropped but keep their text. The dangerous containers listed in S-HTML-01 are dropped together with their content.
3. **HTML structure pass across sibling blocks.** CommonMark ends an HTML block at a blank line, so `<details>` / `<summary>` / Markdown / `</details>` and `<div align="center">` … `</div>` arrive as separate top-level blocks. A bounded stack pass over the block index pairs them into container ranges. Their Markdown content renders inside the container: centered, indented, or collapsible for details (summary row with chevron, closed unless `open`). This replaces Phase 002 deviation 4. Unpaired tags degrade gracefully.
4. **Anchors:** HTML `id`/`name` targets (P-24) register as anchors.

### 4.7 Inline extensions — R-MD-03, -05, -06, -12, -13, R-REN-03
- `==highlight==`: not inside code or URLs; pairing rules analogous to `~~`.
- **Superscript / subscript rendering:** inline objects at 0.75 em, baseline shift +0.35 / −0.2 em.
- **GFM autolink literals:** linkify pass with the GFM trailing-punctuation rules; never inside code, links or HTML.
- **Emoji shortcodes:** a static table generated from the gemoji database (MIT license — record it in an ADR, commit the generated Rust table, no network at build time).
- **Wikilinks and embeds:** resolution per R-LINK-03; bounded directory scans cached per folder.
- **`<kbd>` styling** per B.6.3; `abbr` tooltips.

### 4.8 Links — R-LINK-01…06, S-LINK-01…04, S-EXEC-02/04, S-UI-01…03; "Copy link address" (R-LINK-07) needs the context menu, Phase 004
1. **Anchors:** `#slug`, `{#id}`, HTML `id`/`name`, footnotes → smooth scroll (B.15).
2. **Relative Markdown links:**
   - Open in the same window with a navigation history; `Alt+←` / `Alt+→` and mouse X1/X2 move back and forward, restoring the scroll position per entry.
   - `Ctrl+click` or middle-click opens a new window (same process).
   - A missing target shows the toast "Link target not found: <name>".
3. **External links** (`http`, `https`, `mailto`) follow `general.open_external_links` (default *Ask*):
   - **Ask popover** shows the real URL.
   - **IDN hosts** are also shown in punycode, via the Windows API `IdnToAscii` (offline). Write your own strict URL parser; do not use the `url` crate.
   - **Credentials** (`user:pass@`) are highlighted as suspicious.
   - **Open** uses `ShellExecuteExW` without `SEE_MASK_NOZONECHECKS`.
4. **Local non-Markdown targets:**
   - Folders: confirm, then `SHOpenFolderAndSelectItems`.
   - Allowlisted extensions (S-LINK-03): confirm (Cancel is the default), then open.
   - Everything else: popover "Blocked for safety" with [Show in Explorer].
5. **Other schemes:** inert, with a "Blocked link type" tooltip.
6. **Mark-of-the-Web:** read the document's own `Zone.Identifier` stream. This is a path the app builds itself, so S-FILE-04's ADS rule for document-derived paths does not apply. `ZoneId ≥ 3` → banner "This file came from the internet", and external links and local launches always *Ask*.
7. **Hover:** status tooltip with the destination (B.10.4); the `title` attribute or link title as a tooltip; hand cursor.
8. **Banners:** running elevated (S-PROC-06 / S-UI-03), and lossy decoding (S-UI-03).

### 4.9 Clipboard — completes R-REN-16 except the menu entry
- `Ctrl+C` puts **CF_HTML** on the clipboard as well as plain text. It is written by a new HTML fragment writer in `yamdv-export` (the start of the Phase 011 exporter): everything escaped, only sanitized constructs, images included only as their alt text.
- `Ctrl+Shift+C` → **Copy as Markdown**: the exact source slice covering the selection, expanded to whole inline constructs.

### 4.10 Tests, fuzzing, goldens, corpus
- **Unit tests** for everything above: table layout, highlighter tokens per language, callout detection, footnote numbering, structure pass, sanitizer, linkify, wikilinks, URL parser, IDN, link policy, image path policy.
- **Security matrices (S-TEST-02):**
  - **Link policy:** scheme × target type × MotW × setting.
  - **Image path policy:** every UNC/WebDAV/device/ADS/reparse form, plus the same-share exception.
  - **HTML:** ≥ 50 XSS vectors through sanitizer → model and through the CF_HTML writer. Assert no `<script`, no `on*=`, no `javascript:` / `vbscript:` / non-image `data:` anywhere in the output.
- **Fuzz** (≥ 10 min each): `parse_layout` (now with all extensions), `html` (tokenizer + sanitizer + structure pass), `highlight` (all lexers), `url` (URL parser + linkify). Every crash becomes a regression test.
- **Corpus** (original content only):
  - `typical.md`: the real M1 document per H.3, with `typical-assets/` containing two generated 800×600 PNGs.
  - `tables.md`, `code.md` (every language), `callouts.md`, `footnotes.md`, `images.md` (local, `../`, data URI, missing, remote inline badges + a remote block image, UNC, oversized, corrupt, WebP, GIF, SVG placeholder), `links.md`, `html.md` (README centering, details, anchors, XSS vectors), `frontmatter.md`, `details.md`, `unicode.md` (updated).
- **Goldens** (P-23 policy) at 1.0 and 1.5, Default light and dark, for every corpus document above. Report count and bytes.

### 4.11 Measurements
| ID | Scenario | Soft budget |
|---|---|---|
| M0 | empty state | ≤ 4.8 MiB |
| **M1** | `typical.md` (now replaces M1b), window 1000×700 on CI | ≤ 8.0 MiB |
| M2 | `large-1mb.md` | ≤ 16 MiB |
| **M6** | per P-10, no trim | PWS ≤ M1 + 1 MiB, commit ≤ M1 commit + 1.5 MiB |
| M7 | commit ÷ PWS | ≤ 3× |
| **T1** | first paint of `typical.md` (replaces T1b) | ≤ 150 ms |
| T1L, T2 | as Phase 002 | ≤ 300 / 100 ms |
| T4 | scroll p99, warm (gate) and cold (report), `typical.md` and `large-1mb.md` | warm ≤ 16 ms |
| T7 | idle CPU 30 s — **also with a hover tooltip shown and dismissed**, and after an image decode | ≤ 1 ms |
| T-IMG | block becomes visible → decoded image painted | ≤ 100 ms soft (report) |
| T-DLG | `Ctrl+O` → dialog visible (host) | ≤ 400 ms soft (report) |
| B-SIZE | release exe | ≤ 4.0 MiB |
Also report the subsystem breakdown for M1 and M2, adding the image cache, highlighter state and HTML model.

### 4.12 Out of scope for 003
Math and Mermaid rendering, SVG, custom title bar, sidebar, menus (including "Copy link address"), settings UI, scheme switching, find, zoom, live reload, fullscreen viewers, the table/image ⛶ badges and toolbar, CSV export, task toggling, editing, export to files. **Do not implement them.**

---

## 5. Requirement coverage for Phase 003
Update `docs/TRACEABILITY.md`:
- **DONE:**
  - Markdown: R-MD-02, R-MD-03, R-MD-04, R-MD-05, R-MD-06, R-MD-09, R-MD-11, R-MD-12, R-MD-13, R-MD-14, R-MD-18
  - Rendering: R-REN-03, R-REN-06, R-REN-08, R-REN-10, R-REN-11, R-REN-12, R-REN-13, R-REN-14
  - Code: R-CODE-01…06
  - Images: R-IMG-02, R-IMG-03, R-IMG-04, R-IMG-06
  - Links: R-LINK-01…06
  - Security: S-RES-01…06, S-LINK-01…04, S-HTML-01…03, S-EXEC-04, S-PROC-06 (banner), S-PROC-07 (dialog host), S-IPC-05, S-UI-01…03
  - Measurements: M1, T1
- **PARTIAL:**
  - R-REN-04 (task toggle in Phase 006)
  - R-REN-07 (hover toolbar and fullscreen in Phase 005)
  - R-REN-16 (menu entry in Phase 004)
  - R-IMG-01 (SVG in Phase 010, GIF animation MAY)
  - S-TEST-02 (link, path and HTML matrices)

---

## 6. Acceptance criteria (Phase 003)
| ID | Criterion |
|---|---|
| AC-1 | F-09…F-15 done with evidence; dialog host in use, new M6 met without trim. |
| AC-2 | P-10…P-25 applied by script, logged, `spec-check` green. |
| AC-3 | All gates green on CI including the memprobe budget gate; PR merged; tag `phase-003`. |
| AC-4 | Every corpus document of §4.10 renders without interim placeholders except math, Mermaid and SVG; goldens committed and self-reviewed against `UI_SPEC.md` (list known deviations). |
| AC-5 | Link behaviour per §4.8 covered by the policy matrix and by an automated in-window test (anchor scroll, relative navigation + back/forward, blocked exe link, Ask popover shown for http). |
| AC-6 | Image policy matrix green, including network-path blocking **before any file access** (proved with a mock file-system layer recording calls). |
| AC-7 | XSS vector suite green for the sanitized model and the CF_HTML output. |
| AC-8 | Four fuzz targets ran ≥ 10 min each, no open crash. |
| AC-9 | §4.11 table complete (MiB + bytes), with breakdown and deltas vs Phase 002; warm T4 ≤ 16 ms; T7 holds after tooltips and decodes. |
| AC-10 | Owner kit updated (F-15). |
| AC-11 | Report follows `docs/PROCESS.md` exactly, with §0 and §3b. |

---

**END OF PROMPT 003.** Start with §2 (F-09 first) now.
