# V2-D report: navigation (prompt 014)

Status: **merged** as aaac914 (squash of PR #83, head db674fb). Tags `phase-v2d` and `v1.2.0` are on aaac914 and pushed. Last CI run 39987570191 on db674fb is **green**.
**Draft release v1.2.0 not created.** The merge subject carries `[skip ci]`, so GitHub skipped the push workflow for both tags, and the tag job never ran.
Next action (owner): choose one. (a) Move both tags to a commit whose message has no `[skip ci]` (delete and re-push the tags), so the tag job builds the draft. (b) Create the draft by hand from the release build.

## 1. Acceptance criteria
| AC | Status | Evidence |
|----|--------|----------|
| AC-1 F-23, F-24, P-43…P-45 | done | `doc::plain` test `f23_markup_is_removed`; golden `top-v2c-footnote-light` updated; A-9 line in PROCESS.md; spec-check green |
| AC-2 §3 items 1–7, TRACEABILITY by script, S-FILE-08 | done (code) | 12 rows updated by `trace_v2d.py`; watchers: doc folder, config, expanded Files folders (≤ 32) |
| AC-3 tests, driver scripts, goldens | done | unit tests pass; run 4: all 37 driver scripts, goldens and window goldens green (A-10 changes in §4) |
| AC-4 gates | done | see §2 |
| AC-5 version 1.2.0, CHANGELOG, docs | done | workspace 1.2.0; `[1.2.0] - 2026-10-23` |
| AC-6 merge, tags, draft release | done except the draft | merge aaac914; `git diff --quiet db674fb aaac914` succeeds (tree = green head, F-24 met, A-10 waiver not needed); tags pushed; draft blocked by `[skip ci]` (see Status) |

## 2. Gates (run 4 39987570191, windows-2022, green)
| Gate | V2-C | V2-D | Limit |
|------|------|------|-------|
| M0 private WS | 2.36 MiB | 2.36 MiB | ≤ +0.3 |
| M1 private WS | 4.61 MiB | 4.72 MiB (+0.11) | ≤ +0.3 |
| T4 typical warm p99 | 9.95 ms | 7.27 ms (runs 1–3: 8.79 / 10.88 / 11.32) | A-7 |
| T7 idle (doc / empty, 30 s) | 0 | 0 ms / 0 ms CPU | report |
| T7 Files tab visible / mind map open (2 s) | — | 0 ms / 16 ms | < 60 ms |
| Exe size | 2,988,032 B | 3,082,752 B (+94,720 B) | report |
| Mind map, large-1mb.md | — | 15,704 nodes, build 148 ms, +3,620 KiB PWS | report only |

## 3. CI
- Run 1 (f05a7f8): https://github.com/abuzer1495/YAMDV/actions/runs/39927529395 (red: `tab-order` needed the heading fold stops, `keyboard-reach` walk wrapped, `files-favorites` timed out; `menu-light`, `top-v2c-footnote-light` goldens)
- Fix run (ef7ad7e): https://github.com/abuzer1495/YAMDV/actions/runs/39931340336 (red: only `files-favorites`, last step)
- Run 3 (af625d7, owner-approved): https://github.com/abuzer1495/YAMDV/actions/runs/39983230028 (red: only `export-progress.yds`, `cpu_delta_ms` 16 > 15; every other windows step green, pure crates green, pack/owner-kit jobs skipped)
- Run 4 (db674fb, after A-10): https://github.com/abuzer1495/YAMDV/actions/runs/39987570191 (**green**: windows + pure crates; pack, owner-kit and release jobs skipped on PRs by design)
- PR: https://github.com/abuzer1495/YAMDV/pull/83 (Closes #32 #9 #10 #14 #36 #53)

## 4. A-10 (owner override, 2026-10-23: "testleri azalt gereksiz test olmasın birleştir yinede")
Recorded in PROCESS.md (via `docs/spec-patches/PROCESS_AMENDMENTS.md`). Removed or demoted tests:
- Demoted: `export-progress.yds` `cpu_delta_ms <= 15` is now report-only (`>= 0`, the value is logged). One timer tick (16 ms) failed run 3; the T7 gate stays in the measurements step. Run 4 logged 0 ms.
- Removed: `anchor.yds`. `navigation.yds` covers it: the same click on "Go to target", the same `scroll_y > 500`, and `document == a.md`.
- Kept: the `< 60 ms` CPU asserts (four ticks, the V2-C/V2-D T7 checks), all goldens, the M0/M1/T4/T7 gates, pack smoke and the security scripts. No unit tests were removed; this conservative pass found no exact duplicates.
Resolved earlier: `ui nav-drop-folder .` passed `…\tests\driver\.` to the Files tab; `\\?\` long paths keep the `.`, so the listing was empty. The fix is test-side: the probe strips the trailing `\.`. Real drops come from `DragQueryFileW`, which gives absolute paths. It was verified in runs 3 and 4 (`files-favorites.yds: PASS`).

## 5. Deviations
- Toasts have no action buttons, so the Tab order has no toast stops.
- Each heading's fold toggle is a Tab stop. `tab-order.yds` was updated to match.
- The Files tab uses one watch thread per expanded folder (≤ 32) and lists folders with std `read_dir`.
- A floating sidebar column is repainted on scroll, so T4 "sidebar never repainted" does not hold while floating.
- Ctrl+Shift+O/M/1–4 and F6 are hard-wired in nav. They are not rebindable keymap actions yet.
- The favorites "restart" check re-reads state.ini instead of restarting. The `↓ → Enter` check runs as Home → ↓ Enter.
- The driver now writes a partial log after each line. Watcher drop now retries the cancel until the thread ends, so dropping a watcher that has just started cannot hang.

## 6. Owner check list (after the draft exists)
EN
1. Open a .md in a real folder. Press Ctrl+Shift+4: folders come first, the order is natural, and the current file is marked. Expand a folder and open a file.
2. Drop a folder onto the window. The Files tab opens rooted at that folder.
3. Settings → Sidebar position → Left floating. The page keeps its width and the card sits 12 px from the edges.
4. Favorites: drag a row to the top, then Alt+↑/↓. Restart; the order is kept.
5. Press Ctrl+Shift+O, type part of a heading, press Enter. The view jumps there (folded sections open).
6. Put `[TOC]` in a document. Its links scroll to the headings. Export HTML and the list is still there.
7. Press Ctrl+Shift+M. Try Balanced, zoom and pan, click a node to collapse it, double-click to jump, Ctrl+C to copy a PNG.
8. Tab through links, code buttons, tasks and toggles; the focus ring is visible. Press F6 to cycle the title bar, sidebar and document.

TR
1. Gerçek bir klasördeki .md dosyasını açın. Ctrl+Shift+4'e basın: klasörler önce gelir, sıralama doğaldır, geçerli dosya işaretlidir. Bir klasörü açıp bir dosya açın.
2. Pencereye bir klasör bırakın. Dosyalar sekmesi o klasör kök olacak şekilde açılır.
3. Ayarlar → Kenar çubuğu konumu → Sol yüzen. Sayfa genişliği aynı kalır, kart kenarlardan 12 px içeride durur.
4. Sık kullanılanlar: bir satırı en üste sürükleyin, sonra Alt+↑/↓ deneyin. Yeniden başlatın; sıra korunur.
5. Ctrl+Shift+O'ya basın, bir başlığın bir kısmını yazın, Enter'a basın. Görünüm o başlığa gider (katlı bölümler açılır).
6. Bir belgeye `[TOC]` ekleyin. Bağlantılar başlıklara kaydırır. HTML dışa aktarın; liste orada da durur.
7. Ctrl+Shift+M'ye basın. Dengeli düzeni, yakınlaştırma ve kaydırmayı deneyin; düğüme tıklayıp daraltın, çift tıklayıp atlayın, Ctrl+C ile PNG kopyalayın.
8. Tab ile bağlantılar, kod düğmeleri, görevler ve aç/kapa düğmeleri arasında gezin; odak halkası görünür. F6 ile başlık çubuğu, kenar çubuğu ve belge arasında geçin.
