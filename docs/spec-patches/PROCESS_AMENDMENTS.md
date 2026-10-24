## Amendments

**A-1 (Phase 002, prompt 002 F-01).** `spikes/` may use `unsafe` (excluded from the product; every block still has `// SAFETY:`), resolving the H14 vs E.9 conflict.

**A-2 (Phase 002, prompt 002 F-01).** Report section "0. Fix items status" and section "3b. Spec patches applied" are mandatory from Phase 002 on: `## 0. Fix items status` (table `F-ID | status | evidence`) before section 1, and `## 3b. Spec patches applied` (table `P-ID | doc | verified by spec-check (yes/no)`) after section 3. Sections with nothing to report say "None." and are never omitted.

**A-3 (Phase 003b, prompt 004 F-18).**
```text
A-3 Token economy (from Phase 003b on):
1. Read spec docs selectively: grep for the requirement IDs you are working on; never re-read whole docs or prompt files you have already read.
2. Never print large files or long command outputs into the conversation; use grep, head, tail -n 50 and quiet flags.
3. CI: run all gates locally (cross-compiled) before pushing; push at most 2–3 times per phase; wait with a single `gh run watch <id> --exit-status` — no polling loops; on failure read only `gh run view <id> --log-failed | tail -n 80`.
4. Goldens: regenerate once near the end of the phase, not after every change.
5. Fuzzing and long measurements: start them in the background, check once at the end.
6. ADRs ≤ 30 lines; the phase report ≤ 250 lines — tables, no repeated prose, never restate the prompt.
7. Fix things in one pass where possible; batch commits.
8. If something is ambiguous, choose the simplest option that meets the spec and note it in one line in the report.
9. If the phase cannot be finished, stop at a green, merged state and list exactly what is left (as in Phase 003) — never leave main red.
```

**A-4 (Phase V1-A, prompt 005 §2.1).**
```text
A-4 Delivery mode (owner decision; from Phase V1-A until the v1 release):
1. The priority is a complete, working v1 within the remaining budget. Choose the simplest implementation that meets the phase's acceptance criteria.
2. Tests: unit tests for new pure logic and one driver script per user-visible feature. No new fuzz targets except for new parsers (run them 5 minutes in the background). Goldens only where the prompt lists them.
3. Measurements: run the existing automated table only; build no new measurement tooling.
4. Docs: update TRACEABILITY by script. ADRs only for new dependencies or irreversible design choices, ≤ 15 lines each.
5. Report ≤ 120 lines: §0, an AC table, the measurements table, deviations, and what is left. Omit sections that have nothing to say (this overrides the template rule "never omit").
6. Push to CI once at the end of the phase, plus at most one fix push.
```

**A-5 (Phase V1-D, owner addendum 2026-10-17; applies until v1.0.0, overrides A-4 where different).**
```text
A-5 Owner addendum (V1-D until v1.0.0; restated unchanged by prompt 010 §1.1, which also reads prompt 008 §5 "private repo" as "this repository"):
1. Visibility: the owner made the repository PUBLIC (overrides H2's "private"). Agents never change visibility or any repository settings. Never commit secrets, tokens or personal data. CI keeps `permissions: contents: read`; only the tag/release job may use `contents: write`. Use `pull_request`, never `pull_request_target`. Releases stay drafts until the owner publishes them.
2. Reduced testing: (a) keep every existing test, golden and driver script — delete none. (b) New tests this phase: unit tests for new pure logic only (pagination, CSV); at most 2 new driver scripts (export-html, install/uninstall); no new goldens, no fuzzing; the exported-HTML context is added to the existing XSS suite. (c) CI measurement gates: only M0, M1 and T4 typical warm remain gates, with the retry rule (F-22 applies to these three only); every other measurement is report-only. (d) A flaky existing driver script may be marked report-only with a one-line note in the report.
3. Report ≤ 80 lines: status, AC table, measurements (M0, M1, T1, T4 typical, B-SIZE only), deviations, what is left, final owner QA list.
4. CI: one full run at the end of the phase plus at most one fix push.
```

