# V2-F report: WYSIWYG F2 and table editing (prompt 016)

Status: **merged** as 0a41b78 (squash of PR #85, https://github.com/abuzer1495/YAMDV/pull/85); title has no `[skip ci]`; F-24: tree equals the green head ff0955c.
CI: run 40017140073 (da51d9f) red only at window goldens (intended `live-block-light` change + 2 uncommitted V2-F shots); one fix push → run 40020890007 (ff0955c) **green**. Issues #41 (FUT-43), #42 (FUT-44) closed.
Tags: `phase-v2f` → 0a41b78; `v1.3.0` → 47d68db `chore(release): v1.3.0` (empty, no `[skip ci]`). F-24: trees of 0a41b78 and 47d68db both equal the green head ff0955c.
v1.3.0 draft: https://github.com/abuzer1495/YAMDV/releases/tag/untagged-0bdbd1df5a17b22b9c3a (tag run 40024306004, green) — `YAMDV-1.3.0-win-x64.zip` SHA-256 `1d4d08593055657d6189affb0107eae3975fe7ef1979b670369e7e110576351b`; still a draft (not published).

## 1. Acceptance criteria
| AC | Status | Evidence |
|----|--------|----------|
| AC-1 F-29, F-30, ADR-0020, P-47 | done | `insert_image` Ctrl+Shift+I (F2/F3, rebindable); `cargo xtask size-map`; ADR-0020; spec-check green (P-01…P-47) |
| AC-2 items 1–13, TRACEABILITY, byte identity | done | 14 rows via `tools/trace_set.py` (R-F2-02/03/05/06/08/09, R-TBL-01…08); `file_changed_blocks` asserts in f2-reveal / table-edit (untouched table byte-identical, CRLF + BOM kept) |
| AC-3 tests | done | `f2-reveal`, `table-edit` green on CI; goldens `top-v2f-reveal-light`, `top-v2f-table-dark` committed + self-reviewed; unit tests pass; artifact `yamdv-exe` |
| AC-4 gates, merge, tags, release, report | done | §2; merged, issues closed, tags pushed, draft has zip + SHA-256 |

## 2. Gates (run 40020890007, windows)
| Gate | V2-E | V2-F | Limit | |
|---|---|---|---|---|
| M0 private WS | 2.34 MiB | 2.35 MiB | ≤ 2.64 | OK |
| M1 private WS | 4.65 MiB | 4.72 MiB | ≤ 4.95 | OK |
| T4 typical cold / warm p99 | — | 13.8 / 9.4 ms | gate | OK |
| T5 F3 (split) p99 | 17.7 ms | 14.0 ms | ≤ 25 (≤ 16 stretch) | OK / OK |
| T5 F2 (live) p99, typical / 1 MB | 25.1 ms | 24.8 / 18.7 ms | ≤ 40 (≤ 25 stretch) | OK / OK |
| T7 idle CPU (empty, doc) | 0 | 0 / 0 µs | 0 | OK |
| Exe (MSVC release) | 3,250,176 B | 3,386,368 B (**+136,192 B**) | ≤ +150 KB | OK |

## 3. F-30 size map (top 15, lld map of the hardened cross build: 3,378,688 B vs phase-v2e 3,244,544 B, +134,144 B)
| # | Crate::module | Bytes | Δ vs V2-E |
|---|---|---:|---:|
| 1 | `yamdv_platform_win::window` | 506,042 | +48,769 |
| 2 | `[anonymous constants]` | 405,471 | +8,592 |
| 3 | `tiny_skia::pipeline` | 133,061 | 0 |
| 4 | `encoding_rs::data` | 124,843 | 0 |
| 5 | `core::slice` | 119,575 | +10,322 |
| 6 | `[libucrt]` | 105,056 | +2 |
| 7 | `yamdv_core::doc` | 84,988 | +12,043 |
| 8 | `[other Rust]` | 66,812 | +1,564 |
| 9 | `yamdv_core::emoji_table` | 61,216 | 0 |
| 10 | `encoding_rs::variant` | 58,808 | 0 |
| 11 | `pulldown_cmark::parse` | 50,437 | 0 |
| 12 | `std::sys` | 41,861 | 0 |
| 13 | `pulldown_cmark::firstpass` | 37,143 | 0 |
| 14 | `hashbrown::raw` | 37,072 | +1,202 |
| 15 | `yamdv_layout::viewer` | 36,898 | +12,660 |
Full table: `docs/reports/v2f_sizemap.md`.

## 4. ADR-0020 summary
The active block is one reveal paragraph built from its own parser events; uncovered bytes are markers, shown muted only while a selection end is in/next to their construct or on their line.
Caret moves work on rope offsets and never rest in a hidden marker; a reveal change re-lays out only the active block, leaving it splices only the touched block (ADR-0007).
Transforms, table ops and cross-block edits are single undo steps; code/math/Mermaid/HTML keep block-level source; tables use cell editing, never pipes.

## 5. Deviations
- Table tools are a strip anchored above the active F2 table (below it near the view top), not inside the hover pill; ⛶ and CSV end the strip and the hover badge is hidden meanwhile. The strip covers the line above the table while a cell is active.
- Pretty-align runs on structural ops and when leaving a changed table, not on every keystroke.
- Math/Mermaid live preview is a card under the source, not a popover. Insert ▸ places the new block after the caret's whole block.
- `live-block-light` golden updated on purpose (F2 no longer shows a source box). The fix push also added ⛶/CSV to the strip (`top-v2f-table-dark` 504 px diff, OK).
- Prompt file is stored from the Read output; its attachment hash can't be reproduced (content is identical).
- The `phase-v2f` tag run 40024300810 (tree = ff0955c) failed once in `export-progress`: `pws_delta_kib` 216 > 205 (memory noise). The same tree is green in PR run 40020890007, main run 40024303448 and the v1.3.0 tag run 40024306004. Not rerun.
- Under Wine, edit-live-click, edit-live-up, viewers and keyboard-reach fail on the base 11227f9 too (no DirectWrite hit test / timing); green on CI.

## 6. Owner checklist / Sahip kontrol listesi
1. EN: New .md, F2, type a whole note using only transforms (`# `, `- `, `1. `, `[ ] `, `> `, ```` ```js ````+Enter, `---`+Enter, `$$`+Enter, `| a | b |`+Enter); Ctrl+Z once leaves the literal. TR: Yeni .md, F2, yalnızca dönüşümlerle bir not yazın (`# `, `- `, `1. `, `[ ] `, `> `, ```` ```js ````+Enter, `---`+Enter, `$$`+Enter, `| a | b |`+Enter); tek Ctrl+Z düz metni bırakır.
2. EN: Move the caret through `**bold**`, `*it*`, `` `code` ``, a link and a heading: markers appear only around the caret. TR: İmleci `**kalın**`, `*eğik*`, `` `kod` ``, bir bağlantı ve başlık üzerinde gezdirin: işaretler yalnızca imlecin çevresinde görünür.
3. EN: In a code block click the language label, type `py`, pick from the list. TR: Kod bloğunda dil etiketine tıklayın, `py` yazın, listeden seçin.
4. EN: Edit `$$ x^2 $$` and a Mermaid flowchart: the preview updates live, a typo keeps the last good diagram. TR: `$$ x^2 $$` ve bir Mermaid akışını düzenleyin: önizleme canlı güncellenir, hata son iyi diyagramı korur.
5. EN: In a table: Tab/Shift+Tab/Enter, Shift+Enter, type `|`; use the strip (add/delete, align, sort A→Z then Ctrl+Z, ⛶, CSV); drag a row and a column grip. TR: Tabloda Tab/Shift+Tab/Enter, Shift+Enter, `|` yazın; şeridi kullanın (ekle/sil, hizala, A→Z sırala sonra Ctrl+Z, ⛶, CSV); bir satır ve sütun tutamağını sürükleyin.
6. EN: Ctrl+T: pick 4 × 3 in the picker, Enter. TR: Ctrl+T: seçicide 4 × 3 seçin, Enter.
7. EN: Copy cells from Excel; paste inside a table (cells fill) and in a paragraph (click "Convert to table"). TR: Excel'den hücre kopyalayın; tabloya yapıştırın (hücreler dolar) ve paragrafa yapıştırıp "Tabloya dönüştür"e tıklayın.
8. EN: Edit one paragraph of a real git-tracked file in F2, save, run `git diff`: only that paragraph changed (EOL/BOM kept). TR: Git'teki gerçek bir dosyada F2 ile tek paragraf düzenleyip kaydedin, `git diff`: yalnızca o paragraf değişmiş olmalı (EOL/BOM korunur).
