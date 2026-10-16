# 002_READING_MVP — YAMDV Phase 002: Core document model & reading MVP

> **Prompt 002 of the chained series.** You are executing **Phase 002 only**.
> Read this whole prompt first. The permanent specification is already in the repository (`docs/`); this prompt adds (1) the review results of Phase 001 with fix items, (2) exact spec patches, and (3) the Phase 002 tasks.
> The owner attaches two files to this prompt: `001_FOUNDATION_AND_SPIKE.md` (again) and this file. You will store both in the repo (F-02).

---

## 0. Precedence and inputs

**Precedence:** Hard rules H1–H15 of prompt 001 (to be stored as `docs/PROCESS.md`, F-01) > fix items (§2) > spec patches (§3) > Phase 002 tasks (§4) > `docs/*` > your judgment. Report real conflicts in report section 8, as before.

**Inputs you rely on:** the repository at tag `phase-001`, `docs/SPEC.md`, `docs/UI_SPEC.md`, `docs/SECURITY.md`, `docs/PERFORMANCE.md`, `docs/ARCHITECTURE.md`, `docs/SHORTCUTS.md`, `docs/TESTING.md`, ADR-0001…0006, and the two attached prompt files.

---

## 1. Where we are

Phase 001 was reviewed and **accepted**, with the fix items in §2. Key facts carried forward:
- **Rendering backend A** (DirectWrite GDI-interop bitmap render target + tiny-skia on a reused band DIB, `ScrollWindowEx`) is the product backend (ADR-0002). Measured on CI: 2.39 MB idle PWS for the full reference scene, scroll p99 4.2 ms; skeleton window 1.33 MB (Phase 001 reported 10⁶-byte MB; from now on MiB, see P-05/F-07).
- Default NT heap (no SegmentHeap).
- CI runner = Windows Server 2025, Hyper-V video 1024×768, no GPU. Numbers are trends, not end-user truth — hence the owner kit (F-05).

**Phase 002 goal:** a real, fast, read-only Markdown viewer for the CommonMark core. Open a file (CLI, Ctrl+O, drag & drop) → decode → parse → lay out lazily → render with backend A → smooth, virtualized scrolling → select and copy text. Everything the later phases need from the document model (source ranges, layout↔source mapping, block index, lazy per-block parsing) is built **properly now**. Constructs that are rendered in Phase 003 are already parsed and shown as clearly labeled interim placeholders (§4.4.3).

---

## 2. Fix items from the Phase 001 review (do these first)

