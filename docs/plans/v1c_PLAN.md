# Phase V1-C plan — Math core + Mermaid flowchart/sequence (prompt 007)

Goal: replace the last two reading-view placeholders with native renderings. `$…$`, `$$…$$` and ```` ```math ```` become real formulas laid out from the OpenType `MATH` table of Cambria Math; ```` ```mermaid ```` flowcharts and sequence diagrams become vector diagrams. Pure Rust, no JS, no network. Scope fence: FUT-17 (math core) and FUT-18 (flowchart + sequence). Mode A-4.

Design (simplest):
- `yamdv-math` (pure): `MathFont` trait (cmap glyph ids, design metrics, raw `MATH` bytes) → `table` (MathConstants, italics/top-accent, variants + assemblies; tolerant of missing/garbage data) → `parse` (bounded tokenizer/parser → AST, S-PARSE-05) → `layout` (TeX box model, 4 styles + cramped, atom spacing table) → `MathBox { width, ascent, descent, ops: [Glyph | Rule] }`.
- Platform: lazy Cambria Math `IDWriteFontFace` (`TryGetFontTable('MATH')`, `GetGlyphIndices`, `GetDesignGlyphMetrics`); `Band::math` draws glyphs with `IDWriteBitmapRenderTarget::DrawGlyphRun` and rules with tiny-skia.
- Layout glue (`yamdv-layout/src/rich.rs`): `RichHost` (math font, settings, cache, "render anyway") in `Env`/`Ctx`; display math block centered (margin 16 0) with an `HRegion` when too wide; inline math as an inline object (`ParaSpec::objects` → `IDWriteInlineObject` with the formula's width/ascent/descent; the object range is atomic, copy maps to the source with delimiters); errors in `error` color + message in `text_muted`.
- `yamdv-mermaid` (pure): bounded parsers (flowchart/graph, sequenceDiagram) with line/column errors; Sugiyama layout (DFS cycle breaking, longest path + dummies, 4 barycenter sweeps, median alignment + compaction 40/50, direction transform); sequence layout; output = diagram ops (paths, shapes, texts) in diagram px with palette roles. Text sizes via a `Measure` trait (implemented over `TextShaper`).
- Rendering: card per B.6.12, B.3.6 palettes, scale down to fit; new `Item::Path` (cubic Bézier, dashed, thick, fill) + `Item::Math`; limits (S-PARSE-06 soft/hard + [Render anyway]), 200 ms budget, per-source-hash cache.
- §4: `devtools` API `render_block_png(block, scale)` reusing the `--render-png` band pipeline.
- Tests: unit tests (mock metrics), layout invariants, fuzz `math` + `mermaid` (5 min each, background), corpus `math.md`, `math-heavy.md`, `mermaid.md`, `mermaid-heavy.md`, 4 goldens, M3/T1M/T4 rows in `measure.ps1`.
- Push policy (owner rule): push after every compiling change, never > 10 min unpushed; WIP pushes carry `[skip ci]`; CI once at the end + one fix (A-4 rule 6).

Risks: no Windows GUI locally (visual check of math/mermaid layout with a throwaway local renderer under `target/`, real check via the CI goldens); Cambria Math glyph metrics only on Windows; T4 with many paths per band.

## CHECKPOINT
- [x] V1-B merged (d6ecab1), tag phase-v1b
- [x] prompt stored `docs/prompts/007_V1C_MATH_MERMAID.md` (SHA-256 94b18de380577e7dcff0a3c89f3d05e85605ba49bbffb4749850bea39617e784)
- [ ] yamdv-math: table, parser, layout, tests
- [ ] platform: Cambria Math face + glyph drawing; layout glue (display + inline + errors)
- [ ] yamdv-mermaid: parsers, flowchart layout, sequence layout, tests
- [ ] mermaid rendering (card, palette, limits, cache)
- [ ] §4 block PNG API
- [ ] fuzz targets math/mermaid (5 min each)
- [ ] corpus + goldens + measure.ps1 rows
- [ ] local gates, CI run, report, PR, merge, tag
