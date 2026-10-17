# YAMDV — Phase V1-B Report: Editing (F3 split + block-level F2)

## 0. Status
- Prompt 006 is stored as `docs/prompts/006_V1B_EDITING.md`, SHA-256 `10fce65bdae1465f3b52c570a8b8195df5f6926334df0cdea5006b5bc13d1056`.
- Branch `phase/v1b-editing` → `main` via PR #6. Tag `phase-v1b`.
- Measurements come from CI run 39027801840 (code head 9b7dcd6; later commits are docs, goldens and this report). Every job is green and all 21 driver scripts pass (9 new, 12 carried over).
- Result: **done**. F-20 and F-21 are fixed. Nothing from `docs/FUTURE.md` was built.
- New dependencies: `ropey` 1.6 (pre-approved, ADR-0016) and `proptest` as a dev-dependency (ADR-0017).

## 1. Acceptance criteria
| AC | Status | Evidence |
|---|---|---|
| AC-1 | **pass** | F-20: memprobe opens `typical.md` in place and jumps to the image section (`--line 822`). M1 decoded image bytes are 1,382,400 B. F-21: T4 runs a second attempt, and the gate fails only if both attempts exceed 16 ms (attempt 2 was not needed) |
| AC-2 | **pass** | §2.1–2.6 are below. The 9 §2.7 driver scripts pass on CI. The proptest invariants (400 cases of random edit/undo/redo) pass |
| AC-3 | **pass** | `yamdv-editor` `byte_identity_outside_the_edit` (CRLF + BOM, windows-1254 / UTF-8 BOM / UTF-16LE). Read mode never writes; the only exception is a task-checkbox click when `reading.task_toggle_in_read_mode` is on |
| AC-4 | **pass** | Gates, memprobe, T4 (with F-21) and the M5/T5 soft budgets are green. PR #6 is merged and `phase-v1b` is tagged |
| AC-5 | **pass** | This report (≤ 120 lines) and the QA list in §4 |

| § | Item | Status | Notes |
|---|---|---|---|
| 2.1 | Editor core | done | `yamdv-editor`: ropey buffer, one caret + selection, grapheme and Unicode-word moves, wrapped Home/End, ↑/↓ keep the column, PgUp/PgDn, Ctrl+Home/End. Undo coalesces per word or after a 1 s pause, restores the selection, is capped at 4 MiB, and is shared by F2 and F3. Mouse: click, drag with auto-scroll, double-click (word), triple-click (line). Paste is plain text for both Ctrl+V and Ctrl+Shift+V. IME: inline composition with an underline; `ImmSetCompositionWindow` / `ImmSetCandidateWindow` are placed at the caret |
| 2.1 | Markdown keys | done | Enter continues list, task and quote prefixes and ends the list on an empty item. Tab / Shift+Tab indent and outdent by `editing.list_indent`; elsewhere Tab inserts 4 spaces. All the §2.1 format shortcuts toggle on the selection or on the word at the caret. Ctrl+K builds `[sel](url)` and selects `url` |
| 2.2 | Saving | done | Autosave runs 800 ms after the last edit, on mode exit, on deactivation and on close. Ctrl+S saves at once. The title shows `<file> ● — Live Edit — YAMDV`. Writes are atomic (S-FILE-03) and keep the encoding, BOM, dominant line ending and final-newline state |
| 2.2 | Conflicts | done | Size + mtime + hash are checked before every write. The in-window dialog offers [Overwrite] [Reload (discard mine)] [Save mine as…]; *Save as* uses the new dialog-host `save` kind. While the buffer is dirty, live reload opens the same dialog instead of reloading |
| 2.2 | Editing disabled | done | Read-only → [Save a copy as…]; lossy decode → [Convert to UTF-8 and edit]; plain-text fast mode → reason only |
| 2.3 | Incremental reparse | done | Common prefix/suffix diff → `Index::splice` → `Viewer::splice` (only changed blocks are re-laid out; the anchor is kept). ~30 ms debounce |
| 2.4 | F3 Split Edit | done | Editor left, preview right; Swap via the menu (`editing.split_swapped`). Splitter 6 px, 20–80 %, saved in `editing.split_ratio`. Editor pane: editor font, gutter, soft wrap, current-line highlight, caret blink stops after 5 s. Block scroll sync both ways; a click in the preview places the caret. "?" opens F1 help. Ctrl+F searches the buffer. The sidebar is hidden |
| 2.5 | F2 Live Edit | done | Clicking a top-level block turns it into source in place (editor font, `code_bg`). It renders again on another click, Esc, or moving past its first or last line; ↑/↓ continue into the neighbouring block. Task checkbox click = one source edit. Ctrl+click follows links. Untouched blocks are never re-serialized. F2 ↔ F3 switch directly; Esc leaves the block first, then the mode |
| 2.6 | Menu | done | Edit ▸ (F2, F3, Swap panes); in edit modes Undo · Redo · Cut · Copy · Paste · Format ▸ (13 entries); Settings ▸ Autosave, Editor font (Body / Code), Editor line numbers, Editor soft wrap |
| 2.7 | Tests | done | Driver `type "<text>"` records key → caret-painted latency into the dump. Scripts: `edit-split-type`, `edit-undo`, `edit-autosave`, `edit-conflict`, `edit-live-click`, `edit-live-up`, `edit-task`, `edit-enter-list`, `edit-bold`. 2 goldens: `split-light`, `live-block-light` |

