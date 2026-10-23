# APPENDIX B — UI & Visual Specification (`docs/UI_SPEC.md`)

All sizes are **logical px** (DIPs at 96 DPI); multiply by `dpi / 96`. Snap 1 px lines and rectangle edges to device pixels so borders stay crisp at every scale. Colors are sRGB hex; `rgba()` values are alpha-blended over what is beneath.

## B.1 The look in one paragraph (reference description)
A plain, bright page that feels like a well-made documentation website. On the left, a quiet full-height sidebar (260 px, same background as the page, separated only by a 1 px hairline) with small text tabs at the top — **Contents · History · Favorites · Files** — the active tab in the accent blue with a 2 px underline. Below, the heading outline in 13 px text; the current section is blue. A tiny chevron sits on the sidebar's hairline, vertically centered, to hide it. The document column is centered in the remaining space, ~65 % of the window wide, with generous top padding. Headings are dark and semibold (32 / 24 / 20 px), body text is 16 px Segoe UI with 1.6 line height in `#333`. Code blocks are light-gray rounded cards with a hairline border and a hover-only toolbar (copy, line numbers, wrap, language). Tables have hairline borders and a light-gray header row. Callouts are tinted cards with a 3 px colored left bar and an icon + title row. In dark mode the page becomes `#1e1e1e` with `#d4d4d4` text and slightly lighter cards. There are no toolbars, no status bar, no ribbon: just the document. The title bar is custom-drawn in the page color, so the window looks like one seamless sheet.

## B.2 Typography
| Role | Family (fallbacks) | Size | Weight | Line height |
|---|---|---|---|---|
| Body | setting `body_font` (default `Segoe UI`; DirectWrite system fallback for other scripts) | `font_size` (default 16) | 400 | `line_height` (default 1.6) |
| H1 / H2 / H3 / H4 / H5 / H6 | body family | 2.0 / 1.5 / 1.25 / 1.125 / 1.0 / 0.875 em | 600 | 1.25 |
| Code (blocks & inline) | setting `code_font` (default `Cascadia Mono` → `Consolas` → `Courier New`) | blocks 0.875 em (14 px); inline 0.875 em | 400 | blocks 1.6 |
| Math | `Cambria Math` | 1.21 em | — | — |
| UI chrome (title, tabs, menus, settings, toasts) | `Segoe UI Variable Text` (Win 11) → `Segoe UI` | 12–13 px (per component) | 400 / 600 | 1.4 |
| UI icons | `Segoe Fluent Icons` (Win 11) → `Segoe MDL2 Assets` (Win 10) → vector fallback | per component | — | — |

Text rendering: ClearType when the system uses it, grayscale on rotated/transparent layers, DirectWrite natural symmetric rendering mode; honor the user's ClearType settings.

## B.3 Color tokens

### B.3.1 Document tokens — light palettes
| Scheme | `bg` | `text` | `heading` | `link` | `accent` | `quote_bar` | `quote_text` | `code_bg` | `inline_code_bg` | `table_header_bg` |
|---|---|---|---|---|---|---|---|---|---|---|
| Default | `#ffffff` | `#333333` | `#333333` | `#2563eb` | `#2563eb` | `#dddddd` | `#666666` | `#f5f5f5` | `#f0f0f0` | `#f5f5f5` |
| GitHub | `#ffffff` | `#1f2328` | `#1f2328` | `#0969da` | `#0969da` | `#d1d9e0` | `#59636e` | `#f6f8fa` | `#eff1f3` | `#f6f8fa` |
| Glacier | `#f6f9fe` | `#1a2538` | `#2563eb` | `#2563eb` | `#2563eb` | `#2563eb` | `#6e7684` | `#e8f0fe` | `#e6edfc` | `#e8f0fe` |
| Forest | `#f5faf5` | `#1a2e1a` | `#008a2e` | `#00822b` | `#008a2e` | `#008a2e` | `#6d7c6d` | `#e8f8e8` | `#e2f1e6` | `#e8f8e8` |
| Sunset | `#fef8f0` | `#2c1a0a` | `#b45309` | `#b45309` | `#b45309` | `#b45309` | `#7c6e61` | `#faf0e0` | `#f8ecde` | `#faf0e0` |
| Typewriter | `#f5f0e8` | `#2c2416` | `#8b6914` | `#896714` | `#8b6914` | `#8b6914` | `#787166` | `#ede5d8` | `#ede5d8` | `#ede5d8` |
| Editorial | `#faf8f5` | `#2c2416` | `#8b4513` | `#8b4513` | `#8b4513` | `#8b4513` | `#7a746b` | `#f5f0e8` | `#f1eae3` | `#f5f0e8` |
| Violet | `#faf5fe` | `#271b30` | `#7209b7` | `#7209b7` | `#7209b7` | `#7209b7` | `#776e7f` | `#f4eafb` | `#efe3f8` | `#f4eafb` |
| Lychee | `#fff5f5` | `#2b2726` | `#e63946` | `#db1c2a` | `#e63946` | `#e63946` | `#7c7575` | `#ffeeee` | `#fde6e7` | `#ffeeee` |
| Mint | `#f5fbf8` | `#1a3a2a` | `#0ea271` | `#0b815a` | `#10b981` | `#10b981` | `#6d8378` | `#e8f8f0` | `#e3f6ef` | `#e8f8f0` |
| Neon | `#f0f4f8` | `#1a1a2e` | `#0077b6` | `#0072ae` | `#0077b6` | `#0077b6` | `#6b6d7b` | `#e2e8f0` | `#ddeaf3` | `#e2e8f0` |

