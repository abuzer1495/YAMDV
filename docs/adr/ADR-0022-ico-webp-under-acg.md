# ADR-0022: ICO and WebP under ACG

- Status: accepted (Phase V2-G, prompt 017 §2.2)

Context: the 003b ACG matrix showed WIC failing for ICO and WebP inside the hardened process (S-PROC-04): those
codecs are not available or not loadable under ACG, while WIC PNG/BMP/JPEG/GIF work. R-IMG-01 lists both formats.

Decision — ICO: our own directory parser (pure, `yamdv-core::image::ico`):
- validates `ICONDIR` (reserved 0, type 1, count ≥ 1, ≤ 256 entries) and every `ICONDIRENTRY` (offset + size inside
  the file, size ≤ S-RES-04 `MAX_FILE`); width/height byte 0 means 256;
- picks the entry that best fits the display size: the smallest entry ≥ the display size, else the largest; ties go to
  the higher bit depth;
- PNG payload (starts with the PNG signature) → WIC PNG decoder;
- BMP payload (`BITMAPINFOHEADER`) → we build a 14-byte `BITMAPFILEHEADER` in front, halve the stored double height
  (XOR + AND mask), and decode with WIC BMP; for bit depths < 32 the AND mask is applied as alpha afterwards
  (32-bit entries keep their alpha channel unless it is all zero, then the AND mask is used).

Decision — WebP: the pure-Rust `image-webp` 0.2.4 decoder was measured with the release profile (fat LTO,
`opt-level="s"`, stripped, cross-built probe): 435 200 B → 573 440 B = **+138 KB**, over the +80 KB budget. WebP
therefore keeps the R-IMG-01 placeholder **"WebP codec not installed"** (no helper process, no WIC WebP under ACG).

Consequences: ICO adds only the parser (a few KB) and reuses WIC PNG/BMP, which the ACG matrix covers; ACG matrix
rows: PNG-in-ICO and BMP-in-ICO decode, WebP lossy/lossless/alpha show the placeholder. Revisit WebP when
`image-webp` (or a lossless-only subset) fits the budget, or when the owner raises the budget.
