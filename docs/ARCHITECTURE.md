# APPENDIX E — Architecture (`docs/ARCHITECTURE.md`)

## E.1 Overview
```
 bytes on disk ──decode (encoding_rs)──► Source text (String / rope in edit modes)
                                              │  pulldown-cmark + YAMDV extensions
                                              ▼
                                        Block tree (flat arena, byte ranges into source)
                                              │  lazy, per visible block
                                              ▼
           TextShaper (trait) ◄──────── Layout engine ──────► Display list (draw ops)
           impl: DirectWrite                 │                        │
                                             │                        ▼
                         Math / Mermaid / Highlight            Renderer backend (ADR-0002)
                         (pure crates → display ops)                  │
                                                                      ▼
                                                                   Window
 Edits: Action → editor ops on Source → incremental reparse of affected blocks → relayout → partial repaint
```
Platform-independent logic lives in pure crates (testable on any OS, `#![forbid(unsafe_code)]`); all Win32/COM code lives in `yamdv-platform-win`.

## E.2 Repository layout
```
YAMDV/
├─ Cargo.toml                 # workspace, pinned deps, lints, profiles
├─ Cargo.lock
├─ rust-toolchain.toml
├─ .cargo/config.toml
├─ deny.toml  rustfmt.toml  clippy.toml  .editorconfig  .gitignore  .gitattributes
├─ README.md  CHANGELOG.md  LICENSE
├─ .github/workflows/ci.yml
├─ docs/
│  ├─ SPEC.md  UI_SPEC.md  SECURITY.md  PERFORMANCE.md  ARCHITECTURE.md
│  ├─ SHORTCUTS.md  ROADMAP.md  TESTING.md  THREAT_MODEL.md  TRACEABILITY.md
│  ├─ adr/       0000-…md, 0001-…md, …
│  ├─ plans/     001_PLAN.md, …
│  ├─ reports/   001_REPORT.md, …
│  └─ spike/     backend measurements + screenshots
├─ crates/
│  ├─ yamdv-core/         # source, encoding, md parsing adapter + extensions, block tree,
│  │                      # slugs, outline, link/resource policy (pure), stats, config model,
│  │                      # INI parser, i18n tables, action/keymap model
│  ├─ yamdv-layout/       # TextShaper trait, block layout, virtualization, hit testing,
│  │                      # display list, pagination (for PDF/print)
│  ├─ yamdv-highlight/    # lexers + token classes
│  ├─ yamdv-math/         # TeX parser, MATH-table reader, math layout → display ops
│  ├─ yamdv-mermaid/      # parsers, models, layouts → display ops
│  ├─ yamdv-editor/       # buffer, selections, undo, commands, snippets, table model
│  ├─ yamdv-ui/           # chrome widgets (title bar, sidebar, menus, settings, find, toasts,
│  │                      # viewers) as platform-independent state + paint + input handling
│  ├─ yamdv-export/       # HTML writer, PDF writer, CSV
│  ├─ yamdv-platform-win/ # ALL unsafe/Win32/COM: window, DPI, theme, DirectWrite shaper,
│  │                      # renderer backend, WIC, clipboard, dialogs, file I/O, watcher,
│  │                      # registry, shell/process, single instance, printing, IME, memory
│  └─ yamdv-app/          # bin "yamdv": composition root, app state, command dispatch, CLI
├─ xtask/                 # dev tasks: ci, check-imports, size, memprobe, gen-icons, golden
├─ spikes/                # Phase 001 backend spikes (not part of the product build)
├─ fuzz/                  # cargo-fuzz targets (added when the first parser lands)
└─ tests/
   ├─ corpus/             # Markdown test documents (Appendix H)
   └─ golden/             # golden PNGs for visual regression
```

