# Phase V1-D plan — Export, print, packaging, v1.0.0 (prompt 008)

Goal: finish v1. Static single-file HTML export (FUT-21 boundary), table CSV, headless export CLI, printing and PDF through the print path (FUT-39), per-user install/uninstall, version 1.0.0, README/CHANGELOG, FUT issues in the private repo, a draft release built by CI. First F-22: every measurement gate fails only after two consecutive over-budget attempts. Mode A-4.

Design (simplest):
- `yamdv-export` (pure): `html::document()` extends the CF_HTML node writer (one writer, two modes): inline CSS from the scheme `Palette`, TOC, highlighted code spans (`yamdv-highlight`), callouts, tables, footnotes with back-links, disabled task boxes, `<details>`; images/math/diagrams through an `ExportHost` trait returning PNG bytes; CSP meta, no script. `csv` (RFC 4180, BOM, CRLF, formula guard). `paginate` (page breaks from block/line/row boundaries, scale-to-fit, "n / N").
- Platform: `export.rs` = host (WIC decode of local images, Cambria Math box → 2× PNG, mermaid via the V1-C `snapshot::block_png`), headless `--export-html/--export-pdf`; `print.rs` = printer DC, band rendering + `StretchDIBits`, "Microsoft Print to PDF" with `DOCINFO.lpszOutput`; dialog host `print` kind (`PrintDlgExW`); `install.rs` = `--install/--uninstall` (copy, HKCU registration, `IShellLinkW`, Uninstall key, self-delete helper).
- Release: version 1.0.0, CHANGELOG, README, `cargo xtask future-issues` (gh CLI, private repo only), CI job on tag `v1.0.0` → zip + SHA-256 + **draft** release (H2: never public).
- Push policy (owner rule): push after every compiling change, never > 10 min unpushed; WIP pushes carry `[skip ci]`; CI once at the end + one fix (A-4 rule 6).

Risks: no Windows GUI locally (xwin clippy only; behaviour via CI driver scripts); printing untestable without a printer except "Microsoft Print to PDF" on the runner; self-delete only verifiable by the owner.

## CHECKPOINT
- [x] prompt stored `docs/prompts/008_V1D_EXPORT_PACKAGING_RELEASE.md` (SHA-256 9483e4591ef463021bbe57c339aea144f1ad34f24f55995d6808d0fa1dab9662)
- [x] F-22 in `measure.ps1`
- [x] export HTML (pure writer + host + menu + CLI)
- [x] CSV (pure + menu)
- [x] pagination + print + PDF (+ CLI)
- [x] install / uninstall / welcome button
- [x] version 1.0.0, CHANGELOG, README
- [x] driver scripts, XSS context, unit tests
- [x] future-issues xtask (run once 2026-10-17: 41 issues #8–#48, label `future`, milestone `post-v1`), release CI job
- [x] A-5/A-6 recorded; gates reduced; print worker fix (ADR-0018); local gates green; draft release v1.0.0-rc1; report
- [ ] owner RC1 QA → merge PR #49; 2026-11-01 CI → tags phase-v1d + v1.0.0 → CI draft release
- note: welcome golden renamed `welcome-install-light.png` (install button); commit it from the CI artifact