Shared light tokens: `code_border #e0e0e0`, `table_border #dddddd`, `border_subtle #eeeeee`, `zebra_bg = mix(bg, table_header_bg, 50%)`, `text_muted #7a7a7a`, `marker #b0b0b0` (revealed Markdown syntax in F2), `line_number #b8b8b8`, `selection = accent @ 20 %`, `highlight_bg #fff3a3`, `find_match_bg #fff3a3`, `find_current_bg #ffb547`, `error #cc0000`.
GitHub scheme extras: H1 and H2 get a 1 px bottom border `#d1d9e0` with 0.3 em padding-bottom.
(The values were sampled from the reference screenshots; `link` and Mint `heading` were minimally darkened to meet R-A11Y-04.)
The custom-colors alias `border` (R-THEME-06) maps to `code_border` + `table_border` in both modes.

### B.3.2 Document tokens — dark palettes
Base for every scheme except GitHub: `bg #1e1e1e`, `text #d4d4d4`, `quote_text #aaaaaa`, `inline_code_bg #3a3a3a`, `code_border #3a3a3a`, `table_border #3a3a3a`, `border_subtle #333333`, `text_muted #9a9a9a`, `marker #6a6a6a`, `line_number #5a5a5a`, `selection = accent @ 30 %`, `highlight_bg #665c1e` (text `#f5f0d0`), `find_match_bg #5c5427`, `find_current_bg #9e6a03`, `error #ff6b6b`.

| Scheme | `heading` | `link` / `accent` | `quote_bar` | `code_bg` | `table_header_bg` |
|---|---|---|---|---|---|
| Default | `#d4d4d4` | `#4da5ec` | `#555555` | `#2d2d2d` | `#2d2d2d` |
| Glacier | `#608df0` | `#608df0` | `#4966a6` | `#31353d` | `#323740` |
| Forest | `#5ff290` | `#5ff290` | `#48a868` | `#313d35` | `#324137` |
| Sunset | `#f29e5f` | `#f29e5f` | `#a87148` | `#3d3631` | `#413832` |
| Typewriter | `#e9c467` | `#e9c467` | `#a28a4d` | `#3c3932` | `#403c33` |
| Editorial | `#ea9d66` | `#ea9d66` | `#a3714d` | `#3c3632` | `#403833` |
| Violet | `#b85ff2` | `#b85ff2` | `#8248a8` | `#38313d` | `#3b3241` |
| Lychee | `#ec656f` | `#ec656f` | `#a44c53` | `#3c3132` | `#403334` |
| Mint | `#5ff1c1` | `#5ff1c1` | `#48a788` | `#313d39` | `#32413c` |
| Neon | `#5fbff2` | `#5fbff2` | `#4887a8` | `#31393d` | `#323c41` |

GitHub dark: `bg #0d1117`, `text #e6edf3`, `heading #e6edf3`, `link/accent #4493f8`, `quote_bar #3d444d`, `quote_text #9198a1`, `code_bg #151b23`, `inline_code_bg #262c36`, `code_border #3d444d`, `table_border #3d444d`, `table_header_bg #151b23`, H1/H2 bottom border `#3d444d`.

### B.3.3 Callout tokens
| Type | Light color (bar, icon, title) | Light bg | Dark color | Dark bg |
|---|---|---|---|---|
| note | `#0969da` | `#ddf4ff` | `#4493f8` | `#232c38` |
| tip | `#1a7f37` | `#e8faf0` | `#3fb950` (title `#57ab5a`) | `#202d22` |
| important | `#8250df` | `#f6f0ff` | `#ab7df8` | `#2f2938` |
| warning | `#9a6700` | `#fffbe8` | `#d29922` | `#342d1e` |
| caution | `#cf222e` | `#ffebe9` | `#f85149` | `#382423` |
Callout body text uses `quote_text`.

### B.3.4 Syntax highlighting palettes
| Token class | Light | Dark |
|---|---|---|
| default text | `#24292e` | `#c9d1d9` |
| comment | `#6a737d` (italic) | `#8b949e` (italic) |
| keyword, meta/preprocessor | `#d73a49` | `#ff7b72` |
| string, regex | `#032f62` | `#a5d6ff` |
| number, constant, builtin, attribute | `#005cc5` | `#79c0ff` |
| function, class/title | `#6f42c1` | `#d2a8ff` |
| type, variable (special) | `#e36209` | `#ffa657` |
| tag | `#22863a` | `#7ee787` |
| operator, punctuation | `#24292e` | `#c9d1d9` |
| diff-add (line bg / text) | `#e6ffec` / `#22863a` | `#12261e` / `#aff5b4` |
| diff-del (line bg / text) | `#ffebe9` / `#b31d28` | `#25171c` / `#ffdcd7` |
| markdown heading in code | `#005cc5` bold | `#79c0ff` bold |
| code toolbar icons / language label | `#bcbcbc` (hover `#7a7a7a`) | `#5f5f5f` (hover `#9a9a9a`) |

