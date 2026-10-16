# APPENDIX A — Product Specification (`docs/SPEC.md`)

## A.0 Vision and principles
YAMDV is a reading-first Markdown viewer with in-place WYSIWYG editing for Windows. Double-click a `.md` file → it is on screen, rendered like a clean web page, in well under a second. `Esc` closes it like a photo viewer. `F2` edits in place, `F3` edits side by side.

- **P1 Reading first** — the document is the UI. No toolbars, no status bars, no panels the user didn't ask for.
- **P2 Instant** — open, scroll, switch, type: no perceptible delay.
- **P3 Tiny** — 3–5 MB private working set, ≤ 3 MB exe (Appendix D).
- **P4 Offline & private** — no network, no accounts, no telemetry. Files never leave the machine.
- **P5 Never surprise-modify files** — the app writes only what the user edits, only where expected, byte-identical elsewhere.
- **P6 Keyboard-complete** — everything reachable by keyboard; shortcuts rebindable.
- **P7 Faithful rendering** — CommonMark + GFM + the common extensions people actually use (callouts, math, Mermaid, front matter, wikilinks, highlight).

**Non-goals (v1):** vault/knowledge-base features, sync, plugins/extension API, tabs inside a window, collaboration, DOCX/EPUB/LaTeX export, non-Markdown formats (except `.txt` as plain text), spell checking, macOS/Linux/Android, auto-update, licensing/trial, feedback/telemetry, Microsoft Store packaging.

**Explicitly excluded mdview features (they are online or commercial):** update checks and auto-update, anonymous startup report, Pro licensing/trial counter/"Maybe Later" prompt, device ID, invite codes, payment, in-app feedback form and reply cards, cloud-synced read state, engine downloads (our engines are built in), VS Code/Sublime live-preview plugins (possible future work, local-only), Android/macOS/Linux builds, Microsoft Store edition.

Requirement keywords: **MUST** (v1), **SHOULD** (v1 unless a phase prompt defers it), **MAY** (only when a phase prompt asks).

---

## A.1 Markdown dialect (R-MD)
- **R-MD-01 MUST** CommonMark 0.31 complete (via pulldown-cmark).
- **R-MD-02 MUST** GFM: tables with column alignment, task lists, strikethrough (`~~x~~`), footnotes, GFM alerts (`> [!NOTE]`, `[!TIP]`, `[!IMPORTANT]`, `[!WARNING]`, `[!CAUTION]`).
- **R-MD-03 MUST** GFM *autolink literals* (`https://…`, `http://…`, `www.…`, bare e-mail addresses) following the GFM spec's trailing-punctuation rules. pulldown-cmark does not do this — implement a linkify pass over text events (never inside code, links, or HTML).
- **R-MD-04 MUST** Obsidian-style callouts: `> [!type] Optional title` with optional fold marker `[!type]-` (collapsed) / `[!type]+` (expanded). Type mapping (case-insensitive) → 5 visual styles:
  - *note*: note, info, abstract, summary, tldr, todo, quote, cite
  - *tip*: tip, hint, success, check, done, example
  - *important*: important, question, help, faq
  - *warning*: warning, attention
  - *caution*: caution, danger, error, failure, fail, missing, bug
  Unknown type → *note* style showing the type name capitalized as title.
- **R-MD-05 MUST** `==highlight==` (custom inline extension; not inside code).
- **R-MD-06 MUST** Superscript `^x^` (pulldown-cmark option). Subscript with single tildes `~x~` is **off by default** (setting `reading.subscript_tilde`), because GFM renders `~x~` as strikethrough; when it is off, both `~x~` and `~~x~~` are strikethrough. `<sub>` and `<sup>` HTML always work (R-MD-14).
- **R-MD-07 MUST** Math: inline `$…$`, display `$$…$$` (pulldown-cmark `ENABLE_MATH`), and ` ```math ` fences. `\$` is a literal dollar.
- **R-MD-08 MUST** Mermaid: ` ```mermaid ` fences.
- **R-MD-09 MUST** Front matter: YAML (`---` … `---`) and TOML (`+++` … `+++`) at the very start of the file. Never rendered as headings/rules; shown as a metadata card (R-REN-13).
- **R-MD-10 MUST** Heading IDs: GitHub-compatible slugs (Unicode-lowercase; remove characters that are not letters, numbers, spaces, `-`, `_`; spaces → `-`; keep non-ASCII letters such as `ç ğ ı ö ş ü`; duplicates get `-1`, `-2`, …) and explicit `{#custom-id}` heading attributes.
- **R-MD-11 SHOULD** Definition lists (pulldown-cmark option).
- **R-MD-12 MUST** Wikilinks: `[[Page]]`, `[[Page|alias]]`, `[[Page#Heading]]`, `[[#Heading]]`, and embeds `![[image.png]]`, `![[image.png|300]]` (width). Resolution: R-LINK-03.
- **R-MD-13 SHOULD** Emoji shortcodes `:smile:` (GitHub set, compile-time table; not inside code/URLs).
- **R-MD-14 MUST** Raw HTML is never executed. A **safe subset** is rendered natively; everything else is sanitized (rules in `S-HTML-*`). Allowed tags: `b strong i em u ins s del strike mark sub sup small kbd code var samp abbr br hr p div span center h1–h6 blockquote ul ol li dl dt dd table thead tbody tfoot tr th td pre a img details summary picture source(srcset ignored) figure figcaption`. Allowed attributes: `align` (p, div, h1–h6, img, td, th), `width`/`height` (img), `src`/`alt`/`title` (img), `href`/`title` (a), `open` (details), `colspan`/`rowspan` (td, th), `style` restricted to `color`, `background-color`, `text-align`. HTML comments are hidden (including multi-line). Entities decoded. README patterns like `<p align="center"><img …></p>` MUST render centered.
- **R-MD-15 MUST** Hard breaks: two trailing spaces, backslash. Setting "Single newline as line break" (default **off**).
- **R-MD-16 SHOULD** Smart punctuation (setting, default off).
- **R-MD-17 MUST** Indented code blocks; tabs expand to 4 columns inside code.
- **R-MD-18 MUST** Reference-style links/images; link titles shown as tooltips.
- **R-MD-19 MAY** `[TOC]` / `[[_TOC_]]` placeholder renders an inline table of contents.

