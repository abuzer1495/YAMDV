# V2-G plan (prompt 017: content engines)

Order: §1.1 F-31 → F-32 → §2 ADR-0021 (before SVG code), ADR-0022, P-48, IDEA-05 → §3 items 1–7 → tests/goldens → gates → PR (one CI run, +1 fix) → merge → phase-v2g (no release) → report.

## CHECKPOINT 1 (§1.1 done)
- Prompt stored as docs/prompts/017_V2G_CONTENT_ENGINES.md (rewritten from the Read output; sha256 2433cad3… differs from the attachment name bac86ab5…, content identical as read).
- F-31: ci.yml driver loop reruns a script once when its only FAIL lines are pws_delta_kib / cpu_delta_ms asserts.
- F-32: push tags `['v*', '!phase-*']`; pure/windows skip on tags and on `chore(release):` pushes; release job has no `needs`. One line (A-9 item 10) in PROCESS.md + PROCESS_AMENDMENTS.md.
- Next: §2 ADR-0021 SVG (check D2D SVG device needs, resvg size), ADR-0022, P-48, IDEA-05.

## CHECKPOINT 2 (§2 done, §3.1 math done)
- §2: ADR-0021 (SVG: own pure crate `yamdv-svg` on D2D paths; resvg +561 KB rejected), ADR-0022 (ICO own parser + WIC payloads; WebP keeps the "WebP codec not installed" placeholder: image-webp +138 KB > 80 KB budget), P-48 S-RES-07, IDEA-05 = #86.
- §3.1: parser split (parse/{mod,cmd,env,styled}.rs), extra.rs vocabulary (≥ 500 symbols), layout/{mod,ext,matrix}.rs (stretch_h, Stack, Style, Limits, Middle 2-pass, Phantom, Cancel=Op::Line, Boxed, Tag, array rules), macros.rs (limits 10k steps / 100k tokens), yamdv-layout macros.rs (document scopes via Index.macro_blocks), export running env, renderer Op::Line, display tag right-aligned. docs/MATH_COVERAGE.md 123/123. Commit d71f524.
- Next: §3.2 Mermaid (gantt, pie MUST; state/class/ER/mindmap; init; flowchart shapes/edges; copy PNG/source).

## CHECKPOINT 3 (§3.2 Mermaid, §3.3 SVG done)
- §3.2: gantt (DFS after-resolution, excludes, 600·s width), pie (showData), state/class/ER/mindmap, init themes, flowchart shapes/edges, parallel-edge spread with labels on their own side, Copy as PNG/source. Fixtures v2g-golden-a/b.md.
- §3.3: crates/yamdv-svg (xml/style/path/render, limits), platform images/svg.rs (DWrite text runs, currentColor = UI text), viewer zoom re-render, image/svg+xml data URIs, fuzz svg_parse. Fixture v2g-golden-c.md + assets/logo.svg, dtd.svg. Commit 0a774b6.
- Note: under Wine there is no math font, so display math shows the source fallback locally; CI (Windows) renders it.
- Next: §3.4 animated GIF (pure yamdv-core::image::gif + platform images/gif.rs), then ICO/WebP, lexers, F2 resize.

## CHECKPOINT 4 (§3.4 GIF, §3.5 ICO/WebP, §3.6 lexers done)
- §3.4: yamdv-core image/gif.rs (disposal, timing, Player), platform images/gif.rs (WIC frames at display size), visible-only one-shot TIMER_GIF=23, viewer TIMER_FVGIF=24, setting reading.animate_gifs (default off).
- §3.5: yamdv-core image/ico.rs (directory, pick, PNG/BGRA/BMP+mask payloads), WebP → "WebP codec not installed" (ADR-0022); ACG rows ico-png/ico-bmp/webp-lossy/lossless/alpha (webp = placeholder rows). Commit 3fc4815.
- §3.6: new lexers Console, Jsx, Tsx (jsx.rs element stack in State::ext), Html (`<script>` JS / `<style>` CSS embedded; vue → Html); LESS `@x` variables. Table docs/CODE_LANGUAGES.md (42 lexers). (FUT-01 row edit reverted at CHECKPOINT 7: FUTURE.md is frozen by spec-check; the issue closes it.)
- Next: §3.7 F2 image resize, then drivers content-engines / image-resize, goldens v2g-a/b/c, gates, PR.