| ID | Fix | Done when |
|---|---|---|
| **F-01** | Create `docs/PROCESS.md` containing prompt 001 §2 (Hard rules H1–H15), §3 (Methodology) and §7 (Report template) **copied programmatically** (not re-typed) from the attached `001_FOUNDATION_AND_SPIKE.md`, followed by an "Amendments" section with: **A-1** `spikes/` may use `unsafe` (excluded from the product; every block still has `// SAFETY:`), resolving the H14 vs E.9 conflict; **A-2** report section "0. Fix items status" and section "3b. Spec patches applied" are mandatory from Phase 002 on (template below in §6). | file exists; diff vs the prompt sections is empty except the amendments |
| **F-02** | Store the attached prompts byte-exact as `docs/prompts/001_FOUNDATION_AND_SPIKE.md` and `docs/prompts/002_READING_MVP.md` (`.gitattributes`: `docs/prompts/** -text`). Report the SHA-256 of each. (Reference: the reviewer's copy of 001 has SHA-256 `04e60d0351e7b5860f3474fc778b6a700bb974656422e60a8e37bad9b3d1e00c`. If yours differs, use yours and report the hash.) | files committed, hashes in report |
| **F-03** | **Verify the re-typed appendices.** Add `cargo xtask spec-check`: for each appendix A–H, extract it from `docs/prompts/001_…md` (from the line starting with `# APPENDIX X ` up to, not including, the next `# APPENDIX` line or the `**END OF PROMPT 001.**` line), normalize (CRLF→LF, strip trailing whitespace per line, drop leading/trailing blank lines and trailing `---` lines, end with one `\n`), and compare with the same normalization of the file content of `docs/<file>` **at tag `phase-001`** (`git show phase-001:docs/SPEC.md`). Expected SHA-256 of the normalized appendix texts: A `6d5ab534cff27b29bce64af68ae2d82b513f504190f8ba1db7644f445d3aec86`, B `26dfa16e1cc7ab4560f932b5b4d9236bde80e2091b31e7706bd190c2d0d5c2a3`, C `60fcc8d845dc93cfec157825cf5398957397517bf74d12938bb00715708c5711`, D `57bd7bfb50faa18638a2dac4a2dd18d9a059a69e73804460fa7e6f0972a7a0f7`, E `12b8443a899a99edc1da6f12ba7206b588c077ccf41b34401ef0536a8d3fbd18`, F `49abff061d0aceca9b6322a50aa7c111fa8ee204f6a1ddfb2303ee7033ecb012`, G `9a0778360d59064318c5f7a630d43c2e8eb2fa146db6fb3e96ca8e61cc004d14`, H `1a2c529a77513dda8d65dd985a3315572f5c895bace97034e2c4636f03b6ad23`. First confirm that your extraction of the attached 001 file reproduces exactly these 8 hashes (this validates the extraction code); then compare with the docs. Report per appendix: match / mismatch + unified diff summary. **Any mismatch: restore the doc from the extracted original first, then apply §3 patches.** After patching, `spec-check` compares `docs/*` against "original + patches" (keep the patch application scripted, see §3). | xtask command + report table; all 8 verified |
| **F-04** | Amend `docs/reports/001_REPORT.md` with the template sections it missed: **Acceptance criteria table** (AC-1…AC-7 → PASS / FAIL / NOT VERIFIED + evidence), **Repository state** (merge commit hash, tag, PR, CI run links), **Files added/changed** (trimmed tree), **Manual QA checklist for the owner** (window, light/dark switch, Esc/Ctrl+W, `--help`/`--version`, spike A run). Do not change the existing measured content. | amended file committed |
| **F-05** | **Owner kit.** A CI job (on `push` to `main` and `workflow_dispatch`) uploads artifact `owner-kit` (retention 14 days): release `yamdv.exe`, release `xtask.exe`, `spike-a.exe`, `tests/corpus/` (002 subset), and `OWNER_KIT.md` (English + Turkish) with copy-paste PowerShell steps to: run YAMDV on each corpus file; run `xtask.exe memprobe` for M0/M1b/M2/M6 with `--json`; run spike A at full HD (`--size 1920x1080 --autoscroll`) to get real T4 numbers; switch Windows light/dark while YAMDV is open; test at 150 % display scaling. The kit must not require Rust or admin rights. `OWNER_KIT.md` explains that Windows marks downloaded ZIPs (right-click → Properties → Unblock) and that SmartScreen may warn about the unsigned exe ("More info → Run anyway"). | artifact exists on the latest `main` run |
| **F-06** | `ProcessDynamicCodePolicy` (deferred from 001): enable it after verifying DirectWrite text, color emoji, WIC decode, IFileOpenDialog (incl. a shell-extension-heavy folder), drag & drop and IME composition (if testable on CI, otherwise NOT VERIFIED + owner-kit step). Record the outcome in ADR-0001 (amend) or a new ADR. | ADR updated; policy on or justified off |
| **F-07** | **Units.** Per patch P-05: all memory/size numbers in MiB (2²⁰) with raw bytes alongside; `xtask size`/`memprobe` print both; restate the Phase 001 table in MiB in this report's measurement section (conversion only, no re-measurement). | outputs + report |
| **F-08** | `memprobe --size WxH`: set the window client size explicitly and record the actual client size and monitor resolution in the JSON; CI uses `1000x700` (the runner screen is 1024×768). | JSON fields present |

---

## 3. Spec patches (apply exactly; log them)

Apply each patch with a script (`cargo xtask spec-patch`, reading a committed `docs/spec-patches/NNN.txt` or equivalent) so that `spec-check` can verify "original + patches". Every **OLD** text below must match exactly once in the restored original doc (F-03); the **NEW** text replaces it. "INSERT AFTER" adds the new text as new line(s) directly after the anchor line. Record every patch in a new `docs/SPEC_CHANGES.md` (`ID | date | doc | section | summary | reason`) and update `docs/TRACEABILITY.md` where needed.

**P-01** — `docs/SPEC.md`, R-LINK-05 (wrong cross-reference). OLD:
```text
document types on the allowlist in `S-EXEC-02`
```
NEW:
```text
document types on the allowlist in `S-LINK-03`
```

**P-02** — `docs/SPEC.md`, R-MMD-03 (wrong cross-reference). OLD:
```text
Light and dark palettes (Appendix B §B.6.12)
```
NEW:
```text
Light and dark palettes (Appendix B §B.3.6)
```

**P-03a** — `docs/SPEC.md`, R-THEME-06 (undefined token `border`). OLD:
```text
applies instantly; reset per token / all.
```
NEW:
```text
applies instantly; reset per token / all. `border` is an editor-facing alias that sets both `code_border` and `table_border`; in `custom-theme.ini` an explicit `code_border` or `table_border` key overrides the alias.
```

**P-03b** — `docs/UI_SPEC.md`, §B.3.1. INSERT AFTER the anchor line:
```text
(The values were sampled from the reference screenshots; `link` and Mint `heading` were minimally darkened to meet R-A11Y-04.)
```
NEW line:
```text
The custom-colors alias `border` (R-THEME-06) maps to `code_border` + `table_border` in both modes.
```

**P-04** — `docs/PERFORMANCE.md`, §D.4 rule 8 (SegmentHeap ambiguity). OLD:
```text
8. **Allocator:** the system heap (default). No arena/segment allocators that reserve large regions. `shrink_to_fit` long-lived buffers after load.
```
NEW:
```text
8. **Allocator:** Rust's default global allocator (the Windows process heap via HeapAlloc). No third-party allocators (mimalloc, jemalloc, bump/arena allocators that reserve large regions). The Windows heap *type* (NT heap vs. SegmentHeap via the manifest) is a measured choice recorded in ADR-0002 (currently the default NT heap). `shrink_to_fit` long-lived buffers after load.
```

**P-05** — `docs/PERFORMANCE.md`, §D.3 (units). INSERT AFTER the anchor line:
```text
## D.3 Measurement method (normative)
```
NEW line:
```text
- **Units:** "MB" in this project means **MiB (2^20 bytes)**, the unit Task Manager uses. Tools always print raw bytes and MiB with two decimals. All budgets in §D.2 are MiB.
```

**P-06** — `docs/ARCHITECTURE.md`, §E.9 (spikes vs H14). INSERT AFTER the anchor line:
```text
Tests may opt out locally with `#[allow(...)]` at the test module level.
```
NEW line:
```text
Hard rule H14 (`docs/PROCESS.md`) is amended by A-1: `spikes/` may use `unsafe` (each block with a `// SAFETY:` comment); `spikes/` is never part of the product build.
```

**P-07** — `docs/ARCHITECTURE.md`, §E.4 (document model). OLD:
```text
- `BlockTree`: a flat `Vec<Block>` arena; each block has `kind`, `source_range: Range<u32>`, `parent`, `children` range, and kind-specific data (heading level + slug, list info, code lang, table alignments, callout kind…). Inline content is parsed lazily per block into compact runs referencing source ranges (owned strings only where unescaping requires).
```
NEW:
```text
- **Two-level document model.** (1) At load, one full pulldown-cmark pass builds the **top-level block index**: for every top-level block its kind, source byte range (`u32`) and height-estimate inputs, plus document-wide facts collected on the way — headings (level, source range, text, slug), link reference definitions and footnote definitions (stored in maps), and the front-matter range. Inline events are not kept. (2) A top-level block's **detail tree** (nested blocks in a flat arena with `parent`/`children` ranges and kind-specific data — heading level + slug, list info, code language, table alignments, callout kind… — plus compact inline runs referencing source ranges; owned strings only where unescaping requires) is built **lazily** by re-parsing exactly that block's source slice. Top-level blocks are self-contained in CommonMark once reference and footnote definitions are supplied from the document maps (pulldown-cmark's broken-link callback). Detail trees live in an LRU cache with a byte budget and are dropped when far off-screen.
```

**P-08a** — `docs/SPEC.md`, R-MD-06 (GFM single-tilde strikethrough conflict, found by the reviewer). OLD:
```text
- **R-MD-06 MUST** Superscript `^x^` and subscript `~x~` (pulldown-cmark options; `~~x~~` stays strikethrough).
```
NEW:
```text
- **R-MD-06 MUST** Superscript `^x^` (pulldown-cmark option). Subscript with single tildes `~x~` is **off by default** (setting `reading.subscript_tilde`), because GFM renders `~x~` as strikethrough; when it is off, both `~x~` and `~~x~~` are strikethrough. `<sub>` and `<sup>` HTML always work (R-MD-14).
```

**P-08b** — `docs/UI_SPEC.md`, §B.8.2. INSERT AFTER the anchor line:
```text
| `reading.smart_punctuation` | bool | `false` | |
```
NEW line:
```text
| `reading.subscript_tilde` | bool | `false` | R-MD-06 |
```

**P-09** — `docs/PERFORMANCE.md`, §D.2 (interim scenarios). INSERT AFTER the anchor line:
```text
| **T10** | PDF export of 100 pages | ≤ 3 s; transient memory ≤ +30 MB, released afterwards | same |
```
NEW lines (a blank line, then the paragraph):
```text

Interim scenarios used before the full corpus exists (Phase 002): **M1b** = `typical-basic.md` (soft ≤ 8.0 MiB), **T1b** = first paint of `typical-basic.md` (soft ≤ 150 ms), **T1L** = first paint of `large-1mb.md` (soft ≤ 300 ms). From Phase 003 on they are replaced by M1 and T1.
```

---

## 4. Phase 002 tasks

Write `docs/plans/002_PLAN.md` first (§3 of PROCESS.md). Then:

### 4.1 Core infrastructure (pure crates)
1. **INI dialect parser + Settings model** (`yamdv-core`, E.12, B.8.2 incl. P-08): parse defensively (S-FILE-07), every key typed with default and clamping, unknown keys ignored, round-trip-safe writer (writer is used from Phase 004; implement and test it now, but the app only **reads** `settings.ini` in this phase). Portable-mode path resolution (R-SET-05). Fuzz target (§4.8).
2. **i18n** (E.13, R-I18N-01/02): `Msg` enum + EN/TR tables, language resolution from `GetUserDefaultUILanguage` (`tr-*` → TR), completeness test. Only the strings this phase needs.
3. **Internal logger** (H14, S-PRIV-02): compile-time levels, off by default, enabled by `privacy.diagnostic_log`, rotating files, never logs document text.
4. **Theme token data** (B.3.1, B.3.2, B.3.3, B.3.4, B.3.5 as data for **all 11 schemes, light and dark**) + a unit test computing WCAG contrast for every palette (R-A11Y-04: text ≥ 4.5, link ≥ 4.5, heading ≥ 3.0 vs `bg`). Only Default is used in this phase; mode = Follow System (R-THEME-01, live switch already exists).

### 4.2 Document loading (`yamdv-core` + platform `fs`)
1. **Path intake** for CLI, dialog and drop paths: `GetFullPathNameW`, S-FILE-04 validation (device names, `\\.\`, `\\?\GLOBALROOT`, ADS, NUL), long-path support (internal `\\?\`), display form without prefixes (R-OPEN-12). User-opened UNC paths are allowed.
2. **Guards** (R-OPEN-08, S-PARSE-02): > 256 MiB refuse; > 20 MiB → confirm dialog "Open as plain text (fast) / Render anyway / Cancel"; NUL bytes in the first 8 KiB → confirm "This looks like a binary file — Open anyway / Cancel".
3. **Decoding** (R-OPEN-06): BOM → UTF-8/UTF-16LE/UTF-16BE; valid UTF-8 → UTF-8; else `chardetng` + `encoding_rs`. Store in `Document`: encoding, had-BOM, lossy flag, line-ending style (LF/CRLF/mixed + dominant, R-OPEN-07), final-newline flag, read-only attribute (R-OPEN-10), size, mtime. Corpus files of Appendix H "Encodings & bytes" all decode correctly (unit tests compare decoded text with expected UTF-8 fixtures).
4. **Plain-text mode** (for `.txt` per R-OPEN-09, and for "Open as plain text"): body font, preserved line breaks and spaces, wrapping at the column width, virtualized in chunks of ≤ 200 lines.

### 4.3 Parsing and the document model (`yamdv-core`, P-07)
1. pulldown-cmark options: `TABLES | FOOTNOTES | STRIKETHROUGH | TASKLISTS | HEADING_ATTRIBUTES | YAML_STYLE_METADATA_BLOCKS | PLUSES_DELIMITED_METADATA_BLOCKS | MATH | GFM | DEFINITION_LIST | SUPERSCRIPT | WIKILINKS`; `SUBSCRIPT` only when `reading.subscript_tilde`; `SMART_PUNCTUATION` only when `reading.smart_punctuation`. Single-newline-as-break per `reading.single_newline_break` (R-MD-15). **Parse everything now, render progressively** (§4.4.3).
2. **Top-level block index** + document maps (headings with GitHub slugs and `{#id}` per R-MD-10, link reference definitions, footnote definitions, front matter range).
3. **Lazy detail trees** per top-level block (re-parse of the slice with the broken-link callback resolving references from the maps), LRU-cached with a byte budget; nesting depth capped at 64 (S-PARSE-01; deeper content is attached at level 64).
4. **Layout ↔ source mapping:** every inline run keeps its source byte range so that a position in a laid-out paragraph (UTF-16 index in the layout string) maps to a source byte offset and back. This is required for F2/F3 later — build and unit-test it now.
5. **Splice API** (used from Phase 006): replace the index entries for a source range with newly parsed entries and shift subsequent ranges; unit-tested with synthetic edits. Write **ADR-0007 "Document model"** describing the two-level model, the incremental-reparse algorithm (start at the top-level block containing the edit, re-parse until block boundaries re-synchronize, re-resolve inlines when reference/footnote definitions change) and its measured memory cost (M1b, M2).
6. Text extraction without layout (for `Ctrl+A` + copy and, later, find): walk detail trees built on demand, released afterwards.

### 4.4 Layout and rendering (`yamdv-layout` + `yamdv-platform-win`)
**4.4.1 Text shaping.** `TextShaper` trait (pure) with a deterministic mock implementation for tests, and the DirectWrite implementation:
- `IDWriteTextLayout` per paragraph-like box with styled runs: family, size, weight, style, color (drawing effect), strikethrough, underline; inline-code ranges get their background via `HitTestTextRange` rects drawn before the text (B.6.3).
- **Uniform line spacing**: `SetLineSpacing(UNIFORM, font_size × line_height, baseline = 0.8 × line height)` so fallback fonts (CJK, emoji) never change line pitch.
- Paragraph direction from the first strong character (letters in the Hebrew/Arabic/Syriac/Thaana/NKo ranges U+0590–U+08FF, U+FB1D–U+FDFF, U+FE70–U+FEFF, U+10800–U+10FFF, U+1E800–U+1EFFF → RTL + trailing alignment), R-REN-19.
- Tab stops at 4 × space advance for code (R-MD-17).
- A custom `IDWriteTextRenderer` that draws glyph runs on the band's `IDWriteBitmapRenderTarget` (color emoji via `TranslateColorGlyphRun`, R-REN-20), draws decorations with tiny-skia, uses monitor rendering params (ClearType per user settings, B.2), and is DPI-correct (`pixelsPerDip`).

**4.4.2 Block layout** per `docs/UI_SPEC.md` §B.6.1 (column formula, 65 %, paddings), §B.6.2 (paragraphs, H1–H6, strong/em/strikethrough), §B.6.3 (inline code), §B.6.4 (lists: tight/loose, bullets by depth, ordered with start numbers, nested), §B.6.5 (blockquotes, nested bars), §B.6.9 (horizontal rule only), §B.6.10 (links: color only — no hover underline, no click yet), §B.6.13 (selection), definition lists (term weight 600, definition indented 24 px). Code blocks (§B.6.7) **without** highlighting and toolbar: plain code text, no wrapping, overflow clipped with a 24 px fade to `code_bg` at the right edge (horizontal scrolling arrives in Phase 003; mark `// PHASE-003`).

**4.4.3 Interim rendering of constructs owned by Phase 003+** (mark each `// PHASE-NNN:`; they must look deliberate, not broken):
- **Block level** — tables, HTML blocks, display math, footnote definitions section, front matter, `<details>`: a card like a code block (`code_bg`, radius, padding) showing the block's **source text** in the code font, with a small muted label at the top-right (`table`, `html`, `math`, `footnotes`, `front matter`). GFM alerts render as a normal blockquote whose first line is the localized type title in weight 600 (`Note`, `Tip`, …).
- **Inline** — images: muted italic `[image: <alt>]`; inline HTML: its source text in `text_muted`; inline math: source in inline-code style; footnote references: superscript-styled `[label]` text in `link` color (no navigation); task-list markers: `☐`/`☑` (U+2610/U+2611) instead of the bullet; wikilinks: link color; `^sup^` and `~sub~`: normal text.

**4.4.4 Virtualization** (R-REN-21, D.4 rules 1–2): height estimates for not-yet-laid-out top-level blocks (document the formula; e.g. paragraphs: `ceil(Σ char_width_estimate / column_width) × line_pitch` with 0.5 em per Latin char and 1.0 em per CJK/emoji; code: lines × code pitch + paddings), a Fenwick tree over block heights, lay out only visible blocks ± one screen, keep layouts for ≤ 3 screens (LRU), **scroll anchoring** (top visible block + offset stays fixed when heights above change, on resize and when estimates are refined). Width change → invalidate layouts, re-estimate, keep the anchor.

**4.4.5 Rendering integration:** product code uses backend A from ADR-0002 (move the proven spike code into `yamdv-platform-win::render`, cleaned up and documented). Paint only invalid regions, band by band; scrolling via `ScrollWindowEx` + exposed-strip painting; per-monitor DPI relayout; no work when idle.

**4.4.6 Window and states:** system title bar text `<file name> — YAMDV` (custom title bar is Phase 004). **Empty state** (no file): centered 15 px `text_muted` text "Drop a Markdown file here or press Ctrl+O" (EN/TR) — the real welcome page comes in Phase 004. **Error state**: centered card with a title ("Cannot open file"), the reason and the path. **In-window confirm dialog** component per `UI_SPEC.md` §B.10.6 (first `yamdv-ui` component; keyboard: Tab, Enter = primary, Esc = cancel; focus ring per B.16), used by the guards in §4.2 item 2. Cursor: I-beam over text, arrow elsewhere.

### 4.5 Scrolling and scrollbar (R-REN-17, B.5.6, B.15)
Mouse wheel with smooth animation (120–180 ms ease-out, cumulative while notches arrive), precision touchpad (fractional deltas applied directly, no animation), keys per `docs/SHORTCUTS.md` §F.2 (`↑/↓`, `PgUp/PgDn`, `Space/Shift+Space`, `Home/End`, `Ctrl+Home/End`), overlay auto-hiding scrollbar (8 → 12 px on hover, fade after 1 000 ms idle, thumb drag, track paging). Animation timers run **only** while animating (T7). Reduced motion (`SPI_GETCLIENTAREAANIMATION` off) → instant scrolling and no fades (R-A11Y-02).

### 4.6 Selection and copy (R-REN-16, partial)
Drag selection with auto-scroll at the window edges, Shift+click extend, double-click word (`unicode-segmentation` word bounds), triple-click block, `Ctrl+A` (whole document, including blocks never laid out — via §4.3 item 6), `Ctrl+C` → `CF_UNICODETEXT` with these rules: CRLF line breaks; blank line between blocks; list items on their own lines prefixed with `• ` or `<n>. ` (indented 2 spaces per nesting level); code block lines verbatim; interim placeholders copy their source text. **CF_HTML and "Copy as Markdown" are Phase 003** (`// PHASE-003`). Selection survives scrolling and relayout (anchored to source offsets).

### 4.7 Opening files (R-OPEN-02/03/04/09/10/12)
- CLI: `yamdv a.md b.md` → one window per file **in the same process** (forwarding to an already-running instance is Phase 005). Unknown/invalid paths → error state window.
- `Ctrl+O` → `IFileOpenDialog` with the filters of R-OPEN-03 and multi-select; first file replaces the empty state of the current window, otherwise new windows. After the dialog closes: free transient buffers and trim the working set **once** (D.4 rule 10) — measure M6.
- Drag & drop (`DragAcceptFiles` + `WM_DROPFILES`; check `DragQueryFileW` lengths): first file opens in place, others in new windows; dropped folders → show the localized message "Drop Markdown files, not folders" in the empty/error state (folder support arrives with the Files tab in Phase 004).
- `Esc` (no dialog open) and `Ctrl+W` close the window; closing the last window exits.

### 4.8 Tests, fuzzing, goldens
- Unit tests for everything in §4.1–§4.6 with the mock shaper (block index, lazy detail trees, mapping, splice, estimates, Fenwick, anchoring, selection→text, plain-text rules, encodings, guards, INI, i18n completeness, contrast).
- **CommonMark conformance:** do **not** commit the official CommonMark spec file (third-party content, H12). Instead (a) write your own small vectors covering every CommonMark construct category (≥ 150 cases: each block type, each inline type, container nesting, laziness, tabs, entities, escapes, link reference forms, edge cases) and run them through load → index → detail trees → mock layout → display list, asserting block/inline kinds and source ranges against pulldown-cmark's own events; (b) as a one-off local check (downloaded on your machine, never committed, never at app runtime) run all official spec examples through the same pipeline and report "N examples, 0 panics/hangs".
- **Fuzzing** (S-TEST-01): create `fuzz/` with targets `parse_layout` (bytes → decode → index → all detail trees → mock layout at width 600 → display list) and `ini`; run each ≥ 10 minutes (`cargo +nightly fuzz`, record the nightly version); commit small seed corpora; every crash becomes a regression test.
- **Golden images** (H.4): implement `yamdv --render-png` (feature `dev-tools`, never in release) rendering headlessly through the real pipeline (offscreen band rendering into a full bitmap, PNG via WIC); `cargo xtask golden` compares (per-channel tolerance 16, max 0.5 % differing pixels) and `--update` rewrites. Goldens are generated on the Windows CI runner: run the job, download its artifact (`gh run download`), commit to `tests/golden/002/`. Required goldens at width 1000 and scales 1.0 and 1.5, Default light and dark: `typical-basic`, `headings`, `lists`, `quotes`, `code`, `unicode` (Turkish, CJK, Arabic, emoji ZWJ), `placeholders` (all §4.4.3 interim renderings), `plaintext`.
- New corpus files (original content only): `typical-basic.md` (~20 KB README-like: H1–H6, paragraphs, emphasis, nested lists incl. ordered with start number, nested quotes, inline code, 3 code blocks, HR, links, a table/images/math/footnote to exercise placeholders), `headings.md`, `lists.md`, `quotes.md`, `code.md` (plain), `unicode.md`, `placeholders.md`, `plain.txt`, and a generator for `large-1mb.md`.

### 4.9 Measurements (D.2, D.3 incl. P-05/P-09)
`memprobe` scenarios and CI gate (fail CI if a soft budget is exceeded):
| ID | Scenario | Soft budget |
|---|---|---|
| M0 | empty state, idle 3 s | ≤ 4.8 MiB |
| M1b | `typical-basic.md` | ≤ 8.0 MiB |
| M2 | `large-1mb.md` | ≤ 16 MiB |
| M6 | M1b, then open and cancel `Ctrl+O`, wait 2 s | ≤ M1b + 1 MiB |
| M7 | commit vs PWS for M0/M1b/M2 | ≤ 3 × PWS |
| T1b / T1L / T2 | first paint of `typical-basic.md` / `large-1mb.md` / empty state | ≤ 150 / 300 / 100 ms |
| T4 | scroll p99 (CI at 1000×700; owner kit at 1920×1080) | ≤ 16 ms |
| T7 | idle CPU 30 s, no active timers | ≤ 1 ms CPU |
| B-SIZE | release `yamdv.exe` | ≤ 4.0 MiB |
Also report the subsystem breakdown for M1b and M2 (`mem-stats` feature: source text, block index, detail-tree cache, layout cache, DirectWrite, band buffer).

### 4.10 Explicitly out of scope for 002
Tables/code highlighting/callouts/footnotes/images/HTML/math/Mermaid rendering (beyond §4.4.3), link clicking and hover status, custom title bar, sidebar, menus, settings UI and writing settings, themes other than Default, find, zoom, live reload, single-instance forwarding, history/favorites, editing, export. **Do not implement them.**

---

## 5. Requirement coverage for Phase 002
Update `docs/TRACEABILITY.md`:
- **DONE in 002:** R-MD-01, R-MD-10, R-MD-15, R-MD-16, R-MD-17, R-OPEN-02 (same-process windows; forwarding in 005), R-OPEN-03, R-OPEN-04 (except edit-mode insert and folder→Files tab), R-OPEN-06 (except "Reopen with encoding" menu), R-OPEN-07, R-OPEN-08, R-OPEN-09 (toggle in 004), R-OPEN-10 (detection; edit-mode handling in 006), R-OPEN-12, R-REN-01, R-REN-02, R-REN-05, R-REN-09, R-REN-17, R-REN-19, R-REN-20, R-REN-21, R-I18N-02, R-I18N-04, R-A11Y-04, S-FILE-04, S-FILE-07, S-PARSE-01, S-PARSE-02 (applicable parts), S-PARSE-04, S-PARSE-07, S-PARSE-08, M0, M6 (first measurement).
- **PARTIAL in 002:** R-MD-02 (strikethrough only), R-REN-03 (no highlight/sub/sup/kbd), R-REN-04 (no task checkboxes), R-REN-16 (plain text only), R-THEME-01, R-THEME-02 (data only), R-I18N-01, R-A11Y-02 (reduced motion for scrolling), R-SET-05 (read-only), S-TEST-01 (2 targets).

---

## 6. Acceptance criteria (Phase 002)
| ID | Criterion |
|---|---|
| AC-1 | F-01…F-08 done with evidence. |
| AC-2 | Appendix verification table (F-03) shows 8 × verified; P-01…P-09 applied by script, logged in `SPEC_CHANGES.md`; `spec-check` green. |
| AC-3 | All H9 gates green locally where possible and on CI, **including the memprobe soft-budget gate**; PR merged; tag `phase-002`. |
| AC-4 | Opening via CLI (multi-file), `Ctrl+O` (multi-select) and drag & drop works; every encoding fixture decodes correctly; size/binary guards and error states work (tests + goldens + owner-kit steps). |
| AC-5 | Goldens committed for all §4.8 documents (1.0/1.5, light/dark) and reviewed by you against `UI_SPEC.md` §B.6 — list every visible deviation you know of in the report. |
| AC-6 | Virtualization: `large-1mb.md` meets T1L and M2; scrolling top→bottom→top keeps anchors stable (automated test with the mock shaper + a CI autoscroll run). |
| AC-7 | Selection/copy rules of §4.6 covered by unit tests; `Ctrl+A`+`Ctrl+C` on `large-1mb.md` works within 300 ms. |
| AC-8 | Fuzz targets ran ≥ 10 min each with no open crash. |
| AC-9 | Measurement table §4.9 complete (MiB + bytes), with the M1b/M2 subsystem breakdown and deltas vs Phase 001. |
| AC-10 | `owner-kit` artifact available; `OWNER_KIT.md` in EN + TR. |
| AC-11 | Report follows `docs/PROCESS.md` exactly (see below). |

**Report** (`docs/reports/002_REPORT.md` + final chat message): use the template in `docs/PROCESS.md` **with every section present, in order**, plus the two mandatory additions from amendment A-2:
- `## 0. Fix items status` — table `F-ID | status | evidence`, placed before section 1.
- `## 3b. Spec patches applied` — table `P-ID | doc | verified by spec-check (yes/no)`, placed after section 3.
Sections that have nothing to report say "None." — never omit them. Section 13 (Manual QA checklist) must reference the owner-kit steps.

---

**END OF PROMPT 002.** Start with §2 (F-01, F-02, F-03) now.
