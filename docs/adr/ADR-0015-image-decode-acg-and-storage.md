# ADR-0015: Image decoding under ACG and bitmap storage

- Status: accepted (Phase 003b), storage comparison **open**
- Context: P-13, prompt 004 §4.1 items 4–5, S-PROC-04 (ProhibitDynamicCode on).

## ACG matrix (CI `windows-latest`, release dev-tools build, ACG on; `cargo xtask acg-matrix`)
| Format | Decoded under ACG | Note |
|---|---|---|
| PNG | yes | |
| JPEG | yes | |
| GIF (first frame) | yes | |
| BMP | yes | |
| TIFF | yes | |
| ICO | **no** | probe OK, WIC decode failed; reason not yet isolated (ACG vs. PNG-in-ICO sample) |
| WebP | **no** | probe OK, WIC decode failed; Server 2025 runner may lack the WebP codec (R-IMG-01 "codec not installed") |

ICO/WebP are reported, not gated, until a run without ACG separates "ACG" from "codec missing".

## Decision: storage
Decoded pixels are a heap `Vec<u8>` of `32bppPBGRA` at display size in device pixels, blitted
with tiny-skia into the banded DIB (backend A). No GDI object per image (GDI count stays 14).

## DIB section vs. DDB
**Not measured** in Phase 003b (4-image PWS/commit comparison not built). The heap buffer is
equivalent to a DIB section in memory class (private, committed); a DDB would move pixels into
the display driver and out of the PWS metric, which P-11 treats as hiding cost. Re-open with
numbers in Phase 004.

## Consequences
- Image cache bytes are Rust-side and appear in `--mem-stats` (`image_decoded_bytes`,
  `image_lru_bytes`).