## A.2 Opening files (R-OPEN)
- **R-OPEN-01 MUST** After association, double-clicking a `.md`/`.markdown` file in Explorer opens it rendered.
- **R-OPEN-02 MUST** CLI `yamdv <path> [<path>…]`: one window per path (Appendix F).
- **R-OPEN-03 MUST** `Ctrl+O` open dialog. Filters: *Markdown* (`*.md;*.markdown;*.mdown;*.mkd;*.mkdn;*.mdwn;*.mdtxt;*.mdtext`), *Text* (`*.txt`), *All files*. Multi-select: first file replaces the current document if the window shows the welcome page, otherwise each opens in its own window.
- **R-OPEN-04 MUST** Drag & drop files onto a window: first file opens **in place**, others in new windows. In edit modes, dropping an image/file inserts it (R-EDIT-08). Dropping a folder SHOULD open the Files tab rooted at it.
- **R-OPEN-05 MUST** Optional Explorer context-menu entry "Open with YAMDV" (R-SYS-03).
- **R-OPEN-06 MUST** Encoding detection: BOM (UTF-8, UTF-16LE/BE) → that encoding; else valid UTF-8 → UTF-8; else detect with `chardetng` (expect windows-1254 Turkish, windows-1252, GB18030/GBK, Shift_JIS, EUC-KR, Big5, windows-1251, ISO-8859-x) and decode with `encoding_rs`. The detected encoding is shown in the document info (word-count badge tooltip / About-document info). SHOULD offer "Reopen with encoding ▸" in the menu.
- **R-OPEN-07 MUST** Line endings (LF / CRLF / mixed) are detected and preserved on save; newly inserted lines use the file's dominant ending.
- **R-OPEN-08 MUST** Size guards: > 20 MB → prompt "Large file — open as plain text (fast) / render anyway / cancel"; > 256 MB → refuse with message. NUL bytes in the first 8 KB → warn "This looks like a binary file" with "Open anyway".
- **R-OPEN-09 MUST** Any extension passed explicitly (CLI/drag/dialog) is rendered as Markdown; `.txt` SHOULD render as plain text with a "Render as Markdown" toggle.
- **R-OPEN-10 MUST** Read-only files open normally; edit modes are disabled with a "Read-only" pill and "Save a copy as…".
- **R-OPEN-11 MUST** If the open file is deleted/moved: banner "File was deleted or moved" with [Close] and, if there are unsaved edits, [Save as…]. If it reappears (atomic-save editors), reload silently.
- **R-OPEN-12 MUST** Long paths (> 260 chars) and UNC paths that the **user** opened directly are supported; displayed without `\\?\` / `\\?\UNC\` prefixes.
- **R-OPEN-13 MUST** Every successful open is recorded in History (R-NAV-08).

## A.3 Windows and instances (R-WIN)
- **R-WIN-01 MUST** One process, multiple top-level windows (one document per window). A second launch forwards its arguments to the running instance (`WM_COPYDATA`, validated per `S-IPC-*`) and exits within 50 ms.
- **R-WIN-02 MUST** One window per file: opening an already-open file activates its window (restore if minimized; `FlashWindowEx` if focus stealing is prevented).
- **R-WIN-03 MUST** Window geometry (size, position, maximized, monitor) is remembered; restored geometry is validated against current monitors (never off-screen; falls back to the primary monitor). Additional windows cascade by 28 px.
- **R-WIN-04 MUST** Close with `Esc` (read mode, no overlay/popup open) or `Ctrl+W`. Unsaved changes are never lost (autosave or prompt).
- **R-WIN-05 MUST** Title bar text: file name (default) or full path (setting) + `●` when dirty + mode pill (`Live Edit`, `Split Edit`, `Read-only`).
- **R-WIN-06 MUST** `F11` distraction-free fullscreen (hides title bar and sidebar; `Esc`/`F11` exits).
- **R-WIN-07 MUST** The process exits when the last window closes; all pending state writes are flushed first.

## A.4 Rendering (R-REN) — visual details in Appendix B
- **R-REN-01 MUST** Reading view: one centered column, web-page-like, no chrome.
- **R-REN-02 MUST** Headings H1–H6 (styles per theme), anchors per R-MD-10.
- **R-REN-03 MUST** Paragraphs, emphasis, strong, strikethrough, highlight, sub/superscript, inline code, `<kbd>`.
- **R-REN-04 MUST** Lists: bullets by depth `•` `◦` `▪` (repeat); ordered lists honor the start number (decimal at all depths); tight vs loose spacing; task-list checkboxes (rendered, not interactive in read mode unless the setting allows).
- **R-REN-05 MUST** Blockquotes, nested (one bar per level).
- **R-REN-06 MUST** Callouts (R-MD-02/04) with icon + title row; foldable ones toggle on title click.
- **R-REN-07 MUST** Tables: alignment, header style, borders, inline formatting in cells, wrapping long cell text (break anywhere if needed — never overflow the column), optional zebra rows, hover toolbar with ⛶ fullscreen (R-READ-08) and CSV export (R-EXP-04).
- **R-REN-08 MUST** Code blocks: syntax highlighting (R-CODE), hover toolbar.
- **R-REN-09 MUST** Horizontal rules.
- **R-REN-10 MUST** Footnotes: superscript reference `[n]` → jumps to note; notes section at the end with `↩` back-links. SHOULD show a hover tooltip with the footnote text.
- **R-REN-11 MUST** Images per R-IMG.
- **R-REN-12 MUST** Links styled per theme; hovering shows the destination in a status tooltip at the window's bottom-left (like a browser).
- **R-REN-13 MUST** Front matter card: collapsed by default (one muted line "Front matter · N fields"), expandable to a key/value list (setting: collapsed / expanded / hidden).
- **R-REN-14 MUST** `<details>/<summary>` render as collapsible sections.
- **R-REN-15 MUST** Math (R-MATH) and Mermaid (R-MMD) blocks render natively.
- **R-REN-16 MUST** Text selection: drag, Shift+click, double-click word, triple-click paragraph, `Ctrl+A`; selection spans blocks. `Ctrl+C` puts plain text **and** HTML (`CF_HTML`) on the clipboard; menu "Copy as Markdown" copies the source slice.
- **R-REN-17 MUST** Scrolling: wheel and precision touchpad (fractional deltas), `PgUp`/`PgDn`/`Space`/`Shift+Space`, `Home`/`End`, arrows; smooth (setting); overlay auto-hiding scrollbar (Appendix B §B.5.6). Middle-click autoscroll MAY.
- **R-REN-18 MUST** Incremental re-render: only changed blocks are re-parsed/re-laid-out; no full-document flicker.
- **R-REN-19 MUST** Bidi: paragraph direction from the first strong character; RTL paragraphs right-aligned; mixed runs correct.
- **R-REN-20 MUST** Color emoji, emoji modifiers and ZWJ sequences; font fallback for every script (DirectWrite system fallback).
- **R-REN-21 MUST** Virtualized layout: only visible blocks (plus a small margin) are laid out; the rest are height estimates refined lazily, so 1 MB documents open instantly.

## A.5 Code blocks (R-CODE)
- **R-CODE-01 MUST** Built-in highlighter (no downloads, no runtime grammar files): small hand-written lexers for at least: `bash/sh/shell/zsh/console`, `powershell/ps1`, `bat/cmd`, `c`, `cpp/c++/h/hpp`, `csharp/cs`, `java`, `kotlin/kt`, `go`, `rust/rs`, `python/py`, `javascript/js/mjs/cjs`, `typescript/ts`, `jsx`, `tsx`, `json/jsonc/json5`, `yaml/yml`, `toml`, `ini/conf/properties`, `xml/html/svg/xaml/vue (template only)`, `css/scss/less`, `sql`, `markdown/md`, `diff/patch`, `dockerfile`, `makefile`, `php`, `ruby/rb`, `swift`, `lua`, `dart`, `r`, `scala`, `perl`, `haskell/hs`, `elixir/ex`, `nginx`, `graphql/gql`, `protobuf/proto`, `regex`. Unknown → plain. Token classes: keyword, string, number, comment, function, type, constant/builtin, operator, punctuation, tag, attribute, variable, meta/preprocessor, diff-add, diff-del, heading (md).
- **R-CODE-02 MUST** Hover toolbar (top-right inside the block): **Copy** (raw code → clipboard, toast "Copied"), **Line numbers** toggle, **Wrap** toggle, **language label**. Per-block toggles; global defaults in Settings. Line numbers are drawn, never part of copied text.
- **R-CODE-03 MUST** Unwrapped long lines scroll horizontally per block (Shift+wheel, touchpad, thin always-visible scrollbar when overflowing).
- **R-CODE-04 MUST** Whitespace preserved; tab width 4; final newline trimmed.
- **R-CODE-05 MUST** `diff` blocks: `+` lines green background, `-` lines red background, `@@` lines muted.
- **R-CODE-06 MUST** Very long blocks (> 5 000 lines) are virtualized like the document.

## A.6 Images (R-IMG)
- **R-IMG-01 MUST** Formats: PNG, JPEG, GIF (first frame; animation behind a setting, default off), BMP, ICO, TIFF, WebP (if the OS codec exists; otherwise placeholder "WebP codec not installed"). SVG: SHOULD, via a small native, sandboxed renderer decided by ADR in the phase that implements it (no external references, no scripts, bounded size).
- **R-IMG-02 MUST** Path resolution: relative to the document's folder (`./`, `../` allowed), absolute local paths, `file:///C:/…` (local only), percent-decoding (`%20`), backslashes accepted. Remote (`http/https`) images are **never fetched** (placeholder). Network locations are blocked per `S-RES-02`. `data:` URIs allowed for image MIME types within limits (`S-RES-04`).
- **R-IMG-03 MUST** Size: 1 image pixel = 1 logical px, scaled down to the column width (aspect preserved); HTML `width`/`height` and `![[x.png|300]]` honored; never upscaled beyond 100 % automatically.
- **R-IMG-04 MUST** Lazy: header-only probe for dimensions at layout time; full decode only when visible, **decoded at display size**; LRU cache with a byte budget (Appendix D).
- **R-IMG-05 MUST** Hover ⛶ badge → fullscreen image viewer (R-READ-08).
- **R-IMG-06 MUST** Placeholders: *missing* (icon + alt + path), *blocked remote* ("Remote image not loaded — YAMDV works offline" + [Copy URL]), *blocked network path*, *too large*, *unsupported format*.

