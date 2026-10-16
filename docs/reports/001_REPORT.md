# Phase 001 Report — Foundation and Spike

- Repository: https://github.com/snipeTR/YAMDV (**private**)
- PR: https://github.com/snipeTR/YAMDV/pull/1, branch `phase/001-foundation` → `main`, tag `phase-001`
- Date: 2026-10-16 (Europe/Istanbul)

## 1. Summary
Phase 001 sets up the following:
- The full repository and Cargo workspace (ARCHITECTURE §E.2), with a pinned toolchain, a hardened release profile, the lint and deny policies and the documentation set.
- `cargo xtask` tooling and SHA-pinned, read-only CI.
- A native window skeleton (`yamdv.exe`, 480,768 B).
- Three rendering spikes, measured on GitHub Actions `windows-latest`.

**ADR-0002 chooses backend A** (DirectWrite GDI interop + tiny-skia, banded DIB, ScrollWindowEx):
- 2.39 MB idle private working set (PWS) with the full reference scene. B and C are about 11.7 MB.
- Scroll p99 is 4.2 ms.
- Rendering is correct.

The 3–5 MB PWS target is shown to be **feasible**. The skeleton alone is 1.33 MB.

## 2. Environment
- Engineering host: Linux x86_64, Rust **1.99.0** (pinned), `cargo-xwin` 0.23.1 for cross-builds, cargo-deny 0.20.2.
- Windows bench: GitHub Actions `windows-latest`, which is Windows Server 2025 Datacenter 10.0.26100 with "Microsoft Hyper-V Video" at 1024×768 and no real GPU.
- Per §4.1, all Windows-only checks ran on CI. Local GUI checks are **NOT VERIFIED**.

## 3. Delivered (by task)
| # | Item | Where |
|---|---|---|
| 1 | Private repo, branch, Conventional Commits | GitHub, `git log` |
| 2 | Workspace with 8 pure crates, `yamdv-platform-win`, `yamdv-app`, `xtask` and `spikes`; release/release-dbg profiles; `.cargo/config.toml` hardening; `rust-toolchain.toml`; fmt, clippy, deny, editorconfig and gitattributes config | root |
| 3 | Specification appendices A–H, verbatim | `docs/*.md` |
| 4 | Phase plan | `docs/plans/001_PLAN.md` |
| 5 | THREAT_MODEL skeleton (STRIDE per entry point), TRACEABILITY (all 307 IDs) | `docs/` |
| 6 | ADR-0000 (index), 0001 (toolchain/hardening), 0002 (backend), 0003–0006 (dependencies) | `docs/adr/` |
| 7 | xtask commands `ci`, `check-imports` (imports, delay-imports, DllCharacteristics, CET, W+X), `size`, `memprobe` (QueryWorkingSet shared-bit, JSON `yamdv.memprobe/1`, screenshots, exit check), `gen-icons` | `xtask/` |
| 8 | Platform skeleton (details below) | `crates/yamdv-platform-win`, `crates/yamdv-app` |
| 9 | Original icons, generated in code | `assets/icons/` |
| 10 | Spikes A, B and C with the reference scene, `--autoscroll`, `--log`, `--scale` and `--size` | `spikes/` |
| 11 | CI: ubuntu pure job and windows job (gates, SegmentHeap variant, measurements, artifact) | `.github/` |
| 12 | Measurements and screenshots | `docs/spike/run-38911412595/` |

The platform skeleton (item 8) contains:
- Startup hardening as the first call.
- A 1100×760 window centred on the cursor's monitor, minimum size 480×360, with PerMonitorV2 DPI and live system light/dark theme.
- Esc and Ctrl+W close the window.
- A console-aware CLI (`--help`, `--version`, usage error with exit 2).
- A panic hook that writes crash notes to `%LOCALAPPDATA%\YAMDV\logs`.
- Manifest, icon and version resources, written by a pure-Rust `.res` writer.

## 4. Verification (gates)
| Gate | Linux host | Windows CI (run 38911412595) |
|---|---|---|
| `cargo fmt --check` | pass | pass |
| `cargo clippy --workspace --all-targets -D warnings` | pass (native and xwin) | pass |
| `cargo test --workspace --locked` | pass (xtask 15, app cli 3) | pass, including the `#[cfg(windows)]` CLI tests |
| `cargo deny check` | advisories, bans, licenses, sources all ok | pass |
| release build `--locked` | pass (xwin) | pass |
| `check-imports` (S-NET-03, S-TEST-03) | OK: 11 DLLs, no networking DLLs; DllCharacteristics 0xC160 (incl. GUARD_CF); CET compat; no W+X | pass |
| `size` | 479,232 B (xwin) | 480,768 B (soft budget 4,000,000; final 3,000,000) |
| Lint proof: deliberate `unwrap()` in `yamdv-core` | clippy failed with `unwrap_used` (exit 101) and was reverted, never committed | — |

