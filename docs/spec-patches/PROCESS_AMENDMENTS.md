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
