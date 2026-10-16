# Phase 002 Plan — Core document model & reading MVP

## 1. Goals (restated)
1. Fix items F-01…F-08 from the Phase 001 review come first:
   - byte-exact prompt copies
   - PROCESS.md generated from prompt 001
   - verified appendices (`spec-check`)
   - the missing 001 report sections
   - owner kit
   - the DynamicCode policy decision
   - MiB units
   - `memprobe --size`
2. Spec patches P-01…P-09 are applied by script and are verifiable (`spec-patch` / `spec-check`).
3. A read-only Markdown viewer for the CommonMark core:
   - open via CLI, Ctrl+O or drag & drop
   - decode, parse into a two-level model (block index + lazy detail trees)
   - lazy layout with backend A (ADR-0002)
   - virtualized smooth scrolling with an overlay scrollbar
   - selection and plain-text copy
   - Phase 003 constructs shown as deliberate placeholders

## 2. Host strategy (unchanged from 001)
- Linux host. Pure crates are built and tested natively.
- Windows code is cross-checked with cargo-xwin.
- Every GUI, golden image and memprobe check runs on GitHub Actions `windows-latest`. Results come back as artifacts and the goldens are committed after review.
- Owner-only checks (real GPU, 150 % monitor, IME, light/dark switch, file dialog with shell extensions) go into the owner kit.

## 3. Task breakdown
| # | Task | IDs | Verification |
|---|---|---|---|
| 1 | F-01/F-02/F-03, spec patches, SPEC_CHANGES | F-01…03, P-01…09 | `cargo xtask spec-check` |
| 2 | F-07/F-08: MiB units, `memprobe --size`, monitor and client size in JSON | F-07, F-08 | unit tests, CI JSON |
| 3 | yamdv-core: INI parser/writer + Settings (all B.8.2 keys), portable path rule | S-FILE-07, R-SET-05 | unit tests, fuzz `ini` |
| 4 | yamdv-core: i18n EN/TR, logger, theme tokens (11 × 2) + contrast test | R-I18N-01/02/04, S-PRIV-02, R-A11Y-04, R-THEME-02 | unit tests |
| 5 | yamdv-core: path validation, guards, decoding, line endings | S-FILE-04, R-OPEN-06/07/08/10/12, S-PARSE-02/08 | unit tests + encoding fixtures |
| 6 | yamdv-core: block index, maps, slugs, lazy detail trees, LRU, depth cap, source mapping, splice, text extraction | P-07, R-MD-01/10/15/17, S-PARSE-01 | unit tests, ≥150 own CommonMark vectors |
| 7 | yamdv-layout: TextShaper trait + mock, block layout → display list, estimates, Fenwick, anchoring, selection → text, plain-text mode | R-REN-01/02/05/09/16/19/21 | unit tests with the mock shaper |
| 8 | platform: DirectWrite shaper, renderer A (from spike), window states, scrolling, scrollbar, selection, clipboard, open dialog, drag & drop, confirm dialog, multi-window | R-REN-17/20, R-OPEN-02/03/04 | CI goldens, memprobe, owner kit |
| 9 | `--render-png` (dev-tools), `xtask golden`, goldens | H.4 | CI artifact → committed |
| 10 | Fuzz targets `parse_layout`, `ini` (≥10 min each) | S-TEST-01 | local nightly run log |
| 11 | Measurements M0/M1b/M2/M6/M7/T1b/T1L/T2/T4/T7/B-SIZE with a soft-budget CI gate | D.2 | CI |
| 12 | Owner kit job + OWNER_KIT.md EN/TR | F-05 | artifact on `main` |
| 13 | ADR-0007 document model, ADR-0001 amendment (F-06), dependency ADRs | H5 | docs |
| 14 | 001 report amendment (F-04), 002 report, PR, merge, tag | F-04, AC-11 | PR |

## 4. Top risks
| Risk | Retirement |
|---|---|
| Phase is very large for one increment | Build the pure model first (fully testable on Linux), then a thin Windows integration; report anything incomplete as PARTIAL (H10/H15) |
| No local Windows GUI | CI goldens and memprobe are the bench; the owner kit covers the rest |
| Layout and source-mapping correctness | Mock-shaper tests; detail trees checked against pulldown-cmark events |
| Memory: the source text dominates M2 | Measure with the `mem-stats` breakdown; LRU budgets |
| DynamicCode policy breaks DWrite or shell extensions | Enable it behind measurement on CI; if it is unverifiable, keep it off and add an owner-kit step |

## 5. Test plan
- Unit tests for every module listed in §4.8 of the prompt.
- Own CommonMark vectors (≥150).
- A one-off local run of the official spec examples, not committed.
- Fuzzing ≥10 min per target.
- Goldens at 1.0/1.5, light/dark.
- CI autoscroll anchor run.

## 6. Measurement plan
- `memprobe --size 1000x700` on CI for M0, M1b, M2 and M6, plus timings T1b/T1L/T2, T4 (autoscroll), T7 (CPU time during 30 s idle) and B-SIZE.
- All values are reported in MiB and bytes.
- The soft-budget gate fails CI.
