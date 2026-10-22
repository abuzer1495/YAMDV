# YAMDV — Phase V2-B Report: settings panel (prompt 012)
Prompt: `docs/prompts/012_V2B_SETTINGS_PANEL.md`. Rules: PROCESS A-1…A-8 (A-8: no CI; every commit `[skip ci]`; Windows-only results are OWNER).
Branch `phase/v2b-settings`, PR #79, squash-merged as 9119b44, tagged `phase-v2b`. Version 1.1.0 is prepared, **not tagged**.
Issues closed: #77 BUG-10, #78 BUG-11, #30 FUT-31, #11 FUT-04, #47 FUT-49, #12 FUT-05. #25 FUT-26 got a comment (search done) and stays open.

## 1. Acceptance criteria
| AC | Status | Evidence |
|---|---|---|
| AC-1 §2 setup | pass | A-8, ROADMAP_V2, `tests/fixtures/colors.md`, P-34…P-39 (`spec-check` green), issues #77/#78, R-EXP-07 in TRACEABILITY |
| AC-2 BUG-10 / BUG-11 | pass (code + unit tests); runtime OWNER | `yamdv_core::theme::export_palette`, `yamdv_core::progress` tests; worker job + card in `window/jobui.rs` |
| AC-3 §4.1–4.6 | pass (code + unit tests); runtime OWNER | `yamdv-ui::settings_panel`, `yamdv-core::keymap`, `window/setpanel.rs`; fonts/colour lists load on demand only; 1.1.0 + CHANGELOG |
| AC-4 gates, PR, pack | pass | local gates below; PR #79 merged; `phase-v2b`; draft pre-release `test-v2b` |
| AC-5 report | pass | this file |

## 2. Local gates (phase/v2b-settings before the merge; baseline main 9f6906e)
| Gate | Result |
|---|---|
| `cargo fmt --check` | green |
| clippy native, `-D warnings` (pedantic) | green |
| cargo-xwin clippy x86_64-pc-windows-msvc, with / without `dev-tools,mem-stats` | green / green |
| platform-independent tests | 272 passed, 0 failed (main before V2-B: 253) |
| `cargo deny check` | advisories, bans, licenses, sources ok |
| `xtask spec-check` | match (P-01…P-39) |
| `xtask check-imports` (xwin release) | OK, 19 DLLs, no forbidden imports, CET/hardening flags |
| `xtask size` (xwin release) | 2,916,352 B (2.78 MiB) ≤ 4.0 MiB soft |
| M0 / M1 / T4 / T7 | OWNER (A-8): `run-tests.cmd` |

Exe size (cargo-xwin release, same toolchain): 2,826,752 B (9f6906e) → 2,916,352 B = **+89,600 B**.
Dev-tools build: 2,900,480 B → 3,006,976 B.

## 3. Owner test pack `test-v2b` (draft pre-release)
- https://github.com/snipeTR/YAMDV/releases (draft `test-v2b`, assets `YAMDV-v2b-test.zip` + `.sha256`)
- SHA-256 `1e142d9f53fa19ba63d404ed4f535a68551915dd915b226eb7970b0ca4c88a7e`
- Contents: `yamdv.exe` (release), `yamdv-dev.exe` (dev-tools), `xtask.exe`, `baseline\yamdv-dev.exe`
  (dev-tools build of main 9f6906e), `tests\` (driver scripts, fixtures, goldens, corpus), `run-tests.cmd` / `.ps1`,
  `OWNER_TEST.md`. Built by `tools/testpack/build-pack.sh`.
- `run-tests.cmd`: every driver script with a temp APPDATA/LOCALAPPDATA, golden / golden-shell / acg-matrix (report only,
  diffs to `failures\`), headless export checks, M0/M1 (release budget + V2-B vs baseline ≤ +0.3 MiB), T7, T4,
  install/uninstall only with `--include-system`. Writes `test-results.txt`; screenshots go to `screens\`.

## 4. OWNER (A-8) tests
1. `run-tests.cmd`: 32 driver scripts incl. the 4 new ones (`settings-panel`, `color-editor`, `shortcuts`,
   `export-progress`), updated `titlebar`; M0/M1 delta gate; T7 = 0; T4 reported; goldens reported.
2. The 12 manual items in `OWNER_TEST.md` (export colours ×4 + Follow theme, large PDF export with card/taskbar/Cancel,
   Print + close confirm, large HTML export, panel ×3 ways + keyboard + search, theme/fonts, colour editor, shortcuts +
   Typora, Clear history + copy path, Turkish UI, open V2-A checks).
3. Review of `screens\`: General (light), Appearance (dark), colour picker, Shortcuts, progress card.

## 5. Deviations
- No CI run (A-8); nothing Windows-only was simulated.
- ChooseFontW: the dialog-host kind `Font` stays in dialog_host/ipc but the app no longer uses it (Typography section).
- The panel PWS check is in-process (private working set, no trim) via `ui mem-mark` / `pws_delta_kib`.
- The colour hex input is driven by typing; no separate verb.
- export-progress Run 3: no pixel probes of print page 1 (the dev build has no print-page raster path). It checks the
  HTML body background `#ffffff` in dark UI and a successful PDF job; the PDF look is OWNER item 2.
- export-progress Run 2 "T7 = 0" is checked in-script as CPU ≤ 15 ms over 3 s (the driver timer itself wakes every
  60 ms); the real T7 runs through memprobe in `run-tests.cmd`.
- 96 DPI: the dev build has no DPI override; `run-tests.ps1` uses the per-process `__COMPAT_LAYER=DPIUNAWARE` when the
  system DPI ≠ 96 (driver + golden runs only).
- The M0/M1 delta gate compares dev-tools builds (baseline is a dev-tools build per the prompt); release builds are
  checked against the absolute budgets. CI report-only probes M2/M3/M5/M6/T3/T5/T10 are not in the pack.
- `xtask` finds the repo root via `YAMDV_ROOT`, else the build tree, else the current folder (pack use).
- Only #77 was closed by the PR keyword; #78/#30/#11/#47/#12 were closed by hand, referencing #79.
- `.github/scripts/v1d-checks.ps1` still expects DisplayVersion 1.0.0; CI is off and workflows/scripts are not touched
  (A-8). The pack reads the version from `yamdv.exe --version`.

## 6. Next action
Owner: unzip `test-v2b`, run `run-tests.cmd`, work through `OWNER_TEST.md`, send `test-results.txt` (+ `failures\`).
Then: fix what the results show; when the owner signs off, publish 1.1.0 (tag `v1.1.0`) per A-8.
