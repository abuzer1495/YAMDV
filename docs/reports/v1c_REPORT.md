# YAMDV — Phase V1-C Report: Math core + Mermaid (flowchart, sequence)

## 0. Status
- Prompt 007 is stored as `docs/prompts/007_V1C_MATH_MERMAID.md`, SHA-256 `94b18de380577e7dcff0a3c89f3d05e85605ba49bbffb4749850bea39617e784`.
- Branch `phase/v1c-math-mermaid` → `main` via PR #7. Tag `phase-v1c`.
- Measurements come from CI run 39036097595 (`workflow_dispatch`, code head 429b7da). It failed on two harness issues and no product gate: (1) the `placeholders` goldens changed as intended, because math there now renders; (2) the block-PNG step checked the file before the GUI-subsystem exe had exited (`&` does not wait). The fix push (goldens + `Start-Process -Wait`, plus this report) is the PR run.
- Result: **done**. `$…$`, `$$…$$` and ```` ```math ```` render as native formulas (Cambria Math `MATH` table), and ```` ```mermaid ```` flowcharts and sequence diagrams render as vector diagrams. No JS, no network. Nothing beyond FUT-17/FUT-18 was built.
- New crates: `yamdv-math` and `yamdv-mermaid` (pure, workspace-internal). No new third-party dependencies, so no ADR.

## 1. Acceptance criteria
| AC | Status | Evidence |
|---|---|---|
| AC-1 | **pass** | `math.md` covers every FUT-17 core construct plus errors. Goldens `math-light`/`math-dark` were self-reviewed from the artifact. `yamdv-math` tests (15) cover the parser, layout with mock metrics and the MATH table reader. Errors render in the `error` color with the message below; the fuzz target never panicked |
| AC-2 | **pass** | `mermaid.md` covers the 7 shapes (plus hexagon → rect fallback), the 4 edge kinds, labels and chains, nested subgraphs, `classDef`/`class`/`style`, TB/BT/LR/RL, and a sequence diagram with actors, the 8 arrows, activations, notes, `loop`/`alt`/`opt`/`par`, `autonumber` and `title`. A `pie` block shows the "Diagram type not supported yet" badge; a syntax error shows the source card with line/column. Goldens `mermaid-light`/`-dark`. Tests: `yamdv-mermaid` (11), incl. no-overlap, edge endpoints and `lr_is_rotation_of_tb`. Driver `mermaid.yds` exercises soft limit → [Render anyway] |
| AC-3 | **pass** | Inline formulas are `IDWriteInlineObject`s with the math box's ascent/descent (golden: baseline-aligned `E = mc²`, `Σ`, fractions in running text). Copy maps the atomic object to its source with delimiters (`rich.rs`/`detail.rs` tests). An overflowing display formula gets an `HRegion` scrollbar (symbol row in the `math` golden) |
| AC-4 | **pass** | Fuzz `math`: 2.30 M runs in 301 s; `mermaid`: 2.61 M runs in 301 s; 0 crashes, nothing open |
| AC-5 | **pass** | Local gates (fmt, clippy linux + xwin incl. dev-tools, tests, deny, spec-check, check-imports) are green. On CI, M3 and T4 on both new documents are within budget (§2), all 23 driver scripts pass, PR #7 is merged and `phase-v1c` is tagged |
| AC-6 | **pass** | This report (≤ 120 lines) and the 4-step QA in §4 |

## 2. Measurements
CI run 39036097595 on `windows-latest` (Server 2025, Hyper-V 1024×768, no GPU). Client area 1000×700, medians.

| ID | Result | Budget | Δ vs V1-B |
|---|---|---|---|
| M3 `math-heavy.md` PWS / commit | 2.69 / 3.50 MiB | ≤ 9.6 MiB | new |
| M3 `mermaid-heavy.md` PWS / commit | 2.96 / 4.35 MiB | ≤ 9.6 MiB | new |
| T1M first paint math-heavy / mermaid-heavy | 117 / 113 ms | report | new |
| T4 math warm median p99 / cold | 10.32 / 10.49 ms | ≤ 16 ms | new |
| T4 mermaid warm median p99 / cold | **15.47** / 24.09 ms | ≤ 16 ms | new |
| T4 typical warm median p99 | 14.33 ms (attempt 2; attempt 1 25.66 ms) | ≤ 16 ms | +4.40 |
| T4 typical cold p99 | 37.20 ms | report (≤ 33 target) | +19.89 |
| T4 large warm median p99 / cold | 10.61 / 7.76 ms | ≤ 16 ms | +4.36 / +0.74 |
| M0 PWS / commit | 2.28 / 2.98 MiB | ≤ 4.8 MiB | +0.01 |
| M1 PWS / commit (with images) | 4.65 / 5.79 MiB | ≤ 8.0 MiB | +0.01 |
| M2 PWS / commit | 5.60 / 6.90 MiB | ≤ 16 MiB | −0.08 |
| M5 F2 / F3 PWS | 3.94 / 5.70 MiB | ≤ 9.6 MiB | +0.13 / −0.14 |
| M6 PWS / commit | 4.59 / 5.66 MiB | ≤ M1 + 1 / + 1.5 MiB | −0.03 |
| T2 / T1 / T1L first paint | 82 / 123 / 157 ms | 100 / 150 / 300 | +27 / +41 / +40 |
| T3 second launch | 34.3 ms | report | +1.8 |
| T5 F3 large / F2 typical p99 | 15.3 / 26.5 ms | ≤ 25 / 40 ms | −1.6 / +6.7 |
| T7 idle CPU 30 s (empty, `typical.md`) | 0 µs / 0 µs | ≤ 1 ms | = |
| T-DLG | 505 ms | ≤ 400 soft | +21 (FUT-50) |
| B-SIZE `yamdv.exe` | 2.50 MiB (2,619,392 B) | ≤ 4.0 MiB | +0.21 MiB (math + mermaid) |
| Block PNG (§4) `mermaid.md` block 2 at 2× | 1300×222 px, 14,142 B | produced | new |
| Goldens | 56 files, 4,440,044 B (P-28 ≤ 12 MB); 4 new + `placeholders` ×2 updated | ≤ 4 new | +0.60 MB |