## A.7 Links (R-LINK)
- **R-LINK-01 MUST** `#anchor` links smooth-scroll to the heading/footnote/`id`.
- **R-LINK-02 MUST** Relative Markdown links (`./a.md`, `../b.md#section`, percent-encoded or Unicode names) open **in the current window**, pushing a navigation history entry; `Alt+←` / `Alt+→` and mouse X1/X2 navigate back/forward with per-entry scroll restore. `Ctrl+click` opens in a new window. Missing target → toast "Link target not found: <name>".
- **R-LINK-03 MUST** Wikilink resolution: `Page` → `Page.md` (then `Page.markdown`) in the document's folder, then recursively in subfolders (max depth 4, first match by shortest path); `#Heading` resolved by slug or case-insensitive heading text.
- **R-LINK-04 MUST** `http`, `https`, `mailto` links follow the setting **Open external links**: *Ask* (default — popover showing the full URL with [Open in browser] [Copy link] [Cancel]), *Open directly*, *Never (copy only)*. Opening uses `ShellExecuteExW` with the validated URL only.
- **R-LINK-05 MUST** Other local targets: folders → "Show in Explorer?"; document types on the allowlist in `S-LINK-03` → "Open with default app?" confirmation; everything else (executables, scripts, shortcuts…) is **never** launched — show "Blocked for safety" with [Show in Explorer] (which selects the file, never runs it).
- **R-LINK-06 MUST** Any other scheme (`javascript:`, `file://host/`, `vbscript:`, `data:` as link, `ms-*`, `search-ms:`, custom protocols…) is inert: tooltip "Blocked link type".
- **R-LINK-07 MUST** Right-click on a link → "Copy link address".