CI history:
- Run 1 failed. `xtask` was missing `Win32_Security` when built alone, which `CreateProcessW` needs. Fixed in `b92c662`.
- Run 2 was green, but its screenshots showed a color bug in spike A. Fixed, see ADR-0002.
- Run 3 was green and is the data source for this report.

## 5. Measurements
Method: Appendix D §D.3, `cargo xtask memprobe` (QueryWorkingSet shared-bit counting), median of runs, CI runner. These are not end-user hardware numbers.

| Binary | Idle PWS | After scroll | Handles / GDI / USER / modules | First frame | Scroll p99 | Exe size |
|---|---|---|---|---|---|---|
| yamdv.exe skeleton (5 runs) | **1.33 MB** | — | 150 / 12 / 12 / 29 | — | — | 480,768 B |
| spike A | **2.39 MB** | 2.61 MB | 169 / 11 / 11 / 32 | 98 ms | 4.16 ms | 602,112 B |
| spike B | 11.71 MB | 9.21 MB | 213 / 13 / 11 / 41 | 157 ms | 5.08 ms | 309,760 B |
| spike C | 11.72 MB | 12.39 MB | 227 / 9 / 12 / 44 | 152 ms | 31.73 ms | 311,296 B |

Notes:
- At simulated 150 %, PWS is A 3.01 MB, B 12.15 MB and C 11.01 MB.
- With the SegmentHeap manifest, idle PWS is A 2.57, B 9.83, C 10.01 and skeleton 1.36 MB. The default heap is kept (ADR-0002).
- WIC init costs about 3.5 ms and +60 KB.
- Every process exited cleanly after `WM_CLOSE`.

## 6. Decisions
- ADR-0001: toolchain 1.99.0 and hardening flags. `ProcessDynamicCodePolicy` is deferred.
- ADR-0002: backend **A**, default heap.
- ADR-0003–0006: dependencies `windows`/`windows-core`/`windows-numerics`, `object` (xtask), `serde_json` (xtask), `tiny-skia`.

## 7. Dependencies (S-SUP-02)
- Product (`cargo tree -e normal -p yamdv-app`): `yamdv-platform-win` → `windows 0.62.2` (+ `windows-core`, `-result`, `-strings`, `-link`, `-collections`, `-future`, `-numerics`, `-threading`; proc macros `windows-implement` / `windows-interface`).
- Dev only: xtask uses `object 0.40.0`, `serde_json 1.0.154`, `tiny-skia 0.12.0` (+png). Spikes use `tiny-skia`, `windows-core` and `windows-numerics`.
- Every licence is on the S-SUP-04 allowlist.

## 8. Security
- Repo is private. Actions are pinned by full SHA, with `permissions: contents: read`, `persist-credentials: false` and no secrets. Nothing is published or released, and Pages is not enabled.
- No tokens are in files or commits. No file is over 1 MB. No personal paths are committed.
- S-PROC-01…04 are implemented in `hardening.rs`. S-PROC-05 is verified by xtask. S-PROC-06 is covered by the `asInvoker` manifest; the elevation banner is planned.
- `check-imports`: no ws2_32, wsock32, winhttp, wininet, urlmon or similar.
- Note: `combase.dll` and `oleaut32.dll` are imported through `windows-core` error info.

## 9. Tests
- xtask: 15 unit tests. They cover the PE checker with synthetic allowed and forbidden fixtures, forbidden-function matching, malformed input, size logic, memprobe args, median and the JSON schema.
- Platform: window helper unit tests.
- App: `cli` unit tests, plus `#[cfg(windows)]` integration tests for `--version`, `--help` and an unknown flag (exit 2), which ran on CI.
- No fuzzing yet (planned).

## 10. Deviations from the prompt
- `ProcessDynamicCodePolicy` (SHOULD) is deferred to Phase 002 (ADR-0001).
- Positional file paths are parsed and then ignored until Phase 002.
- 150 % DPI is simulated with `--scale 1.5`.
- Spike D (optional) was not built.
- Spikes use their own relaxed lint table (E.9 vs H14).
- Appendix files were re-typed verbatim from the attachment; line counts were checked against the source ranges.

## 11. NOT VERIFIED / BLOCKED
- NOT VERIFIED: running the GUI on a local machine (the host is Linux).
- NOT VERIFIED: a real per-monitor DPI change and a real 150 % monitor.
- NOT VERIFIED: GPU hardware timing for spike C. The runner has no GPU, and C's ~16.7 ms frames look vsync-throttled.
- NOT VERIFIED: the dark-mode switch at runtime and the visual check of the dark title bar. The CI screenshot shows the light skeleton only.
- BLOCKED: none.

## 12. Spec defects found
1. R-LINK-05 points to S-EXEC-02, but the allowlist is in S-LINK-03.
2. R-MMD-03 cites B.6.12, but the palettes are in B.3.6.
3. R-THEME-06 uses a `border` token that B.3 does not define.
4. SegmentHeap (§4.7 measurement) conflicts with Appendix D §D.4 rule 8.
5. "MB" is ambiguous (10⁶ vs 2²⁰). This project uses 10⁶.
6. E.9 forbids unsafe outside platform-win, while H14 allows spikes. They conflict for `spikes/`.

