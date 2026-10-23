# YAMDV — V2-B fix report: startup crash, CI back on (A-9)
Branch `fix/v2b-startup-crash`, PR #80, squash-merged as a5ebb9a, tag `phase-v2b-fix`. Repo abuzer1495/YAMDV (gh account abuzer1495).

## 1. Root cause
The V2-B test pack exes were cross-built on Linux with cargo-xwin (lld-link). On Windows they exit with
0xC0000409 at startup: `--version` prints nothing, and the dev exe even finishes a script and then dies.
The MSVC-built exes of the same commit start fine. This was shown on windows-latest and windows-2022 in run
39850233795 (pack-smoke jobs). The product code was not the cause. The exact faulting site is not pinned (§7).

## 2. Fix
- CI job `pack-exes` (windows-latest, MSVC) builds yamdv, xtask, the dev-tools exe and the baseline dev exe
  (9f6906e), starts them (startup smoke + baseline `--version`) and uploads them as an artifact;
  `pack-smoke` starts them again on windows-2022.
- `tools/testpack/build-pack.sh <run-id|dir>` assembles the pack only from that artifact and refuses exes
  without the MSVC Rich header.
- The smoke step scans %LOCALAPPDATA%\YAMDV\logs for crash notes; run-tests copies crash notes into failures\.
- Found by CI and fixed in the product: a cancelled PDF export could leave the file behind, because Microsoft
  Print to PDF writes it after AbortDoc. `print.rs` now keeps removing it until it stays gone for 1 s (max 6 s).
- Test fixes: color-editor default, titlebar submenu path, export-progress (page counter only while
  printing; CPU mark after the toast hides), settings-panel (5 passes; passes 3–5 gate GDI/USER objects
  exactly back and PWS ≤ 306 KiB).
- PROCESS A-8 §5 now says the pack is built from the CI MSVC exes (owner request, with this report).

## 3. CI runs
| Run | Result |
|---|---|
| [39846032458](https://github.com/abuzer1495/YAMDV/actions/runs/39846032458) | first run with CI back on: V2-B driver failures |
| [39850233795](https://github.com/abuzer1495/YAMDV/actions/runs/39850233795) | xwin pack exes crash on both runners (proof); MSVC fine |
| [39852373334](https://github.com/abuzer1495/YAMDV/actions/runs/39852373334) | diagnosis variants hung; cancelled |
| [39857147395](https://github.com/abuzer1495/YAMDV/actions/runs/39857147395) | only failure: export-progress partial file after cancel |
| [39860028614](https://github.com/abuzer1495/YAMDV/actions/runs/39860028614) | only failure: settings-panel single-pass noise (+152 KiB) |
| [39863513532](https://github.com/abuzer1495/YAMDV/actions/runs/39863513532) | **green**: 32/32 driver scripts, goldens, gates, pack-exes, pack-smoke, pure |

## 4. Gates (run 39863513532, windows-latest)
| ID | Value | Budget |
|---|---|---|
| M0 PWS | 2.35 MiB (2.36 in the T7 probe) | ≤ 4.8 MiB; ≤ +0.3 vs V2-A 2.35 |
| M1 PWS | 4.71 MiB | ≤ 8.0 MiB; ≤ +0.3 vs V2-A 4.66 |
| T4 typical warm p99 | 8.59 ms (cold 10.12) | ≤ 16 ms |
| T7 idle CPU empty / doc | 0 / 0 µs | 0 |
| yamdv.exe | 2,919,936 B (2.78 MiB), +88,064 B vs V2-A | ≤ 4.0 MiB |

## 5. Test pack
Rebuilt from run 39863513532: SHA-256 `0626a0b993e725b391ee1ea770182e9a3679562623453fc289954f7f7a16fcdb`.
Not uploaded: prompt 013 retires the test-v2b draft.

## 6. Deviations
- Commit 808df13 carried `[skip ci]` despite the no-skip rule on this branch; its line ended up in the squash
  message of a5ebb9a, so neither the main push nor the `phase-v2b-fix` tag push ran CI (the tree equals the
  green PR head ead3674).
- Six PR runs instead of one plus one fix push.
- Temporary branches `diag/v2b-pack` and `wip/v2b-fix` were created and deleted.
- Diagnosis run 39852373334 hung the runners and gave no data on the crash mechanism.
- settings-panel: the gate moved from one pass to passes 3–5 (single-pass noise 40–152 KiB); new driver
  keys `gdi_delta` / `user_delta`. export-progress: phase-aware page sampling; sleep after cancel 1.5 → 4.5 s.
- No new V2-B goldens were added.

## 7. Open items (owner)
1. The v1.0.0-rc1 draft release was built with cargo-xwin and very likely crashes the same way. Left alone
   by owner decision.
2. The crash mechanism of lld-linked builds (CFG / CET / static CRT interplay?) is not pinned; the
   Rich-header check in build-pack.sh blocks such exes.
3. v1.1.0 is tagged in V2-C (prompt 013 §2 step 4), after CI is green on main.

## 8. Next action
Prompt 013 (V2-C): A-9 official text, v1.1.0 draft release, then the V2-C features.
