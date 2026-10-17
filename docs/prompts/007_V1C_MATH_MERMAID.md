# 007_V1C_MATH_MERMAID — YAMDV Phase V1-C: Math core + Mermaid (flowchart, sequence)

> **Prompt 007.** Phase **V1-C**. Branch `phase/v1c-math-mermaid`, tag `phase-v1c`, report `docs/reports/v1c_REPORT.md` (A-4: ≤ 120 lines). Store this file as received in `docs/prompts/`.
> **Precedence:** `docs/PROCESS.md` (A-4 delivery mode) > this prompt > `docs/*`. Scope boundaries are FUT-17 (math) and FUT-18 (Mermaid) in `docs/FUTURE.md`. Build nothing beyond them.

---

## 1. Review of V1-B — accepted, no fixes
V1-B is **accepted**: every AC passed, all 21 driver scripts pass, byte identity is proven, and T5 is 16.9 / 19.8 ms.

**Note for later (no action):** M1 with images is 4.64 MiB against a final budget of 5.0 MiB (post-v1 optimization, FUT-50).

**Goal of V1-C:**
- `$…$`, `$$…$$` and ` ```math ` render as real formulas.
- ` ```mermaid ` flowcharts and sequence diagrams render as real diagrams.

Both are **native Rust: no JavaScript, no downloads, no network.** These are the last two interim placeholders in the reading view.

---

## 2. Math — `yamdv-math` (pure) + platform glue (R-MATH-01, -03…-06 within the FUT-17 core)

### 2.1 Font and metrics
- Use the system font **Cambria Math**, loaded lazily: only when a document contains math.
- The platform layer reads the OpenType `MATH` table through `IDWriteFontFace::TryGetFontTable` and hands the raw bytes to the pure crate.
- The pure crate parses what layout needs:
  - `MathConstants`
  - `MathGlyphInfo`: italics correction and top-accent attachment
  - `MathVariants`: size variants and glyph assembly for big delimiters, radicals and big operators
- Glyph IDs come from `cmap` via the platform (`GetGlyphIndices`).

### 2.2 Parser
- A bounded TeX-math tokenizer and parser (S-PARSE-05 limits) that produces an AST.
- It covers exactly the core subset in FUT-17: scripts, `\frac \dfrac \tfrac \binom`, `\sqrt[n]{}`, big operators with limits, `\left \right` and `\big`-family sizes, the environments `matrix pmatrix bmatrix vmatrix cases aligned` (with `&` and `\\`), font commands, accents, Greek letters, about 150 common symbols/relations/arrows, spacing, `\color{}`, `\ldots \cdots`, and primes.
- `\mathbb`, `\mathcal`, `\mathbf`, `\mathit` map to the Unicode Mathematical Alphanumeric Symbols (U+1D400 block), which Cambria Math covers.
- An unknown command is an error, rendered per R-MATH-03.

### 2.3 Layout
- A TeX-style box model: hbox/vbox, the four styles (display / text / script / scriptscript) and the cramped variants.
- Spacing between atom classes (ord, op, bin, rel, open, close, punct, inner) follows the TeX spacing table.
- The output is display ops: glyph runs in Cambria Math with positions, plus rules (fraction bars, radical overbars).
- Formula size is 1.21 em of the body size (B.6.11).

