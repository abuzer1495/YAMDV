# Phase V1-B plan — Editing: F3 split + block-level F2 (prompt 006)

Goal: edit any Markdown file safely. One shared editor component (`yamdv-editor`, rope + history + Markdown keys) drives F3 (source left, live preview right) and F2 (click a top-level block → its source in place; leave → re-render). Mode: A-4 (simplest implementation meeting the ACs; unit tests for new pure logic; one driver script per user-visible feature; goldens only the 2 listed).

Design (simplest):
- `yamdv-editor`: `Buffer` (ropey rope, caret/anchor, grapheme/word moves), `History` (coalescing per word / 1 s, selection restore, 4 MiB cap), `md` (Enter/Tab/format toggles), `save` (encode back keeping encoding/BOM/EOL/final newline; edits recorded as byte ranges in decoded text; conflict check size+mtime+hash). proptest invariants.
- Sync rope → `Document` by common prefix/suffix diff → `String::replace_range` + `Index::splice` + `Viewer::splice` (new: shifts heights/layouts/measured, keeps anchor).
- F2: the active block is laid out by the viewer from the editor text (verbatim, editor font, `code_bg`); the Document stays frozen until the block is left, then one splice. Undo that lands outside the block leaves it first.
- F3: editor pane = second `Viewer` over `Content::Plain` of the buffer (virtualized, soft wrap, gutter), preview = the document viewer in the right pane.
- Push policy: CI runs only on `pull_request` / `main` / tags, so WIP pushes to the branch (no PR yet) trigger no CI; after the PR is open, WIP pushes carry `[skip ci]` (GitHub skips push/pull_request runs for it). A-4 rule 6: one CI run at the end + one fix run.

Risks: window.rs is 3.4 kLOC (new UI in `window/edit.rs`); no Windows GUI locally (cross-check with `cargo xwin clippy` only; behaviour verified on CI by driver scripts); T4 typical at 14.2 ms of 16 ms (editing must stay out of the read-mode scroll path); IME only checkable by the owner.

## CHECKPOINT
- [x] V1-A plan checkpoint closed (merged 5fdf661, tag phase-v1a)
- [x] prompt stored `docs/prompts/006_V1B_EDITING.md` (SHA-256 10fce65bdae1465f3b52c570a8b8195df5f6926334df0cdea5006b5bc13d1056)
- [x] F-20 memprobe in place (cwd = doc folder) + M1 with images (`--line` to the image section), image cache bytes
- [x] F-21 T4 gate: second attempt, fail only if both > 16 ms
- [x] editor core (buffer, history, moves, md keys, proptest)
- [x] save (encoding/BOM/EOL, atomic, conflict) + byte-identity test
- [x] incremental reparse wiring (Viewer::splice)
- [x] F3 split, F2 block edit, menu/settings, title ●, autosave, conflict dialog, disabled-edit dialogs, IME
- [x] driver `type` latency + 9 scripts, 2 golden steps, M5/T5 wiring (tests/perf, measure.ps1)
- [ ] local gates, PR (CI run 1), golden PNGs from the CI artifact (fix push), merge, tag, report
