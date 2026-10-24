# V2-G report: content engines (prompt 017)

Status: **not merged — CI red on one driver assert** (stop rule of the user instruction: no further fix push).
PR #87 https://github.com/abuzer1495/YAMDV/pull/87 (branch `phase/v2g-content`). `phase-v2g` **not tagged**.
CI: run 40040071370 (a343a40) red on mermaid view goldens, `content-engines` GIF check, `export-progress` PWS;
fix push 54bbca7 → run 40043352587 red **only** on `export-progress.yds`: `pws_delta_kib <= 205` failed twice
(attempt 1: 492, attempt 2: 256 KiB; V2-F: 196). Every other step is green (goldens, window goldens, ACG, measurements).

## Done / partial / not started
| Item | Status |
|---|---|
| §1.1 F-31, F-32; §2 ADR-0021, ADR-0022, P-48, IDEA-05 (#86) | done |
| §3.1 math (macros, environments; 123/123) | done |
| §3.2 Mermaid (gantt, pie, state, class, ER, mindmap, `%%{init}%%`, copy PNG/source) | done |
| §3.3 SVG (`yamdv-svg`, S-RES-07) | done |
| §3.4 animated GIF, §3.5 ICO / WebP placeholder | done |
| §3.6 lexers (Console, Jsx, Tsx, Html; LESS) | done |
| §3.7 F2 image resize (R-F2-12) | done |
| §4 merge, tag, issues closed | **not started** (CI red) |

## Acceptance criteria
| AC | Status | Evidence |
|---|---|---|
| AC-1 | done | F-31 retry rule, F-32 triggers; ADR-0021/0022; P-48 spec-check green; IDEA-05 #86 |
| AC-2 | done | TRACEABILITY via `tools/trace_set.py` (12 rows); MATH_COVERAGE 123/123; CODE_LANGUAGES 42 lexers |
| AC-3 | done | `content-engines`, `image-resize` PASS on CI; goldens v2g-a/b/c committed (self-reviewed); unit tests; fuzz `svg_parse`; artifact `yamdv-exe` |
| AC-4 | **open** | gates and size green, but the CI run is red (export-progress PWS) → not merged, not tagged |

## Gates (run 40043352587; run 1 in brackets)
| Gate | V2-F | V2-G | Limit | |
|---|---|---|---|---|
| M0 private WS | 2.35 MiB | 2.39 (2.37) | ≤ 2.65 | OK |
| M1 private WS | 4.72 MiB | 4.63 (4.67) | ≤ 5.02 | OK |
| T4 typical cold / warm p99 | 13.8 / 9.4 ms | 13.3 / 9.3 (14.5 / 10.4) | gate | OK |
| T5 F3 split p99 | 14.0 | 13.0 (14.3) ms | ≤ 25 | OK |
| T5 F2 live p99 typical / 1 MB | 24.8 / 18.7 | 23.0 / 19.4 (**25.9** / 20.3) ms | ≤ 40 (stretch 25) | OK; run 1 missed the stretch |
| T7 idle CPU (empty, doc) | 0 / 0 | 0 / 0 µs | 0 | OK |
| M3 math / mermaid-heavy (report) | — | 2.73 / 3.07 MiB | — | report |
| PWS with 5 GIFs (report) | — | not measured as its own number (`content-engines` T7 CPU asserts pass) | — | report |
| Exe MSVC release | 3,386,368 B | 3,599,872 B (**+213,504 B**) | ≤ +250 KB | OK |

Size shares (hardened lld map, 3,381,760 → 3,626,496 B, +244,736 B; `docs/reports/v2g_sizemap.md`):
Mermaid +37.3 KB · SVG +36.5 KB (≤ 120) · math +22.6 KB · GIF/ICO +17.3 KB, WebP 0 (≤ 80) · lexers +3.3 KB net ·
F2 resize +7.2 KB plus the handles inside `window` · the rest is UI wiring, std/alloc, constants and the CRT.
Budget fix: `opt-level = "z"` for yamdv-mermaid/svg/math/highlight/export (before that: +266,240 B).

## ADR decisions
- ADR-0021: a pure crate `yamdv-svg` (bounded XML, style, path, render to D2D paths, S-RES-07 limits); resvg (+561 KB) rejected.
  Text runs go through DirectWrite; `currentColor` = UI text color; the fuzz target is `svg_parse`.
- ADR-0022: ICO uses our own directory parser and picks the best-fit entry; the PNG/BMP payloads go through WIC (ACG matrix rows).
  WebP: `image-webp` measured at +138 KB, over the 80 KB budget, so the "WebP codec not installed" placeholder stays.

Math coverage: **123/123** MUST items (docs/MATH_COVERAGE.md). Lexers: see docs/CODE_LANGUAGES.md (42, none unmapped).

## Deviations
- **Red CI:** export-progress PWS after cancelling the 1 MB PDF export is 256–492 KiB (limit 205). The fix push made the
  driver's full-document `math_errors` probe lazy; that did not cure it. Cause not found: Wine reports PWS as 0, so it
  can't be measured locally.
- A math macro applies to its whole block. Gantt has no today marker in goldens (only with a clock). Two items for the state/class/gantt list: the gantt milestone label may overflow the card edge; ER uses a basic notation.
- SVG: group opacity is applied per shape; text has no shaping or fallback; unsupported features are skipped; ≤ 200k draws; a hostile SVG is drawn blank.
- GIF: loops forever with no catch-up, is decoded on the UI thread, and is dropped outside ±1 screen. It respects reduced motion (CI uses the dev-tools `ui motion-on`).
- ICO intrinsic size = the largest entry. The image store keeps 2 decoded sizes per image. Resize drops the height; reference images can't be resized.
- Lexer limits are in docs/CODE_LANGUAGES.md. The `math_errors` probe is font-free. The mermaid view goldens were updated (pie now renders).

**Next action:** find the PWS growth in export-progress (measure on Windows: V2-F vs V2-G, after cancelling the 1 MB PDF
job: print Viewer math/macro caches, highlight `State` 2→8 B, heap retention). Fix it in a new CI run (it needs the owner's
OK for a run beyond the budget), then merge #87 (closes #19 #20 #21 #46 #8 #25) and tag `phase-v2g`.

## Owner checklist / Sahip kontrol listesi
1. EN: Open a doc with a real ```` ```mermaid gantt ```` (sections, `after`, `excludes weekends`, a milestone). TR: Gerçek bir gantt (bölümler, `after`, `excludes weekends`, kilometre taşı) içeren belge açın.
2. EN: Add pie (`showData`), stateDiagram, classDiagram and erDiagram blocks; right-click → Copy as PNG, then paste in Paint. TR: pie, state, class ve ER blokları ekleyin; sağ tık → PNG olarak kopyala, Paint'e yapıştırın.
3. EN: Put `%%{init: {"theme": "dark"}}%%` on a flowchart: the theme changes for that diagram only. TR: Bir akışa `%%{init: {"theme": "dark"}}%%` ekleyin: yalnızca o diyagramın teması değişir.
4. EN: `$\newcommand{\RR}{\mathbb R}$`, then `$$A \in \RR^{2\times2}$$` with `pmatrix`/`cases`; a `\def` bomb shows red. TR: `\newcommand` ile makro tanımlayıp `pmatrix`/`cases` içeren formülde kullanın; `\def` bombası kırmızı görünür.
5. EN: `![logo](logo.svg)` with a real SVG logo: it is sharp at 200 % zoom and in the image viewer. TR: Gerçek bir SVG logo ekleyin: %200 yakınlaştırmada ve görüntüleyicide net görünür.
6. EN: `![](app.ico)` renders the best size; `![](photo.webp)` shows "WebP codec not installed". TR: `.ico` en uygun boyutta görünür; `.webp` "WebP codec not installed" gösterir.
7. EN: An animated GIF is still by default; Settings → Reading → Animate GIFs on: it plays, and stops when scrolled away. TR: Hareketli GIF varsayılan olarak durağandır; Ayarlar → Okuma → GIF'leri canlandır açın: oynar, kaydırınca durur.
8. EN: F2, click an image, drag a corner handle (the "W px" label shows), release; Ctrl+Z restores; Esc cancels. TR: F2'de resme tıklayıp köşe tutamağını sürükleyin ("W px" görünür), bırakın; Ctrl+Z geri alır; Esc iptal eder.