## 2. Measurements
CI run 39027801840 on `windows-latest` (Server 2025, Hyper-V 1024×768, no GPU). Client area 1000×700, medians.

| ID | Result | Budget | Δ vs V1-A |
|---|---|---|---|
| M0 PWS / commit | 2.27 MiB (2,379,776 B) / 2.97 MiB | ≤ 4.8 MiB | = |
| M1 PWS / commit (`typical.md` at the images, F-20) | 4.64 MiB (4,870,144 B) / 5.95 MiB | ≤ 8.0 MiB | +1.71 (now includes images) |
| M1 image cache: decoded / LRU | 1,382,400 B / 0 B | LRU ≤ 1.5 MiB | new |
| M2 PWS / commit | 5.68 MiB (5,955,584 B) / 7.01 MiB | ≤ 16 MiB | +0.01 |
| M6 PWS / commit | 4.62 / 5.78 MiB | ≤ M1 + 1 / + 1.5 MiB | +1.56 (M1 base moved) |
| M5 F2 / F3 after 200 edits (`typical.md`) PWS | 3.81 / 5.84 MiB (commit 4.87 / 7.28) | ≤ 9.6 MiB | new |
| T2 / T1 / T1L first paint (M0 / M1 / M2) | 55 / 82 / 117 ms | 100 / 150 / 300 | −21 / −30 / −31 |
| T3 second launch → file visible | 32.5 ms (median of 5) | report | −3.7 |
| T4 typical warm median p99 (runs) | **9.93 ms** (9.93 / 10.19 / 8.59); attempt 2 not run | ≤ 16 ms | −4.24 |
| T4 typical cold p99 | 17.31 ms | report | +3.30 |
| T4 large warm median p99 / cold | 6.25 / 7.02 ms | ≤ 16 ms | +0.13 / −0.03 |
| T5 typing p99: F3 on `large-1mb.md` / F2 on `typical.md` | 16.9 / 19.8 ms (60 samples each) | ≤ 25 / 40 ms | new |
| T7 idle CPU 30 s (empty, `typical.md`) | 0 µs / 0 µs | ≤ 1 ms | = |
| T-DLG | 348 ms | ≤ 400 soft | −235 (now under the soft budget; no code change, runner variance) |
| B-SIZE `yamdv.exe` | 2.29 MiB (2,399,744 B) | ≤ 4.0 MiB | +0.24 MiB (editor) |
| ACG matrix | PNG, JPEG, GIF, BMP, TIFF yes; ICO, WebP no (FUT-48) | — | = |
| Goldens | 51 files, 3,842,864 B (P-28 ≤ 12 MB); 2 new in `tests/golden/v1a/` | 2 listed | +0.13 MB |

