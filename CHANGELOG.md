# Changelog

All notable changes to this project are documented in this file.
The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [1.0.0] - 2026-10-17

First complete release (owner decision 2026-10-17: v1 scope; deferred items are in FUTURE.md).

### Added
- Reading: CommonMark + GFM with callouts, footnotes, task lists, `<details>`, tables, 30 code
  lexers, native math (core TeX subset) and Mermaid flowchart + sequence diagrams, images
  (PNG, JPEG, GIF first frame, BMP) decoded under Arbitrary Code Guard.
- Shell: welcome page, sidebar outline / favorites / history, find bar, 11 color schemes with
  light/dark/system mode, `custom-theme.ini` hot reload, EN/TR UI, single instance, file watcher.
- Editing: F2 Live Edit and F3 Split Edit with undo/redo, formatting shortcuts, atomic saves,
  autosave, conflict handling; F4 opens an external editor.
- Export (V1-D): self-contained static HTML (no script, strict CSP, images/math/diagrams as
  `data:` URIs); Print… (Ctrl+P) and Export PDF… (Ctrl+Shift+E) through the print path
  ("Microsoft Print to PDF"); Copy / Export table as CSV (UTF-8 BOM, CRLF, RFC 4180, formula
  guard); headless `--export-html` / `--export-pdf`.
- Packaging (V1-D): `--install` / `--uninstall` per user (Start-menu shortcut, Apps & features
  entry, registration), welcome-page "Install YAMDV for this user", portable mode via
  `yamdv.portable`; version 1.0.0 in the version resource and About.

### Changed
- F-22: every measurement gate fails only on two consecutive failing attempts.

## Phase history (before 1.0.0)

### Added
- Phase 002 (reading MVP):
  - Settings model with INI read and write, EN/TR strings, an internal log, and theme data for all 11 schemes.
  - Defensive path checks, size and binary guards, and encoding detection (BOM, UTF-8, chardetng).
  - Two-level document model: a block index plus lazily parsed detail trees in an LRU cache, with splice support (ADR-0007).
  - Layout ↔ source mapping and the copy-text rules.
  - DirectWrite text shaping and rendering backend A, with virtualized layout and scroll anchoring.
  - Plain-text mode and interim placeholders for constructs that are rendered from Phase 003 on.
  - Reading window: open via CLI (multi-file), Ctrl+O and drag & drop; in-window confirm dialog; smooth scrolling with an overlay scrollbar; selection and copy.
  - Tooling: `--render-png` (dev-tools), `cargo xtask golden` and `gen-large`, memprobe `--size`/`--open-dialog`/`--cpu-ms` with budget gates, fuzz targets, owner kit, spec-check and spec-patch.
- ADR-0007…0012. Arbitrary Code Guard enabled (ADR-0001 amendment).

### Changed
- Memory and size numbers are in MiB with raw bytes (P-05). The B-SIZE soft budget is 4.0 MiB.

### Added (Phase 001)
- Phase 001: repository, Cargo workspace, documentation set (specification appendices A–H),
  `cargo xtask` tooling (`ci`, `check-imports`, `size`, `memprobe`, `gen-icons`), CI workflow,
  native window skeleton (`yamdv.exe`), rendering-backend spike and ADR-0002 (backend A: DirectWrite GDI interop + tiny-skia).
- THREAT_MODEL skeleton, TRACEABILITY matrix, ADR-0000/0001/0003–0006.
