# V2-F plan (prompt 016)

Order: §1.1 F-29 → F-30 → §2 ADR-0020 + P-47 → Part 1 (items 1–7) → Part 2 (8–13) → tests → gates → PR (one CI run, +1 fix) → merge → phase-v2f → v1.3.0 release commit + tag → report.

## CHECKPOINT
- 2026-10-23 21:51 F-29 (insert_image Ctrl+Shift+I, md::image), F-30 (xtask size-map), ADR-0020, P-47 (spec-check green) pushed. Prompt stored from Read output (hash of attachment not reproducible; content identical as read). Next: core reveal module (yamdv-core/src/doc/reveal.rs).

CHECKPOINT 2 — item 1 wired: Palette.marker, yamdv-layout/src/reveal.rs (reveal paragraph layout),
SourceBlock.reveal, window/wysi/{reveal,drive}.rs (live_reveal on enter/edit/caret move; `revealed`
probe; `ui v2f-caret "k|needle"`). tests/driver/f2-reveal.yds started (passes under Wine).
Next: item 2 block constructs (code lang picker, math/mermaid popover), then item 3 transforms.

CHECKPOINT 3 — item 2 done: core doc/fence.rs (fence parse + language suggest, tested); layout
reveal::source_block (fence markers + live highlight, math/Mermaid preview card, sole image under
its source line); platform wysi/{lang,reveal}.rs (picker ↑↓ Enter/Tab Esc, 150 ms Mermaid
debounce keeping the last good diagram, TIMER_F2_PREVIEW=32); probes `f2_preview`, `lang_items`.
Next: item 3 typing transforms (pure in yamdv-editor), then items 4–7.

CHECKPOINT 4 — item 3 done: yamdv-editor transform.rs (space: # … ######, - * +, 1. 1), >, [ ]/[x]
in items — two sealed steps so Ctrl+Z leaves the escaped literal; enter: ```/```lang/~~~/$$ open
blocks, --- rule, pipe header → delimiter + empty row) + tests_transform.rs; wired via
wysi/typing.rs (f2_type, f2_enter; off inside a fence body). Next: item 4 links (Ctrl+click
follows, Ctrl+K popover), item 5 cross-block selection, 6 checkbox/context menu, 7 data safety.

CHECKPOINT 5 (item 4 done): Ctrl+K / toolbar Link in F2 opens the link popover (Text/URL/Title,
Tab fields, Enter = one undo step, Esc); Ctrl+click follows via the viewer path. Pure model
yamdv_editor::linkedit + test; driver f2-reveal covers edit + new link + undo.
Next: item 5 cross-block selection + multi-format clipboard, 6 checkbox/context menu, 7 data safety.

CHECKPOINT 6 (items 5–6 done): F2 cross-block selection (wysi/xsel.rs: Shift+click, drag,
Shift+arrows past the block edge, Ctrl+A → document selection; copy = text/markdown + CF_HTML +
plain; cut/delete/typing/paste replace as one step); multi-block paste re-splits blocks.
Checkbox click in the active block toggles (transform::task_toggle_at); context menu Insert ▸
(13 items incl. Callout ▸ 5) via yamdv_editor::insert (+tests). Table ▸ submenu comes with the
table commands of item 9 (Part 2). Next: item 7 data safety (byte-identical untouched blocks,
EOL/BOM kept) then Part 2.

## CHECKPOINT 7 (item 7 done — Part 1 complete)
- Data-safety probes: `ui v2f-hash-mark`, `file_changed_blocks` (multiset of blank-line block hashes), `file_bom`, `file_crlf`; f2-reveal covers BOM+CRLF save, `---`/`$$` transforms with CRLF.
- Fixed F2 diff alignment at a block end (an inserted line break matched the following EOL, so Enter at block end left the block).
- T5 F2 on large-1mb.md: tests/perf/t5-live-large.yds + measure.ps1 (Wine: 85 ms on 1 MB vs 90 ms on typical — no size dependence; CI gives real numbers).
- Next: Part 2 items 8–13 (table model in yamdv-editor with pure tests first).

