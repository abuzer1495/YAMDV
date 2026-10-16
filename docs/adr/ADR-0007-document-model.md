# ADR-0007: Document model (two-level), incremental reparse, estimates

- Status: accepted (Phase 002)
- Requirements: ARCHITECTURE §E.4 (P-07), R-REN-21, S-PARSE-01, D.4 rules 1–2

## Context
YAMDV must open 1 MiB+ documents with a first paint ≤ 300 ms and ≤ 16 MiB private working set, and later (Phase 006) re-parse incrementally while editing. A full AST with inline nodes for the whole document costs several times the source size.

## Decision
1. **Top-level block index** (`yamdv_core::doc::index`). One full pulldown-cmark 0.13 pass with the §4.3 options. For every top-level block it stores: `kind`, `range: Range<u32>` and `Estimate { half_ems, lines }`. It also builds the document maps: headings (level, range, text, GitHub slug or `{#id}`, de-duplicated with `Slugger`), link reference definitions (`fold_label`: lowercase plus the Unicode full case folds pulldown applies, e.g. ß→ss), footnote definitions, and the front-matter range. Inline events are not kept.
2. **Detail trees** (`doc::detail`) are built lazily per top-level block by re-parsing exactly `source[range]`:
   - The broken-link callback resolves references from the document map.
   - Footnote references whose definition lives outside the slice would stay plain text in pulldown, so stub definitions `[^label]: x` are appended after the slice and all events past the slice end are dropped.
   - Nested blocks live in a flat arena (`parent`, `end`, `depth`). Depth is capped at 64: deeper nodes are attached at level 64 (S-PARSE-01).
   - Inline content is stored as `Run { kind, range, owned, style }`. `owned` is set only where unescaping or entities change the text.
   - `Cache` is an LRU keyed by block index with a byte budget (`DEFAULT_BUDGET` = 2 MiB, measured with `heap_bytes`).
3. **Layout ↔ source mapping** (`doc::text::ParaText`): each paragraph string is a sequence of segments (UTF-16 offset ↔ source byte range). `to_source`/`from_source` convert both ways and are unit-tested, including owned and entity segments.
4. **Splice** (`Index::splice`), used from Phase 006:
   - Start at the top-level block that contains the edit start, minus one block, because the edit may merge into the previous block.
   - Re-parse forward from that block's start until a new block boundary equals an old boundary shifted by the edit delta (resync), then shift all later ranges.
   - If any reference or footnote definition changed, rebuild the maps and the headings. Heading text can depend on references, and inlines must then be re-resolved: the detail cache is cleared.
   - Definitions sit in the gaps between blocks, so the change check covers the old region from the base up to the old resync block start.
   - Tested with synthetic edits, including > 500 random splices compared against a full rebuild.
5. **Height estimates** for blocks that have not been laid out (`yamdv_layout::block::estimate`):
   - Paragraph-like blocks: `max(ceil(Σ char_em × font_size ÷ column_width), source_lines) × line_pitch`, where Latin characters count 0.5 em and CJK/emoji (`is_wide`) count 1.0 em.
   - Headings: the same formula with the heading size and 1.25 × size pitch.
   - Code-like cards: `lines × code_pitch + header + paddings`. Fences are excluded for code blocks.
   - Rules: 2 px.
   - Inter-block gaps are added separately (`gap_before`).
   - A Fenwick tree over block heights gives O(log n) offset ↔ block lookups. Scroll anchoring keeps the top visible block and its offset fixed when heights above it change.

## Lint exception
`doc/{detail,index,text}.rs`, `layout/{block,shaper,viewer}.rs` and the platform rendering modules use a module-level `#![allow(clippy::indexing_slicing, clippy::cast_*)]` with a justification comment.
- Ranges and indices come from the parser, from `u32` source positions bounded by the 256 MiB guard, or from pixel geometry bounded by window and bitmap sizes.
- The fuzz target `parse_layout` exercises these paths under ASan with debug assertions.

## Other decisions recorded here
- **Damaged UTF-8 heuristic** (`decode::mostly_utf8`): if invalid UTF-8 bytes make up ≤ 1 % of non-ASCII sequences and valid multi-byte sequences dominate, decode as lossy UTF-8. Without this, chardetng guessed Cyrillic for damaged Turkish UTF-8.
- **RTL paragraphs**: DirectWrite RTL reading direction with `LEADING` alignment, which is right-aligned in RTL. The spec's "trailing" read literally would left-align RTL text (see report §7).
- **Code overflow**: the 24 px fade is a gradient strip painted over the right padding after the text (Fill → label → Para → right-pad fill + FadeRight → Stroke). No clip layer is needed.

## Measured memory cost
See `docs/reports/002_REPORT.md` §5: M1b and M2 with the `mem-stats` breakdown (source text, block index, detail cache, layout cache, band buffer).

## Consequences
Measured memory cost: about 90 bytes per top-level block, including the maps (large-1mb.md: 13,303 blocks → 1,205,180 B index), plus the source text, plus a 2 MiB cap on the detail cache, plus the layouts of at most 3 screens. Phase 006 can splice without a full reparse.

## Amendment (Phase 003, F-11): re-entrancy audit (P-18)

All window state lives in `Rc<RefCell<Win>>`. Every message goes through `reentry::dispatch`. It never panics on a busy borrow: it counts the event and, in debug builds, logs the message id (`reentry` log target). A re-entrant `WM_PAINT` returns 0 without validating, so the region stays invalid. Other re-entrant messages fall back to `DefWindowProcW`.

Audited call sites in `yamdv-platform-win/src/window.rs` that can dispatch messages synchronously:

| Call | Where | Handling |
|---|---|---|
| `UpdateWindow` (WM_PAINT) | `set_scroll` | deferred (`Deferred::Update`) |
| `SetWindowTextW` (WM_SETTEXT) | `set_title` (open, autoscroll) | deferred (`Deferred::Title`) |
| `SetWindowPos` (WM_WINDOWPOSCHANGED, WM_SIZE) | WM_DPICHANGED | deferred (`Deferred::SetPos`) |
| `DestroyWindow` (WM_DESTROY, …) | `run_actions` | no borrow held (action queue) |
| `ShowWindow` | `create_window` | no borrow held |
| Modal file dialog | `run_actions` | no borrow held; F-09 moves it to the dialog host |
| `SetCapture` / `ReleaseCapture` (WM_CAPTURECHANGED) | selection, scrollbar drag | message not handled, so `DefWindowProcW` is safe |
| `SetCursor`, `InvalidateRect`, `ScrollWindowEx(SW_INVALIDATE)`, `SetTimer`, `KillTimer`, `TrackMouseEvent` | various | do not dispatch synchronously |

Deferred calls are flushed by `wndproc` after the handler returns, and after `run_actions` and `create_window`. The test `reentry::tests::synchronous_send_message_is_detected` (Windows CI) provokes a real synchronous `SendMessageW` from inside a handler and asserts that the guard detects it.
