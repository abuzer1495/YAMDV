# Phase 001 Plan — Foundation and Spike

## 1. Goals (restated)
1. A professional repository: Cargo workspace with every crate from ARCHITECTURE §E.2, pinned
   toolchain, hardened release profile, lint policy, `cargo deny` policy, docs set, CI.
2. Prove with **measurements** whether a 3–5 MB private working set is feasible by building the
   same reference scene with the candidate rendering backends (A: DirectWrite + GDI interop banded,
   B: Direct2D software DC render target banded, C: Direct2D hardware flip-model swap chain) and
   choosing one in ADR-0002.
3. A minimal, correct native window skeleton (`yamdv.exe`) that later phases build on — nothing more.

## 2. Host constraints and strategy
- The engineering host is **Linux** (Debian 13, x86_64). Windows GUI binaries cannot be run here
  (no Wine). Per prompt §4.1:
  - pure crates and xtask logic are built/tested natively on Linux;
  - Windows code is cross-compiled for `x86_64-pc-windows-msvc` with `cargo-xwin`;
  - every Windows-only check (release build, `check-imports`, `size`, `memprobe`, spike
    measurements, screenshots, `#[cfg(windows)]` tests) runs on GitHub Actions `windows-latest`
    and is published as workflow artifacts;
  - local GUI checks are reported as `NOT VERIFIED`.

## 3. Task breakdown
| # | Task | Requirement IDs | Verification |
|---|---|---|---|
| 1 | Env discovery | §4.1 | report §2 |
| 2 | Repo, private GitHub repo, branch `phase/001-foundation` | H2, H12 | `gh repo view` |
| 3 | Workspace layout, profiles, `.cargo/config.toml`, toolchain file, config files | §4.2, S-PROC-05, S-SUP-01/03/04 | build, `cargo deny check` |
| 4 | Docs: appendices verbatim, THREAT_MODEL skeleton, TRACEABILITY, ADR-0000/0001 | §4.3, S-TEST-04 | file review |
| 5 | xtask: check-imports (+ PE hardening flags), size, memprobe, ci, gen-icons | S-NET-03, S-TEST-03, B-SIZE, D.3, B.14 | unit tests (pass/fail fixtures), CI run |
| 6 | Platform skeleton: hardening, window, DPI, theme, keys, CLI, panic hook, manifest/version/icon resource | S-PROC-01..06, R-WIN-04, R-THEME-01, R-SYS-07 | `#[cfg(windows)]` integration test, CI memprobe |
| 7 | Rendering spike A/B/C + measurements + screenshots | §4.7 | CI artifacts, `docs/spike/` |
| 8 | ADR-0002 rendering backend decision | §4.7 | doc |
| 9 | CI workflow (windows + pure jobs) | §4.5, S-SUP-06 | green run |
| 10 | Report, PR, merge, tag `phase-001` | H11, H12, §7 | PR + tag |

## 4. Top risks and how they are retired early
| Risk | Retirement |
|---|---|
| No Windows host → GUI code can only be validated remotely | Cross-compile early with cargo-xwin; push early and use CI as the Windows test bench; instrument binaries to self-report (log lines, screenshots via `PrintWindow`/BitBlt) |
| Rust `std` pulls `ws2_32.dll` into the import table | `check-imports` runs on the first real release build; if present, report the cause, never weaken the check |
| DirectWrite/D2D baseline memory alone exceeds the budget | Measure the bare skeleton (no DWrite) and each backend separately; per-step deltas (factory, first layout, WIC) |
| Resource embedding (manifest/icon/version) without build-time downloads | Own minimal pure-Rust `.res` writer in `build.rs` (no extra crates); validated by `check-imports`/PE inspection in CI |
| CI minutes (private repo, Windows ×2) | Single Windows job, cargo cache, spikes and memprobe in the same job; no matrix |

## 5. Test plan
- xtask unit tests: PE import checker with an allowed and a forbidden synthetic PE fixture
  (generated in-test with the `object` writer — no binary files committed); forbidden-function
  name matching; size budget logic; memprobe JSON schema round-trip.
- `yamdv-app`: `#[cfg(windows)]` integration test runs `yamdv.exe --version`, `--help`, and an
  unknown flag (exit code 2).
- Lint proof: a deliberate `unwrap()` in a pure crate must fail `cargo clippy` (done once locally,
  not committed).

## 6. Measurement plan
- `memprobe` on CI `windows-latest` (Windows Server 2022/2025 runner, software rendering, no GPU):
  skeleton window, each spike candidate at scene top and after scrolling to the end, 3 runs median,
  with and without `heapType=SegmentHeap`.
- Spikes log `first_frame_ms` and per-frame paint times while auto-scrolling (instrumented mode
  triggered by a CLI flag); screenshots at 100 % and 150 % rendering scale saved as PNG.
- All numbers are labelled with the machine (CI runner) and method. Runner numbers are **not**
  the reference machine of Appendix D; they are indicative only.