## E.3 Dependency rules
```
yamdv-app ─► yamdv-ui ─► yamdv-layout ─► yamdv-core
    │            │            ├────────► yamdv-highlight
    │            │            ├────────► yamdv-math
    │            │            └────────► yamdv-mermaid
    │            └──────────► yamdv-editor ─► yamdv-core
    ├──────────► yamdv-export ─► yamdv-layout
    └──────────► yamdv-platform-win (implements traits defined in core/layout/ui)
```
- Pure crates never depend on `yamdv-platform-win` or the `windows` crate.
- Platform services are reached through traits (`TextShaper`, `ImageDecoder`, `Clipboard`, `FileSystem`, `Clock`, `Shell`…) so pure logic is unit-testable with fakes.
- No cyclic dependencies; no "utils" grab-bag crate.

## E.4 Document model
- `Document { source, encoding, line_endings, bom, path, disk_state(size, mtime, hash) }`.
- **Two-level document model.** (1) At load, one full pulldown-cmark pass builds the **top-level block index**: for every top-level block its kind, source byte range (`u32`) and height-estimate inputs, plus document-wide facts collected on the way — headings (level, source range, text, slug), link reference definitions and footnote definitions (stored in maps), and the front-matter range. Inline events are not kept. (2) A top-level block's **detail tree** (nested blocks in a flat arena with `parent`/`children` ranges and kind-specific data — heading level + slug, list info, code language, table alignments, callout kind… — plus compact inline runs referencing source ranges; owned strings only where unescaping requires) is built **lazily** by re-parsing exactly that block's source slice. Top-level blocks are self-contained in CommonMark once reference and footnote definitions are supplied from the document maps (pulldown-cmark's broken-link callback). Detail trees live in an LRU cache with a byte budget and are dropped when far off-screen.
- Extensions (autolink literals, `==highlight==`, callout detection, wikilinks, emoji, front matter, sanitized HTML) run as passes on the pulldown-cmark event stream (`into_offset_iter`) so every element keeps its source range — required for F2/F3 mapping.
- **Incremental reparse** (designed in Phase 002, used from Phase 006): after an edit, reparse from the start of the top-level block containing the edit until the new block boundaries re-synchronize with the old ones; splice the arena; global constructs (link reference definitions, footnote definitions, front matter) trigger an inline re-resolution pass rather than a full block reparse.

## E.5 Layout
- `TextShaper` builds paragraph layouts from styled runs and a width, and answers metrics + hit tests; the DirectWrite implementation wraps `IDWriteTextLayout` (wrapping, bidi, fallback, hit testing, inline objects for images/math/checkboxes).
- Block layout computes boxes top-down; heights of unseen blocks are estimates refined when laid out; a Fenwick (binary indexed) tree over block heights maps scroll offset ↔ block in O(log n). Scroll anchoring keeps the top visible block stable when heights above it change.
- Output is a **display list**: `Rect`, `RoundRect`, `Line`, `Path` (lines/quad/cubic), `Text(layout handle, origin, color overrides)`, `GlyphRun` (math), `Image(handle, rect)`, `PushClip/PopClip`, `Opacity`. The same display list feeds the screen renderer, the print path and the PDF writer (via pagination).

## E.6 Rendering
Backend per ADR-0002. Requirements regardless of backend: dirty-rect invalidation; scrolling by pixel reuse + exposed-strip painting; overlays (find bar, toasts, badges, viewers) drawn as top layers of the same window; menus, popovers and tooltips as separate layered popup windows; correct per-monitor DPI; no work when nothing changed.

## E.7 UI layer and commands
- Small retained model for chrome components (title bar, sidebar, panels, popups) with explicit state, painting into display lists, and input handling (hover, press, capture, focus, keyboard navigation).
- **Every user-visible operation is an `Action`** (enum with a stable snake_case id, e.g. `toggle_sidebar`, `live_edit`, `export_pdf`). Menus, shortcuts, buttons and CLI map to Actions. The keymap = preset + user overrides; conflicts detected; Actions have enable/visibility predicates. This makes rebinding (Appendix F) and testing trivial.

## E.8 Platform layer (`yamdv-platform-win`) modules
`window` (class, custom frame, hit testing, DPI) · `theme` (app mode, high contrast, reduced motion, accent) · `text` (DirectWrite factory, font collection, `TextShaper` impl, color glyph runs, MATH table access) · `render` (backend) · `image` (WIC decode at size, probing) · `clipboard` (`CF_UNICODETEXT`, `CF_HTML`, `CF_DIB`/PNG) · `dialogs` (`IFileOpenDialog`, `IFileSaveDialog`) · `fs` (read, atomic save, path validation helpers, MotW read) · `watch` (`ReadDirectoryChangesW` on the watcher thread, overlapped I/O, clean cancellation) · `registry` · `shell` (`ShellExecuteExW`, `CreateProcessW`, `SHOpenFolderAndSelectItems`) · `instance` (mutex + `WM_COPYDATA`) · `print` (`PrintDlgExW`, GDI printing) · `ime` (Imm32) · `memory` (working-set queries, trim) · `timers` (only while animating) · `uia` (Phase 012).

## E.9 Lints and code standards (workspace)
```toml
# Pure crates use `[lints] workspace = true`. Cargo does not allow partial overrides, so
# yamdv-platform-win, xtask and spikes declare their own complete [lints] tables with
# unsafe_code = "deny" and allow it only in the modules that need it (every block has a SAFETY comment).
[workspace.lints.rust]
unsafe_code = "forbid"
missing_docs = "warn"
unused_must_use = "deny"

[workspace.lints.clippy]
unwrap_used = "deny"
expect_used = "deny"
panic = "deny"
todo = "deny"
unimplemented = "deny"
dbg_macro = "deny"
print_stdout = "deny"             # allowed only in the CLI output module
print_stderr = "deny"
undocumented_unsafe_blocks = "deny"
indexing_slicing = "warn"
cast_possible_truncation = "warn"
cast_sign_loss = "warn"
large_stack_arrays = "warn"
large_types_passed_by_value = "warn"
```
Tests may opt out locally with `#[allow(...)]` at the test module level.
Hard rule H14 (`docs/PROCESS.md`) is amended by A-1: `spikes/` may use `unsafe` (each block with a `// SAFETY:` comment); `spikes/` is never part of the product build.

## E.10 Threading
- **UI thread:** everything interactive.
- **Watcher thread:** blocks on `ReadDirectoryChangesW`; posts a custom window message on change (debounced 50 ms on the UI side).
- **Worker thread (on demand):** PDF/HTML export, printing preparation, very large layouts; results posted back; cancellable; created when needed and exits when idle for 5 s.
- Communication via `PostMessageW` + a lock-free or mutex-protected queue. No async runtime.

## E.11 Errors and logging
Own error enums per crate (no `anyhow`/`thiserror` in product crates unless an ADR approves). User-facing errors become toasts/banners with localized messages. Internal logger: compile-time levels, off by default, writes to `%LOCALAPPDATA%\YAMDV\logs\` when enabled.

## E.12 Persistence formats
- **INI dialect** (own parser, ADR): UTF-8 (BOM tolerated), `[section]`, `key = value`, `#`/`;` comments, quoted strings with `\" \\ \n \t \u{…}` escapes, booleans `true/false`, repeated keys for lists. Unknown keys preserved on rewrite where possible.
- `settings.ini` per Appendix B §B.8.2.
- `state.ini`: `[window]` geometry; `[history]` entries `item = <unix_ms>|<path>`; `[favorites]` `item = <path>`; `[positions]` `item = <unix_ms>|<block_index>|<offset_ppm>|<path>`; `[toc]` `item = <path>|<collapsed ids…>`. Caps per R-NAV/R-READ; LRU eviction.
- All writes atomic (S-FILE-03), debounced (R-REL-02).

## E.13 Internationalization
`enum Msg { … }` + `fn tr(lang, Msg) -> &'static str` generated from two tables (EN, TR) in source; a unit test asserts completeness; placeholders `{0}`; plural helper for "1 word / 2 words" (Turkish has no plural agreement after numbers: "2 kelime").

## E.14 Future portability
Only `yamdv-platform-win` and the thin wiring in `yamdv-app` are Windows-specific. A future macOS/Linux port would add another platform crate implementing the same traits; no pure crate may assume Windows.