**A-6 (Phase V1-D, owner addendum 2026-10-17; restated as a fallback by prompt 010 §1.2: it applies only while GitHub Actions cannot run (minutes or billing); otherwise A-5 §4 applies).**
```text
A-6 No-Actions fallback (owner decision; only when GitHub Actions cannot run):
1. Push nothing that triggers CI: every commit carries `[skip ci]` (or the push is branch-only). Do not re-run or wait for CI runs.
2. Verify locally: cargo fmt, clippy (native + cargo-xwin x86_64-pc-windows-msvc incl. dev-tools), tests of all platform-independent crates, deny, spec-check, check-imports and size on the cross-compiled exe.
3. Windows-only tests, goldens, driver scripts and measurements are deferred and marked "NOT RUN (A-6)" in the report; never simulated.
4. Test build: release exe built with cargo-xwin, zipped as `YAMDV-1.0.0-rc1-win-x64.zip` (yamdv.exe, README.md, LICENSE, CHANGELOG.md, FUTURE.md) plus its SHA-256 file, uploaded with `gh release create v1.0.0-rc1 --draft --title "YAMDV 1.0.0 RC1"` (uses no Actions minutes; an existing draft gets its assets replaced with `gh release upload --clobber`). Stays a DRAFT. Build artifacts stay out of git.
5. Do not merge to `main` and do not tag `v1.0.0` until a CI run is green (V1-D: PR #49 stays open; the owner also tests RC1 on a real PC).
6. The report (≤ 80 lines, A-5) includes a step-by-step manual QA list for RC1, replacing CI evidence.
```

**A-7 (Phase V2-A, prompt 011 §2 step 1; owner decision 2026-10-22).**
```text
A-7 v2 process (owner decision, 2026-10-22) — applies from Phase V2-A on; where it differs from A-4/A-5, A-7 wins.
1. Goal: implement docs/ROADMAP_V2.md phase by phase — the originally specified product — while staying as fast and light as v1.
2. "Light" gates in every phase: M0, M1, T4 typical warm and T7 idle CPU are CI gates (two-attempt retry rule). Per phase, M0 and M1 may grow by at most 0.3 MiB versus the previous phase unless the prompt allows more. Report the exe-size delta. Idle CPU must stay at 0: timers run only while something animates.
3. Tests: unit tests for new pure logic; at most 3 new driver scripts and 4 new goldens per phase (P-28 total ≤ 12 MB still holds).
4. For every implemented item: update TRACEABILITY by script (deferred → DONE); close the GitHub issue through the merged PR ("Closes #n" in the PR body; find the numbers with `gh issue list --search "<ID> in:title"`).
5. Report ≤ 80 lines. One full CI run at the end, plus at most one fix push. If the quota runs low: commit with [skip ci], write the report with "Next action", stop.
6. Releases at the versions in docs/ROADMAP_V2.md, as drafts; the owner publishes them.
```

**A-8 (Phase V2-B, prompt 012 §2 step 1; owner decision 2026-10-22).**
```text
A-8 No CI; the owner runs the Windows tests (owner decision, 2026-10-22). Applies from Phase V2-B until the owner turns CI back on; where it differs from A-7, A-8 wins.
1. Every commit message ends with [skip ci], which also skips pull_request runs. Do not change workflows or repository settings.
2. Local gates before every push: fmt; clippy native + cargo-xwin x86_64-pc-windows-msvc, with and without dev-tools; tests of the platform-independent crates; deny; spec-check; check-imports; size.
3. Put as much new logic as possible into platform-independent crates with unit tests, so it is verified without Windows.
4. Windows-only checks (driver scripts, golden compare, M0/M1/T4/T7) are written as usual; the owner runs them from the test pack, and the report marks them "OWNER (A-8)". No new goldens: planned goldens become screenshots in the pack's screens\ folder for the owner to look at.
5. Test pack, once per phase:
   - YAMDV-<phase>-test.zip plus its SHA-256, assembled by tools/testpack/build-pack.sh from the MSVC exes of a green CI run (artifact pack-exes); since the V2-B fix, cargo-xwin/lld-linked exes are not used because they crash at startup on Windows;
   - uploaded as the draft pre-release test-<phase> (gh release create test-<phase> --draft --prerelease; to replace assets, gh release upload --clobber);
   - contents: yamdv.exe (release), yamdv-dev.exe (dev-tools), baseline\yamdv-dev.exe (the dev-tools build of main before this phase, for same-PC comparisons), the driver scripts, the existing goldens, the fixtures, run-tests.cmd and OWNER_TEST.md (numbered manual checks in EN and TR).
6. run-tests.cmd:
   - one double-click, no admin rights;
   - runs every driver script, the golden compare and M0/M1/T4/T7 the way CI did, and M0/M1 also for the baseline;
   - writes test-results.txt (one line per test: PASS / FAIL / SKIP + numbers + delta vs the baseline) and puts the diff PNGs of failures in failures\;
   - golden diffs on the owner's PC are reported with their pixel percentage and are not a gate (DPI, fonts and ClearType differ from the CI runner);
   - uses a temporary config folder and never touches the owner's settings, history, favorites, registry, or any files outside its own folder and %TEMP%;
   - scripts that touch the registry or install anything are skipped unless --include-system is given;
   - prints the expected duration and asks the owner not to use the mouse or keyboard while it runs.
7. Gates on the owner's run: M0 and M1 at most +0.3 MiB vs the baseline on the same PC; T7 = 0; T4 is reported.
8. Merge: after the local gates pass, merge to main and tag phase-<x>. Version tags (v1.x.0) and their releases wait for the owner's OK; the next prompt then tags them and uploads the zip with gh (no Actions). Owner-reported failures go first in the next prompt.
9. Report ≤ 80 lines: a local-gates table, the pending owner tests, and the test-pack link.
```