## CHECKPOINT 8 (Part 2 in progress)
- Done: yamdv_editor::table (grid/parse/serialize pretty EA-width, ops, Turkish natural sort, escape) + tsv (RFC 4180) + tests_table; insert::block goes after the caret's whole block (R-TBL-08).
- Layout: yamdv_layout::table_edit (active table = grid of reveal paras, HotKind::Cell per cell, SourceBlock.table).
- Next: platform wysi/table/ (reveal build, live_para by caret, cell hit, keys Tab/Enter/Shift+Enter/arrows, `|`→`\|`, ops + pill tools + Table ▸ menu, grips drag, 10×10 picker on Ctrl+T / Insert ▸ Table…, TSV paste + offer, pretty-align on leave), table-edit.yds, goldens, §4.

## CHECKPOINT 9 (Part 2 items 8–13 implemented)
- platform wysi/table/{mod,ops,grips,pick,paste}.rs: cell keys, `|`→`\|`, 17 TblOps (tool strip above the active table + Table ▸ submenu + Ctrl+Alt arrows / Ctrl+Shift+Backspace), grips drag with accent indicator, 10×10 picker (Ctrl+T, Insert ▸ Table…), TSV paste in/out of table + "Convert to table", pretty-align on leave (changed tables only).
- tests/driver/table-edit.yds PASS (untouched table byte-identical, file_changed_blocks == 3).
- Goldens: shots.yds + SHELL_SHOTS gain top-v2f-reveal-light / top-v2f-table-dark (committed from first CI artifact); ⛶/CSV hover badge hidden on the active F2 table (strip carries them).
- Wine regressions under check: edit-live-click, edit-live-up, viewers, keyboard-reach (base 11227f9 build compare).
- Next: base compare → TRACEABILITY/docs → gates/size-map → PR + §4.

## CHECKPOINT 10 (pre-PR)
- Wine failures edit-live-click / edit-live-up / viewers / keyboard-reach reproduce on base 11227f9 → pre-existing Wine-only (no DirectWrite hit test, CPU timing, tab walk).
- TRACEABILITY: 14 rows (R-F2-02/03/05/06/08/09, R-TBL-01…08) via tools/trace_set.py.
- Size (cross lld build, hardened flags): 3,378,688 B vs phase-v2e 3,244,544 B = +134,144 B (< 150 KB); size map in docs/reports/v2f_sizemap.md.
- Version 1.3.0 (Cargo.toml + lock; About and version resource use CARGO_PKG_VERSION); CHANGELOG [1.3.0] for V2-E + V2-F.
- Next: push without [skip ci] → PR (Closes FUT-43/44) → CI run → goldens from artifact (fix push) → merge → phase-v2f → v1.3.0.

## CHECKPOINT 11 (fix push)
- CI run 40017140073 (PR #85, da51d9f): all steps green up to window goldens; live-block-light FAIL (44,310 px — intended: F2 is WYSIWYG now, no source pane), two V2-F shots not committed yet; later steps skipped.
- Fix push: goldens top-v2f-reveal-light, top-v2f-table-dark, live-block-light from that artifact (self-reviewed); ⛶ and CSV buttons end the table tool strip (badge stays hidden on the active table; Wine diff vs committed shot 0.06 % < 0.5 %).
- Next: CI run 2 → merge → F-24 → phase-v2f → v1.3.0.

## CHECKPOINT 12 (done)
- Merged 0a41b78 (PR #85) after green run 40020890007; #41, #42 closed. phase-v2f → 0a41b78; v1.3.0 → 47d68db (empty release commit). The trees of 0a41b78 and 47d68db equal ff0955c.
- Tag run 40024306004 green; draft v1.3.0 has YAMDV-1.3.0-win-x64.zip, SHA-256 1d4d0859…6351b (verified), still a draft. Main run 40024303448 green; phase-v2f tag run 40024300810: export-progress pws_delta_kib 216 > 205 (noise, not rerun).
- Report: docs/reports/v2f_REPORT.md. Next action: owner checks + publishes the draft.