### B.3.5 Chrome tokens
| Token | Light | Dark |
|---|---|---|
| `chrome_bg` (title bar, sidebar) | = `bg` | = `bg` |
| `chrome_border` (sidebar hairline, splitter, cards) | `#e0e0e0` | `#333333` |
| `tab_inactive` | `#7a7a7a` | `#9a9a9a` |
| `history_group` | `#9e9e9e` | `#7a7a7a` |
| `chevron` | `#858585` | `#8a8a8a` |
| `hover` / `pressed` overlay | `rgba(0,0,0,.05)` / `rgba(0,0,0,.09)` | `rgba(255,255,255,.06)` / `rgba(255,255,255,.10)` |
| `menu_bg` / `menu_border` / `menu_hover` / `separator` | `#ffffff` / `#e0e0e0` / `#f0f0f0` / `#eeeeee` | `#2b2b2b` / `#3a3a3a` / `#383838` / `#3a3a3a` |
| `shadow` | `0 8 24 rgba(0,0,0,.14)` | `0 8 24 rgba(0,0,0,.45)` |
| `tooltip_bg` / `tooltip_text` | `#333333` / `#ffffff` | `#f0f0f0` / `#1e1e1e` |
| `toast_bg` / `toast_text` | `rgba(32,32,32,.92)` / `#ffffff` | same |
| `caption_close_hover` | `#c42b1c` (glyph `#ffffff`) | same |
| `gutter_bg` / `gutter_border` (F3) | `#fafafa` / `#f0f0f0` | `#252526` / `#2d2d2d` |
| `scrim` (fullscreen viewers) | `rgba(0,0,0,.82)` | same |
| `focus_ring` | `accent`, 2 px | `accent`, 2 px |
| `scrollbar_thumb` / hover | `rgba(0,0,0,.28)` / `.42` | `rgba(255,255,255,.28)` / `.42` |
Chrome accents (active tab, active TOC item, toggles, focus) use the scheme's `accent`.

### B.3.6 Mermaid palettes
| Element | Light | Dark |
|---|---|---|
| card bg / border | `#f5f5f5` / `#e0e0e0` (follows `code_bg`/`code_border` for colored schemes) | `#2d2d2d` / `#3a3a3a` |
| node fill / stroke / text | `#eff6ff` / `#93c5fd` / `#333333` | `#1e3a5f` / `#60a5fa` / `#e5e7eb` |
| edge / arrowhead | `#94a3b8` | `#94a3b8` |
| edge label bg / text | card bg / `#333333` | card bg / `#e5e7eb` |
| subgraph fill / stroke / title | `#f8fafc` / `#cbd5e1` / `#475569` | `#262b33` / `#475569` / `#cbd5e1` |
| sequence actor box / lifeline / message | `#eff6ff`+`#93c5fd` / `#94a3b8` / `#333333` | `#1e3a5f`+`#60a5fa` / `#64748b` / `#e5e7eb` |
| note fill / stroke | `#fff8c5` / `#e3c34d` | `#3b3415` / `#a88f2a` |
| gantt bar / done / active / crit / milestone / grid / today | `#93c5fd` / `#cbd5e1` / `#60a5fa` / `#f87171` / `#2563eb` / `#e5e7eb` / `#ef4444` | `#3b82f6` / `#475569` / `#60a5fa` / `#ef4444` / `#93c5fd` / `#374151` / `#f87171` |
| pie / categorical series | `#60a5fa #34d399 #fbbf24 #f87171 #a78bfa #f472b6 #22d3ee #a3e635 #fb923c #94a3b8` | same hues at 85 % lightness-adjusted for dark bg |
Font: body family, 14 px; node padding 12 × 8; corner radius 6 for rect nodes (sharp style → 0).

## B.4 Window frame and title bar
- Custom-drawn title bar (`WM_NCCALCSIZE` extends the client area over the caption), **height 32 px**, background `chrome_bg` (seamless with the page). When the document is scrolled away from the top, a 1 px `border_subtle` line appears under the title bar.
- Left → right: 12 px padding · app icon 16 px · 8 px · title text 12 px (`text`; inactive window → 55 % opacity), ellipsized in the middle for long paths · 8 px · mode pill (11 px, padding 2 × 8, radius 9, bg `accent @ 12 %`, text `accent`; "Read-only" uses warning colors) · dirty `●` (`text_muted`).
- Right → left: Close, Maximize/Restore, Minimize (each **46 × 32**, glyphs 10 px from the icon font), then 4 px gap, **⋯ More** (32 × 32, glyph 14 px), **⚙ Settings** (32 × 32, glyph 14 px). Hover/pressed overlays per B.3.5; Close hover `#c42b1c` with white glyph.
- Behavior: drag to move; double-click toggles maximize (single click never restores); dragging a maximized window restores it under the cursor; right-click / `Alt+Space` → system menu; `WM_NCHITTEST` returns `HTMAXBUTTON` over Maximize so Windows 11 **Snap Layouts** appear; resize borders 6 px (outside the visible frame on Windows 11); DWM shadow and Windows 11 rounded corners kept (`DWMWA_WINDOW_CORNER_PREFERENCE`).
- Fullscreen (`F11`): title bar hidden; moving the mouse to the top 4 px reveals it temporarily.