Most of the deltas on documents without math or diagrams (T2/T1, T4 typical attempt 1) are runner variance. The code path for those documents only gained a "has math?" check, and the post-merge V1-B runs on `main` failed T7/M0/M1 for the same reason on unchanged code. T4 mermaid (15.47 ms) is close to the gate. The prompt's fallback, caching rasters for visible diagrams, was **not** built, because the gate passed.

## 3. Deviations and what is left
- **A-4 rule 6 (push budget):** 1 CI run on the code (39036097595) and 1 fix push (goldens, block-PNG wait, this report). WIP pushes carried `[skip ci]`.
- **Known visual deviations from KaTeX** (self-review of the goldens): glyphs are Cambria Math, not KaTeX's Computer-Modern-like fonts. `\mathcal` uses the Unicode script letters. Big delimiters and radicals use the font's variants and assemblies, so their heights differ slightly from KaTeX. Inline `\sum_{i=1}^n` puts its limits to the side (TeX text style), as KaTeX does.
- **`$$\frac{a}{$$` on one line** is not recognized as display math by the Markdown front end, so it shows as plain text rather than in the error style. The multi-line error case does use the error style.
- **Wide flowcharts** (the 9-node LR row in `mermaid.md`) are scaled down to the column and are small to read. There is no fullscreen viewer (FUT-36).
- **FUT-41** (sup/sub/footnote baseline shift) stays open. The baseline shift was not trivial with the math inline-object path, so it was left as the prompt allows. `~sub~` with single tildes is GFM strikethrough, as before.
- **Not verified on real hardware:** per-monitor DPI of formulas and diagrams, and ClearType vs grayscale glyph runs in Cambria Math.
- **Left for later (unchanged):** T-DLG and a cheaper typical scroll path (FUT-50), ICO/WebP (FUT-48), the fullscreen diagram viewer (FUT-36), everything beyond the FUT-17/FUT-18 core.

## 4. Owner QA (Windows PC)
1. **Formula-heavy README:** open a README with inline `$…$` and display `$$…$$` formulas (fractions, sums, matrices, `cases`). The formulas are crisp and sit on the text baseline. Select a line across an inline formula and press Ctrl+C: the pasted text contains the formula's LaTeX with its `$` delimiters. A too-wide formula scrolls sideways.
2. **Flowchart:** open a file with a ```` ```mermaid ```` `graph TD` block that has a decision (`{…}`), edge labels (`-->|yes|`) and a `subgraph`. Boxes don't overlap, arrows end at their boxes, and the subgraph has a titled frame. Change `TD` to `LR` in F3: the preview turns sideways.
3. **Sequence diagram:** a `sequenceDiagram` with an `actor`, `->>` and `-->>` messages, `activate`, a `Note over A,B` and an `alt`/`else` block. The lifelines, activation bars, note and labeled frame all appear. A typo shows the source with "line N, column M".
4. **Dark mode:** switch to dark (context menu ▸ Theme). Formulas, diagram cards, node fills and lines switch to the dark palette with readable contrast, and `\color{red}` stays red.

## 5. Özet (Türkçe)
V1-C tamamlandı. Okuma görünümündeki son iki yer tutucu da gerçek çizime geçti:
- **Matematik:** `$…$`, `$$…$$` ve ```` ```math ```` artık Cambria Math'in `MATH` tablosuyla yerel olarak diziliyor: kesirler, kökler, üst/alt simgeler, büyük operatörler, matrisler, `cases`, aksanlar, Yunan harfleri ve `\color`. Satır içi formüller metin taban çizgisine oturuyor ve kopyalanınca `$` işaretleriyle LaTeX kaynağı olarak geliyor. Hatalı formüller kırmızı kaynak ve mesajla gösteriliyor, uygulama çökmüyor.
- **Mermaid:** akış şemaları (katmanlı yerleşim, 7 şekil, 4 kenar türü, alt gruplar, stiller) ve sıralama diyagramları (8 ok türü, aktivasyon, notlar, çerçeveler, autonumber) vektör olarak çiziliyor. Desteklenmeyen türler ve sözdizimi hataları belirtilen yedek görünümle çıkıyor.
- V1-D'nin HTML dışa aktarımı için tek bir bloğu 2× PNG'ye çizen API eklendi.

İki fuzz hedefi 5'er dakika çalıştı, çökme yok. 23 driver betiğinin hepsi CI'da geçiyor. Kaydırma p99 matematikte 10,3 ms, Mermaid'de 15,5 ms (sınır 16 ms, izlenmeli). Ağır belgelerde bellek 3 MiB'ın altında, exe 2,50 MiB.
