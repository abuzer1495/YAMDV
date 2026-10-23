# Traceability

Every requirement ID in the spec appendices, with its planned phase (Appendix G) and current status.
Status values: `planned`, `partial (NNN)`, `implemented (NNN)`, `measured (NNN)`, `deferred (FUT-xx)` (v1 scope, `docs/FUTURE.md`; applied by `cargo xtask future`). A requirement is only `implemented` when code and a test or check exist.

| ID | Phase | Status |
|---|---|---|
| B-SIZE | 013 | measured (002): xtask size, soft 4.0 MiB |
| M0 | 013 | measured (002): memprobe gate ≤ 4.8 MiB |
| M1 | 013 | measured (003): typical.md, CI |
| M2 | 013 | measured (002): large-1mb.md gate ≤ 16 MiB |
| M3 | 013 | planned |
| M4 | 013 | planned |
| M5 | 013 | planned |
| M6 | 013 | measured (002): memprobe --open-dialog gate |
| M7 | 013 | measured (002): commit ≤ 3×PWS gate |
| R-CODE-01 | 003 | implemented (003); part deferred: FUT-01 |
| R-CODE-02 | 003 | implemented (003) |
| R-CODE-03 | 003 | implemented (003) |
| R-CODE-04 | 003 | implemented (003) |
| R-CODE-05 | 003 | partial (003): toolbar not animated |
| R-CODE-06 | 003 | partial (003): chunked paragraphs, not lazy shaping |
| R-EDIT-01 | 006 | implemented (006 V1-B): edits on the source buffer; byte-identity test |
| R-EDIT-02 | 006 | implemented (006 V1-B): autosave 800 ms/mode exit/deactivate/close, Ctrl+S, S-FILE-03 write, title ● |
| R-EDIT-03 | 006 | implemented (006 V1-B): Stamp check + in-window dialog |
| R-EDIT-04 | 006 | implemented (006 V1-B): yamdv-editor history, proptest |
| R-EDIT-05 | 006 | implemented (V2-E 015): CF_HTML → Markdown via the S-HTML allowlist, Ctrl+Shift+V plain (FUT-15); paste::tests html_to_markdown_cases, xss.rs; driver paste-recovery |
| R-EDIT-06 | 006 | implemented (V2-E 015): Tab snippets with placeholders (FUT-45); tests_snippets; driver edit-plus |
| R-EDIT-07 | 006 | implemented (V2-E 015): Ctrl+Shift+D, Ctrl+L, Alt+↑/↓, Ctrl+T, Ctrl+Shift+K, auto-pair (FUT-12); tests_lines (line ops + auto-pair); driver edit-plus |
| R-EDIT-08 | 006 | implemented (V2-E 015): bitmap → WIC PNG asset, file drop copy, S-FILE-05 sandbox (FUT-45); paste::tests assets + junction; driver paste-recovery |
| R-EDIT-09 | 006 | partial (006 V1-B): IMM composition + candidate at caret, AltGr guard; not verified on a real IME |
| R-EDIT-10 | 006 | implemented (V2-E 015): Ctrl+H, Replace all = one undo, case/word/Turkish folding, regex-lite with 100 ms guard (ADR-0019); tests_replace; driver edit-plus |
| R-EDIT-11 | 006 | implemented (006 V1-B) |
| R-EDIT-12 | 006 | implemented (006 V1-B): read-only / lossy / plain dialogs |
| R-EDIT-13 | 006 | implemented (V2-E 015): recovery journal, banner Restore/Discard (FUT-45); journal tests; driver paste-recovery |
| R-EDIT-14 | 006 | implemented (006 V1-B) |
| R-EXP-01 | 011 | partial (008 V1-D): PDF via the print path (print.rs, CI v1d-checks); native writer deferred (FUT-39) |
| R-EXP-02 | 011 | implemented (008 V1-D): static HTML, math/diagrams as PNG data URIs (html::document, r_exp_02_static_document, driver export-html); scripted parts deferred (FUT-21) |
| R-EXP-03 | 011 | implemented (008 V1-D): PrintDlgExW in the dialog host, paginate.rs (r_exp_03_*) |
| R-EXP-04 | 011 | implemented (008 V1-D): context menu, csv.rs (r_exp_04_*), driver export-csv |
| R-EXP-05 | 011 | implemented (008 V1-D): --export-html / --export-pdf (r_exp_05_export_flags, CI v1d-checks) |
| R-EXP-06 | 011 | deferred (FUT-23) |
| R-EXP-07 | V2-B | implemented (V2-B; BUG-11 #78: worker job, progress card, Cancel, taskbar) |
| R-EXT-01 | 005 | implemented (005 V1-A) |
| R-EXT-02 | 005 | deferred (FUT-10) |
| R-EXT-03 | 005 | partial (005): template tokenizer + CreateProcessW; no settings UI |
| R-EXT-04 | 005 | planned |
| R-IMG-01 | 003 | partial (003): header probe for all formats; WIC decode not done; part deferred: FUT-20, FUT-48 |
| R-IMG-02 | 003 | partial (003): yamdv_core::image::resolve (tests); not wired to a decoder |
| R-IMG-03 | 003 | partial (003): display_size/obsidian_size/html_len (tests); not used by layout |
| R-IMG-04 | 003 | partial (003): probe + LRU (tests); no worker/decode |
| R-IMG-05 | 003 | implemented (V2-C 013): image viewer, ⛶ badge, zoom at cursor (FUT-36) |
| R-IMG-06 | 003 | partial (003): P-25 chip for all images; block placeholders + tooltip missing |
| R-LINK-01 | 003 | implemented (003) |
| R-LINK-02 | 003 | implemented (003) |
| R-LINK-03 | 003 | implemented (003) |
| R-LINK-04 | 003 | partial (003): Ask in-window dialog, not anchored popover |
| R-LINK-05 | 003 | implemented (003) |
| R-LINK-06 | 003 | implemented (003) |
| R-LINK-07 | 003 | planned |
| R-MAP-01 | 010 | implemented (V2-D 014): mind map viewer, right/balanced, accent hues (FUT-37); mindmap layout tests |
| R-MAP-02 | 010 | implemented (V2-D 014): wheel zoom, pan, click collapse, double-click jump, Esc, Ctrl+C PNG; driver palette-mindmap-toc |
| R-MATH-01 | 009 | implemented (007): yamdv-math + Cambria Math MATH table (dwrite.rs) |
| R-MATH-02 | 009 | partial (007): core subset (parse.rs); part deferred: FUT-17 |
| R-MATH-03 | 009 | implemented (007): error color + message line; tooltip (V2-C 013, FUT-40) |
| R-MATH-04 | 009 | implemented (007): copy maps to source with delimiters (detail.rs test) |
| R-MATH-05 | 009 | implemented (007): centered, HRegion scroll (rich.rs tests) |
| R-MATH-06 | 009 | implemented (007): theme text color; \color override |
| R-MATH-07 | 009 | deferred (FUT-17) |
| R-MD-01 | 002/003 | implemented (002) |
| R-MD-02 | 002/003 | implemented (003): GFM tables, tasks, strikethrough, autolinks |
| R-MD-03 | 002/003 | partial (003): highlight/sup/sub; no baseline shift |
| R-MD-04 | 002/003 | implemented (003): callouts |
| R-MD-05 | 002/003 | implemented (003): ==highlight== |
| R-MD-06 | 002/003 | partial (003): sup/sub 0.75 em, no baseline shift |
| R-MD-07 | 002/003 | planned |
| R-MD-08 | 002/003 | planned |
| R-MD-09 | 002/003 | implemented (003) |
| R-MD-10 | 002/003 | implemented (002) |
| R-MD-11 | 002/003 | implemented (003): definition lists |
| R-MD-12 | 002/003 | implemented (003): emoji shortcodes (ADR-0014) |
| R-MD-13 | 002/003 | implemented (003): autolink literals |
| R-MD-14 | 002/003 | partial (003): sanitizer + structure pass; entity subset, colspan=1 |
| R-MD-15 | 002/003 | implemented (002) |
| R-MD-16 | 002/003 | implemented (002) |
| R-MD-17 | 002/003 | implemented (002) |
| R-MD-18 | 002/003 | implemented (003) |
| R-MD-19 | 002/003 | implemented (V2-D 014): [TOC] / [[_TOC_]] Contents list, HTML/PDF/Print (doc::toc tests, toc_marker_exports_anchored_list) |
| R-MMD-01 | 010 | partial (007): flowchart + sequence (yamdv-mermaid); part deferred: FUT-18 |
| R-MMD-02 | 010 | implemented (007 + V2-C 013): card, fit-to-column, min scale + hscroll (BUG-08), diagram viewer (FUT-36); part deferred: FUT-18 |
| R-MMD-03 | 010 | partial (007): B.3.6 light/dark palettes; part deferred: FUT-18 (init) |
| R-MMD-04 | 010 | implemented (007): source card + line/column message; part deferred: FUT-18 |
| R-MMD-05 | 010 | implemented (007): placeholder + Render anyway (mermaid.yds); part deferred: FUT-18 |
| R-MMD-06 | 010 | partial (007): directions, core shapes/edges, subgraph, classDef/style; part deferred: FUT-18 |
| R-MMD-07 | 010 | implemented (007): seq.rs + tests; part deferred: FUT-18 |
| R-MMD-08 | 010 | deferred (FUT-18) |
| R-MMD-09 | 010 | deferred (FUT-18) |
| R-MMD-10 | 010 | deferred (FUT-18) |
| R-NAV-01 | 004/005 | implemented (005 + V2-D 014): Contents/History/Favorites/Files tabs, each hideable; no tab row when all three hidden |
| R-NAV-02 | 004/005 | implemented (V2-D 014): left/right docked and floating card (FUT-02); driver keyboard-reach, golden v2d-floating-dark |
| R-NAV-03 | 004/005 | planned |
| R-NAV-04 | 004/005 | planned |
| R-NAV-05 | 004/005 | planned |
| R-NAV-06 | 004/005 | planned |
| R-NAV-07 | 004/005 | planned |
| R-NAV-08 | 004/005 | planned |
| R-NAV-09 | 004/005 | implemented (V2-D 014): favorites drag reorder + Alt+Up/Down (FUT-03); favs::reorder test, driver files-favorites |
| R-NAV-10 | 004/005 | implemented (V2-D 014): lazy Files tree, worker listing, skip rules, watched (FUT-33); core files tests, driver files-favorites |
| R-NAV-11 | 004/005 | implemented (V2-D 014): arrows/Enter/Home/End in sidebar rows, Ctrl+Shift+1..4 tabs |
| R-OPEN-01 | 002/005 | planned |
| R-OPEN-02 | 002/005 | implemented (002) (same-process windows; forwarding 005) |
| R-OPEN-03 | 002/005 | implemented (002) |
| R-OPEN-04 | 002/005 | implemented (002 + V2-D 014): a dropped folder opens the Files tab rooted at it (no edit-mode insert) |
| R-OPEN-05 | 002/005 | planned |
| R-OPEN-06 | 002/005 | implemented (002) (no Reopen-with-encoding menu); part deferred: FUT-09 |
| R-OPEN-07 | 002/005 | implemented (002) |
| R-OPEN-08 | 002/005 | implemented (002) |
| R-OPEN-09 | 002/005 | implemented (002) (toggle 004) |
| R-OPEN-10 | 002/005 | implemented (002) (detection; edit handling 006) |
| R-OPEN-11 | 002/005 | planned |
| R-OPEN-12 | 002/005 | implemented (002) |
| R-OPEN-13 | 002/005 | planned |
| R-READ-01 | 005 | implemented (005 V1-A) |
| R-READ-02 | 005 | implemented (005 V1-A) |
| R-READ-03 | 005 | implemented (005 V1-A) |
| R-READ-04 | 005 | implemented (005 V1-A) |
| R-READ-05 | 005 | implemented (005 V1-A) |
| R-READ-06 | 005 | implemented (V2-C 013): heading folding, Alt+click level, find expands (FUT-34) |
| R-READ-07 | 005 | implemented (011): word-count badge, chrome.rs count + fullscreen-wordcount.yds (FUT-35) |
| R-READ-08 | 005 | implemented (V2-C 013): table/image/diagram viewers (FUT-06, FUT-36) |
| R-READ-09 | 005 | planned |
| R-READ-10 | 005 | implemented (005 V1-A) |
| R-READ-11 | 005 | implemented (005 V1-A) |
| R-READ-12 | 005 | implemented (V2-D 014): heading palette Ctrl+Shift+O (FUT-08); palette filter tests, driver palette-mindmap-toc |
| R-REL-01 | 012 | planned |
| R-REL-02 | 012 | planned |
| R-REL-03 | 012 | planned |
| R-REL-04 | 012 | planned |
| R-REN-01 | 002/003 | implemented (002) |
| R-REN-02 | 002/003 | implemented (002) |
| R-REN-03 | 002/003 | implemented (003 + V2-C 013): sup/sub/footnote-ref inline objects (FUT-41) |
| R-REN-04 | 002/003 | partial (003): vector task boxes; toggle in 006 |
| R-REN-05 | 002/003 | implemented (002) |
| R-REN-06 | 002/003 | implemented (003) |
| R-REN-07 | 002/003 | implemented (003 + V2-C 013): tables; table viewer + CSV pill (FUT-06); part deferred: FUT-42 |
| R-REN-08 | 002/003 | implemented (003) |
| R-REN-09 | 002/003 | implemented (002) |
| R-REN-10 | 002/003 | implemented (003 + V2-C 013): footnotes; hover preview tooltip (FUT-40) |
| R-REN-11 | 002/003 | implemented (003) |
| R-REN-12 | 002/003 | implemented (003) |
| R-REN-13 | 002/003 | implemented (003): front matter card |
| R-REN-14 | 002/003 | implemented (003) |
| R-REN-15 | 002/003 | planned |
| R-REN-16 | 002/003 | partial (003): CF_HTML + Copy as Markdown; menu entry 004 |
| R-REN-17 | 002/003 | implemented (002) |
| R-REN-18 | 002/003 | planned |
| R-REN-19 | 002/003 | implemented (002) |
| R-REN-20 | 002/003 | implemented (002) |
| R-REN-21 | 002/003 | implemented (002) |
| R-SET-01 | 004 | implemented (012 V2-B): settings panel, window/setpanel.rs + settings-panel.yds (FUT-31) |
| R-SET-02 | 004 | partial (012 V2-B): panel search done (FUT-26 first half), FUT-26 rest open |
| R-SET-03 | 004 | implemented (012 V2-B): live apply + settings.ini, Reset section/all (FUT-31) |
| R-SET-04 | 004 | implemented (012 V2-B): yamdv-ui::settings_panel rows/geometry tests (FUT-31) |
| R-SET-05 | 004 | implemented (005 V1-A) |
| R-SET-06 | 004 | implemented (012 V2-B): yamdv-core::keymap, capture/conflict/Typora, shortcuts.yds (FUT-05) |
| R-SYS-01 | 005 | implemented (005 V1-A) |
| R-SYS-02 | 005 | implemented (005 V1-A) |
| R-SYS-03 | 005 | implemented (005 V1-A) |
| R-SYS-04 | 005 | partial (001): --help/--version/usage error |
| R-SYS-05 | 005 | implemented (008 V1-D): install.rs (r_sys_05_*, CI v1d-checks) |
| R-SYS-06 | 005 | implemented (005 V1-A) |
| R-SYS-07 | 005 | implemented (008 V1-D): icons (001), version resource 1.1.0 (012) |
| R-SYS-08 | 005 | deferred (FUT-25) |
| R-SYS-09 | 005 | deferred (FUT-25) |
| R-TBL-01 | 003/008 | deferred (FUT-44) |
| R-TBL-02 | 003/008 | deferred (FUT-44) |
| R-TBL-03 | 003/008 | deferred (FUT-44) |
| R-TBL-04 | 003/008 | deferred (FUT-44) |
| R-TBL-05 | 003/008 | deferred (FUT-44) |
| R-TBL-06 | 003/008 | deferred (FUT-44) |
| R-TBL-07 | 003/008 | deferred (FUT-44) |
| R-TBL-08 | 003/008 | deferred (FUT-44) |
| R-THEME-01 | 004 | implemented (005 V1-A) |
| R-THEME-02 | 004 | partial (002): data only |
| R-THEME-03 | 004 | implemented (011): scheme cards, popup.rs + scheme-cards.yds (FUT-32) |
| R-THEME-04 | 004 | implemented (012 V2-B): font pickers with resolved fallback (FUT-49) |
| R-THEME-05 | 004 | planned |
| R-THEME-06 | 004 | implemented (012 V2-B): custom-color editor, color-editor.yds (FUT-04) |
| R-THEME-07 | 004 | implemented (005 V1-A) |
| R-THEME-08 | 004 | deferred (FUT-47) |
| R-THEME-09 | 004 | planned |
| R-THEME-10 | 004 | planned |
| R-WIN-01 | 005 | implemented (005 V1-A) |
| R-WIN-02 | 005 | implemented (005 V1-A) |
| R-WIN-03 | 005 | implemented (005 V1-A) |
| R-WIN-04 | 005 | partial (001): window skeleton (size/min size/centering) |
| R-WIN-05 | 005 | implemented (011): custom title bar, window/chrome.rs + titlebar.yds (FUT-30, BUG-09) |
| R-WIN-06 | 005 | implemented (011): F11 fullscreen + fullscreen-wordcount.yds (FUT-46) |
| R-WIN-07 | 005 | planned |
| S-EXEC-01 | 003/005 | planned |
| S-EXEC-02 | 003/005 | planned |
| S-EXEC-03 | 003/005 | implemented (005 V1-A) |
| S-EXEC-04 | 003/005 | implemented (003): ShellExecuteExW, no NOZONECHECKS |
| S-EXP-01 | 011 | implemented (008 V1-D): write_atomic to the dialog path only |
| S-EXP-02 | 011 | implemented (008 V1-D) without script (CSP drops script-src; s_test_02_xss_exported_html); scripted parts deferred (FUT-21) |
| S-EXP-03 | 011 | implemented (008 V1-D): raster print-path PDF has no script, actions, files or forms |
| S-EXP-04 | 011 | implemented (008 V1-D): s_exp_04_formula_guard |
| S-FILE-01 | 002/005 | implemented (006 V1-B): document writes only from edit modes + opt-in task toggle |
| S-FILE-02 | 002/005 | planned |
| S-FILE-03 | 002/005 | implemented (006 V1-B): shell::save_document (ReplaceFileW / MoveFileExW) |
| S-FILE-04 | 002/005 | implemented (002) |
| S-FILE-05 | 002/005 | planned |
| S-FILE-06 | 002/005 | planned |
| S-FILE-07 | 002/005 | implemented (002) |
| S-FILE-08 | 002/005 | implemented (V2-D 014): document folder, config folder, expanded Files folders (non-recursive, at most 32) |
| S-FILE-09 | 002/005 | planned |
| S-FILE-10 | 002/005 | implemented (006 V1-B): encode.rs keeps EOL/BOM/encoding; byte-identity test |
| S-HTML-01 | 003 | implemented (003): allowlist sanitizer, XSS suite |
| S-HTML-02 | 003 | implemented (003) |
| S-HTML-03 | 003 | implemented (003) |
| S-IPC-01 | 005 | implemented (005 V1-A) |
| S-IPC-02 | 005 | implemented (005 V1-A) |
| S-IPC-03 | 005 | implemented (005 V1-A) |
| S-IPC-04 | 005 | planned |
| S-LINK-01 | 003 | implemented (003) |
| S-LINK-02 | 003 | implemented (003) |
| S-LINK-03 | 003 | implemented (003) |
| S-LINK-04 | 003 | implemented (003) |
| S-NET-01 | 003/012 | partial (001): no networking code; enforced by S-SUP-03/S-NET-03 |
| S-NET-02 | 003/012 | planned |
| S-NET-03 | 003/012 | implemented (001): cargo xtask check-imports |
| S-NET-04 | 003/012 | planned |
| S-NET-05 | 003/012 | planned |
| S-PARSE-01 | 003/012 | implemented (002) |
| S-PARSE-02 | 003/012 | implemented (002) (applicable parts) |
| S-PARSE-03 | 003/012 | partial (007): 200 ms budget for diagrams |
| S-PARSE-04 | 003/012 | implemented (002) |
| S-PARSE-05 | 003/012 | implemented (007): 10 000 chars, depth 64, 5 000 formulas |
| S-PARSE-06 | 003/012 | implemented (007): 100 KB, soft 500/1 500, hard 2 000/6 000, 100 diagrams |
| S-PARSE-07 | 003/012 | implemented (002) |
| S-PARSE-08 | 003/012 | implemented (002) |
| S-PRIV-01 | 004 | planned |
| S-PRIV-02 | 004 | planned |
| S-PRIV-03 | 004 | planned |
| S-PRIV-04 | 004 | planned |
| S-PRIV-05 | 004 | planned |
| S-PRIV-06 | 004 | partial (008 V1-D): HTML/PDF title = first heading or file name; PDF Producer is set by the print driver |
| S-PROC-01 | 001 | implemented (001): hardening.rs |
| S-PROC-02 | 001 | implemented (001): hardening.rs |
| S-PROC-03 | 001 | implemented (001): hardening.rs |
| S-PROC-04 | 001 | partial (001): ImageLoad+StrictHandle; DynamicCode SHOULD deferred |
| S-PROC-05 | 001 | implemented (001): profile + .cargo/config.toml, verified by check-imports |
| S-PROC-06 | 001 | partial (001): manifest asInvoker; elevation banner planned 004/005 |
| S-PROC-07 | 001 | implemented (003): dialog host (ADR-0013) |
| S-PROC-08 | 001 | planned |
| S-REG-01 | 005 | implemented (005 V1-A) |
| S-REG-02 | 005 | implemented (005 V1-A) |
| S-REG-03 | 005 | implemented (005 V1-A) |
| S-REG-04 | 005 | implemented (005 V1-A) |
| S-RES-01 | 003 | partial (003): policy + tests, not wired |
| S-RES-02 | 003 | partial (003): policy + mock-FS matrix (AC-6); Windows Fs impl not wired |
| S-RES-03 | 003 | implemented (003): remote never fetched (no image I/O at all) |
| S-RES-04 | 003 | partial (003): limits + header check (tests) |
| S-RES-05 | 003 | implemented (003): CF_HTML exports alt text only |
| S-RES-06 | 003 | implemented (003): DirectWrite system fonts only |
| S-SUP-01 | 001 | implemented (001): Cargo.lock, cargo deny in CI, --locked |
| S-SUP-02 | 001 | implemented (001): ADR-0003…0006 |
| S-SUP-03 | 001 | implemented (001): deny.toml |
| S-SUP-04 | 001 | implemented (001): deny.toml |
| S-SUP-05 | 001 | implemented (001): build_support/res.rs |
| S-SUP-06 | 001 | implemented (001): .github/workflows/ci.yml |
| S-TEST-01 | 012 | partial (002): 2 fuzz targets |
| S-TEST-02 | 012 | partial (003): link, image-path and 66-vector HTML matrices |
| S-TEST-03 | 012 | implemented (001): cargo xtask check-imports |
| S-TEST-04 | 012 | partial (001): THREAT_MODEL.md skeleton |
| S-UI-01 | 004 | planned |
| S-UI-02 | 004 | planned |
| S-UI-03 | 004 | implemented (V2-C 013): tooltips, anchored popover, banners (FUT-40) |
| T1 | 013 | measured (003): typical.md, CI |
| T2 | 013 | planned |
| T3 | 013 | measured (005): CI measure.ps1, see v1a_REPORT |
| T4 | 013 | planned |
| T5 | 013 | planned |
| T6 | 013 | planned |
| T7 | 013 | planned |
| T8 | 013 | planned |
| T9 | 013 | planned |
| T10 | 013 | planned |
| R-A11Y-01 | 012 | implemented (V2-D 014): Tab order over all interactive parts, 2 px ring after keyboard use, F6 regions (HARD-04); focus::order tests, driver keyboard-reach; UIA/HC with FUT-47 |
| R-A11Y-02 | 012 | partial (002): reduced motion for scrolling |
| R-A11Y-03 | 012 | deferred (FUT-47) |
| R-A11Y-04 | 012 | implemented (002) |
| R-F2-01 | 007 | implemented (006 V1-B): block-level model |
| R-F2-02 | 007 | deferred (FUT-43) |
| R-F2-03 | 007 | deferred (FUT-43) |
| R-F2-04 | 007 | implemented (V2-E 015): F2 format toolbar, editing.format_toolbar (FUT-45); driver edit-plus, golden v2e-toolbar-light |
| R-F2-05 | 007 | partial (006 V1-B): Undo/Redo/Cut/Copy/Paste + Format ▸ |
| R-F2-06 | 007 | planned |
| R-F2-07 | 007 | implemented (006 V1-B) |
| R-F2-08 | 007 | partial (006 V1-B): click enters block, Ctrl+click follows, Ctrl+K wraps |
| R-F2-09 | 007 | planned |
| R-F2-10 | 007 | implemented (006 V1-B) |
| R-F2-11 | 007 | deferred (FUT-18) |
| R-F2-12 | 007 | deferred (FUT-26) |
| R-F3-01 | 006 | implemented (006 V1-B) |
| R-F3-02 | 006 | implemented (006 V1-B) |
| R-F3-03 | 006 | implemented (006 V1-B): 30 ms debounce + Viewer::splice |
| R-F3-04 | 006 | implemented (006 V1-B): block-based |
| R-F3-05 | 006 | partial (006 V1-B): click → caret; preview selection not mirrored |
| R-F3-06 | 006 | implemented (006 V1-B): opens the F1 help document |
| R-F3-07 | 006 | implemented (V2-E 015): find/replace in the editor pane, matches mirrored in the preview (FUT-45); driver edit-plus, golden v2e-replace-dark |
| R-I18N-01 | 004 | partial (002): EN/TR strings of 002; part deferred: FUT-29 |
| R-I18N-02 | 004 | implemented (002) |
| R-I18N-03 | 004 | planned |
| R-I18N-04 | 004 | implemented (002) |

Interim scenarios (P-09): M1b, T1b, T1L measured (002) with CI soft-budget gates; T2, T4, T7 measured (002).
