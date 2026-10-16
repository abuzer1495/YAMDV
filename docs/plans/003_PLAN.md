# Phase 003 plan — Full rendering, links & images

Prompt: `docs/prompts/003_FULL_RENDERING.md` (SHA-256 `89b1c52c81c59470532990d0b8f29e802271d2c51e0b78680a1fc520e35cfbe1`, LF).
Branch `phase/003-full-rendering` → PR → CI green → merge → tag `phase-003`.
Previous phase: tag `phase-002` = merge commit `a444acdc5890340f5c581df52b5d2255fca89b24` (F-14).

## Milestones (in order; each one is committed and pushed)
1. Prompt stored, P-10…P-25 applied (`docs/spec-patches/003.txt`), SPEC_CHANGES, plan.
2. F-12 band cap; F-11 re-entrancy guard + audit (ADR-0007 amendment); F-10 font warm-up; F-13 inline code character spacing.
3. F-09 dialog host (`--dialog-host open`), WM_COPYDATA protocol, remove the in-process dialog and its trim; ADR-0013; memprobe M6 without trim, T-DLG.
4. Renderer: PushClip/PopClip (glyphs, shapes, images); remove overflow masking; horizontal scroll regions for code.
5. `yamdv-highlight` lexers (per-line state) + tests; code toolbar (copy / line numbers / wrap / label), diff, > 5 000-line virtualization.
6. Tables (auto layout, row laziness, limits).
7. Callouts (GFM + Obsidian, folding, icons), vector task boxes, footnotes section + navigation, front matter card, definition lists.
8. Inline: highlight `==`, sup/sub inline objects, autolink literals, emoji shortcodes (gemoji table, ADR), wikilinks, kbd/abbr.
9. Safe HTML: tokenizer, sanitizer → model, structure pass (details/div center), anchors.
10. Links: URL parser, IDN, policy matrix, Ask popover, navigation history, MotW, banners; UI components (toast, tooltip, status, popover, banner); keyboard focus.
11. Images: worker thread, path policy with a mock FS, WIC probe/decode, LRU, sizing, placeholders + inline chip; DIB vs DDB measurement ADR.
12. Clipboard: CF_HTML writer (`yamdv-export`), Copy as Markdown.
13. Corpus, goldens (P-23), XSS suite, fuzz (4 targets ≥ 10 min), measurements (M1, M6, T1, T4 cold/warm, T7 tooltip/decode, T-IMG, T-DLG), owner kit (F-15), traceability, report.

## Progress note (update on every push)
- 2026-10-16: milestone 1 done. F-12 band cap done. F-11 reentry guard done. F-10 warm-up done (CI run 38950372447: all measurement gates green). F-13 char spacing done (goldens pending). Draft PR #3 open. F-09 dialog host done (CI M6 gate green, run 38950372447). M4 clipping + code hscroll done. M5 code highlighting + toolbar done (toolbar fade not animated; R-CODE-06 chunked paragraphs, not lazy shaping — PARTIAL). M6 tables done (auto layout, alignment, header/zebra/rounded, S-PARSE-02 limits, TSV copy, DirectWrite emergency break; row-level laziness PARTIAL: all rows laid out once per block, column measurement samples the first 1 000 rows; multi-line cells align the box, not each line). M7 part 1 done: callouts (GFM + Obsidian `[!type][+-] title`, full R-MD-04 mapping, B.6.6 geometry, vector icons for all 5 styles, fold chevron + click toggle via `HotKind::Fold` / `CodeOpts::fold_toggled` session state) and vector task boxes (B.6.4). Next: M7 part 2 — footnotes section + navigation + tooltip, front matter card, def-list goldens; then M8.
