# V2-G report: content engines (prompt 017)

Status: **merged and tagged.** PR #87 squash-merged as `a66c426` (tree equals green head `a26d927`, F-24 OK); tag
`phase-v2g` on `a66c426` started no workflow (F-32). #19 closed by the merge; #20 #21 #46 #8 #25 are still open (the PR
body listed them on one `Closes` line, so GitHub linked only the first one) and need a manual close.
CI: 40040071370 (a343a40) red → 40043352587 (54bbca7) red only on export-progress PWS (492 / 256 KiB). **Owner-approved budget extension (2026-10-24):** "Ek CI çalışmalarına izin veriyorum, bellek artışını düzelt sonra
birleştir" (extra runs allowed to fix the memory growth, then merge). 2 of the ≤ 4 extra runs were used: 40069253365 (adcf06b,
temporary diagnostic) and **40070840370 (a26d927) green**. 40070817248 was cancelled after 23 s (a stray file was committed).

## Export-progress PWS: cause and fix
- Diagnostic (removed before merge): dev-tools counting allocator + `ui mem-report` around the cancel. After cancelling the
  1 MB PDF job: Rust live heap 2,766 → 2,768 KiB, process heap commit unchanged (6,060 KiB), GDI/USER unchanged. Real PWS
  growth was 6,288 → 6,440 KiB (+152). Nothing is retained (the print job runs in a worker process). PWS with 5 GIFs: not measured separately.
- **Cause:** the driver measured PWS **after building its own state**. `assert pws_delta_kib` built the full driver
  state (≈ 60 fields, some of them scan the whole 1 MB document) and only then read PWS. On a 1 MB document that build alone
  commits about 190–250 KiB of transient heap. An idle control with no export showed +248 KiB from the assert alone.
  V2-G added more state fields, which pushed the reading over 205.
- **Fix:** `pws_delta_kib` is sampled first, before the state is built (as `math_errors` is). The limit stays 205.
  Result in run 40070840370: **100 KiB** on the first attempt (V2-F: 196), with no F-31 retry.

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
| §4 merge, tag | done (`a66c426`, `phase-v2g`); issues #20 #21 #46 #8 #25 need a manual close |

## Acceptance criteria
| AC | Status | Evidence |
|---|---|---|
| AC-1 | done | F-31 retry rule, F-32 triggers; ADR-0021/0022; P-48 spec-check green; IDEA-05 #86 |
| AC-2 | done | TRACEABILITY via `tools/trace_set.py` (12 rows); MATH_COVERAGE 123/123; CODE_LANGUAGES 42 lexers |
| AC-3 | done | `content-engines`, `image-resize` PASS on CI; goldens v2g-a/b/c committed (self-reviewed); unit tests; fuzz `svg_parse`; artifact `yamdv-exe` |
| AC-4 | done | run 40070840370 green; gates below; merged, tagged |

## Gates (final run 40070840370; run 40043352587 in brackets)
| Gate | V2-F | V2-G | Limit | |
|---|---|---|---|---|
| M0 private WS | 2.35 MiB | 2.40 (2.39) | ≤ 2.65 | OK |
| M1 private WS | 4.72 MiB | 4.71 (4.63) | ≤ 5.02 | OK |
| T4 typical cold / warm p99 | 13.8 / 9.4 ms | 13.4 / 8.7 (13.3 / 9.3) | gate | OK |
| T5 F3 split p99 | 14.0 | 12.9 (13.0) ms | ≤ 25 | OK |
| T5 F2 live p99 typical / 1 MB | 24.8 / 18.7 | 22.6 / 19.5 (23.0 / 19.4) ms | ≤ 40 (stretch 25) | OK; run 40040071370 had 25.9 (missed the stretch) |
| T7 idle CPU (empty, doc) | 0 / 0 | 0 / 0 µs | 0 | OK |
| M3 math / mermaid-heavy (report) | — | 2.74 / 3.10 MiB | — | report |
| export-progress `pws_delta_kib` | 196 | **100** (256 / 492) | ≤ 205 | OK |
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

## Deviations
- A math macro applies to its whole block. Gantt has no today marker in goldens (only with a clock). Two items for the state/class/gantt list: the gantt milestone label may overflow the card edge; ER uses a basic notation.
- SVG: group opacity is applied per shape; text has no shaping or fallback; unsupported features are skipped; ≤ 200k draws; a hostile SVG is drawn blank.
- GIF: loops forever with no catch-up, is decoded on the UI thread, and is dropped outside ±1 screen. It respects reduced motion (CI uses the dev-tools `ui motion-on`).
- ICO intrinsic size = the largest entry. The image store keeps 2 decoded sizes per image. Resize drops the height; reference images can't be resized.
- Math coverage 123/123 MUST items (docs/MATH_COVERAGE.md); lexer limits in docs/CODE_LANGUAGES.md (42, none unmapped). The `math_errors` probe is font-free. The mermaid view goldens were updated (pie now renders).

## Owner checklist / Sahip kontrol listesi
1. EN: Open a doc with a real ```` ```mermaid gantt ```` (sections, `after`, `excludes weekends`, a milestone). TR: Gerçek bir gantt (bölümler, `after`, `excludes weekends`, kilometre taşı) içeren belge açın.
2. EN: Add pie (`showData`), stateDiagram, classDiagram and erDiagram blocks; right-click → Copy as PNG, then paste in Paint. TR: pie, state, class ve ER blokları ekleyin; sağ tık → PNG olarak kopyala, Paint'e yapıştırın.
3. EN: Put `%%{init: {"theme": "dark"}}%%` on a flowchart: the theme changes for that diagram only. TR: Bir akışa `%%{init: {"theme": "dark"}}%%` ekleyin: yalnızca o diyagramın teması değişir.
4. EN: `$\newcommand{\RR}{\mathbb R}$`, then `$$A \in \RR^{2\times2}$$` with `pmatrix`/`cases`; a `\def` bomb shows red. TR: `\newcommand` ile makro tanımlayıp `pmatrix`/`cases` içeren formülde kullanın; `\def` bombası kırmızı görünür.
5. EN: `![logo](logo.svg)` with a real SVG logo: it is sharp at 200 % zoom and in the image viewer. TR: Gerçek bir SVG logo ekleyin: %200 yakınlaştırmada ve görüntüleyicide net görünür.
6. EN: `![](app.ico)` renders the best size; `![](photo.webp)` shows "WebP codec not installed". TR: `.ico` en uygun boyutta görünür; `.webp` "WebP codec not installed" gösterir.
7. EN: An animated GIF is still by default; Settings → Reading → Animate GIFs on: it plays, and stops when scrolled away. TR: Hareketli GIF varsayılan olarak durağandır; Ayarlar → Okuma → GIF'leri canlandır açın: oynar, kaydırınca durur.
8. EN: F2, click an image, drag a corner handle (the "W px" label shows), release; Ctrl+Z restores; Esc cancels. TR: F2'de resme tıklayıp köşe tutamağını sürükleyin ("W px" görünür), bırakın; Ctrl+Z geri alır; Esc iptal eder.
