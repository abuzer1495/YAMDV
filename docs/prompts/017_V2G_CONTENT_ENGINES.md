# 017_V2G_CONTENT_ENGINES — YAMDV Phase V2-G: math, Mermaid types, SVG, GIF, ICO/WebP, lexers, image resize

> **Prompt 017.** Phase **V2-G** (`docs/ROADMAP_V2.md`).
> - Branch `phase/v2g-content`, tag `phase-v2g` (no release in this phase)
> - Report `docs/reports/v2g_REPORT.md` (≤ 80 lines)
> - Store this file as received in `docs/prompts/`
>
> **Precedence:** `docs/PROCESS.md` (A-1…A-10) > this prompt > `docs/UI_SPEC.md` / `docs/SPEC.md` / `docs/SECURITY.md` > the rest of `docs/*`.
> **Order:** §1 → §2 → §3 (items in order) → §4. **Resumable:** see §6.

---

## 1. Review of V2-F — accepted
V2-F is **accepted**:
- **CI:** run 40020890007 is green.
- **T5:** now within the stretch targets (F3 14.0 ms, F2 24.8 / 18.7 ms).
- **Memory and size:** M0 2.35 MiB, M1 4.72 MiB; the exe grew by 136 KB, inside its budget.
- **v1.3.0:** the draft exists.
- **ADR-0020** is accepted, and so are the table strip and the preview-card deviations.

### 1.1 Fix first
| ID | Fix |
|---|---|
| **F-31** | **Flaky memory asserts.** The `phase-v2f` tag run failed on `export-progress` `pws_delta_kib` 216 > 205, which is noise. Apply the two-attempt rule (F-21/F-22) to every `pws_delta_kib` and `cpu_delta_ms` assert in driver scripts: fail only if both attempts exceed. |
| **F-32** | **CI minute economy.** (1) `phase-*` tags trigger no workflow (`tags-ignore`). (2) `v*` tags run **only** the release job (build, zip, SHA-256, draft): no test jobs. (3) A branch push whose head commit message starts with `chore(release):` skips the test jobs; its tree equals a green head (A-9 / F-25). Record these as one line in A-9. |

### 1.2 Owner notes (only if attached)
If the owner pasted notes from his checks below this prompt, each one becomes an item F-33, F-34, …, fixed right after §1.1.

---

## 2. Setup
1. **ADR-0021 "SVG renderer"** (before any SVG code). It chooses one of:
   - (a) **Direct2D SVG.** Check first whether it needs a D3D/WARP device. WARP JIT-compiles shaders, which conflicts with ACG (S-PROC-04), so (a) is likely out.
   - (b) **`usvg` + `resvg` without the `text` feature.** It reuses `tiny-skia`.
   - (c) **Our own SVG-subset renderer** on `tiny-skia`:
     - drawing: paths, basic shapes, `g`, `use`, `transform`, `viewBox`, fill and stroke, solid colors, and linear and radial gradients;
     - text through DirectWrite;
     - parsing with a small bounded XML tokenizer.

   **Criteria:**
   - works under ACG;
   - **≤ +120 KB** exe;
   - zero cost when no SVG is shown;
   - meets the P-48 rules.

   If none of them fits, keep the placeholder and record that.
2. **ADR-0022 "ICO and WebP under ACG".** The 003b ACG matrix showed that WIC fails for ICO and WebP.
   - **ICO:** our own directory parser.
     - Pick the entry that best fits the display size.
     - Decode a PNG payload with WIC PNG.
     - Decode a BMP payload with WIC BMP: build the file header and handle the double height and the AND mask.
   - **WebP:** use the pure-Rust `image-webp` crate if it adds **≤ +80 KB** and runs within the S-RES-04 limits. Otherwise keep the R-IMG-01 placeholder "WebP codec not installed". No helper process for this.
3. **Spec patch** (`docs/spec-patches/v2g.txt`, applied with `xtask spec-patch`):

