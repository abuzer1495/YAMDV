# YAMDV — Phase V2-A Report: window chrome (prompt 011)
The prompt is stored as `docs/prompts/011_V2A_WINDOW_CHROME.md` (SHA-256 `d0364143…`). Rules: PROCESS A-1…A-7 (A-7 was added in this phase; spec patches P-32/P-33). Branch `phase/v2a-chrome`, PR #76, squash-merged as 7deb0b3 and tagged `phase-v2a`. Issues closed: #29 FUT-30, #31 FUT-32, #34 FUT-35, #44 FUT-46, #69 BUG-09.

## 1. Acceptance criteria
| AC | Status | Evidence |
|---|---|---|
| AC-1 title bar + popups (§3.1, §3.2) | pass | `window/chrome.rs` covers NCCALCSIZE/NCHITTEST, HTMAXBUTTON, ⚙/⋯, Alt/F10 and the system menu. `window/popup.rs` provides the layered menus. The `titlebar.yds` script passes: ⋯ shows 64 items, ⚙ submenu opens/closes with → / ←, Max/Restore work by button and by double-click |
| AC-2 scheme cards (§3.3) | pass | The `scheme-cards.yds` script passes: hover previews glacier, Esc reverts, a click persists forest to `settings.ini` (`disk_scheme`) |
| AC-3 F11, word count, zoom pill (§3.4–3.6) | pass | The `fullscreen-wordcount.yds` script passes. Core word count (CJK counts 1 per character) has unit tests in `yamdv-core/src/chrome.rs` |
| AC-4 tests, gates, CI | pass | CI run 39770339165 is green: 28/28 driver scripts pass and all 57 goldens have 0 px diff, including the 4 new ones (`top-light`, `top-dark`, `menu-light`, `cards-light`) |

## 2. Gates (CI run 39770339165, windows-2025, feb5c87)
| ID | V2-A | v1.0.0 | Delta | Budget |
|---|---|---|---|---|
| M0 PWS | 2.35 MiB | 2.30 MiB | +0.05 MiB | ≤ +0.3 MiB; ≤ 4.8 MiB |
| M1 PWS | 4.66 MiB | 4.71 MiB | −0.05 MiB | ≤ +0.3 MiB; ≤ 8.0 MiB |
| T4 typical warm median p99 | 6.20 ms (cold 7.64) | 11.04 ms | −4.84 ms (runner noise) | ≤ 16 ms |
| T7 idle CPU (empty / doc) | 0 / 0 µs | 0 | 0 | 0 wake-ups |
| `yamdv.exe` | 2,831,872 B (2.70 MiB) | 2,788,864 B | **+43,008 B** | ≤ 4.0 MiB (soft) |

Local gates passed on feb5c87: fmt, clippy native + cargo-xwin x86_64-pc-windows-msvc (with and without dev-tools), workspace tests, deny, spec-check, check-imports (19 DLLs) and size.

