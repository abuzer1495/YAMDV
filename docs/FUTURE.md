# Possible future developments (post-v1)

Owner decision 2026-10-17: v1 is a complete product; the items below are deferred. Each becomes a GitHub issue (label `future`, milestone `post-v1`) at the v1 release. "Spec IDs" lists the requirements (or parts of them) that are deferred.

| FUT | Deferred feature | v1 instead | Spec IDs |
|---|---|---|---|
| FUT-01 | Highlighting for languages beyond the 30 existing lexers | 30 lexers + aliases | R-CODE-01 |
| FUT-02 | Floating sidebar positions | left / right docked | R-NAV-02 |
| FUT-03 | Favorites drag-to-reorder | add / remove | R-NAV-09 |
| FUT-04 | In-app custom-color editor with picker | `custom-theme.ini` + hot reload | R-THEME-06 |
| FUT-05 | Shortcut presets and in-app rebinding UI | overrides in `settings.ini` | R-SET-06, F.6 |
| FUT-06 | Table fullscreen viewer | — | R-READ-08, R-REN-07 |
| FUT-08 | Heading jump palette; `[TOC]` placeholder | sidebar outline | R-READ-12, R-MD-19 |
| FUT-09 | "Reopen with encoding" menu | automatic detection | R-OPEN-06 |
| FUT-10 | Editor auto-detection beyond VS Code, Notepad++, Notepad | custom command | R-EXT-02 |
| FUT-12 | Duplicate/select/move line, auto-pairing | core Markdown keys | R-EDIT-07 |
| FUT-15 | HTML clipboard → Markdown on paste | plain-text paste | R-EDIT-05 |
| FUT-17 | Math beyond the core subset; macros | core subset (V1-C) | R-MATH-02, R-MATH-07 |
| FUT-18 | Mermaid gantt, pie and all other types; `%%{init}%%`; visual editing; Copy as PNG; extra shapes; `o`/`x` edges; `linkStyle` | flowchart + sequence (V1-C) | R-MMD-01…10, R-F2-11 |
| FUT-20 | SVG images; animated GIF | placeholder; first frame | R-IMG-01 |
| FUT-21 | Interactive HTML export (scripted features) | static HTML, no script | R-EXP-02, S-EXP-02 |
| FUT-23 | Copy document as rich HTML; PNG export of diagrams | — | R-EXP-06 |
| FUT-25 | Jump list; custom app/document icons | — | R-SYS-08, R-SYS-09 |
| FUT-26 | Settings search; F2 image resize handles | — | R-SET-02, R-F2-12 |
| FUT-27 | VS Code / Sublime live-preview plugins | — | A.0 |
| FUT-28 | macOS / Linux ports | Windows only | E.14 |
| FUT-29 | UI languages beyond English and Turkish | EN, TR | R-I18N-01 |
| FUT-30 | Custom-drawn title bar with ⚙/⋯ buttons and Snap-Layouts hit testing | system title bar (dark-mode aware) | B.4, R-WIN-05 |
| FUT-31 | Settings panel UI | settings in the context menu + "Edit settings file" | R-SET-01…04, B.8.1 |
| FUT-32 | Color-scheme cards with hover preview | radio list in the menu | R-THEME-03, B.7.3 |
| FUT-33 | Files tab in the sidebar | — | R-NAV-10 |
| FUT-34 | Heading folding (incl. Alt+click fold-all) | — | R-READ-06 |
| FUT-35 | Word-count badge | — | R-READ-07 |
| FUT-36 | Fullscreen viewers and ⛶ badges (image, table, diagram) | images/diagrams fit the column | R-IMG-05, R-READ-08, R-MMD-02 |
| FUT-37 | Mind-map view | — | R-MAP-01, R-MAP-02 |
| FUT-39 | Native PDF writer with selectable text, bookmarks, links | PDF via the print path to "Microsoft Print to PDF" | R-EXP-01 |
| FUT-40 | Tooltip, anchored popover and banner components (link titles, abbr, footnote preview, MotW/elevation/lossy banners) | in-window dialog for confirmations, toast for notices; MotW still forces *Ask* | B.10.4–B.10.6, R-REN-10 (tooltip), S-UI-03 |
| FUT-41 | Inline objects for sup/sub/footnote refs (baseline shift) | smaller text without shift | R-REN-03 |
| FUT-42 | Table row laziness; code-toolbar fade; DIB vs DDB measurement | — | R-REN-07, B.15, D.4 |
| FUT-43 | Obsidian-style inline marker reveal in F2; table cell editing in F2 | block-level source reveal | R-F2-02, R-F2-03 |
| FUT-44 | Table editing operations and toolbar | edit tables as source | R-TBL-01…08 |
| FUT-45 | Snippets; image/file paste into the document; recovery journal; find & replace; regex; F2 format toolbar | formatting shortcuts | R-EDIT-06, R-EDIT-08, R-EDIT-10, R-EDIT-13, R-F2-04, R-F3-07 |
| FUT-46 | F11 distraction-free fullscreen | — | R-WIN-06 |
| FUT-47 | High Contrast mode and full UI Automation | — | R-THEME-08, R-A11Y-03 |
| FUT-48 | ICO and WebP decoding under ACG | "unsupported format" placeholder | R-IMG-01 |
| FUT-49 | Code-font picker | body font via the system font dialog | R-THEME-04 |
| FUT-50 | Security hardening phase and optimization phase (final budgets) | soft budgets | Appendix C review, Appendix D final |

(FUT-07, -11, -13, -14, -16, -19, -22 and -24 were merged into FUT-34, -45, -45, -44, -45, -37, -39 and -47.)