**P-48** — `docs/SECURITY.md`. INSERT AFTER the anchor line:
```text
- **S-RES-06 MUST** Fonts come only from the system/user font collections via DirectWrite; documents cannot load fonts.
```
NEW line:
```text
- **S-RES-07 MUST** SVG images: no external references (only same-document `#id` references are resolved; `href`/`xlink:href`/`url()` to files or URLs are ignored), no scripts, no `foreignObject`, no event attributes, no CSS `@import`; DTDs and entity declarations are rejected (no entity expansion); limits: file ≤ 10 MB, ≤ 50 000 elements, nesting ≤ 64, `use` depth ≤ 8 and ≤ 10 000 resolved instances, ≤ 1 000 000 path commands; rendering is bounded by the display size and the S-RES-04 pixel limits.
```

4. **Issue for a MAY item.** R-F2-11 (visual Mermaid editing) is not planned for v2. Create `IDEA-05: Visual Mermaid editing (R-F2-11)` with label `idea` and milestone `post-v1`.

---

## 3. V2-G tasks
Specifications:
- `docs/SPEC.md` R-MATH-01…07, R-MMD-01…10, R-IMG-01…06, R-CODE-01, R-F2-12
- `docs/SECURITY.md` S-PARSE-05/06, S-RES-04, S-RES-07 (P-48), S-PROC-04
- `docs/UI_SPEC.md` B.3.6 (Mermaid palettes), B.6.12

Issues: FUT-17, FUT-18, FUT-20, FUT-48, FUT-01, FUT-26.

1. **Math beyond the core** (R-MATH-02, R-MATH-07; FUT-17)
   - **Coverage:** implement the rest of the R-MATH-02 list. Keep a coverage table in `docs/` that lists every command as done or missing; the target is 0 missing among the MUST commands.
   - **Macros:** per-document `\newcommand`, `\renewcommand` and `\def` (simple parameters `#1…#9`).
     - A definition applies to the formulas after it, in document order.
     - Macro expansion is bounded per S-PARSE-05 (≤ 10 000 steps, ≤ 100 000 tokens). Over a limit, the formula shows in the error color (R-MATH-03).
   - **Export and print:** they use the same engine (V1-C block-PNG API).
2. **Mermaid diagram types** (R-MMD-01, R-MMD-03, R-MMD-06…10; FUT-18)
   - **New types:**
     - **MUST:** **gantt** (R-MMD-08: dateFormat / axisFormat tokens, `excludes weekends`, sections, tags, `after id`, durations, the today marker) and **pie** (R-MMD-09).
     - **SHOULD:** **stateDiagram(-v2)**, **classDiagram**, **erDiagram**, and **mindmap** (reuse the V2-D mind-map layout).
   - **Theme directive:** `%%{init: {"theme": …, "themeVariables": {…}}}%%` with the R-MMD-03 subset.
   - **Flowchart extras:**
     - every R-MMD-06 shape and edge, including `--o`, `--x`, `o--o`, `x--x` and the extra-length forms;
     - `linkStyle` (basic);
     - `style` with stroke-dasharray.
   - **Context menu** on a diagram: *Copy as PNG* (2×, `CF_DIB` + PNG) and *Copy source* (R-MMD-10).
   - **Limits:** every new type stays within S-PARSE-06, with deterministic layout.
   - **Not supported:** timeline, journey, gitGraph and quadrantChart (MAY) keep the "Diagram type not supported" badge.
3. **SVG images** (R-IMG-01, P-48; FUT-20) per ADR-0021.
   - **Decoding:** at display size, through the existing image worker and LRU, with ⛶ viewer support. The viewer re-renders at the zoom scale.
   - **Colors:** `currentColor` is the text color.
   - **Fuzzing:** add a fuzz target `svg_parse`. It is not run in CI; V2-I runs it long.
4. **Animated GIF** (R-IMG-01; FUT-20)
   - **Setting:** `reading.animate_gifs` (default off). When it is off, the first frame shows, as today.
   - **Decoding:** WIC GIF frames with delay and disposal handling, decoded frame by frame at display size. Keep at most a canvas plus one frame per GIF; the frame limit is 1 000 frames (S-RES-04).
   - **Playback:** a GIF plays only while it is visible, the window is active and not minimized, and reduced motion is off.
     - The timer runs only while at least one visible GIF plays, so T7 stays 0 otherwise.
     - The viewer plays it too.
   - **Data:** the frame timing and disposal logic is pure code with unit tests.
5. **ICO and WebP** (R-IMG-01; FUT-48) per ADR-0022, including the ACG matrix rows (PNG-in-ICO, BMP-in-ICO, WebP lossy / lossless / alpha).
6. **Highlight lexers** (R-CODE-01; FUT-01)
   - **Coverage:** every R-CODE-01 language name gets a correct lexer: its own, or a shared one only where the grammars really match (e.g. jsx/tsx on the TS lexer). Update the F-19 coverage table.
   - **Unit tests:** each lexer gets a sample test asserting the token classes.