## B.5 Sidebar
### B.5.1 Frame
Width 260 (180–480, drag the inner edge; cursor `IDC_SIZEWE` over a 6 px grip). Background `chrome_bg`; 1 px `chrome_border` on its inner edge. Floating positions: card 12 px from the window edges and 8 px below the title bar, radius 12, `shadow`, 1 px `chrome_border`.
### B.5.2 Tabs row
Height 38; tabs left-aligned starting at x = 4; each tab: 13 px text, horizontal padding 10; inactive `tab_inactive`, hover `text`, active `accent` with a 2 px `accent` underline spanning the tab width; 1 px `chrome_border` under the row. Hidden tabs are removed; if only Contents remains, the row is hidden.
### B.5.3 Contents (outline)
List top padding 12; row height `round(toc_font_size × 1.95)` (25 at 13 px); text `toc_font_size` px, color `text`, single line with ellipsis + tooltip; left padding 20 + 12 px per depth level (after title flattening, R-NAV-05); H1 rows weight 600; **active row** color `accent`; hover overlay radius 4 (inset 4 px horizontally). Collapsible rows show a muted 8 px chevron at x = (indent + 6) — ▾ expanded / ▸ collapsed. Top fade: when the list is scrolled, a 16 px gradient from `chrome_bg` to transparent overlays the top.
### B.5.4 History
Group header: 11 px `history_group`, 14 px top margin, row height 24; hover shows "Clear" (11 px link) at the right. Item rows: 13 px `text`, height 29, left padding 12; current file tagged "Current" (11 px `accent`, right-aligned 34 px from the sidebar's inner edge); hover shows × (12 px) at the right; missing files 50 % opacity.
### B.5.5 Favorites & Files
Favorites: same row style as History items (star 12 px muted at left). Empty state: centered star icon 24 px muted + hint text 12 px muted. Files: tree rows 13 px, height 26, folder chevron + folder glyph 14 px, file glyph 14 px, indent 16 per level, current file `accent`.
### B.5.6 Edge toggle and scrollbars
Toggle: a 16 × 32 hit area centered vertically **on the sidebar's inner hairline**, glyph chevron 12 px `chevron` ("‹" when open on the left, "›" when closed — then it sits 4 px from the window's left edge). Hover: `hover` overlay radius 4, glyph `text`. It follows the edge while resizing. Mirror for right positions.
Overlay scrollbars (document, sidebar lists, panels): 8 px wide (12 px on hover), 2 px from the edge, thumb radius = width / 2, min thumb length 32, track invisible (hover: `hover` overlay). Shown on scroll/hover, hidden after 1 000 ms idle with a 300 ms fade (setting off → always visible). Thumb drag; track click pages.

