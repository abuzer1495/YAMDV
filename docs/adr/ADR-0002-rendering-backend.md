# ADR-0002: Rendering backend

- Status: **accepted**. Candidate **A** is chosen: DirectWrite → GDI interop on a banded DIB, with tiny-skia for shapes and images.
- Data: CI run 38911412595 on GitHub Actions `windows-latest`. Raw files are in `docs/spike/run-38911412595/`.

## Context
§4.7 asks for three spikes that render the same reference scene. The scene is repeated 200 times and contains:
- a heading and a mixed-script paragraph (Turkish, CJK, Arabic, color emoji, bold, italic, a link color, a selection)
- a code block with four token colors
- a 4×3 table
- a circle, a line, an arrowhead, a Bézier curve and a quote bar
- a generated PNG decoded through WIC

Decision rule: pick the lowest steady private working set (PWS) that renders everything correctly with scroll p99 ≤ 16 ms. On a tie, pick the smaller exe.

## Candidates
- **A**: `IDWriteBitmapRenderTarget` with a 128-px band and a custom `IDWriteTextRenderer`. Color glyphs go through `IDWriteFactory4::TranslateColorGlyphRun`. tiny-skia draws on the same DIB bits. Output reaches the window with `BitBlt` and scrolls with `ScrollWindowEx`.
- **B**: `ID2D1DCRenderTarget` with `D2D1_RENDER_TARGET_TYPE_SOFTWARE`, banded into a DIB, `BitBlt` and `ScrollWindowEx`.
- **C**: D3D11 device with `ID2D1DeviceContext` and a `FLIP_DISCARD` swap chain. It redraws the full frame and calls `Present(0)`.
- **D** (optional): not built. A met every criterion, so a fourth candidate would not change the decision (H8).

## Measurements (window 1100×760, default heap, median of 3 runs)
| | A | B | C |
|---|---|---|---|
| Idle PWS after first paint (memprobe, QueryWorkingSet) | **2.39 MB** | 11.71 MB | 11.72 MB |
| PWS after scrolling to the end | **2.61 MB** | 9.21 MB | 12.39 MB |
| PWS at simulated 150 % (1650×1140) | **3.01 MB** | 12.15 MB | 11.01 MB |
| Working set (incl. shared) | 21.0 MB | 37.7 MB | 37.3 MB |
| Handles / GDI / modules | 169 / 11 / 32 | 213 / 13 / 41 | 227 / 9 / 44 |
| First frame since process creation | 98 ms | 157 ms | 152 ms |
| Scroll, 3300 steps of 40 px: avg / p99 / max | 1.48 / **4.16** / 13.95 ms | 2.89 / 5.08 / 7.66 ms | 17.17 / 31.73 / 35.64 ms |
| Frames over 16 ms | 0 | 0 | 1136 |
| Release exe size | 602,112 B | 309,760 B | 311,296 B |
| Correct rendering (screenshots 100 % / 150 %) | yes¹ | yes | yes |
| Clean exit after WM_CLOSE | yes | yes | yes |

MB means 10⁶ bytes. Other costs:
- WIC init: about 3.5 ms and +60 KB PWS. Decoding and scaling the 400×300 image takes about 5 ms.
- DWrite factory: about +0.4 MB.
- In B and C most of the cost comes at the first D2D draw: from about 2.9 MB at "backend ready" to about 10 MB.

¹ The first CI run showed a bug in A: link and code colors were missing because DirectWrite does not split runs at color ranges by itself. A marker drawing effect fixed it (`fix(spikes)` commit). The table above is from the run after that fix.

### SegmentHeap variant (manifest `heapType`)
| | A | B | C | skeleton |
|---|---|---|---|---|
| Idle PWS, default → SegmentHeap | 2.39 → 2.57 MB | 11.71 → 9.83 MB | 11.72 → 10.01 MB | 1.33 → 1.36 MB |

SegmentHeap makes the chosen backend and the skeleton slightly worse, and it conflicts with Appendix D §D.4 rule 8. **Decision: keep the default heap.**

## Decision
**A.**
- It has the lowest PWS by about 4.5×. That fits the M0/M1 budgets with headroom.
- p99 is 4.2 ms, well under 16 ms, and no frame went over 16 ms.
- It renders everything correctly, including color emoji, fallback and the selection.
- It has the fastest first frame.

Its weak point is exe size: +290 KB, which is tiny-skia plus the custom renderer. That matters only for ties, and the PWS difference is large.

## Caveats
- The runner is a Hyper-V VM with "Microsoft Hyper-V Video" and no real GPU. `d3d.driver=hardware` is the basic render adapter.
  - C's timing (~16.7 ms) looks like it is throttled to vsync by `Present`, so it is not a fair GPU result.
  - C's PWS also includes D3D allocations.
  - Re-measuring on real hardware was **NOT VERIFIED**. It would not change the choice, because A already wins on PWS.
- 150 % is simulated with `--scale 1.5` at 96 DPI. A real 150 % DPI monitor was **NOT VERIFIED**.
- Scroll time is measured on the CPU (ScrollWindowEx + UpdateWindow, or draw + Present). DWM composition latency is not included.
- GUI checks on a local machine are NOT VERIFIED (the host is Linux, §4.1).

## Consequences
- The product renders on the CPU into DIB bands. Phase 002 builds the layout/paint layer on DirectWrite GDI interop plus tiny-skia, which ADR-0006 already covers.
- Smooth scrolling uses `ScrollWindowEx` and repaints only the exposed strip.
- Spikes stay in `spikes/` as reference and are not shipped.
- Spikes use their own lint table (unsafe allowed without SAFETY comments). This resolves the E.9 vs H14 tension for throwaway code only.

## Revisit
If later phases (math, Mermaid, large images) push p99 over 16 ms, or real-GPU measurements show B or C dramatically ahead.
