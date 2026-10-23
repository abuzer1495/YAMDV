# Changelog

All notable changes to this project are documented in this file.
The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [1.3.0] - 2026-10-23

V2-E editing plus (prompt 015) and V2-F WYSIWYG Live Edit with table editing (prompt 016).

### Added
- Live Edit (F2) is WYSIWYG (V2-F, FUT-43, ADR-0020): the page stays rendered and Markdown
  markers appear muted only around the caret; code blocks show their fences with a language
  input and autocomplete; math and Mermaid get a live preview card (last good diagram kept).
- Typing transforms in F2: `# `…`###### `, `- `/`* `/`+ `, `1. `, `[ ] `/`[x] `, `> `,
  ```` ``` ````/`$$`/`---` + `Enter`, and a pipe header row + `Enter` → table; one `Ctrl+Z`
  leaves the typed literal.
- F2 links: `Ctrl+K` popover (text, URL, title), `Ctrl+click` follows; cross-block selection
  with Markdown + HTML + plain-text clipboard; checkbox click toggles; context menu Format ▸,
  Insert ▸ (13 items) and Table ▸.
- Table editing (V2-F, FUT-44): in-cell editing with `Tab`/`Shift+Tab`/`Enter`/`Shift+Enter`
  (`<br>`), `|` escaped automatically; a tool strip (rows, columns, alignment, sort, move,
  delete, ⛶, CSV), `Ctrl+Alt+arrows` and `Ctrl+Shift+Backspace`; row and column drag grips;
  a 10×10 insert picker (`Ctrl+T`, Insert ▸ Table…); TSV/CSV paste (e.g. from Excel) fills
  cells or offers "Convert to table"; edited tables are pretty-aligned
  (`editing.auto_format_tables`, East-Asian width aware).
- Insert image (`Ctrl+Shift+I`, rebindable) in F2 and F3.
- F2 format toolbar above a selection (V2-E, FUT-45; `editing.format_toolbar`).
- Find & replace (`Ctrl+H`) with optional regex (`regex-lite`, ADR-0019), `$1`/`${name}`,
  Replace all as one undo step, Turkish-aware case folding for plain search.
- Tab snippets (`editing.snippets`) and editor extras: duplicate line, select line, move lines,
  insert table/code block, auto-pair (FUT-12).
- Rich paste and drop (FUT-15): HTML is converted to Markdown through an allowlist; pasted or
  dropped images are saved under `assets/<doc>/`.
- Recovery journal: unsaved edits survive a crash and are offered on the next open.

### Changed
- Blocks and tables the user did not touch are never re-serialized; EOL style and BOM are kept.
- `cargo xtask size-map` lists the largest exe-size contributors from the linker map.

## [1.2.0] - 2026-10-23

V2-C tooltips, banners and viewers (prompt 013) and V2-D navigation (prompt 014).

### Added
- Files tab (V2-D, FUT-33): a lazy tree of the Markdown files under the document's folder
  (folders first, natural order, worker-thread listing, 5 000 entries / depth 8, hidden,
  system, build and reparse-point folders skipped, no network crossing), watched for changes;
  a dropped folder opens it; `Ctrl+Shift+1…4` switch tabs; `sidebar.show_files`.
- Floating sidebar (FUT-02): `left-floating` / `right-floating` cards over the page without
  reflow; Appearance ▸ Sidebar position ▸ Floating.
- Favorites reorder (FUT-03): drag with a drop line, or `Alt+↑/↓` on a focused favorite.
- Go to heading (FUT-08): `Ctrl+Shift+O` palette with fuzzy filtering.
- `[TOC]` / `[[_TOC_]]` render a linked "Contents" list (also in HTML export, PDF and Print).
- Mind map (FUT-37): `Ctrl+Shift+M` or Export ▸ View as mind map; right or balanced layout,
  zoom, pan, collapse, double-click to jump, copy as PNG.
- Keyboard reach (HARD-04): every interactive element is in the `Tab` order with a focus ring
  after keyboard use; `F6` / `Shift+F6` cycle title bar, banners, sidebar and document.
- Tooltips (V2-C, FUT-40): link titles, `<abbr>` and image titles, footnote previews, the
  word-count badge, title-bar and viewer buttons; 500 ms delay, also on keyboard focus; a link
  status pill at the bottom left.
- Banners (FUT-40/34): edit conflict, deleted file, read-only, decoding, Mark of the Web,
  elevated, export failure; at most two, newest on top; content keeps its anchor.
- Anchored Ask popover for external links (FUT-41 / R-LINK-04) with IDN and credential warnings.
- Fullscreen viewers (FUT-36, FUT-06): images, Mermaid diagrams and tables via a ⛶ badge or a
  double-click; zoom at the cursor (10–800 %), pan, fit / 1:1, copy (bitmap, PNG, CSV).
- Heading folding (FUT-34): ▸/▾ on H1–H3, Alt+click folds a level, find expands, session only.
- Sup/sub and footnote references as inline objects (FUT-41).

### Fixed
- Wide flowcharts keep ≥ 10 px text and scroll horizontally, also in HTML export (BUG-08).

### Changed
- The edit conflict and the external-link Ask are no longer modal dialogs.

## [1.1.0] - 2026-10-22

V2-A window chrome (prompt 011) and V2-B settings panel (prompt 012). Not tagged yet: the
`v1.1.0` tag waits for the owner test (A-8).

### Added
- Window chrome (V2-A, FUT-30/32/35/46): custom 32 px title bar with ⚙ / ⋯ and Min/Max/Close,
  layered popup menus with keyboard navigation, color-scheme cards, F11 full screen with a
  reveal strip, optional word-count badge and a zoom pill.
- Settings panel (V2-B, FUT-31): ⚙ / Ctrl+, open an in-window panel with 12 sections (General,
  Appearance, Typography, Sidebar, Reading, Editing, Shortcuts, External editor, Export, Files,
  Privacy, About); every change is
  live and saved to `settings.ini`; Reset section / Reset all; full keyboard path.
- Settings search (FUT-26, first half): filters rows by label and description in the UI
  language.
- Font pickers (FUT-49): body / heading / code font lists with live preview and the resolved
  fallback font shown under each row.
- Custom-color editor (FUT-04): per-token light/dark swatches with a hex popover, written to
  `custom-theme.ini`; "Use custom colors" switch.
- Shortcuts (FUT-05): every action can be rebound by capture, with conflict detection, a
  Typora preset and Reset; the Help page lists the active bindings.

### Changed
- The ⚙ button and Ctrl+, open the settings panel instead of the Settings submenu; the old
  "Body font…" dialog is replaced by the Typography section.
- The diagnostic log switch (Privacy) applies at once.

### Fixed
- BUG-10: PDF / print / HTML export use only the export color setting (default Light), never
  the dark UI palette, selection or find highlights.
- BUG-11: background PDF / HTML export runs in a worker process with page progress and
  Cancel; the window stays responsive and a cancelled job leaves no file behind.

## [1.0.0] - 2026-10-17

First complete release (owner decision 2026-10-17: v1 scope; deferred items are in FUTURE.md).

### Added
- Reading: CommonMark + GFM with callouts, footnotes, task lists, `<details>`, tables, 30 code
  lexers, native math (core TeX subset) and Mermaid flowchart + sequence diagrams, images
  (PNG, JPEG, GIF first frame, BMP) decoded under Arbitrary Code Guard.
- Shell: welcome page, sidebar outline / favorites / history, find bar, 11 color schemes with
  light/dark/system mode, `custom-theme.ini` hot reload, EN/TR UI, single instance, file watcher.
- Editing: F2 Live Edit and F3 Split Edit with undo/redo, formatting shortcuts, atomic saves,
  autosave, conflict handling; F4 opens an external editor.
- Export (V1-D): self-contained static HTML (no script, strict CSP, images/math/diagrams as
  `data:` URIs); Print… (Ctrl+P) and Export PDF… (Ctrl+Shift+E) through the print path
  ("Microsoft Print to PDF"); Copy / Export table as CSV (UTF-8 BOM, CRLF, RFC 4180, formula
  guard); headless `--export-html` / `--export-pdf`.
- Packaging (V1-D): `--install` / `--uninstall` per user (Start-menu shortcut, Apps & features
  entry, registration), welcome-page "Install YAMDV for this user", portable mode via
  `yamdv.portable`; version 1.0.0 in the version resource and About.

### Changed
- F-22: every measurement gate fails only on two consecutive failing attempts.

## Phase history (before 1.0.0)

### Added
- Phase 002 (reading MVP):
  - Settings model with INI read and write, EN/TR strings, an internal log, and theme data for all 11 schemes.
  - Defensive path checks, size and binary guards, and encoding detection (BOM, UTF-8, chardetng).
  - Two-level document model: a block index plus lazily parsed detail trees in an LRU cache, with splice support (ADR-0007).
  - Layout ↔ source mapping and the copy-text rules.
  - DirectWrite text shaping and rendering backend A, with virtualized layout and scroll anchoring.
  - Plain-text mode and interim placeholders for constructs that are rendered from Phase 003 on.
  - Reading window: open via CLI (multi-file), Ctrl+O and drag & drop; in-window confirm dialog; smooth scrolling with an overlay scrollbar; selection and copy.
  - Tooling: `--render-png` (dev-tools), `cargo xtask golden` and `gen-large`, memprobe `--size`/`--open-dialog`/`--cpu-ms` with budget gates, fuzz targets, owner kit, spec-check and spec-patch.
- ADR-0007…0012. Arbitrary Code Guard enabled (ADR-0001 amendment).

### Changed
- Memory and size numbers are in MiB with raw bytes (P-05). The B-SIZE soft budget is 4.0 MiB.

### Added (Phase 001)
- Phase 001: repository, Cargo workspace, documentation set (specification appendices A–H),
  `cargo xtask` tooling (`ci`, `check-imports`, `size`, `memprobe`, `gen-icons`), CI workflow,
  native window skeleton (`yamdv.exe`), rendering-backend spike and ADR-0002 (backend A: DirectWrite GDI interop + tiny-skia).
- THREAT_MODEL skeleton, TRACEABILITY matrix, ADR-0000/0001/0003–0006.