## A.8 Sidebar / navigation (R-NAV)
- **R-NAV-01 MUST** Sidebar with tabs **Contents · History · Favorites · Files**. History, Favorites and Files are individually hideable (Settings); if all three are hidden, the tab row disappears (pure outline).
- **R-NAV-02 MUST** Positions: *Left docked* (default), *Right docked*, *Left floating*, *Right floating* (floating = card overlaying the page with shadow, 12 px from edges).
- **R-NAV-03 MUST** Toggle via the chevron button on the sidebar edge and `Ctrl+\`; visibility remembered globally.
- **R-NAV-04 MUST** Resizable width by dragging its inner edge (180–480 px), remembered.
- **R-NAV-05 MUST** *Contents*: heading tree (levels 1..`toc_max_depth`, default 4). Click → smooth scroll to the heading. **Scroll spy**: the heading whose section contains the reading line (top of viewport + 80 px) is highlighted; the list auto-scrolls to keep it visible. Title flattening: if the document has exactly one H1 and it is the first heading, its H2 children are shown at the same indent as the H1.
- **R-NAV-06 MUST** Collapsing: per-node toggle (hover an item with children → ▸/▾); "Collapse one level / Expand one level" buttons in the tab's header area on hover; state remembered per document.
- **R-NAV-07 MUST** TOC font size adjustable (11–18 px, default 13).
- **R-NAV-08 MUST** *History*: groups **Current Session** (files opened since launch; the current file tagged "Current"), **Today**, **Yesterday**, then one group per date (locale short date). Item = file name; tooltip = full path; hover shows × (remove); group header hover shows "Clear". Click opens in the current window. Missing files shown muted; clicking → toast. Max 500 entries.
- **R-NAV-09 MUST** *Favorites*: add/remove via `Ctrl+D` or menu; hover × to remove; drag to reorder (SHOULD); empty state: centered hint "Press Ctrl+D to favorite this file" with a star icon.
- **R-NAV-10 MUST** *Files*: Markdown files under the current document's folder as a lazy explorer tree (folders first, natural sort, current file highlighted). Click switches the document **in place without flicker**. Updates on directory changes. Limits: 5 000 entries, depth 8; skips hidden/system items and `.git`, `node_modules`, `target`, `.venv`, `__pycache__`.
- **R-NAV-11 MUST** Keyboard: `↑/↓` move, `Enter` open, `←/→` collapse/expand, `Home/End`; `Ctrl+Shift+1…4` switch tabs (rebindable).

## A.9 Reading features (R-READ)
- **R-READ-01 MUST** **Live reload**: the open file is watched (directory watch, so atomic-rename saves are caught). External change → re-render within 100 ms, only changed blocks, scroll anchored to the same block, no flicker. Deleted → R-OPEN-11. If there are unsaved in-app edits → conflict banner (R-EDIT-03).
- **R-READ-02 MUST** **Reading position memory** per file (block index + offset within block, fallback scroll ratio), restored on reopen; LRU 2 000 entries; setting to disable. Following a link and coming back restores that document's own position (never carries over).
- **R-READ-03 MUST** **Font zoom**: `Ctrl+wheel`, `Ctrl+=`/`Ctrl+-`/`Ctrl+0`, `Ctrl+↑/↓`; 10–32 px, step 1 px; real reflow (not bitmap scaling); transient zoom indicator; persisted globally.
- **R-READ-04 MUST** **Column width**: `Alt+wheel`, `Alt+↑/↓`, `Alt+0` reset; 30–100 % of client width, step 5 %, default 65 %; indicator; persisted.
- **R-READ-05 MUST** **Find** `Ctrl+F` (Appendix B §B.9): incremental, highlights all matches, "n/m" count, `Enter`/`Shift+Enter` in the find bar and `Ctrl+G`/`Ctrl+Shift+G` anywhere (`F3` is reserved for Split Edit), toggles *Match case* and *Whole word*; searches rendered text (including tables, code, footnotes, collapsed sections — auto-expanded on hit; excluding hidden markup); case-insensitive mode uses Unicode case folding **plus Turkish equivalence (`i/İ/ı/I` match each other)**; `Esc` closes; highlights are never written to the file.
- **R-READ-06 MUST** **Heading folding**: hovering the left gutter of H1–H3 shows ▸/▾; click folds/unfolds the section (read mode and F2). `Alt+click` folds/unfolds all headings of that level (SHOULD). Session-only state.
- **R-READ-07 MUST** **Word-count badge** (off by default): bottom-right, "1,234 words · 7,890 chars"; with a selection adds "· 12 selected". CJK ideographs count as one word each. Tooltip: lines, reading time (230 wpm), encoding, line endings.
- **R-READ-08 MUST** **Fullscreen viewers** (overlay in the same window; `Esc` closes): image, table (natural width, no wrapping, horizontal scroll, resizable/movable card), Mermaid diagram (vector-sharp at any zoom), mind map (R-MAP). Common controls: wheel zoom at cursor (10 %–800 %), drag to pan, double-click toggles fit/100 %, `+`/`-`/`0`, toolbar (zoom out, %, zoom in, fit, 1:1, close).
- **R-READ-09 MUST** Toasts for transient feedback (Copied, Saved, Reloaded, errors).
- **R-READ-10 MUST** "Single newline as line break" toggle in the menu (⏎ icon with check state).
- **R-READ-11 MUST** Copy, Copy as Markdown, Select All in the context menu.
- **R-READ-12 MAY** Quick heading jump palette (`Ctrl+Shift+O`).

## A.10 Themes and typography (R-THEME)
- **R-THEME-01 MUST** Mode: *Follow System* (default) / *Light* / *Dark*; live switch on system change.
- **R-THEME-02 MUST** 11 color schemes: **Default, GitHub, Glacier, Forest, Sunset, Typewriter, Editorial, Violet, Lychee, Mint, Neon**, each with light and dark palettes (exact tokens in Appendix B §B.3). Schemes change colors only (fonts stay user-selected).
- **R-THEME-03 MUST** Scheme picker as **cards** (mini preview: page color, heading line, text lines, accent bar); hover = live preview, click = apply + persist; leaving without clicking reverts.
- **R-THEME-04 MUST** Fonts: body font (any installed family; searchable list; each name previewed in its own face) and code font (monospace families listed first). Defaults: body `Segoe UI`, code `Cascadia Mono` → `Consolas`. Settings show the font actually resolved.
- **R-THEME-05 MUST** Corner style: *Rounded* (default) / *Sharp* — applies to code blocks, tables, callouts, math/Mermaid cards, menus, cards, exported HTML.
- **R-THEME-06 MUST** Custom colors editor: 11 tokens (`bg`, `text`, `heading`, `link`, `accent`, `quote_bar`, `code_bg`, `border`, `table_header_bg`, `zebra_bg`, `selection`) per mode, with an in-app color picker (hue/saturation square, hue slider, hex input); applies instantly; reset per token / all. `border` is an editor-facing alias that sets both `code_border` and `table_border`; in `custom-theme.ini` an explicit `code_border` or `table_border` key overrides the alias.
- **R-THEME-07 MUST** `custom-theme.ini` in the config folder (documented keys = the tokens above in `[light]` / `[dark]`), hot-reloaded on save; invalid values ignored with a toast.
- **R-THEME-08 MUST** Windows High Contrast: use system colors, disable custom colors and scheme tinting.
- **R-THEME-09 MUST** Table zebra striping toggle (default off).
- **R-THEME-10 MUST** All chrome (title bar, menus, dialogs, settings, toasts, viewers, scrollbars) follows the active mode and scheme.

## A.11 Settings (R-SET) — panel layout in Appendix B §B.8
- **R-SET-01 MUST** Open via ⚙ title-bar button, menu, `Ctrl+,`.
- **R-SET-02 MUST** In-window overlay panel, draggable by its header, section navigation on the left; MAY have a search box.
- **R-SET-03 MUST** Changes apply instantly (no OK/Apply) and persist atomically.
- **R-SET-04 MUST** "Reset section" and "Reset all".
- **R-SET-05 MUST** Storage: `%APPDATA%\YAMDV\settings.ini`, `state.ini` (window, history, favorites, positions, TOC states), `custom-theme.ini`, `%LOCALAPPDATA%\YAMDV\{logs,recovery}`. **Portable mode**: a file `yamdv.portable` next to the exe → all data in `.\YAMDV-data\` beside the exe, and no registry writes unless the user explicitly registers.
- **R-SET-06 MUST** Settings keys and defaults: Appendix B §B.8.2 (that table is normative).

## A.12 Editing — common (R-EDIT)
- **R-EDIT-01 MUST** The **source text is the single source of truth**. Every edit (F2 or F3) is a text operation on the source buffer; no lossy HTML↔Markdown round-trips. Bytes outside edited ranges stay identical on save.
- **R-EDIT-02 MUST** Saving: autosave (default on; 800 ms after the last edit, on mode exit, on window deactivation, on close) and `Ctrl+S`. Atomic write (`S-FILE-03`). Preserve encoding, BOM, line endings and final-newline state. Dirty `●` in the title.
- **R-EDIT-03 MUST** Conflict detection: before writing, compare the file's current size/mtime/hash with the last known state. If it changed externally: banner "Changed on disk" with [Overwrite] [Reload (discard mine)] [Save mine as…].
- **R-EDIT-04 MUST** Undo/redo `Ctrl+Z`, `Ctrl+Y`, `Ctrl+Shift+Z`; typing coalesced (word boundary or 1 s pause); restores selection; history capped by memory (4 MB) with oldest dropped; shared between F2 and F3 for the same document.
- **R-EDIT-05 MUST** Clipboard: cut/copy/paste; `Ctrl+Shift+V` paste as plain text; pasting `CF_HTML` from browsers SHOULD convert to Markdown (sanitized, bounded); pasting an image → R-EDIT-08.
- **R-EDIT-06 MUST** **Tab snippets** (type trigger, press `Tab`): `h1`…`h6`, `table` (3×3), `tableRxC` (e.g. `table2x3`), `code`, any known language name (`python`, `rust`, `js`, …) → fenced block with that language, `link`, `img`, `line`/`hr`, `quote`, `task`, `note`/`tip`/`important`/`warning`/`caution`/`red` (red = caution) → callout, `math`, `mermaid`, `flow` (flowchart template), `seq` (sequence template), `gantt`, `date` (today ISO), `time`, `details`, `footnote`. Snippets place the caret at the first placeholder; `Tab` jumps to the next placeholder.
- **R-EDIT-07 MUST** Markdown-aware keys: `Enter` continues lists/tasks/quotes (empty item ends the list); `Tab`/`Shift+Tab` indent/outdent list items; `Ctrl+B` bold, `Ctrl+I` italic, `Ctrl+Shift+X` strikethrough, `Ctrl+Shift+H` highlight, ``Ctrl+` `` inline code, `Ctrl+K` link, `Ctrl+Shift+K` code block, `Ctrl+1…6` heading level, `Ctrl+Shift+0` paragraph, `Ctrl+Shift+Q` quote, `Ctrl+Shift+8` bullet list, `Ctrl+Shift+7` numbered list, `Ctrl+Shift+9` task list, `Ctrl+Enter` toggle task checkbox, `Ctrl+T` insert table, `Alt+↑/↓` move line/block, `Ctrl+Shift+D` duplicate line, `Ctrl+L` select line. Formatting toggles apply to the selection or the word at the caret. Auto-pairing of brackets/quotes/markers: setting, default off.
- **R-EDIT-08 MUST** Paste/drop image or file in edit modes → saved into `<doc folder>/<assets_dir>/<doc stem>/` (default `assets_dir = "assets"`), images as PNG named `image-YYYYMMDD-HHMMSS[-n].png`, files keep their sanitized original name (dedupe `-1`, `-2`); never overwrite; inserted as `![](assets/<stem>/<name>)` / `[name](assets/<stem>/<name>)` with forward slashes and `%20` for spaces. Files > 50 MB ask for confirmation. Rules `S-FILE-05`.
- **R-EDIT-09 MUST** IME: inline composition rendering, candidate window positioned at the caret (CJK, Korean, etc.); dead keys and AltGr layouts (Turkish Q/F) work.
- **R-EDIT-10 MUST** Find & Replace `Ctrl+H` (replace, replace all — one undo step); case / whole word. Regex MAY (only with a linear-time engine, ADR).
- **R-EDIT-11 MUST** Mode switching keeps caret and scroll on the same content: Read ↔ F2 ↔ F3 directly; `Esc` returns to read mode; pressing the active mode's key again also exits.
- **R-EDIT-12 MUST** Editing disabled (with reason) for read-only files and for files that could not be decoded losslessly (offer "Convert to UTF-8 and edit").
- **R-EDIT-13 MUST** Crash/power-loss safety: unsaved buffers are journaled to `%LOCALAPPDATA%\YAMDV\recovery\` (only when autosave is off or a save failed); offered for restore at next launch; deleted after successful save or discard.
- **R-EDIT-14 MUST** Grapheme-aware caret movement/deletion; `Ctrl+←/→` word movement (Unicode word boundaries); `Home`/`End` respect wrapped lines.

## A.13 Split edit — F3 (R-F3)
- **R-F3-01 MUST** Source editor + live preview side by side; swap sides via ⇄ button / menu (remembered); draggable splitter (20–80 %, default 50 %, remembered, scales with window; layout preserved on maximize/restore).
- **R-F3-02 MUST** Editor: line-number gutter (toggle), soft wrap (default on), current-line highlight (subtle), editor font = body font (default, matching the reference) or code font (setting); optional Markdown syntax tint (setting, default off).
- **R-F3-03 MUST** Preview updates ≤ 50 ms after typing stops (debounce ~30 ms, incremental).
- **R-F3-04 MUST** Scroll sync by block/section in both directions.
- **R-F3-05 MUST** Click in preview → caret to the matching source position; selection in preview selects the matching source range.
- **R-F3-06 MUST** "?" help button bottom-right of the editor pane → cheat-sheet overlay (syntax, shortcuts, snippets).
- **R-F3-07 MUST** Find works in the editor pane; matches mirrored in the preview (SHOULD).

## A.14 Live Edit — F2 WYSIWYG (R-F2)
- **R-F2-01 MUST** `F2` makes the rendered page editable in place; title pill "Live Edit"; reading position kept.
- **R-F2-02 MUST** **Reveal model** (Obsidian Live Preview / Typora style): the document stays rendered with the same typography; Markdown markers become visible (muted color) only where the caret is:
  - inline markers (`**`, `*`, `~~`, `==`, `` ` ``, `[`…`](url)`, `$`) appear when the caret is inside or adjacent to that span;
  - line/block markers (`## `, `> `, `- `, `1. `, `- [ ] `) appear when the caret is on that line;
  - code blocks: fence lines (```` ```lang ````) appear when the caret is in the block; the code is edited as monospace text with live highlighting;
  - math: caret inside `$…$`/`$$…$$` shows the source with a live-rendered preview popover below;
  - Mermaid: caret inside the block shows the source with the live diagram below;
  - images: caret on the image line shows `![alt](src)` above the still-visible image;
  - tables are **never** shown as pipes — they use cell editing (R-TBL).
- **R-F2-03 MUST** Typing transforms immediately: `# `…`###### ` at line start → heading; `- `/`* `/`+ ` → bullet; `1. ` → numbered; `[ ] `/`[x] ` in a list → task; `> ` → quote; ```` ``` ```` or ```` ```lang ```` + `Enter` → code block with caret inside (no stray blank lines); `---` + `Enter` → rule; `$$` + `Enter` → math block; a pipe header row + `Enter` → table with delimiter row.
- **R-F2-04 MUST** Floating format toolbar above a non-empty selection (setting to disable; `Esc` dismisses): Bold, Italic, Strikethrough, Highlight, Inline code, Link, H1, H2, H3, Quote, Bullet list, Numbered list, Task list, Clear formatting.
- **R-F2-05 MUST** Context menu adds: Cut/Copy/Paste/Paste as plain text, **Format ▸**, **Insert ▸** (Table…, Code block, Callout ▸ 5 types, Math block, Mermaid diagram, Horizontal rule, Image…, Link…, Footnote), **Table ▸** (when inside a table).
- **R-F2-06 MUST** Cross-block selection: copy/cut/delete across paragraphs, lists, tables and code in one step (one undo step); clipboard gets Markdown + HTML + plain text; pasting multi-block Markdown creates the blocks.
- **R-F2-07 MUST** Lists: `Enter`/`Tab`/`Shift+Tab` per R-EDIT-07; clicking a checkbox toggles `[ ]`↔`[x]`.
- **R-F2-08 MUST** Links: click places the caret; `Ctrl+click` follows (R-LINK); `Ctrl+K` opens an inline popover (text, URL, title).
- **R-F2-09 MUST** Code block language label is editable (click → input with autocomplete of known languages).
- **R-F2-10 MUST** Blocks the user did not touch are never re-serialized.
- **R-F2-11 MAY** Visual Mermaid editing for flowcharts (double-click node → edit label; written back to source).
- **R-F2-12 MAY** Image resize handles (write `|width` / HTML `width`).

## A.15 Table editing (R-TBL)
- **R-TBL-01 MUST** In-cell editing; `Tab`/`Shift+Tab` next/previous cell; `Tab` in the last cell adds a row; `Enter` → cell below (adds a row at the end); `Shift+Enter` inserts `<br>`; empty cells keep row height so the caret can be placed.
- **R-TBL-02 MUST** Quiet hover toolbar (top-right of the table): add row above/below, add column left/right, delete row/column, align column (left/center/right/none), sort column ascending/descending (natural, locale-aware; one undo step), move row up/down, move column left/right, delete table, ⛶ fullscreen, export CSV.
- **R-TBL-03 MUST** Drag handles: column grip (on header hover) and row grip (left edge) to reorder, with a live drop indicator.
- **R-TBL-04 MUST** `\|` escaping inside cells handled both ways.
- **R-TBL-05 MUST** Re-serialization of an edited table pretty-aligns its columns (setting "Auto-format tables", default on). Other tables are untouched.
- **R-TBL-06 MUST** Insert-table picker: grid up to 10×10 with live "R × C" label.
- **R-TBL-07 SHOULD** Pasting TSV/CSV (e.g. from Excel) into a table expands into cells; pasting TSV outside a table offers "Convert to table".
- **R-TBL-08 MUST** Inserting a table adjacent to another table never merges them (blank-line handling in source).

## A.16 Math (R-MATH)
- **R-MATH-01 MUST** Native TeX-math layout engine (no JS, no download) using the OpenType `MATH` table of the system font **Cambria Math** (present on every Windows 10/11), read via DirectWrite.
- **R-MATH-02 MUST** KaTeX-compatible subset, at least: `^ _` (with limits rules), `\frac \dfrac \tfrac \cfrac \binom \dbinom \tbinom`, `\sqrt[n]{}`, big operators `\sum \prod \coprod \int \iint \iiint \oint \bigcup \bigcap \bigoplus \bigotimes \lim \limsup \liminf \max \min \sup \inf \det \gcd`, `\limits \nolimits`, delimiters `\left \right \middle` and `\big \Big \bigg \Bigg`, environments `matrix pmatrix bmatrix Bmatrix vmatrix Vmatrix smallmatrix cases dcases aligned align align* gathered gather split array{…} equation`, `\\` and `&`, accents `\hat \widehat \bar \overline \underline \vec \overrightarrow \overleftarrow \dot \ddot \tilde \widetilde \acute \grave \breve \check \mathring`, `\overbrace{}^{} \underbrace{}_{}`, `\overset \underset \stackrel`, fonts `\mathrm \mathbf \mathit \mathsf \mathtt \mathcal \mathbb \mathfrak \mathscr \boldsymbol \bm \text \textbf \textit \textrm \operatorname`, all Greek letters and variants, ≈ 500 common symbols/relations/arrows/operators (KaTeX "supported functions" core list), spacing `\, \: \; \! \quad \qquad \hspace{}`, `\not`, `\cancel`, `\boxed`, `\color{name|#hex}{…}`, `\textcolor`, `\tag{}` (right-aligned number), `\displaystyle \textstyle \scriptstyle`, `\phantom`, `\mod \bmod \pmod`, `\ldots \cdots \vdots \ddots`, primes `'`.
- **R-MATH-03 MUST** Errors never crash: the source is rendered in the error color (`#cc0000` light / `#ff6b6b` dark) with a tooltip containing the message and position.
- **R-MATH-04 MUST** Copying a formula copies its LaTeX source with delimiters.
- **R-MATH-05 MUST** Display math centered; too-wide formulas scroll horizontally within the block.
- **R-MATH-06 MUST** Colors follow the theme; `\color` overrides.
- **R-MATH-07 SHOULD** Per-document `\newcommand`, `\renewcommand`, `\def` (simple, bounded expansion — `S-PARSE-05`).

## A.17 Mermaid (R-MMD)
- **R-MMD-01 MUST** Native parser + layout + renderer (no JS). Diagram types: **flowchart/graph (MUST), sequenceDiagram (MUST), gantt (MUST), pie (MUST)**, stateDiagram/stateDiagram-v2 (SHOULD), classDiagram (SHOULD), erDiagram (SHOULD), mindmap (SHOULD), timeline, journey, gitGraph, quadrantChart (MAY). Unsupported types → shown as a code block with badge "Diagram type not supported".
- **R-MMD-02 MUST** Rendered in a card (Appendix B §B.6.12), scaled down to fit the column (never up); hover ⛶ / double-click → fullscreen viewer.
- **R-MMD-03 MUST** Light and dark palettes (Appendix B §B.3.6); `%%{init: {"theme": "…"}}%%` directive honors `default`, `neutral`, `dark`, `forest`, `base` and a subset of `themeVariables` (primaryColor, primaryTextColor, primaryBorderColor, lineColor, secondaryColor, tertiaryColor, fontSize) (SHOULD).
- **R-MMD-04 MUST** Syntax errors → card shows the source plus "Mermaid syntax error at line L, column C: <message>".
- **R-MMD-05 MUST** Limits per `S-PARSE-06`; over limit → placeholder "Diagram too large (N nodes)" with [Render anyway] (still capped by the hard limit).
- **R-MMD-06 MUST Flowchart:** directions `TB TD BT LR RL`; node shapes `[]` rect, `()` round, `([])` stadium, `[[]]` subroutine, `[()]` cylinder, `(())` circle, `((()))` double circle, `>]` asymmetric, `{}` rhombus, `{{}}` hexagon, `[/ /]` parallelogram, `[\ \]` alt-parallelogram, `[/ \]` trapezoid, `[\ /]` alt-trapezoid; edges `-->` `---` `-.->` `-.-` `==>` `===` `--o` `--x` `<-->` `o--o` `x--x`, extra-length variants (`--->`, `-.-->`), labels `-->|text|` and `-- text -->`, chaining `A --> B --> C`, `A & B --> C & D`; nested `subgraph id [title] … end` with optional `direction`; `classDef`, `class`, `:::cls`, `style` (fill, stroke, color, stroke-width, stroke-dasharray), `linkStyle` (basic); `%%` comments; quoted labels with entities; `click`/`callback` lines are parsed and **ignored** (security). Layout: layered (Sugiyama) — cycle breaking, layer assignment, crossing reduction (barycenter, a few sweeps), coordinate assignment, orthogonal-ish or spline edges with arrowheads, label placement.
- **R-MMD-07 MUST Sequence:** `participant`/`actor` with `as` alias, messages `->` `-->` `->>` `-->>` `-x` `--x` `-)` `--)`, activations (`+`/`-`, `activate`/`deactivate`), `Note left of|right of|over A[,B]`, blocks `loop alt else opt par and critical option break rect`, `autonumber`, `title`, participant boxes at top and bottom.
- **R-MMD-08 MUST Gantt:** `title`, `dateFormat` (common Day.js tokens `YYYY MM DD HH mm`), `axisFormat` (common strftime tokens), `excludes weekends`, `section`, tasks with tags `done active crit milestone`, ids, `after id`, start dates, durations `d w h m`, today marker (local clock).
- **R-MMD-09 MUST Pie:** `title`, `showData`, `"label" : value`; legend; palette in Appendix B.
- **R-MMD-10 SHOULD** Context menu on a diagram: "Copy as PNG" (clipboard), "Copy source".

## A.18 Mind map view (R-MAP)
- **R-MAP-01 MUST** Menu "View as mind map" (`Ctrl+Shift+M`): fullscreen overlay; root = document title (first H1 or file name); headings → branches by level; list items → leaves (text truncated to 80 chars); right-expanding tree layout (SHOULD offer balanced two-sided layout); curved connectors; branch colors from the scheme accent family; theme-aware.
- **R-MAP-02 MUST** Interactions: wheel zoom, drag pan, click a node to collapse/expand, double-click → close and jump to that heading; `Esc` closes. "Copy as PNG" SHOULD.

## A.19 Export and print (R-EXP)
- **R-EXP-01 MUST** **Export PDF** (native writer — `pdf-writer` + `subsetter`, no external tools): vector, selectable/searchable text with subset-embedded fonts; images embedded; colors per setting `pdf_colors` (*Follow theme* default, or *Light*), backgrounds kept (dark themes do not turn white unless *Light* is chosen); page size *Auto* (Letter for en-US/en-CA locales, else A4) / A4 / Letter; margins 18 mm; optional footer page numbers "n / N"; PDF outline (bookmarks) from headings; internal links clickable; external links as link annotations; code wraps in PDF; avoid breaking inside short blocks and table rows; never emit a blank trailing page. Default file name `<doc stem>.pdf`. Progress toast and a completion toast with [Show in folder].
- **R-EXP-02 MUST** **Export single-file HTML**: one `.html` with inline CSS for the current theme, images embedded as `data:` URIs (only files that decoded successfully as images, within limits), math and Mermaid as inline SVG generated from our own layouts, highlighted code as spans, TOC sidebar, collapsible H1–H3, light/dark toggle, in-page find — implemented with a tiny inline script protected by a hash-based CSP (`S-EXP-02`). No external references of any kind.
- **R-EXP-03 MUST** **Print** `Ctrl+P`: system print dialog (`PrintDlgExW`); same pagination engine as PDF; colors per `print_colors` (*Light* default / *Follow theme*).
- **R-EXP-04 MUST** **Export table as CSV** (from table toolbar or fullscreen table): UTF-8 with BOM, CRLF, RFC 4180 quoting; formatting stripped to plain text.
- **R-EXP-05 MUST** Headless CLI export: `yamdv --export-html <in.md> [-o <out.html>]`, `yamdv --export-pdf <in.md> [-o <out.pdf>]`, options `--theme <scheme>`, `--mode light|dark`; no window; non-zero exit code on error.
- **R-EXP-06 SHOULD** Copy whole document as rich HTML; export a Mermaid diagram or the mind map as PNG.

## A.20 External editor (R-EXT)
- **R-EXT-01 MUST** `F4` / `Ctrl+E` opens the file in an external editor at the line of the selection start, or of the topmost visible block.
- **R-EXT-02 MUST** Auto-detection (first found): VS Code (`Code.exe --goto "<file>:<line>:<col>"`), VSCodium, Cursor (same CLI as VS Code), Sublime Text (`subl.exe "<file>:<line>:<col>"`), Notepad++ (`notepad++.exe -n<line> -c<col> "<file>"`), EmEditor (`EmEditor.exe /l <line> /cl <col> "<file>"`), Typora (no line), Notepad (`notepad.exe "<file>"`). Detection uses known install folders and the read-only `App Paths` registry keys; it resolves to real `.exe` files only.
- **R-EXT-03 MUST** Custom editor: absolute path to an existing `.exe` plus an argument template with `{file}`, `{line}`, `{col}` placeholders; arguments are tokenized and quoted per Windows rules and passed to `CreateProcessW` — **never through a shell** (`S-EXEC-03`).
- **R-EXT-04 MUST** Changes saved by the external editor appear via live reload.

## A.21 System integration (R-SYS)
- **R-SYS-01 MUST** Per-user registration only (HKCU, no admin): ProgID `YAMDV.Markdown` (friendly type name, `DefaultIcon` = document icon in the exe, `shell\open\command` = `"<exe>" "%1"`), `Software\YAMDV\Capabilities` (FileAssociations for all Markdown extensions) + `Software\RegisteredApplications\YAMDV`, `OpenWithProgids` entries for `.md .markdown .mdown .mkd .mkdn .mdwn .mdtxt .mdtext`, `Applications\yamdv.exe` (SupportedTypes); then `SHChangeNotify(SHCNE_ASSOCCHANGED)`.
- **R-SYS-02 MUST** Opening files never changes the system default. "Set as default app" (welcome page and Settings) **never writes `UserChoice`**; it opens Windows Settings for the app (`ms-settings:defaultapps?registeredAppUser=YAMDV` on Windows 11; `ms-settings:defaultapps` + an instruction toast on Windows 10). Association status (default / not default) is queried with `AssocQueryStringW` / `IApplicationAssociationRegistration` and shown on the welcome page and in Settings.
- **R-SYS-03 MUST** Optional Explorer context-menu verb "Open with YAMDV" for the Markdown extensions (HKCU `SystemFileAssociations`), toggled in Settings or via CLI.
- **R-SYS-04 MUST** CLI per Appendix F §F.4.
- **R-SYS-05 MUST** Self-install (no third-party installer): `yamdv --install` copies the exe to `%LOCALAPPDATA%\Programs\YAMDV\`, registers per R-SYS-01, creates a Start-menu shortcut, writes an HKCU `Uninstall` entry (DisplayName, DisplayIcon, DisplayVersion, Publisher, EstimatedSize, InstallLocation, UninstallString `"<exe>" --uninstall`), and offers "Set as default app". `--uninstall` removes every key/shortcut it created and the install folder (self-deletion via a temp copy helper that waits for the parent to exit), asking whether to keep settings.
- **R-SYS-06 MUST** Portable mode per R-SET-05.
- **R-SYS-07 MUST** Version resource; original icons (app + document) embedded.
- **R-SYS-08 MAY** User-supplied custom app/document icon.
- **R-SYS-09 MAY** Taskbar jump list with recent files.

## A.22 Internationalization (R-I18N)
- **R-I18N-01 MUST** UI languages **English** and **Türkçe**; setting *Follow System* (default: `tr-*` → Turkish, otherwise English) / English / Türkçe; switching is live.
- **R-I18N-02 MUST** All UI strings are compile-time `&'static str` tables keyed by an enum (no runtime files); a test fails if any key is missing in a language. Dates via `GetDateFormatEx` with the user locale; numbers with locale digit grouping.
- **R-I18N-03 MUST** Document content in any script; grapheme-correct editing.
- **R-I18N-04 MUST** Locale-independent internal processing: file extensions, URL schemes, setting keys, language tags compared **ASCII-case-insensitively** (never culture-sensitive — avoids the Turkish-İ bug). User-facing find uses R-READ-05 rules.

## A.23 Accessibility (R-A11Y)
- **R-A11Y-01 MUST** Every UI element is keyboard-operable with a visible focus ring (2 px accent).
- **R-A11Y-02 MUST** High Contrast (R-THEME-08); reduced motion (`SPI_GETCLIENTAREAANIMATION` off → no animations, instant scrolling).
- **R-A11Y-03 SHOULD** UI Automation: window, buttons, menus, tabs, list items, settings controls with names/roles; the document exposed as a Document control with TextPattern (read-only text) — Phase 012.
- **R-A11Y-04 MUST** Contrast: body text ≥ 4.5:1, links ≥ 4.5:1, headings ≥ 3:1 against the page background in every palette (unit test computes WCAG ratios for all tokens).

## A.24 Reliability (R-REL)
- **R-REL-01 MUST** No input file can crash, hang (> 2 s UI block) or exhaust memory: fuzzed parsers, depth/size limits (`S-PARSE-*`), time budgets with graceful placeholders.
- **R-REL-02 MUST** State files are written atomically and debounced (≤ 1 write per 2 s); corrupt state files are renamed `*.corrupt-<ts>` and defaults used.
- **R-REL-03 MUST** All OS failures (fonts missing, WIC codec missing, clipboard busy, file locked) degrade gracefully with a toast/placeholder — never a crash.
- **R-REL-04 MUST** Works without admin rights, on locked-down machines, from read-only media (portable mode then falls back to `%APPDATA%`).