7. **F2 image resize handles** (R-F2-12; the rest of FUT-26)
   - **Handles:** in F2, clicking an image selects it and shows 4 corner handles (8 × 8, `accent`). Dragging keeps the aspect ratio and shows a live "W px" label.
   - **Writing:**
     - `![[x.png|300]]` updates `|N`;
     - `![alt](src)` becomes `<img src="…" alt="…" width="N">` (allowlisted attributes);
     - an existing `<img>` updates its `width`.
   - **Undo:** one undo step. `Esc` cancels a drag.

**Size budget:** this phase may add at most **+250 KB** to the exe (SVG ≤ 120 KB and WebP ≤ 80 KB of that). Report each item's share with `cargo xtask size-map`.

**Out of scope:** the V2-H export work and everything later.

**Tests (A-7/A-9 as reduced by A-10):**
- **Driver scripts (2):**
  - `content-engines`:
    - a fixture with gantt, pie, state, class, ER and mindmap diagrams plus a `%%{init: theme forest}%%` flowchart: every diagram renders with no "not supported" badge;
    - *Copy as PNG* puts an image on the clipboard;
    - a math fixture covering matrices, `cases`, `aligned`, accents and `\newcommand`: no error color; a macro bomb shows the error color within the limits;
    - an SVG fixture renders paths, gradients, text and `use`; an external `href`, a `<script>` and an entity DTD are ignored or rejected;
    - PNG-in-ICO and BMP-in-ICO render;
    - WebP renders, or shows the placeholder as ADR-0022 decides;
    - animated GIF: with the setting off, the first frame shows and T7 is 0; with it on, the frames advance; scrolled out of view, the timer stops (T7 0).
  - `image-resize`, in F2:
    - dragging a corner handle writes `width` for a Markdown image and `|N` for a wikilink embed;
    - one `Ctrl+Z` restores it.
- **Goldens (3):**
  - gantt + pie (light)
  - state + class + ER (dark)
  - SVG sample + math with matrices / macros (light)
- **Unit tests:**
  - lexers
  - math macros and limits
  - gantt date math (dateFormat, `excludes weekends`, `after`)
  - pie percentages
  - the SVG parser limits
  - the ICO directory
  - GIF timing / disposal
  - the resize writer

**Gates (A-7/A-9):**
- M0 and M1 at most +0.3 MiB vs V2-F (2.35 / 4.72 MiB).
- T4 typical warm, T5 (limits 25 / 40 ms) and T7.
- The size budget above.
- Report-only: M3 (`mermaid-heavy.md`) and PWS with 5 animated GIFs visible.

---

## 4. Merge and report
- **PR and CI:** one CI run at the end, plus at most one fix push.
- **Issues:** the PR body closes FUT-17, FUT-18, FUT-20, FUT-48, FUT-01 and FUT-26 with "Closes #n". FUT-18's R-F2-11 part moves to IDEA-05.
- **After CI is green:** merge, then tag `phase-v2g`.
- **Report (≤ 80 lines):**
  - the AC table
  - the gates, with each item's size share
  - the ADR-0021 / ADR-0022 decisions (2 lines each)
  - the math coverage line (done / total)
  - deviations
  - the owner check list (≤ 8 steps, EN + TR), which must cover:
    - a real gantt;
    - pie, state, class and ER diagrams;
    - `%%{init}%%`;
    - a formula with macros;
    - an SVG logo;
    - an `.ico` and a `.webp`;
    - an animated GIF with the setting on and off;
    - resizing an image in F2.

## 5. Acceptance criteria
| ID | Criterion |
|---|---|
| AC-1 | F-31 and F-32 are done (and §1.2 if notes were attached); ADR-0021 and ADR-0022 are written; P-48 is applied with `spec-check` green; IDEA-05 exists. |
| AC-2 | §3 items 1–7 work per the listed specs; TRACEABILITY is updated by script; S-RES-07 holds; the math and lexer coverage tables are updated. |
| AC-3 | The 2 driver scripts pass on CI; the 3 goldens are committed and self-reviewed; the unit tests pass; the fuzz target builds; the PR artifact `yamdv-exe` exists. |
| AC-4 | The gates and the size budget are green; the PR is merged with the issues closed; `phase-v2g` is tagged; the report follows §4. |

## 6. Stop rule (quota)
If the Grok quota or the Actions minutes run low at any point:
1. commit and push with `[skip ci]`;
2. write `docs/reports/v2g_REPORT.md` with a done / partial / not-started table (one row per §3 item) and **"Next action: …"**;
3. stop.

The owner will give this same prompt again.

**END OF PROMPT 017.** Start with §1.1 now.