## CHECKPOINT 5 (§3.7 F2 image resize done)
- yamdv-editor imgsize.rs `set_width` (wiki `|N`, `![alt](src)` → `<img … width>`, `<img>` width replaced, height dropped); test resize_writer.
- platform wysi/imgsize.rs: select on click in F2 (hooked first in edit_mouse_down F2 path), 4 accent 8×8 handles, aspect-locked drag with "W px" label, seal/replace/seal (one undo), Esc cancels drag / clears selection; driver verb `v2f-img "needle[@n] dx [esc]"`, dump `img_sel`. tests/driver/image-resize.yds PASS in Wine.
- Fix: images.rs Entry keeps a second decoded size (`alt`); the same image at two sizes in one doc ping-ponged decodes and starved the UI thread (pre-existing).
- Next: content-engines driver (fold zz-gif/zz-img), goldens v2g-a/b/c, gates, PR.

## CHECKPOINT 6 (drivers + goldens)
- tests/driver/content-engines.yds PASS in Wine (diagrams_cached, `ui diagram-copy-png` → clip_img, `math_errors` font-free probe via yamdv_math::layout::check, ICO/WebP/SVG dtd+hostile, GIF off/on/scrolled-out T7); fixtures ce-img.md, ce-gif.md, assets/hostile.svg; zz-* removed.
- v2g-golden-c used `\newcommand{\R}` (builtin → error) → `\RR`.
- shots.yds + SHELL_SHOTS: top-v2g-a-light, top-v2g-b-dark, top-v2g-c-light (commit from first CI artifact; Wine has no math font).
- Next: size-map shares, gates (M0/M1, T4, T5, T7, size; M3, PWS 5 GIFs), TRACEABILITY, PR.

## CHECKPOINT 7 (traceability, spec-check)
- TRACEABILITY via tools/trace_set.py: R-MATH-02/07, R-MMD-01/03/06/08(partial: no today marker)/09/10, R-IMG-01, R-CODE-01, R-F2-11 (IDEA-05 #86), R-F2-12.
- Reverted direct edits to FUTURE.md (FUT-01) and UI_SPEC.md (duplicate reading.animate_gifs row; spec already has it) → spec-check green.
- Next: size maps (running), PR.

## CHECKPOINT 8 (size budget)
- Size map was +266,240 B (over +250 KB). Fix: release `opt-level = "z"` for mermaid/svg/math/highlight/export packages → 3,626,496 B, **+244,736 B**. Shares in docs/reports/v2g_sizemap.md. gate.sh GATE-OK.
- Next: open PR (no [skip ci]), one CI run, fix push with v2g goldens from the artifact, gates (watch T4 with z-level engines), merge, tag phase-v2g, report.

## CHECKPOINT 9 (CI run 40040071370 → fix push)
- Run 1 red on: (1) view goldens mermaid-light/dark (pie now renders instead of the "unsupported" card — intended; t0 PNGs replaced from the artifact); (2) content-engines `gif_frames >= 4` = 0: the runner has client-area animations off (reduced motion, correct behavior) → dev-tools `ui motion-on` override; (3) export-progress `pws_delta_kib` 372/468 > 205: every driver assert built the full state incl. the new `math_errors` probe, which walks all blocks of the 1 MB doc → now computed only when asserted.
- Gates from run 1 (green): M0 2.37 / M1 4.67 MiB, T4 typical warm 10.4 ms, T5 split 14.3 / live 25.9 / live-large 20.3 ms, T7 0/0, MSVC exe 3,599,872 B (+213,504 B). Window shots v2g-a/b/c committed from the artifact (self-reviewed).