## B.6 Document layout and elements
### B.6.1 Column
Available area = client area minus docked sidebar. Column width = `clamp(column_width_pct % × client_width, min(360, available − 48), available − 48)`, centered within the available area, always ≥ 20 px from the sidebar. Top padding 48 (below the title bar), bottom padding 120. Cursor: I-beam over text, hand over links/buttons, arrow elsewhere.
### B.6.2 Blocks and spacing
Paragraph margin-bottom `paragraph_spacing` (16). Headings: H1 margin 0 0 16 (top margin 0 if first block, else 32); H2 24 0 16; H3 24 0 12; H4–H6 20 0 8; H6 color `text_muted`. Strong = weight 600 (DirectWrite chooses the nearest available weight). Strikethrough line in `text`. `==highlight==` → `highlight_bg` with 2 px horizontal padding.
### B.6.3 Inline code and kbd
Inline code: code font 0.875 em, bg `inline_code_bg`, padding 0.15 em × 0.4 em (the horizontal padding is realized as extra character spacing on the span's first and last character, so neighbouring glyphs never overlap), radius 4 (sharp 0). `<kbd>`: code font 0.8125 em, bg `code_bg`, 1 px `code_border` with a 2 px bottom border, radius 4, padding 1 × 5.
### B.6.4 Lists
Content starts 40 px right of the list's left edge; the marker sits right-aligned in that box with an 8 px gap. Bullet: filled disc (depth 1), ring (depth 2), square (depth 3+), size 0.35 em, color `text`. Ordered: "1." in `text`, tabular figures. Tight lists: no extra spacing; loose lists: 8 px between items. Task items: 16 × 16 checkbox at the marker position, radius 4, 1.5 px border `#b0b0b0` (dark `#6a6a6a`); checked → `accent` fill with a white check.
### B.6.5 Blockquote
Margin 16 0; 3 px bar `quote_bar` at the left; content starts 16 px after the bar; text `quote_text`; nested quotes add another bar 16 px further in.
### B.6.6 Callout
Margin 16 0; bg per B.3.3; 3 px bar in the callout color; padding 12 16 12 16 (measured from the bar); radius 0 6 6 0 (sharp 0). Title row: 14 px icon glyph + 6 px + title 14 px weight 600 in the callout color; 8 px gap to the body; foldable callouts have a chevron at the far right of the title row. Default titles (localized): Note, Tip, Important, Warning, Caution.
### B.6.7 Code block
Margin 16 0; bg `code_bg`; 1 px `code_border`; radius 8 (sharp 0). A **28 px header strip** inside the top + 16 px padding before the first line; left/right padding 32; bottom padding 16. Code text per B.2; line numbers (when on): right-aligned in a gutter `digits × digit_width + 16`, color `line_number`, text starts 12 px after the gutter. Toolbar inside the header strip, right-aligned, 8 px from the right: [copy] [line numbers] [wrap] buttons 24 × 24 (glyph 13 px) + language label 11 px; colors per B.3.4; visible on hover (120 ms fade) or keyboard focus; active toggles shown in `text` color (never blue). Copy → glyph becomes a check for 1.2 s + toast "Copied". Overflow: a 6 px horizontal scrollbar at the block's bottom, always visible while overflowing.
### B.6.8 Table
Margin 16 0; auto layout (shrink-to-content up to the column width; min column 48 px; then wrap, breaking anywhere if a token is too long). 1 px `table_border` grid; header bg `table_header_bg`, header weight 600; cell padding 8 × 8; alignment per column; zebra (setting) on even body rows with `zebra_bg`; rounded style clips outer corners at radius 6. Hover toolbar: a small pill above the table's top-right corner (28 px tall, icons 16 px, 60 % opacity → 100 % on hover): read mode [⛶] [CSV]; F2 adds the editing tools (R-TBL-02).
### B.6.9 Horizontal rule, footnotes, images, front matter, details
- HR: 2 px `border_subtle`, margin 24 0.
- Footnote reference: superscript 0.75 em `link` text `[n]`. Footnotes section: margin-top 32, 1 px `border_subtle` top border, padding-top 16, 14 px `text_muted`, numbered; `↩` back-link in `link`.
- Images: block images margin 8 0, left-aligned (centered when the HTML says `align="center"` or inside a centered `<p>/<div>`); inline images sit on the baseline. Hover badge ⛶: 28 × 28 circle `toast_bg` with a white 13 px glyph, 8 px inside the top-right corner. Placeholder: 1 px dashed `code_border`, padding 12, icon 16 px + text 13 px `text_muted`.
- Front matter card: margin 0 0 16, bg `code_bg`, radius 6, padding 8 × 12; collapsed: 13 px `text_muted` line "Front matter · N fields" + chevron; expanded: 2-column grid, keys in code font 13 px `text_muted`, values 13 px `text`.
- `<details>`: summary row with 12 px chevron and weight 600; content indented 16; margin 8 0.
### B.6.10 Links
`link` color, no underline; underline on hover; hand cursor; status tooltip (B.10.4).
### B.6.11 Math
Inline: baseline-aligned, 1.21 em. Display: centered, margin 16 0, no card; overflow scrolls horizontally; `\tag` numbers right-aligned in `text_muted`. Errors in `error` color.
### B.6.12 Mermaid card
Margin 16 0; bg/border per B.3.6; radius 8 (sharp 0); padding 24; diagram centered and scaled to fit (never upscaled), not below the minimum scale of R-MMD-02; a wider diagram scrolls horizontally inside the card (thin overlay scrollbar on hover, `Shift+Wheel`, like code blocks); hover ⛶ badge like images.
### B.6.13 Selection and find highlights
Selection fill `selection`, drawn beneath text, per line box (no gaps between lines). Find: all matches `find_match_bg`, current `find_current_bg`, radius 2.

## B.7 Menus
### B.7.1 Style
Popup windows (layered, owner-drawn), width 260 (min; grows for long labels up to 360), padding 4, radius 8 (sharp 0), bg `menu_bg`, 1 px `menu_border`, `shadow`. Item: height 32, padding 0 12, 20 px icon column (16 px glyph), label 13 px, shortcut right-aligned 12 px `text_muted`, submenu chevron; hover/keyboard-selected: `menu_hover` radius 4; disabled 40 % opacity; check/radio marks in the icon column; separators 1 px `separator` with 4 px vertical margin. Opens at the cursor (or under the ⋯ button), flips to stay on-screen and on the same monitor. Submenus open after 150 ms hover or with `→`/`Enter`; `←`/`Esc` closes one level; `↑/↓/Home/End` navigate; typing a letter jumps to the next item starting with it. Appear: 80 ms fade.
### B.7.2 Document context menu (read mode) — also shown by the ⋯ button
1. **Copy** `Ctrl+C` *(selection only)* · **Copy as Markdown** *(selection only)* · **Copy link address** *(on a link)* · **Select all** `Ctrl+A`
2. ─
3. **Appearance ▸** — Theme mode ▸ (◉ Follow system / Light / Dark) · **Color scheme…** (opens the card flyout, B.7.3) · **Layout settings…** (Settings → Typography & Layout) · ✓ Show outline `Ctrl+\` · ✓ ⏎ Single newline as line break
4. **Edit ▸** — Live Edit `F2` · Split Edit `F3` · Open in external editor `F4`
5. **Export ▸** — Export PDF… `Ctrl+Shift+E` · Export HTML… · Print… `Ctrl+P` · View as mind map `Ctrl+Shift+M`
6. **☆ Add to favorites** `Ctrl+D` / **★ Remove from favorites**
7. **File ▸** — Open… `Ctrl+O` · Reload `F5` · Show in Explorer · Copy file path · Reopen with encoding ▸ · Document info…
8. ─
9. **Settings…** `Ctrl+,` · **Keyboard shortcuts** `F1` · **About YAMDV**

In **F2 / F3** the menu starts with: Cut · Copy · Paste · Paste as plain text · ─ · **Format ▸** · **Insert ▸** · **Table ▸** *(inside a table)* · ─ and then continues with items 3–9.
### B.7.3 Color-scheme flyout
Grid of 11 cards, 3 columns, card 112 × 84 (radius 8): top 56 px preview drawn in that scheme (page bg; a 40 %-wide 6 px heading bar in `heading`; two 4 px text lines in `text` @ 35 %; a 3 px `accent` bar; a tiny code rect in `code_bg`), name 12 px below. Hover = live preview of the whole window + 1 px `accent` ring; selected = 2 px `accent` ring + check badge. `Esc` / leaving the flyout reverts the preview.

## B.8 Settings panel
### B.8.1 Layout
In-window modal panel 720 × 560 (clamped to window − 48), centered, radius 12, `menu_bg`, `shadow`, backdrop `rgba(0,0,0,.25)`; draggable by its 48 px header (title "Settings" 15 px 600, close ×). Left navigation 180 px: rows 32 px, 13 px, active row `menu_hover` + 3 px `accent` bar. Right content: scrollable, padding 24; section headings 14 px 600, 1 px `separator` between groups; each setting row: label 13 px + optional 12 px `text_muted` description on the left, control on the right. Controls (all custom-drawn, keyboard-operable): toggle switch 36 × 20 (on = `accent`), segmented control (height 28), dropdown (opens a menu-styled list), slider (with live value label), text input (height 32, radius 6, 1 px `chrome_border`, focus ring), buttons (primary: `accent` bg + white text; secondary: 1 px border), color swatch (24 × 24, radius 6) opening the color picker popover, shortcut capture field ("Press a key combination…", `Esc` cancels, `Backspace` clears, conflicts shown inline in `error`).
Sections: **General · Appearance · Typography & Layout · Sidebar · Reading · Editing · Shortcuts · External editor · Export & Print · Files & Integration · Privacy & Security · About**.

### B.8.2 Settings table (normative — `settings.ini`)
| Section / key | Type | Default | Range / values |
|---|---|---|---|
| `general.language` | enum | `system` | `system`, `en`, `tr` |
| `general.title_format` | enum | `filename` | `filename`, `fullpath` |
| `general.open_external_links` | enum | `ask` | `ask`, `direct`, `never` |
| `general.restore_reading_position` | bool | `true` | |
| `general.remember_window_geometry` | bool | `true` | |
| `appearance.mode` | enum | `system` | `system`, `light`, `dark` |
| `appearance.scheme` | enum | `default` | 11 schemes (R-THEME-02) |
| `appearance.corner_style` | enum | `rounded` | `rounded`, `sharp` |
| `appearance.table_zebra` | bool | `false` | |
| `appearance.use_custom_colors` | bool | `true` | applies `custom-theme.ini` if present |
| `typography.body_font` | string | `Segoe UI` | installed family |
| `typography.code_font` | string | `Cascadia Mono` | installed family (fallback chain B.2) |
| `typography.font_size` | int | `16` | 10–32 |
| `typography.line_height` | float | `1.6` | 1.2–2.2, step 0.05 |
| `typography.paragraph_spacing` | int | `16` | 0–48 |
| `typography.column_width_pct` | int | `65` | 30–100, step 5 |
| `sidebar.visible` | bool | `true` | |
| `sidebar.position` | enum | `left` | `left`, `right`, `left-floating`, `right-floating` |
| `sidebar.width` | int | `260` | 180–480 |
| `sidebar.toc_font_size` | int | `13` | 11–18 |
| `sidebar.toc_max_depth` | int | `4` | 1–6 |
| `sidebar.show_history` / `show_favorites` / `show_files` | bool | `true` | |
| `reading.single_newline_break` | bool | `false` | |
| `reading.smart_punctuation` | bool | `false` | |
| `reading.subscript_tilde` | bool | `false` | R-MD-06 |
| `reading.code_line_numbers` | bool | `false` | |
| `reading.code_wrap` | bool | `false` | |
| `reading.front_matter` | enum | `collapsed` | `collapsed`, `expanded`, `hidden` |
| `reading.render_math` / `render_mermaid` / `render_html` / `emoji_shortcodes` | bool | `true` | |
| `reading.animate_gifs` | bool | `false` | |
| `reading.smooth_scrolling` | bool | `true` | |
| `reading.scrollbar_autohide` | bool | `true` | |
| `reading.word_count_badge` | bool | `false` | |
| `reading.task_toggle_in_read_mode` | bool | `false` | |
| `editing.autosave` | bool | `true` | |
| `editing.autosave_delay_ms` | int | `800` | 300–5000 |
| `editing.assets_dir` | string | `assets` | sanitized relative folder name |
| `editing.editor_font` | enum | `body` | `body`, `code` |
| `editing.editor_line_numbers` | bool | `true` | |
| `editing.editor_soft_wrap` | bool | `true` | |
| `editing.editor_syntax_tint` | bool | `false` | |
| `editing.split_ratio` | float | `0.5` | 0.2–0.8 |
| `editing.split_swapped` | bool | `false` | |
| `editing.format_toolbar` | bool | `true` | |
| `editing.auto_format_tables` | bool | `true` | |
| `editing.snippets` | bool | `true` | |
| `editing.auto_pair` | bool | `false` | |
| `editing.list_indent` | enum | `2` | `2`, `4`, `tab` |
| `shortcuts.preset` | enum | `default` | `default`, `typora` (Appendix F §F.6) |
| `shortcuts.<action_id>` | chord | — | overrides (Appendix F) |
| `external_editor.mode` | enum | `auto` | `auto`, `custom` |
| `external_editor.custom_exe` | path | `""` | absolute `.exe` |
| `external_editor.custom_args` | string | `{file}` | placeholders `{file}` `{line}` `{col}` |
| `export.pdf_colors` | enum | `light` | `light`, `theme` |
| `export.pdf_page_size` | enum | `auto` | `auto`, `a4`, `letter` |
| `export.pdf_margins_mm` | int | `18` | 5–40 |
| `export.pdf_page_numbers` | bool | `false` | |
| `export.print_colors` | enum | `light` | `light`, `theme` |
| `export.html_colors` | enum | `light` | `light`, `theme` |
| `export.html_include_toc` | bool | `true` | |
| `export.csv_formula_guard` | bool | `true` | S-EXP-04 |
| `integration.context_menu` | bool | `false` | mirrors registry state |
| `privacy.diagnostic_log` | bool | `false` | local file only, no content/paths unless enabled |
"Files & Integration" also shows: default-app status + [Set as default app], config folder path + [Open folder], [Edit custom-theme.ini], portable-mode indicator. "Privacy & Security" shows the offline statement ("YAMDV never connects to the internet; it has no network code") and how to verify it (firewall test). "About" shows version, build commit, license, exe path (click to copy), config path.

## B.9 Find bar
Anchored top-right of the document area: 12 px below the title bar, 16 px from the right (left of the scrollbar). 380 × 40, radius 8, `menu_bg`, 1 px `menu_border`, `shadow`. Contents: search glyph · input (flex) · count "3/17" 12 px `text_muted` ("No results" in `error`) · [↑] [↓] 24 × 24 · [Aa] match case · [ab] whole word · [×]. In edit modes a [▸] toggles a replace row (height becomes 76): input · [Replace] · [Replace all]. Opening pre-fills with the selection; `Enter` next, `Shift+Enter` previous, `Esc` closes and keeps the current match selected.

## B.10 Transient UI
### B.10.1 Toasts
Bottom-center, 24 px above the bottom edge; height 32, padding 0 14, radius 16, `toast_bg`, 13 px `toast_text`, optional 14 px icon and an action link (`#8ab4ff`). Max 3 stacked (8 px apart). Duration 1 600 ms (errors 4 000 ms); fade in 120 ms, out 200 ms.
### B.10.2 Zoom indicator
Window center, 140 × 48, radius 10, `toast_bg`, 15 px text ("18 px", "Column 70 %"), visible 900 ms.
### B.10.3 Word-count badge
Bottom-right, 12 px from the bottom and from the scrollbar; height 24, padding 0 10, radius 12, `chrome_bg` @ 90 % with 1 px `chrome_border`, 12 px `text_muted`; tooltip with details (R-READ-07).
### B.10.4 Link status tooltip
Bottom-left, 8 px inset; height 22; max width 60 % of the window; 12 px; `tooltip_bg`/`tooltip_text`; radius 4; middle-ellipsis for long URLs/paths.
### B.10.5 Banners
Top of the document area (under the title bar), full available width, height 36, bg = warning or note callout bg, 13 px text with a leading icon, text buttons right-aligned, × when dismissible. Used for: file changed on disk (with unsaved edits), file deleted, read-only, decoding fallback notice, recovery available, Mark-of-the-Web (S-LINK-04), running elevated (S-PROC-06), export failure (R-EXP-07). At most 2 banners show at once (newest on top); the content moves down by their height and keeps its scroll anchor.
### B.10.6 Popovers and dialogs
Popover (external link "Ask", link editor, language picker, color picker): width 360, radius 8, `menu_bg`, `shadow`, anchored to its target; URL shown in code font 12 px, wrapped. In-window confirm dialog: width 420, title 15 px 600, body 13 px, buttons right-aligned (primary last), `Enter` = primary, `Esc` = cancel, backdrop `rgba(0,0,0,.25)`. Native dialogs are used only for open/save files and printing. Tooltips: 500 ms delay, 12 px, padding 4 × 8, radius 4.
### B.10.7 Export progress card
Bottom-center, above the toast stack; width 360, radius 8, `menu_bg`, `shadow`; shown only when a job runs longer than 300 ms. Line 1: 13 px 600 "Exporting PDF…" / "Printing…" / "Exporting HTML…" plus the file name (middle ellipsis). Bar: 4 px, `accent` on a `border` track; indeterminate while preparing (static with reduced motion), determinate after. Line 2: 12 px `text_muted` "Page 37 of 1093 · 3 % · about 3 min left" (HTML counts blocks; the estimate appears after 5 pages or 5 %). [Cancel] right-aligned; `Esc` cancels while the card has focus. Success replaces it with a toast "PDF saved — <name>" with [Open] · [Show in folder]; failure replaces it with an error banner (B.10.5). The taskbar button mirrors the progress (`ITaskbarList3`).

## B.11 Fullscreen viewers
Overlay inside the window. Image / table / Mermaid: backdrop `scrim`; content centered at "fit" (≤ 92 % of the window); toolbar pill top-right (12 px inset), height 36, bg `rgba(32,32,32,.9)`, white 14 px glyphs: [−] [100 %] [+] [fit] [1:1] [copy] [×]. Wheel zooms at the cursor (10 %–800 %, ×1.1 per notch), drag pans (hand cursor), double-click toggles fit/100 %, `+ - 0` keys, `Esc` closes. Table viewer: the table on a page-colored card (radius 8) with a 28 px header ("Table · R × C" + [Export CSV]) that moves the card, a 14 px resize grip at the bottom-right, natural column widths, scrollbars. Mind map: opaque `bg` instead of the scrim, same toolbar plus [Right | Balanced] layout toggle.

## B.12 Welcome page (no document open)
Sidebar hidden. Centered vertical stack (max width 560): app icon 64 px · 16 px · "YAMDV" 28 px 600 · subtitle "Read instantly. Edit in place." 14 px `text_muted` · 32 px · **drop zone** 560 × 200, 2 px dashed `chrome_border`, radius 12 (on drag-over: border `accent`, bg `accent @ 6 %`), containing a 32 px document glyph, "Drop a Markdown file here" 15 px, "or" 12 px `text_muted`, primary button "Open file…  Ctrl+O" · 24 px · **Recent** (12 px `text_muted` label; up to 8 rows of 32 px: file name 13 px + folder 12 px `text_muted`, middle-ellipsized; hover ×) · 24 px · **default-app status**: if not default → warning-tinted pill "YAMDV is not your default Markdown app" + [Set as default]; if default → `text_muted` "✓ Default app for .md files". Footer 16 px from the bottom: "v<version> · Shortcuts (F1) · Settings (Ctrl+,)" 12 px `text_muted`, the last two are links.

## B.13 Edit modes
### B.13.1 Split Edit (F3)
Title pill "Split Edit". Editor pane (left by default): bg `bg`; gutter `gutter_bg` with 1 px `gutter_border` on its right, width `max(3, digits) × digit_width + 20`, line numbers 11 px `line_number` right-aligned (current line `text_muted`); text area left padding 8, top padding 48 (aligned with the preview's first block); editor font per `editor_font`, size = body size, line height = body line height. Caret: 1.5 px wide (2 px at ≥ 150 % scale), color `text`, blink at the system rate, **blinking stops after 5 s without input** (keeps idle CPU at 0). Current line: `hover` overlay. Splitter 6 px `chrome_border` (hover `accent @ 40 %`, cursor `IDC_SIZEWE`); a 24 px ⇄ swap button appears at the splitter's top on hover. Preview pane: normal reading layout scaled into its width; the sidebar toggle chevron sits at the preview's inner edge. "?" help button: 32 px circle at the editor pane's bottom-right (16 px inset), bg `menu_hover`, glyph 14 px `text_muted`.
### B.13.2 Live Edit (F2)
Title pill "Live Edit". Revealed markers in `marker` color, same size as the surrounding text. Caret as in F3. Format toolbar: height 36, radius 8, `menu_bg`, `shadow`, 28 × 28 icon buttons (14 px glyphs) in groups separated by 1 px `separator`, placed 8 px above the selection (below if there is no room). Table tools per B.6.8; row/column grips 12 × 20 with ⋮⋮ dots in `text_muted`, drop indicator 2 px `accent`. Code-block language label becomes an input on click. Inline popovers per B.10.6.

## B.14 Icons and logo (original design — generated by `cargo xtask gen-icons`)
- **App icon:** rounded square (corner radius 22 % of size) with a vertical gradient `#2563eb → #1d4ed8`; a white geometric "Y" built from three strokes occupying the left ~60 %, and a white downward arrow (stem + chevron) in the right ~40 % ("Y↓"). Sizes 16, 20, 24, 32, 40, 48, 64, 128, 256 (256 stored as PNG inside the ICO). At 16 px use a simplified drawing (thicker strokes, no gradient).
- **Document icon:** white page with a folded top-right corner and a `#d0d0d0` outline; the app badge (Y↓, blue) centered in the lower half.
- **UI glyphs:** from the system icon font (B.2). Suggested code points (verify each with `GetGlyphIndices` at startup; draw a simple vector fallback if missing): Settings `E713`, More `E712`, ChevronLeft `E76B`, ChevronRight `E76C`, ChevronDown `E70D`, ChevronUp `E70E`, Copy `E8C8`, Search `E721`, Close `E8BB`, Minimize `E921`, Maximize `E922`, Restore `E923`, FavoriteStar `E734`, FavoriteStarFill `E735`, FullScreen `E740`, BackToWindow `E73F`, ZoomIn `E8A3`, ZoomOut `E71F`, Delete `E74D`, Folder `E8B7`, Document `E8A5`, History `E81C`, Edit `E70F`, Link `E71B`, Print `E749`, Save `E74E`, Info `E946`, Warning `E7BA`, Help `E897`, Add `E710`, Remove `E738`, CheckMark `E73E`, Sort `E8CB`, Lightbulb `EA80`, Code `E943`, BulletedList `E8FD`. Callout icons: note → Info, tip → Lightbulb, important → a filled-circle "!" (vector), warning → Warning, caution → an octagon "!" (vector).

## B.15 Motion
| Animation | Duration | Easing |
|---|---|---|
| Smooth scroll (wheel notch, keys, TOC jump) | 120–180 ms (TOC jump up to 300 ms) | ease-out cubic |
| Sidebar show/hide | 160 ms | ease-out |
| Menu / popover appear | 80 ms fade | linear |
| Toast in / out | 120 / 200 ms | ease-out |
| Scrollbar fade-out | 300 ms after 1 000 ms idle | linear |
| Code toolbar fade | 120 ms | linear |
| Heading fold / TOC collapse | 120 ms | ease-out |
With reduced motion (R-A11Y-02) every animation is instant. Animation timers run **only while something animates**.

## B.16 Interaction states
Hover → `hover` overlay; pressed → `pressed` overlay; keyboard focus → 2 px `focus_ring` with 1 px offset, shown only after keyboard interaction (like `:focus-visible`); disabled → 40 % opacity, no hover. All clickable targets ≥ 24 × 24 px.

## B.17 Visual acceptance
Every UI phase delivers golden PNGs (Appendix H §H.4) of: welcome page; reading view with the sidebar (Default light, Default dark, and every scheme in light); each element of B.6; menus; settings (every section); find bar; viewers; F2 and F3. The owner compares them with the reference descriptions above.