## 3. What changed
- Title bar: 32 px in `chrome_bg`, with a `border_subtle` line when scrolled. Left side: icon, middle-ellipsized title (`general.title_format`), mode pill, dirty ●. Right side: ⚙ ⋯ (32×32), then Min/Max/Close (46×32; Close hover #c42b1c). Glyphs use Segoe Fluent Icons, then MDL2, then a vector fallback. The title dims to 55 % when the window is inactive.
- Window behavior: the DWM frame is kept, so resize borders, shadow and rounded corners stay. Content sits `top` DIPs lower via `Band::set_origin`, and every input and invalidation path is offset.
- Popup menus: the `TrackPopupMenuEx` owner-draw is gone. Menus are WS_EX_LAYERED with an 80 ms fade (instant with reduced motion), radius 8 (0 when sharp) and DWM border/corner settings. Keys: 150 ms or → / Enter opens a submenu; ← / Esc closes one level; ↑ ↓ Home End and letter jump move the selection. Menus flip to stay on the monitor and close on capture loss. They serve right-click, Shift+F10, the Menu key, ⋯ and ⚙/Ctrl+,.
- Scheme cards: Appearance ▸ Color scheme… opens a 3-column grid of 11 cards (112×84). The selected card has a 2 px accent ring and a check badge.
- F11 hides the bar and the sidebar. Moving the mouse into the top 4 px reveals the bar; F11 or Esc exits.
- Badge: Appearance ▸ Word count (`reading.word_count_badge`, default off). Zoom pill: 140×48 for 900 ms, replacing the zoom toast.

## 4. Deviations
- A-7 single CI run: the first full run (39768308649) was red for three reasons:
  - Windows creates the window with WM_NCCALCSIZE wParam=FALSE, so the client gained 31 px and the 7 old goldens failed on size.
  - The zoom timer id clashed with the autosave timer id.
  - The real cursor hovered a card.
  
  The one fix push (feb5c87) fixed all three and committed the 4 goldens from that run's artifact. The fix-push run reproduced them byte-identically. I reviewed them by eye: bar and buttons look correct in both modes, the menu has 14 items, the cards show 11 schemes.
- The top goldens are taken at content 1000×731 (+32 px bar), the size of the first run. `shots.yds` resizes to that before taking them. The cards golden shows a hovered card (Mint) on purpose so the state is deterministic.
- When a driver script runs, popups ignore the real mouse (dev-tools only) so the tests are deterministic.
- The ⚙ popup is the existing Settings submenu shown as a root menu. The settings panel itself is V2-B.
- Snap Layouts, live DPI change, Alt+Space and dark-mode visuals on a real desktop are not CI-testable and are in the owner QA list. `TRACEABILITY.md` was edited for R-WIN-05/06, R-READ-07 and R-THEME-03; `FUTURE.md` is unchanged (it is generated from prompt 005).

## 5. Owner QA list / Sahip QA listesi
1. EN: Hover Min/Max/Close (Close turns red), then hover Max: the Windows 11 Snap Layouts flyout appears. Drag the bar, double-click it, and drag a maximized window down to restore it. TR: Küçült/Büyüt/Kapat üzerine gel (Kapat kırmızı olur), Büyüt'te Snap Layouts açılır. Çubuğu sürükle, çift tıkla, ekranı kaplayan pencereyi aşağı sürükleyerek geri al.
2. EN: Switch Appearance ▸ Mode to Dark (and Windows dark mode under System): the title bar, buttons and frame are dark; an inactive window dims its title. TR: Görünüm ▸ Mod'u Koyu yap (Sistem'de Windows koyu modu): başlık çubuğu, düğmeler ve çerçeve koyu olur; etkin olmayan pencerenin başlığı soluklaşır.
3. EN: Click ⋯ and use the menu with the mouse (submenus open after a short pause), then press Alt or F10 and use ↑↓→←, Enter, Esc and a letter key. Right-click on the bar shows the system menu. TR: ⋯'ye tıkla, menüyü fareyle kullan (alt menüler kısa beklemeyle açılır); sonra Alt veya F10 ile ↑↓→←, Enter, Esc ve harf tuşunu dene. Çubukta sağ tık sistem menüsünü açar.
4. EN: Appearance ▸ Color scheme…: hovering a card previews it, Esc or moving out reverts, a click applies the scheme and it is still there after a restart. TR: Görünüm ▸ Renk şeması…: karta gelince önizlenir, Esc veya dışarı çıkınca geri döner, tıklayınca uygulanır ve yeniden başlatınca kalır.
5. EN: F11: the bar and sidebar disappear; touch the top edge to reveal the bar; F11 or Esc exits. Also try Ctrl+= (zoom pill) and Appearance ▸ Word count (badge at bottom right). TR: F11: çubuk ve kenar çubuğu gizlenir; üst kenara dokununca çubuk görünür; F11 veya Esc ile çık. Ctrl+= (yakınlaştırma göstergesi) ve Görünüm ▸ Sözcük sayısı'nı (sağ alttaki rozet) da dene.

## 6. Next action
V2-B per `docs/ROADMAP_V2.md`. The owner runs the QA list above on a real PC.