Editing stays out of the read-mode scroll path, so T4 typical did not regress; this run's warm p99 is lower than V1-A's (runner variance, the same code path). The typical cold p99 (17.3 ms, report-only) is the first-pass number including image decode.

## 3. Deviations and what is left
- **A-4 rule 6 (push budget):** 1 CI run on the code (39027801840, started by the interrupted run as `workflow_dispatch`) plus 1 run on the PR for the goldens and this report. A duplicate PR run on a docs-only commit was cancelled. Earlier WIP pushes carried `[skip ci]`.
- **`live-block-light` golden** shows the sidebar: the sidebar is hidden only in F3 (§2.4), not in F2.
- **IME, AltGr and dead keys** were not exercised on CI, because the runner has no IME or Turkish layout. They are in the owner QA (steps 5–6).
- **Not verified on real hardware** (the host is Linux): per-monitor DPI in the split view, and the real Save-as dialog from the conflict dialog (the driver checks the dialog-host `save` verb).
- **Left for later (unchanged):** T-DLG and a cheaper typical scroll path are in FUT-50; the ICO/WebP decode failures are in FUT-48.

## 4. Owner QA (Windows PC)
1. **F3 typing:** open a `.md` file and press F3. Type a heading and a list. The preview on the right updates within a moment, and the title shows `●` until autosave runs. Drag the splitter, then use *Edit ▸ Swap panes*.
2. **F2 click-edit-Esc:** press F2 and click a paragraph. It turns into its Markdown source on a grey background. Edit a word and press Esc: it renders again, and the blocks below stay in place. Press ↑ on the first line to move into the previous block.
3. **Autosave + reopen:** edit, wait 1 s, close YAMDV and reopen the file. The edit is there, and a diff in another editor shows only the changed line (same encoding, line endings and BOM).
4. **External edit conflict:** start an edit in F3 without waiting for autosave, then change and save the same file in Notepad. YAMDV shows [Overwrite] [Reload (discard mine)] [Save mine as…]; try each one.
5. **Turkish keyboard input:** with the Turkish Q layout, type `ğüşıöçİ` and `Ğ Ü Ş I Ö Ç` in both F3 and F2, including AltGr characters (`@ € {}`) and a dead-key accent. Every character appears once, and Ctrl+Z undoes them by word.
6. **IME (if available):** switch to Japanese or Chinese IME and type in F3. The composition is underlined at the caret, the candidate window opens next to the caret, and committing inserts the text.

## 5. Özet (Türkçe)
V1-B tamamlandı. YAMDV'de artık Markdown dosyaları güvenle düzenlenebiliyor:
- **F3:** solda kaynak, sağda canlı önizleme. Bölücü sürüklenebiliyor, paneller yer değiştirebiliyor, kaydırma iki yönde eşleniyor.
- **F2:** tıklanan blok yerinde Markdown kaynağına dönüşüyor; Esc ile ya da başka bir bloğa tıklayınca yeniden çiziliyor. Görev kutuları tek tıkla değişiyor.
- Ortak düzenleyici: geri al / yinele, Markdown tuşları ve biçim kısayolları, IME.
- Otomatik kaydetme ve Ctrl+S. Dosyanın kodlaması, BOM'u ve satır sonları korunuyor; değişmeyen baytlara dokunulmuyor. Dosya dışarıdan değişirse çakışma penceresi açılıyor.

21 driver betiğinin hepsi CI'da geçiyor. Kaydırma p99 9,9 ms, yazma gecikmesi p99 F3'te 16,9 ms, F2'de 19,8 ms; düzenleme sonrası bellek 5,8 MiB'ın altında. Görselli M1 ölçümü artık yapılıyor (4,64 MiB). Aç penceresi bu çalıştırmada 348 ms ile ilk kez yumuşak hedefin altında kaldı.
