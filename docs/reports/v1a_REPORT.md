# YAMDV — Phase V1-A Report: App shell & daily use

## 0. Status
- Prompt 005 is stored as `docs/prompts/005_V1A_SHELL_DAILY_USE.md`, SHA-256 `6c6c8e482e5a213268fafd7bbfef410ce8d51f516772c0b01ad5196d5b46c04b`.
- Branch `phase/v1a-shell` → `main` via PR #5. Tag `phase-v1a`.
- Measurements come from CI run 39019374434 (code head 67f99d5). Every job is green, and all 12 driver scripts pass (6 new, 6 carried over).
- Result: **done**. Two smaller items are open, listed in §4: the menu-open golden and M1 with decoded images.
- §2 process and scope work was done by script:
  - A-4 was added to `PROCESS.md`.
  - `docs/FUTURE.md` was extracted from the prompt block by `cargo xtask future`, and README has a "Possible future developments" section.
  - P-30 and P-31 are in `docs/spec-patches/v1a.txt`.
  - TRACEABILITY now has 64 `deferred (FUT-xx)` rows plus "part deferred" notes, also by `cargo xtask future`.
  - `spec-check` is green.

## 1. Acceptance criteria
| AC | Status | Evidence |
|---|---|---|
| AC-1 | **pass** | `cargo xtask future` and `spec-patch`; `spec-check` green in the ubuntu job |
| AC-2 | **pass** | Items 1–13 are below. The 6 V1-A driver scripts pass on CI: `theme`, `find`, `instance`, `reload`, `sidebar`, `favorites` |
| AC-3 | **pass** | Gates, memprobe and T4 typical warm 14.17 ms (≤ 16) are green. PR #5 is merged and `phase-v1a` is tagged |
| AC-4 | **pass** | This report (≤ 120 lines) and the QA list in §5 |

Items 1–13 (prompt §3):