## 13. Risks
- Spike A's CPU raster cost grows with large images, math and Mermaid. Re-measure p99 in Phases 003, 009 and 010.
- `windows-core` brings COM DLL imports.
- CI-runner numbers differ from user hardware.
- Exe size: tiny-skia adds about 290 KB, and icons about 230 KB.

## 14. Next phase (002) proposal
- Core document model.
- DirectWrite layout on the A backend in `yamdv-layout` and `yamdv-platform-win`.
- Virtualised scrolling with ScrollWindowEx.
- Default light/dark tokens.
- Golden-image tool based on the memprobe screenshots.
- Enable and verify `ProcessDynamicCodePolicy`.

## 15. Özet (Türkçe)
Faz 001 tamamlandı. Kurulanlar:
- Özel GitHub deposu ve Cargo çalışma alanı
- Sabitlenmiş Rust 1.99.0 ve güvenlik sertleştirmeli derleme
- Belgeler: ekler A–H, tehdit modeli iskeleti, izlenebilirlik tablosu, ADR'ler
- `cargo xtask` araçları
- SHA ile sabitlenmiş, salt okunur CI
- Yerel pencere iskeleti

Üç çizim altyapısı Windows CI üzerinde ölçüldü. **A seçildi:** DirectWrite GDI interop + tiny-skia.
- Boşta özel bellek 2,39 MB; B ve C yaklaşık 11,7 MB
- Kaydırmada p99 4,2 ms
- Çizim doğru
- İskeletin boşta özel belleği 1,33 MB

3–5 MB hedefi ulaşılabilir. Bilgisayarda yerel GUI testi ve gerçek GPU/DPI ölçümleri doğrulanmadı (Linux ana makine). Engelleyen bir sorun yok.

---

## Amendment (Phase 002, fix item F-04)

These template sections were missing from the original report and are added after the fact. The measured content above is unchanged.

### A. Acceptance criteria
| AC | Status | Evidence |
|---|---|---|
| AC-1 | PASS | Workspace, docs set and toolchain at tag `phase-001` (see §3) |
| AC-2 | PASS | `cargo xtask ci` gates green locally, and on CI run [38913424764](https://github.com/snipeTR/YAMDV/actions/runs/38913424764) (PR) and [38915093751](https://github.com/snipeTR/YAMDV/actions/runs/38915093751) (main) |
| AC-3 | PASS | Window skeleton: memprobe on CI, exit_ok true, 1.33 MB PWS (§5) |
| AC-4 | PASS | Three spikes measured on CI; ADR-0002 chooses backend A |
| AC-5 | PASS | check-imports: no networking DLLs, CFG/CET flags (§4) |
| AC-6 | PASS | CI is SHA-pinned, `permissions: contents: read`, no secrets |
| AC-7 | PASS (with one gap) | The report exists. The template sections missing from it are added in this amendment |

In Phase 001 the GUI behaviour (window, theme switch, Esc/Ctrl+W) was only checked on CI through memprobe and screenshots. Local checks: **NOT VERIFIED**, see owner checklist D.

### B. Repository state
- Branch `phase/001-foundation` → `main`, PR https://github.com/snipeTR/YAMDV/pull/1
- Merge commit `df8dabce384877eb5673312c07d4231c0a74e656`, tag `phase-001`
- CI: PR run 38913424764 (success), main push run 38915093751 (success); the measurement data comes from run 38911412595.

### C. Files added/changed (trimmed)
```
.cargo/config.toml  .github/{workflows/ci.yml,scripts/measure.ps1}  Cargo.toml  clippy.toml  deny.toml
assets/icons/*  crates/{yamdv-app,yamdv-core,yamdv-layout,yamdv-ui,yamdv-editor,yamdv-highlight,
yamdv-math,yamdv-mermaid,yamdv-export,yamdv-platform-win}/  spikes/  xtask/
docs/{SPEC,UI_SPEC,SECURITY,PERFORMANCE,ARCHITECTURE,SHORTCUTS,TESTING,ROADMAP,THREAT_MODEL,TRACEABILITY}.md
docs/adr/ADR-0000…0006  docs/plans/001_PLAN.md  docs/reports/001_REPORT.md  docs/spike/
```

### D. Manual QA checklist for the owner
1. Run `yamdv.exe`: a window titled "YAMDV" opens with the theme background and no flicker.
2. Switch Windows light/dark while it is open: the background follows immediately.
3. Press Esc, then start it again and press Ctrl+W: the window closes and the process exits (Task Manager shows no `yamdv.exe`).
4. `yamdv.exe --help` and `yamdv.exe --version` in a console print the help and the version, and exit with code 0. An unknown flag exits with code 2.
5. Spike A: `spike-a.exe --autoscroll --log a.log`. The reference scene renders correctly, scrolling is smooth and the title shows `SCROLL_DONE`.
From Phase 002 on, these steps are in the owner kit (`docs/OWNER_KIT.md`).
