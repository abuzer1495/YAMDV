# V2-E report: editing plus (prompt 015)

Status: **merged** as a9a9bc0 (squash of PR #84, https://github.com/abuzer1495/YAMDV/pull/84). The merge title has no `[skip ci]`; the tree equals the green head f4b19f4.
CI: one run, 40000635873 (https://github.com/abuzer1495/YAMDV/actions/runs/40000635873), **green**; no fix push. Tag `phase-v2e` → a9a9bc0, pushed (no release this phase). Issues #43 (FUT-45), #17 (FUT-12), #18 (FUT-15) closed.
v1.2.0 (F-25): draft https://github.com/abuzer1495/YAMDV/releases/tag/untagged-cd2b602efd39746f47e9, `YAMDV-1.2.0-win-x64.zip` SHA-256 `3f0de83ec3dbefb2cb6cd114d01a0b4eee5ab78c71c3f30f2fdcab7d07ad9fea`; still a draft.

## 1. Acceptance criteria
| AC | Status | Evidence |
|----|--------|----------|
| AC-1 F-25…F-28, P-46, regex ADR | done | `chore(release): v1.2.0` 2534afb (f99904c vs aaac914: docs only), tags moved, draft built by tag run 39993219747; F-25 line in A-9; F-26 (§6), F-27, F-28 per V2E_PLAN CHECKPOINT; P-46 applied, spec-check green; ADR-0019 |
| AC-2 §3 items 1–6, TRACEABILITY, item-5 security | done | 8 rows set by `tools/trace_set.py docs/plans/v2e_trace.tsv`; item-5 rules: HTML paste is allowlist-converted (no script/handlers; `!contains` asserts), assets only under `assets/<doc>/`, no network fetches |
| AC-3 tests | done | `edit-plus`, `paste-recovery` green on CI; goldens `v2e-toolbar-light`, `v2e-replace-dark` committed; unit tests pass; artifact `yamdv-exe` present |
| AC-4 gates, merge, tag, report | done | §2; merged, issues closed, `phase-v2e` tagged |

## 2. Gates (run 40000635873, windows, all measurement gates green)
| Gate | V2-D | V2-E | Limit |
|------|------|------|-------|
| M0 private WS | 2.36 MiB | 2.34 MiB | ≤ +0.3 |
| M1 private WS | 4.72 MiB | 4.65 MiB | ≤ +0.3 |
| T4 typical warm p99 | 7.27 ms | 9.04 ms (cold 14.46) | A-7 |
| T7 idle doc / empty (30 s) | 0 / 0 | 0 / 0 ms CPU | report |
| T7 F3, autosave off, after a journal write (2 s) | — | 16 ms (paste-recovery) | < 60 ms |
| T5 split / live p99 | — | 17.7 / 25.1 ms | report |
| Exe size (MSVC release) | 3,082,752 B | 3,250,176 B (**+167,424 B**) | report (B-SIZE OK) |

Exe delta split: `regex-lite` ≈ 42.7 KB of code by symbol (lld map of the cross build; ADR-0019 probe: +59.0 KB, budget 150 KB). The rest: HTML→Markdown converter, journal, toolbar, find/replace row, snippets, paste/drop.

## 3. Regex decision
Shipped: `regex-lite` 0.1.9 (ADR-0019), `[.*]` toggle off by default; `$1` / `${name}`; 100 ms guard ("Regex too slow"), unit test `(a+)+$` on 100 KB < 50 ms. Turkish i/İ/ı/I folding for plain search only; regex case folding is ASCII.

## 4. Implemented R-EDIT-07 keys that were missing
`Ctrl+T` (insert table, 2×2 snippet) and `Ctrl+Shift+K` (code block; wraps a selection). Already present before: the rest of R-EDIT-07. `Ctrl+Shift+I` (image) is listed in SHORTCUTS.md but is not in R-EDIT-07 and has no keymap action (left as is). Table-context keys (R-TBL) are V2-F.

## 5. Deviations
- `TerminateProcess` in `paste-recovery` is simulated in-process (`ui v2e-kill`: buffer and journal state dropped, the journal file stays, the next `open` is the restart). A real kill is in the owner checklist (step 6).
- F2 selection in `edit-plus` uses `ui v2e-live-select` (opens the block and selects the word) instead of `click-text`, so the script also runs under Wine (no DirectWrite hit test there).
- Driver additions: `!contains` assert operator; `v2e-*` verbs (dev-tools only).
- Ctrl+V and Shift+Insert in edit modes now go through rich paste (HTML / bitmap), plain text otherwise.

## 6. F-26 thread count
Wine: `watch_threads` 2>2 (delta 0, 14 live threads). CI `files-favorites`: 5>5 after ten extra folder watches (delta 0).

## 7. Owner checklist / Sahip kontrol listesi
1. EN: Open a .md, F2, select a word: the toolbar shows above it; Bold → `**word**`; Esc hides it. TR: Bir .md açın, F2, bir kelime seçin: araç çubuğu üstünde görünür; Kalın → `**kelime**`; Esc gizler.
2. EN: F3, Ctrl+H: replace `foo`→`bar`, Replace all, one Ctrl+Z restores; try `[.*]` with `\d+`→`#`. In read mode Ctrl+H switches to split edit first. TR: F3, Ctrl+H: `foo`→`bar` Tümünü değiştir, tek Ctrl+Z geri alır; `[.*]` ile `\d+`→`#` deneyin. Okuma modunda Ctrl+H önce bölünmüş düzenlemeye geçer.
3. EN: In F3 type `table2x3` Tab (Tab next field), `python` Tab, `note` Tab. TR: F3'te `table2x3` Tab (Tab sonraki alan), `python` Tab, `note` Tab yazın.
4. EN: Copy part of a web page (heading, list, link, table) and Ctrl+V: Markdown, no scripts; Ctrl+Shift+V: plain text. TR: Bir web sayfasından (başlık, liste, bağlantı, tablo) kopyalayıp Ctrl+V: Markdown, betik yok; Ctrl+Shift+V: düz metin.
5. EN: Win+Shift+S a screenshot, Ctrl+V in a saved doc: `assets/<name>/image-….png` and `![](…)`. TR: Win+Shift+S ile ekran görüntüsü, kayıtlı belgede Ctrl+V: `assets/<ad>/image-….png` ve `![](…)`.
6. EN: Settings → autosave off; edit, wait 3 s, kill YAMDV in Task Manager, reopen the file: banner → Restore gives the unsaved text (dirty); repeat with Discard. TR: Ayarlar → otomatik kaydetme kapalı; düzenleyin, 3 sn bekleyin, Görev Yöneticisi'nden YAMDV'yi sonlandırın, dosyayı yeniden açın: şerit → Geri yükle kaydedilmemiş metni getirir (kirli); Vazgeç ile tekrarlayın.
7. EN: Settings → auto-pair on: `(` gives `()`, Backspace removes both; off: `(` stays single. TR: Ayarlar → otomatik eşleme açık: `(` → `()`, Backspace ikisini siler; kapalı: `(` tek kalır.
8. EN: Publish the v1.2.0 draft when ready (it stays a draft until you do). TR: Hazır olduğunuzda v1.2.0 taslağını yayınlayın (siz yayınlayana kadar taslak kalır).

Next action (owner): run the checklist; publish v1.2.0 if wanted; send prompt 016 (V2-F).