**A-9 (Phase V2-C, prompt 013 §2 step 1; owner decision 2026-10-23).**
```text
A-9 CI is back (owner decision, 2026-10-23). The repository moved to https://github.com/abuzer1495/YAMDV, where GitHub Actions minutes are available. A-8 ends and A-7's CI rules apply again; where they differ from A-8, A-9 wins.
1. CI gates as in A-7: M0, M1, T4 typical warm and T7 (two-attempt retry rule). M0/M1 may grow by at most 0.3 MiB per phase. At most 3 new driver scripts and 4 new goldens per phase.
2. Minute and token economy: one full CI run at the end of a phase plus at most one fix push. WIP pushes use [skip ci].
3. Kept from A-8:
   - run-tests.cmd and the test-pack builder stay working, with no new features; build a test pack only when the owner asks;
   - new pure logic goes into platform-independent crates with unit tests.
4. Version tags are set when CI is green on main. The tag job creates a draft release; the owner tests the draft by hand and publishes it himself. Problems he reports go first in the next prompt.
5. Every PR run uploads the release exe as the artifact yamdv-exe (retention 7 days), so the owner can try a phase before its release.
6. Every report ends with an owner check list (≤ 8 steps, EN + TR) covering what CI cannot judge.
7. Security rules are unchanged:
   - no secrets in the repository;
   - workflows keep permissions: contents: read, and only the tag/release job may write;
   - pull_request, never pull_request_target;
   - releases stay drafts.
8. F-24 (prompt 014): a version or phase tag goes on a squash-merge commit only when its tree equals a PR head with a green CI run (`git diff --quiet <pr-head> <merge>`); otherwise run CI on main first.
9. F-25 (prompt 015): A version tag always goes on a `chore(release): vX.Y.Z` commit without [skip ci] whose tree equals a green CI head; phase tags may stay on the squash merge.
10. F-32 (prompt 017): phase-* tags trigger no workflow; v* tags run only the release job (build, zip, SHA-256, draft); a push whose head commit message starts with `chore(release):` skips the test jobs (its tree equals a green head, F-25); F-31: pws_delta_kib / cpu_delta_ms driver asserts get the two-attempt rule (a script failing only on them is rerun once).
```

**A-10 (Phase V2-D, prompt 014; owner decision 2026-10-23).**
```text
A-10 Owner override (2026-10-23, verbatim): "testleri azalt gereksiz test olmasın birleştir yinede" (reduce the tests, no unnecessary tests, merge anyway).
1. Redundant tests (same feature, same assertions as another test) are removed; timing-noise asserts (one timer tick, 15.6 ms) are report-only. Goldens, the M0/M1/T4/T7 gates, pack smoke and security checks stay.
2. PR #83 (V2-D) is squash-merged although its last CI run is not green. F-24 is waived for this merge only and the report says so; the phase-v2d and v1.2.0 tags go on that merge commit.
3. Releases stay drafts; repository settings are not changed. A-9 applies again from the next phase.
```
