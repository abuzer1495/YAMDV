# APPENDIX F — Keyboard, Mouse & CLI (`docs/SHORTCUTS.md`)

Every binding maps to an `Action` id (E.7) and is rebindable in Settings → Shortcuts. macOS-style alternatives are not needed (Windows only). "Read" = read mode, "Edit" = F2 or F3.

## F.1 Global
| Action id | Default binding | Scope / notes |
|---|---|---|
| `open_file` | `Ctrl+O` | all |
| `close_window` | `Ctrl+W`; `Esc` | `Esc` only in read mode with no overlay/popup open |
| `reload` | `F5` | all (edit: asks if dirty) |
| `toggle_sidebar` | `Ctrl+\` | all |
| `sidebar_contents` / `sidebar_history` / `sidebar_favorites` / `sidebar_files` | `Ctrl+Shift+1` / `2` / `3` / `4` | all |
| `toggle_favorite` | `Ctrl+D` | all |
| `find` | `Ctrl+F` | all |
| `find_next` / `find_prev` | `Enter` / `Shift+Enter` in the find bar; `Ctrl+G` / `Ctrl+Shift+G` anywhere | all |
| `font_bigger` / `font_smaller` / `font_reset` | `Ctrl+=` (`Ctrl+Num+`), `Ctrl+↑`, `Ctrl+Wheel↑` / `Ctrl+-`, `Ctrl+↓`, `Ctrl+Wheel↓` / `Ctrl+0` | all |
| `column_wider` / `column_narrower` / `column_reset` | `Alt+↑`, `Alt+Wheel↑` / `Alt+↓`, `Alt+Wheel↓` / `Alt+0` | read (in edit, `Alt+↑/↓` = move line; wheel variants still work) |
| `live_edit` | `F2` | toggles |
| `split_edit` | `F3` | toggles |
| `external_editor` | `F4`, `Ctrl+E` | all |
| `export_pdf` | `Ctrl+Shift+E` | all |
| `export_html` | — | all |
| `print` | `Ctrl+P` | all |
| `mind_map` | `Ctrl+Shift+M` | all |
| `settings` | `Ctrl+,` | all |
| `shortcuts_help` | `F1` | all — overlay listing current bindings |
| `fullscreen` | `F11` | all |
| `nav_back` / `nav_forward` | `Alt+←`, mouse X1 / `Alt+→`, mouse X2 | all |
| `copy` / `copy_markdown` / `select_all` | `Ctrl+C` / `Ctrl+Shift+C` / `Ctrl+A` | all |
| `heading_jump` (MAY) | `Ctrl+Shift+O` | all |
| `toggle_word_count`, `toggle_single_newline_break`, `toggle_line_numbers_all`, `toggle_wrap_all` | unbound | menu/Settings |

## F.2 Read-mode navigation
`↑/↓` 40 px · `PgUp/PgDn`, `Space/Shift+Space` one viewport minus 40 px · `Home/End`, `Ctrl+Home/End` start/end · `Tab`/`Shift+Tab` move focus through links and interactive elements (code toolbars, table toolbars, fold toggles) · `Enter` activates the focused element.

## F.3 Edit modes (F2 and F3)
| Action id | Binding |
|---|---|
| `save` | `Ctrl+S` |
| `undo` / `redo` | `Ctrl+Z` / `Ctrl+Y`, `Ctrl+Shift+Z` |
| `cut` / `paste` / `paste_plain` | `Ctrl+X` / `Ctrl+V` / `Ctrl+Shift+V` |
| `bold` / `italic` / `strikethrough` / `highlight` / `inline_code` | `Ctrl+B` / `Ctrl+I` / `Ctrl+Shift+X` / `Ctrl+Shift+H` / ``Ctrl+` `` |
| `link` / `image` | `Ctrl+K` / `Ctrl+Shift+I` |
| `code_block` | `Ctrl+Shift+K` |
| `heading_1` … `heading_6` / `paragraph` | `Ctrl+1` … `Ctrl+6` / `Ctrl+Shift+0` |
| `quote` / `bullet_list` / `numbered_list` / `task_list` | `Ctrl+Shift+Q` / `Ctrl+Shift+8` / `Ctrl+Shift+7` / `Ctrl+Shift+9` |
| `toggle_task` | `Ctrl+Enter` |
| `insert_table` | `Ctrl+T` |
| `move_line_up` / `move_line_down` | `Alt+↑` / `Alt+↓` |
| `duplicate_line` / `select_line` | `Ctrl+Shift+D` / `Ctrl+L` |
| `replace` | `Ctrl+H` |
| `indent` / `outdent` / `snippet_expand` | `Tab` / `Shift+Tab` / `Tab` (snippet takes precedence when the word before the caret is a trigger) |
| `exit_edit` | `Esc` |
Table context (R-TBL): `Tab`/`Shift+Tab` next/previous cell, `Enter` cell below, `Shift+Enter` `<br>`, `Ctrl+Alt+↑/↓` move row, `Ctrl+Alt+←/→` move column, `Ctrl+Shift+Backspace` delete row.

## F.4 Command line
```
yamdv [<file> …]                 open files (one window each; forwarded to a running instance)
yamdv <file> --line <n>          open and scroll to source line n
yamdv --help | --version
yamdv --settings                 open the settings panel
yamdv --register | --unregister  per-user file-type registration (never changes the default app)
yamdv --set-default              open Windows "Default apps" for YAMDV
yamdv --context-menu on|off      Explorer "Open with YAMDV" entry
yamdv --install | --uninstall    per-user self-install / removal (R-SYS-05)
yamdv --export-html <in.md> [-o <out.html>] [--theme <scheme>] [--mode light|dark]
yamdv --export-pdf  <in.md> [-o <out.pdf>]  [--theme <scheme>] [--mode light|dark] [--page-size a4|letter]
```
Dev builds only (feature `dev-tools`, never in release): `--render-png <in.md> -o <out.png> [--width 1200] [--scale 1.0|1.5] [--theme …] [--mode …]`, `--mem-hud`.
Exit codes: `0` success · `1` runtime error · `2` usage error. Console output only when started from a console (`AttachConsole`).
Internal flag, not shown in `--help`: `--dialog-host <kind> <owner-hwnd> <nonce> …` — used only by YAMDV itself (S-PROC-07, S-IPC-05); it exits with code 2 when started any other way.

## F.5 Mouse
Wheel scroll (smooth) · `Shift+Wheel` horizontal in code blocks/tables · `Ctrl+Wheel` font size · `Alt+Wheel` column width · double-click selects a word, triple-click a paragraph · `Ctrl+click` / middle-click on a link → new window (Markdown targets) · X1/X2 back/forward · right-click → context menu · hover the left gutter of H1–H3 → fold toggle · hover image/diagram/table → ⛶ badge · double-click image/diagram → fullscreen viewer · drag files onto the window → open (or insert in edit modes) · drag the sidebar edge / splitter → resize.

## F.6 Presets
- `default` — the tables above.
- `typora` — differences: `split_edit` = `Ctrl+/`, `toggle_sidebar` = `Ctrl+Shift+L`, `inline_code` = ``Ctrl+Shift+` ``, `strikethrough` = `Alt+Shift+5`, `bullet_list` = `Ctrl+Shift+]`, `numbered_list` = `Ctrl+Shift+[`, `paragraph` = `Ctrl+0` (then `font_reset` = `Ctrl+Shift+Alt+0`).
User overrides apply on top of the selected preset; the Shortcuts section shows conflicts and allows "Reset to preset".
