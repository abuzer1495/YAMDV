# V2-F plan (prompt 016)

Order: §1.1 F-29 → F-30 → §2 ADR-0020 + P-47 → Part 1 (items 1–7) → Part 2 (8–13) → tests → gates → PR (one CI run, +1 fix) → merge → phase-v2f → v1.3.0 release commit + tag → report.

## CHECKPOINT
- 2026-10-23 21:51 F-29 (insert_image Ctrl+Shift+I, md::image), F-30 (xtask size-map), ADR-0020, P-47 (spec-check green) pushed. Prompt stored from Read output (hash of attachment not reproducible; content identical as read). Next: core reveal module (yamdv-core/src/doc/reveal.rs).

CHECKPOINT 2 — item 1 wired: Palette.marker, yamdv-layout/src/reveal.rs (reveal paragraph layout),
SourceBlock.reveal, window/wysi/{reveal,drive}.rs (live_reveal on enter/edit/caret move; `revealed`
probe; `ui v2f-caret "k|needle"`). tests/driver/f2-reveal.yds started (passes under Wine).
Next: item 2 block constructs (code lang picker, math/mermaid popover), then item 3 transforms.
