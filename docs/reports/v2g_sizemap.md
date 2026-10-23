# V2-G size map (prompt 017 §1 budget)

Hardened MSVC-target release (`+crt-static`, CFG, /OPT:REF,ICF) linked with `/MAP`; `cargo xtask size-map <map> --top 400`, v2g vs `phase-v2f`.

- Exe: 3,381,760 B (phase-v2f) → 3,626,496 B (**+244,736 B**, budget +250 KB).
- Budget fix: `opt-level = "z"` for yamdv-mermaid, yamdv-svg, yamdv-math, yamdv-highlight and yamdv-export (cached or per-action paths; hot paths stay `"s"`). Before it: +266,240 B.

## Per-item share

| Item | Modules | Δ bytes |
|---|---|---:|
| Mermaid engines (state/class/gantt/pie/seq/themes) | `yamdv_mermaid::*`, `yamdv_layout::diagram` | +37,339 |
| SVG (≤ 120 KB) | `yamdv_svg::*`, `tiny_skia` | +36,508 |
| Math (\def/\newcommand, environments) | `yamdv_math::*`, `yamdv_layout::rich` | +22,575 |
| GIF / ICO / WebP (WebP ≤ 80 KB: +0, placeholder) | `yamdv_core::image`, `yamdv_platform_win::images` | +17,260 |
| Lexers (net; z-level offsets the new modules) | `yamdv_highlight::*` | +3,332 |
| F2 resize | `yamdv_editor::imgsize` (+ wysi handles inside `window`) | +7,181 (+window) |
| UI wiring, std/alloc/hashbrown, constants, CRT | rest | remainder |

## Largest module deltas

| Module | Δ bytes |
|---|---:|
| `[anonymous constants]` | +28,696 |
| `yamdv_platform_win::window` | +26,967 |
| `yamdv_svg::render` | +23,944 |
| `yamdv_mermaid::parse` | +17,704 |
| `yamdv_platform_win::images` | +15,543 |
| `yamdv_math::parse` | +11,673 |
| `yamdv_mermaid::7Dia` | +8,526 |
| `alloc::vec` | +8,241 |
| `[libwindows-98521244da230211]` | +8,160 |
| `yamdv_highlight::clike` | +7,957 |
| `yamdv_math::macros` | +7,481 |
| `yamdv_editor::imgsize` | +7,181 |
| `core::str` | +7,138 |
| `core::ptr` | +6,856 |
| `hashbrown::raw` | +6,290 |
| `[libucrt]` | +5,418 |
| `yamdv_svg::style` | +5,240 |
| `yamdv_ui::mindmap` | +4,352 |
| `yamdv_svg::xml` | +4,032 |
| `yamdv_layout::viewer` | +4,029 |
| `yamdv_layout::diagram` | +3,320 |
| `yamdv_mermaid::graph` | +3,292 |
| `yamdv_highlight::markup` | +2,977 |
| `alloc::raw_vec` | +2,779 |
| `yamdv_mermaid::gantt` | +2,523 |

| … smallest … | |
| `yamdv_highlight::highlight_line` | -8,876 |
| `[libwindows-264e3c436ee817a0]` | -8,040 |
| `yamdv_export::html` | -7,153 |
| `yamdv_mermaid::flow` | -2,501 |
| `yamdv_mermaid::layered` | -953 |
| `yamdv_math::symbols` | -902 |
| `yamdv_ui::settings_panel` | -850 |
| `yamdv_layout::callout` | -815 |