### 2.4 Integration
- **Inline math** needs baseline alignment inside text, so implement `IDWriteInlineObject` for inline formulas (ascent/descent from the math layout). Copying selects the formula as **one unit** and copies its LaTeX source **with** delimiters (R-MATH-04). If the same mechanism gives `^sup^`/`~sub~`/footnote references their baseline shift trivially, do it and close FUT-41; otherwise leave it.
- **Display math** (`$$…$$` and ` ```math `) is centered with margin 16 0. Overflowing formulas scroll horizontally, using the existing scroll regions.
- **Errors** (R-MATH-03): the source is drawn in the theme's `error` color, with the message in small `text_muted` text below display formulas; inline errors show the colored source only. The app never panics.
- **Colors** follow the theme; `\color` overrides them.
- `reading.render_math = false` shows the source as code.
- F2 editing needs nothing new: the block becomes source when clicked, and inline math sits inside its paragraph's source.
- Layout results are cached in the detail tree.

### 2.5 Tests
- Parser and layout unit tests with a mock metrics provider: fractions, scripts, radicals, delimiter sizing, matrices, accents, errors.
- A new fuzz target `math` (parser + layout), run 5 minutes in the background (A-4).

---

## 3. Mermaid — `yamdv-mermaid` (pure) + rendering (R-MMD-01, -02 without fullscreen, -04, -05, R-MMD-06/-07 within the FUT-18 boundary)

### 3.1 Parsing
- Bounded parsers for `flowchart`/`graph` and `sequenceDiagram`, exactly as listed in FUT-18.
- Errors carry line and column (R-MMD-04).
- `%%` comments are ignored. `click`/callback lines are parsed and **ignored** (security).
- Any other diagram type (gantt, pie, class, …) renders as a code block labeled `mermaid` with the badge "Diagram type not supported yet".

### 3.2 Flowchart layout — layered (Sugiyama), kept simple
1. Cycle breaking by DFS edge reversal.
2. Longest-path layer assignment, with dummy nodes on long edges.
3. Barycenter crossing reduction: 4 down/up sweeps.
4. Coordinate assignment: median alignment, then compaction with node/rank separation of 40/50 px.
5. Directions `TB`/`TD`/`BT`/`LR`/`RL` applied as a final transform.

Nodes:
- Node text is laid out with DirectWrite through the existing `TextShaper` trait: body family, 14 px, max width 200 px with wrapping.
- Node padding is 12 × 8.

Shapes:
- rect, round, stadium, subroutine, cylinder, circle, rhombus.
- Other shapes fall back to rect.

Edges:
- Edge polylines go through the dummy nodes and are drawn as smooth cubic Bézier curves with arrowheads.
- Kinds: `-->` arrow, `---` open, `-.->` dotted, `==>` thick.
- Labels sit at the midpoint on a label background.

Subgraphs:
- Members stay contiguous in the ordering.
- Each subgraph is drawn as a titled rectangle around its members' bounding box after layout.
- Nested subgraphs are allowed.

Styles:
- `classDef`, `class`, `:::` and `style` (fill, stroke, color).

### 3.3 Sequence layout
- **Participants and actors:** boxes at the top and bottom; an actor is drawn as a simple stick figure.
- **Lifelines.**
- **Messages:** the 8 arrow types (solid/dotted × arrow/open/cross/async), with the label above the line.
- **Activation bars**, set with `+`/`-` or `activate`/`deactivate`.
- **Notes:** left of / right of / over A[,B].
- **Frames:** `loop`, `alt`/`else`, `opt` and `par`/`and`, each a labeled frame.
- **Also:** `autonumber`, `title`.

### 3.4 Rendering
- **Card** per B.6.12, palette per B.3.6 (light and dark).
- **Fit:** the diagram is scaled down to fit the column and never up. There is no fullscreen viewer (FUT-36).
- **Drawing:**
  - Vector, with the existing tiny-skia path fill/stroke and DirectWrite text, drawn per band.
  - If T4 regresses past the gate, cache rasters for **visible** diagrams only, inside the image LRU budget.
- **Limits** (S-PARSE-06): above the soft limit, show the placeholder "Diagram too large (N nodes)" with a [Render anyway] action that respects the hard limit.
- **Time budget:** layout > 200 ms → "Content too complex" placeholder (S-PARSE-03).
- **Caching:** the layout is cached per diagram source hash.
- `reading.render_mermaid = false` shows the code block.

### 3.5 Tests
- Parser unit tests for every listed construct, including error positions.
- Layout invariants: no overlapping nodes; every edge connects its endpoints; LR is a rotation of TB.
- A new fuzz target `mermaid` (both parsers + layout), run 5 minutes in the background.

---

## 4. Prepare for V1-D (small, required)
Add one platform API: render a **single block's display list** offscreen to a PNG at a given scale. V1-D's static HTML export will embed formulas and diagrams as 2× PNG data URIs. Reuse the `--render-png` pipeline; no new tooling.

---

## 5. Corpus, goldens, measurements
**Corpus** (original content):
- `math.md`: every core construct, plus errors.
- `math-heavy.md`: 50 formulas.
- `mermaid.md`: every flowchart and sequence construct, an unsupported type and a syntax error.
- `mermaid-heavy.md`: 5 diagrams of about 20–40 nodes.

**Goldens (≤ 4 new):** `math` light, `math` dark, `mermaid` light, `mermaid` dark.

**Measurements** — the existing table plus:
| ID | Scenario | Soft budget |
|---|---|---|
| M3 | `math-heavy.md` / `mermaid-heavy.md` PWS | ≤ 9.6 MiB each |
| T1M | first paint of `math-heavy.md` / `mermaid-heavy.md` | report |
| T4 | scroll p99 on both documents (warm) | ≤ 16 ms (gate) |

**Out of scope:** everything in FUT-17/FUT-18 beyond the core; fullscreen viewers; mind map; export; packaging.

---

## 6. Acceptance criteria
| ID | Criterion |
|---|---|
| AC-1 | `math.md` renders every core construct correctly in light and dark (goldens self-reviewed; list known visual deviations from KaTeX output). Errors never crash. |
| AC-2 | `mermaid.md` renders every listed flowchart and sequence construct. Unsupported types and syntax errors show the specified fallbacks. |
| AC-3 | Inline math is baseline-aligned and copies as LaTeX with delimiters. Display math scrolls horizontally when too wide. |
| AC-4 | Two new fuzz targets ran ≥ 5 minutes each with no open crash. |
| AC-5 | Gates green, incl. M3 and T4 on both new documents; PR merged; tag `phase-v1c`. |
| AC-6 | Report ≤ 120 lines with a 4-step owner QA list: a formula-heavy README, a flowchart, a sequence diagram, dark mode. |

**END OF PROMPT 007.** Start with §2 now.
