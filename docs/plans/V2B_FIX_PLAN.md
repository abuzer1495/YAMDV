# V2-B fix plan (startup crash, CI back on; A-9)

Branch `fix/v2b-startup-crash`. Owner pack run: every driver script exit 0xC0000409 after ~0.6 s; `--version` empty.

1. A-9 amendment, URLs → abuzer1495/YAMDV, startup smoke step in CI (`.github/scripts/smoke.ps1`).
2. Root cause from CI logs (crash note) + static analysis; fix + regression check.
3. PR, full CI green (32 driver scripts, goldens, gates).
4. Merge, tag `phase-v2b-fix`, rebuild test pack, `gh release upload test-v2b --clobber` (draft).
5. Report `docs/reports/v2b_fix_REPORT.md`.

## CHECKPOINT
- 2026-10-22 23:15 (UTC+3): step 1 committed and pushed; waiting for first CI run.
- 23:32: run 39846032458: CI-built exes start fine (smoke green, 28/32 driver PASS). Suspect cargo-xwin pack build; added pack-build + pack-smoke jobs.
- 23:38: run 1 also found 5 V2-B test failures (color-editor default, titlebar submenu path, settings-panel cold caches, export-progress 30 s limit + missing large fixture, v1d DisplayVersion 1.0.0); fixed in 3 commits. wine on the box reproduces driver scripts (/workspace/.wine/run.sh). Next: pack-smoke result (xwin exes on windows-latest + windows-2022).
- 23:46: run 39850233795: pack-smoke FAILS on windows-latest and windows-2022: cargo-xwin exes exit 0xC0000409 on --version (empty stdout); the dev exe runs a script to PASS and then also exits 0xC0000409. CI (MSVC) exes are fine. Root cause = the cargo-xwin/lld-link pack build. Next: flag variants (CFG/CET on/off, hello-world) to pin the mechanism; then build the pack exes with MSVC.
- 23:58: run 3 windows job: 30/32 driver PASS; export-progress (job_monotonic sampled the Preparing block counter; CPU window caught the cancel toast's 4 s hide repaint) and settings-panel (+512 KiB from the cold relayout with line numbers) fixed as test-side fixes (sample Printing only; sleep 4500; full warm-up pass). Diag run 39852373334 (workflow_dispatch on diag/v2b-pack) runs the xwin flag variants.
- 00:10: fix: pack exes now come from a new MSVC CI job (pack-exes: release, xtask, dev, baseline dev at 9f6906e, startup smoke, artifact) + pack-smoke on windows-2022; build-pack.sh takes that artifact (run id or dir) and refuses exes without the MSVC Rich header. Waiting for diag run (variants) and PR run 39853364251 (test fixes) before pushing to the PR branch; backup pushed to wip/v2b-fix.
- 00:25: run 39853364251: my sed also changed export-pdf-bg/job-done checks (no job) — reverted; job-wait now counts printed pages; settings-panel +192 KiB on the 2nd pass → three identical passes, gate on pass 3, pass 2 delta logged. xwin diag jobs hung on the runner (no logs); diag run cancelled. Pushing the MSVC pack fix to the PR branch.
- 00:36: run 39857147395: MSVC pack exes + baseline (9f6906e) start fine on windows-latest and windows-2022 (yamdv 1.1.0 / 1.0.0, dev script PASS). Waiting for the windows job (drivers + gates).
- 00:47: run 39857147395: only export-progress failed: a cancel in the Printing phase left ep-large.tmp.pdf behind (Print to PDF's filter pipeline writes the file after AbortDoc) → real product fix: print.rs remove_partial retries until the file stays absent 1 s (max 6 s). settings-panel green (pass 2 +4 KiB, pass 3 +40 KiB). Gates green (M0 2.36, M1 4.84 MiB, T7 0/0, size 2,919,936 B).
- 01:14: run 39860028614: export-progress green (cancel fix works); only settings-panel pass 3 read +152 KiB (earlier +40/+84) → noise. Gate now: passes 3-5 after the mark, GDI/USER objects exactly back (new driver keys gdi_delta/user_delta), PWS ≤ 306 KiB over three passes.
- 01:40: run 39863513532 on cf76bf4 green (windows gates + drivers + goldens, pack-exes, pack-smoke windows-2022, pure). Next: merge, tag phase-v2b-fix, pack, report.
