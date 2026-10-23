# V2-D report: navigation (prompt 014)

Status: **CI red on the owner-approved run 3 → no merge, no tags.** One check failed: a V2-B timing assert in `export-progress.yds` (`cpu_delta_ms <= 15`, actual 16 ms). The V2-D fix worked: `files-favorites` passes, and 37/38 driver scripts pass. Following the instruction, no more fixes were pushed.
Next action: the owner decides on one of two fixes. (a) Re-run the failed job unchanged. 16 ms is one Windows timer tick (15.6 ms) of CPU over 3 s, which looks like timer-granularity noise. (b) Approve a one-line fix that relaxes the assert to `< 32` (two ticks), then one CI run. If it goes green: squash-merge, check `git diff --quiet <green head> <merge>` (F-24), tag `phase-v2d` and `v1.2.0`, keep the release as a draft.

## 1. Acceptance criteria
| AC | Status | Evidence |
|----|--------|----------|
| AC-1 F-23, F-24, P-43…P-45 | done | `doc::plain` test `f23_markup_is_removed`; golden `top-v2c-footnote-light` updated; A-9 line in PROCESS.md; spec-check green |
| AC-2 §3 items 1–7, TRACEABILITY by script, S-FILE-08 | done (code) | 12 rows updated by `trace_v2d.py`; watchers: doc folder, config, expanded Files folders (≤ 32) |
| AC-3 tests, driver scripts, goldens | partial | unit tests pass; run 3: goldens + window goldens green, 37/38 driver scripts pass (`files-favorites` fixed); `export-progress` fails by 1 ms (see §4) |
| AC-4 gates | done | see §2 |
| AC-5 version 1.2.0, CHANGELOG, docs | done | workspace 1.2.0; `[1.2.0] - 2026-10-23` |
| AC-6 merge, tags, draft release | **not started** | blocked by F-24 (no green PR head); run 3 red |

## 2. Gates (run 3 39983230028, windows-2022; measurements step green)
| Gate | V2-C | V2-D | Limit |
|------|------|------|-------|
| M0 private WS | 2.36 MiB | 2.36 MiB | ≤ +0.3 |
| M1 private WS | 4.61 MiB | 4.73 MiB (fix run 4.63) | ≤ +0.3 |
| T4 typical warm p99 | 9.95 ms | 11.32 ms (fix run 10.88, run 1 8.79) | A-7 |
| T7 idle (doc / empty, 30 s) | 0 | 0 ms / 0 ms CPU | report |
| T7 Files tab visible / mind map open (2 s) | — | 0 ms / 16 ms | < 60 ms |
| Exe size | 2,988,032 B | 3,082,752 B (+94,720 B) | report |
| Mind map, large-1mb.md | — | 15,704 nodes, build 191 ms, +3,136 KiB PWS | report only |

## 3. CI
- Run 1 (f05a7f8): https://github.com/abuzer1495/YAMDV/actions/runs/39927529395 (red: `tab-order` needed the heading fold stops, `keyboard-reach` walk wrapped, `files-favorites` timed out; `menu-light`, `top-v2c-footnote-light` goldens)
- Fix run (ef7ad7e): https://github.com/abuzer1495/YAMDV/actions/runs/39931340336 (red: only `files-favorites`, last step)
- Run 3 (af625d7, owner-approved): https://github.com/abuzer1495/YAMDV/actions/runs/39983230028 (red: only `export-progress.yds`, `cpu_delta_ms` 16 > 15; every other windows step green, pure crates green, pack/owner-kit jobs skipped)
- PR: https://github.com/abuzer1495/YAMDV/pull/83 (Closes #32 #9 #10 #14 #36 #53)

## 4. Open item
Run 3 failure: `tests/driver/export-progress.yds:46` `assert cpu_delta_ms <= 15` failed with 16 ms, measured over 3 s idle after a cancelled background PDF export. The script was not changed in V2-D (last change #80). Earlier runs passed it.
Resolved: `ui nav-drop-folder .` passed `…\tests\driver\.` to the Files tab; `\\?\` long paths keep the `.`, so the listing was empty. The fix is test-side: the probe strips the trailing `\.`. Real drops come from `DragQueryFileW`, which gives absolute paths. It was verified in run 3 (`files-favorites.yds: PASS`).

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