| # | Item | Status | Notes |
|---|---|---|---|
| 1 | Popup menu | done | Owner-drawn `TrackPopupMenuEx` with theme colours, check and radio marks, shortcut column, submenus and separators. Arrows, Enter and Esc come from the system menu loop. Letter jump uses `WM_MENUCHAR`. Opens on right-click, Shift+F10 and the Menu key |
| 2 | Context menu content | done | All §3.2 entries (59 items). Settings are written to `settings.ini` and apply live. Body font uses `ChooseFontW` through the dialog host. F1 opens a built-in help document; About is an in-window dialog |
| 3 | Sidebar | done | Contents (flattened titles, scroll spy, smooth scroll, collapse), History (groups, remove, Clear per group), Favorites. Edge chevron and `Ctrl+\`, drag-resize 180–480 px, left or right. State is kept in `state.ini` |
| 4 | Themes | done | Mode and 11 schemes apply live to the document, sidebar, menus, dialogs, toasts and scrollbar. `custom-theme.ini` hot-reloads; it is watched only while custom colours are on |
| 5 | Windows and instances | done | Mutex plus forward IPC (S-IPC-01…03). One window per file. Geometry is validated against the current monitors. Title is `<file> — YAMDV` |
| 6 | Live reload | done | Directory watcher catches atomic-rename saves. Reload is debounced and keeps the scroll anchor. A deleted file gives a dimmed document and a toast |
| 7 | Reading position | done | Per file in `state.ini` |
| 8 | Find | done | Incremental, highlights all matches, n/m counter, Enter / Shift+Enter, Ctrl+G / Ctrl+Shift+G, Match case, Turkish i/İ/ı/I equivalence, Esc |
| 9 | Zoom | done | Font: Ctrl+wheel, `Ctrl+=` / `Ctrl+-` / `Ctrl+0`, Ctrl+↑/↓. Column: Alt+wheel, Alt+↑/↓, Alt+0. Shows a toast and is persisted |
| 10 | External editor | done | F4 / Ctrl+E open VS Code, Notepad++, Notepad or the custom command at the current line. Uses `CreateProcessW` with S-EXEC-03 quoting |
| 11 | System integration | done | HKCU registration plan writer, set-default flow, Explorer context-menu toggle, the CLI flags `--register`, `--unregister`, `--set-default`, `--context-menu on\|off` and `--line <n>`, and portable mode. Nothing is registered without an explicit action (S-REG-03) |
| 12 | Welcome page | done | Icon, title, drop-zone text, [Open file…], the last 8 recent files, and a default-app status line (UserChoice ProgId) with [Set as default] |
| 13 | Toast | done | B.10.1; used for zoom, reload and delete |

## 2. Measurements
CI run 39019374434 on `windows-latest` (Server 2025, Hyper-V 1024×768, no GPU). Client area 1000×700, medians of the runs.

| ID | Result | Budget | Δ vs 003b |
|---|---|---|---|
| M0 PWS / commit | 2.27 MiB (2,383,872 B) / 2.97 MiB | ≤ 4.8 MiB | +0.20 |
| M1 PWS / commit (`typical.md`) | 2.93 MiB (3,067,904 B) / 4.01 MiB | ≤ 8.0 MiB | +0.36 |
| M1 image cache bytes | 0 (corpus has no images) | — | see §4 |
| M2 PWS / commit | 5.67 MiB (5,943,296 B) / 7.00 MiB | ≤ 16 MiB | +0.29 |
| M6 PWS / commit | 3.06 / 4.20 MiB | ≤ M1 + 1 / + 1.5 MiB | +0.51 |
| T2 / T1 / T1L first paint (M0 / M1 / M2) | 76 / 112 / 148 ms | 100 / 150 / 300 | −5 / +25 / −20 |
| T3 second launch → file visible | 36.2 ms (47 / 34.4 / 32.5 / 48.6 / 36.2) | report | new |
| T4 typical warm median p99 (runs) | **14.17 ms** (14.17 / 13.61 / 14.38) | ≤ 16 ms | −0.07 |
| T4 typical cold p99 | 14.01 ms | report | +0.05 |
| T4 large warm median p99 / cold | 6.12 / 7.05 ms | ≤ 16 ms | +0.54 / +2.32 |
| T7 idle CPU 30 s (empty, `typical.md`) | 0 µs / 0 µs | ≤ 1 ms | = |
| T-DLG | 583 ms | ≤ 400 soft | +77 (over the soft budget) |
| B-SIZE `yamdv.exe` | 2.05 MiB (2,146,304 B) | ≤ 4.0 MiB | +0.16 MiB |
| ACG matrix | PNG, JPEG, GIF, BMP, TIFF yes; ICO, WebP no (FUT-48) | — | = |
| Goldens | 50 files, 3,708,993 B (P-28 ≤ 12 MB); 5 new in `tests/golden/v1a/` | ≤ 6 new | +0.31 MB |

T4 watch item (prompt §1):
- CI run 2 showed the shell chrome pushing typical warm p99 to 18.6 ms. Cause: every scroll step repainted the full height, through the scrollbar strip and then the sidebar scroll-spy column, because `Renderer::paint` bands over the `rcPaint` bounding box.
- Fixed inside this phase:
  - `ScrollWindowEx` is limited to the document rect.
  - Only the vertical spans of the update region are painted.
  - Only the old and new scrollbar thumb are invalidated.
  - The scroll-spy repaint is deferred by a 120 ms one-shot timer.
- The result is back to 14.2 ms.

## 3. Deviations
- **A-4 rule 6 (push budget):** 4 CI pushes plus this report push, against 1 + 1. The extra pushes fixed a process-exit crash (0xc0000409: a watcher-thread join in a TLS destructor, now an explicit teardown on WM_QUIT), a `measure.ps1` parse error, and the T4 regression (AC-3 plus the prompt §1 watch item). Run 3 also exposed a T4 gate that compared numbers as strings; it is now numeric.
- **Popup menu:** uses Win32 owner-draw menus, not a custom layered window. This is the simplest option that meets item 1 (A-4 rule 1). There are no layered shadows or animation beyond what the system menu provides.
- **Goldens:** 5 of the 6 listed (welcome light, doc + sidebar light and dark, find bar, Glacier light). There is no "menu open" golden, because the menu runs in the system modal loop as a separate window that the driver's renderer snapshot cannot capture. The menu is covered by the `theme` driver script, which asserts the item count (> 20; 59 at run time) and the checked radio state.
- **M1 with decoded images:** not measured. The M1 corpus (`typical.md`) has no images, so the image cache shows 0 B. A-4 rule 3 forbids new measurement tooling, and changing the corpus would change M1's baseline.
- **T-DLG** is 583 ms, still over the 400 ms soft budget. The Open dialog runs in a separate process; the time is dominated by comdlg32/shell start-up.

## 4. Left open
- A menu-open golden, which needs a desktop-capture snapshot (driver work).
- M1 with images and image cache bytes, which needs an image corpus document. Suggested for V1-B if the owner wants it.
- T-DLG over the soft budget (unchanged since 003).
- Not verified on real hardware (the host is Linux): per-monitor DPI, a real `.md` default-app switch through the Settings UI, and Explorer context-menu appearance. These are covered by the QA list below.

## 5. Owner QA (Windows PC)
1. **Install as the default app:** run `yamdv.exe --register`, then right-click → *Settings ▸ Set YAMDV as default app…* and choose YAMDV for `.md` in Windows Settings. The welcome page status line then shows YAMDV as the default.
2. **Double-click a `.md` file** in Explorer. It opens in YAMDV with the sidebar Contents. Double-click it again: the same window comes to the front instead of a second window opening.
3. **Live reload:** edit and save the file in another editor (F4 opens it at the current line). YAMDV updates within a moment and keeps the scroll position. Delete the file: the document dims and a toast appears.
4. **Theme switch:** right-click → *Appearance ▸ Theme mode* Dark, then *Color scheme* Glacier. The document, sidebar, menu and scrollbar change immediately, and the choice survives a restart.
5. **Find:** press Ctrl+F and type a word. All matches are highlighted and the n/m counter is shown. Enter / Shift+Enter and Ctrl+G move between matches; try `i` against `İ` in Turkish text; Esc closes the bar.

## 6. Özet (Türkçe)
V1-A tamamlandı. Eklenenler:
- Sağ tık menüsünden canlı ayarlar.
- Kenar çubuğu: İçindekiler, Geçmiş ve Favoriler.
- Canlı tema değişimi.
- Tek örnek çalışma, dosya başına tek pencere.
- Canlı yeniden yükleme, okuma konumu, arama, yakınlaştırma, harici düzenleyici.
- Sistem kaydı ve karşılama sayfası.

Driver betiklerinin 12'si de CI'da geçiyor. Kaydırma p99 değeri 14,17 ms (sınır 16 ms). Bu fazda ortaya çıkan gerilemeyi düzelttik: kaydırmada artık yalnızca gereken bölgeler boyanıyor.

Eksik kalanlar: menü açıkken golden görüntüsü ve görsel içeren M1 ölçümü. Aç penceresi 583 ms sürüyor ve 400 ms yumuşak hedefinin üstünde.
