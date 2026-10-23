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
