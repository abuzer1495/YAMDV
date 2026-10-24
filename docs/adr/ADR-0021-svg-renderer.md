# ADR-0021: SVG renderer

- Status: accepted (Phase V2-G, prompt 017 §2.1)

Context: R-IMG-01 lists SVG; until V2-G an SVG image showed the "Unsupported" placeholder. Criteria (prompt 017):
works under ACG (S-PROC-04), **≤ +120 KB** of `yamdv.exe`, zero cost when no SVG is shown, meets S-RES-07 (P-48).

Options checked:
| Option | Finding | Verdict |
|---|---|---|
| (a) Direct2D SVG (`ID2D1DeviceContext5::CreateSvgDocument`) | needs an `ID2D1DeviceContext5`, i.e. a D2D device on a D3D11 device; our renderer is CPU (`tiny-skia`, ADR-0002) and the software D2D path is WARP, which JIT-compiles shaders → blocked by ACG. Also no text, and the D2D parser resolves its own references (no S-RES-07 control). | out |
| (b) `usvg` + `resvg` 0.48.1, `default-features = false` (no `text`, `svgz`, raster images) | cross-built probe, release profile (`opt-level="s"`, fat LTO, stripped): tiny-skia-only probe 435 200 B → with resvg 996 352 B = **+561 KB**. `roxmltree` also expands internal DTD entities (S-RES-07 wants them rejected). | out (size) |
| (c) own subset renderer on `tiny-skia` | only the subset the prompt lists; parser and limits are ours, so every S-RES-07 rule is a unit test. | **chosen** |

Decision: (c). New pure crate **`yamdv-svg`** (no Windows code, fuzzable):
- `xml`: bounded tokenizer (elements, attributes, text, comments, CDATA; `<!DOCTYPE`/`<!ENTITY` → error, no entity
  expansion beyond the five predefined and numeric references); limits: input ≤ 10 MB, ≤ 50 000 elements, depth ≤ 64.
- `tree`: `svg`, `g`, `use` (same-document `#id` only, depth ≤ 8, ≤ 10 000 instances), `defs`, `symbol`, `path`,
  `rect`, `circle`, `ellipse`, `line`, `polyline`, `polygon`, `text`/`tspan` (single-line runs), `linearGradient`,
  `radialGradient` (`stop`, `gradientUnits`, `gradientTransform`, `spreadMethod`, `href` inheritance), `style`
  attribute and presentation attributes, `transform`, `viewBox` + `preserveAspectRatio`, `opacity`/`fill-opacity`/
  `stroke-opacity`, `stroke-width/linecap/linejoin/miterlimit/dasharray`, `fill-rule`, `currentColor`.
- Ignored (S-RES-07): `script`, `foreignObject`, `image`, `on*` attributes, `<style>` elements (no CSS, so no `@import`),
  external `href`/`xlink:href`/`url()`; path data capped at 1 000 000 commands.
- `render`: to a `tiny_skia::Pixmap` at display size (bounded by S-RES-04 `MAX_DIM`/`MAX_PIXELS`); text is handed
  back as runs that the Windows side draws through DirectWrite (system fonts only, S-RES-06).
- Integration: the image worker decodes SVG at display size like raster images (same LRU); the ⛶ viewer asks
  for a new decode at the zoom scale. Nothing is parsed until an SVG image is visible → zero cost otherwise.
- Fuzz target `svg_parse` (not run in CI; V2-I runs it long).

Consequences: SVG filters, masks, clip paths, patterns, markers, CSS and animation are not rendered (the element is
skipped, the rest draws). Exe delta is measured with `cargo xtask size-map` in the V2-G report; if it exceeds
+120 KB the feature is cut back until it fits. Revisit (b) if `resvg` gets a much smaller minimal build.
