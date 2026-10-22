# 010_V1E_RESUME_AND_RELEASE — YAMDV Phase V1-E: Resume V1-D, finish, release v1.0.0

> **Prompt 010.** Phase **V1-E** finishes the work of prompt 008 (V1-D), which was paused when the Grok quota and the GitHub Actions minutes ran out.
> - Branch `phase/v1d-release` (keep it; create it from `main` if it does not exist)
> - Tags `phase-v1e` and **`v1.0.0`**
> - Report `docs/reports/v1e_REPORT.md` (≤ 80 lines)
> - Store this file as received in `docs/prompts/`
>
> **Precedence:** `docs/PROCESS.md` (A-1…A-4, plus A-5 and A-6 below) > this prompt > prompt 008 (`docs/prompts/008_V1D_EXPORT_PACKAGING_RELEASE.md`) > `docs/*`. **Do not redo finished work.**
> **Resumable:** if the quota runs low again, follow §6 and stop. This same prompt can be given again to continue.

---

## 1. Owner decisions since prompt 008

1. **Amendment A-5** — add it to `docs/PROCESS.md` if it is not there yet:
```text
A-5 (owner decision, 2026-10-17) — public repository and reduced testing, until v1.0.0 is tagged; where it differs from A-4, A-5 wins.
1. The owner made the repository PUBLIC; this overrides H2's "never make the repo public". Do not change visibility or repository settings yourself.
   - Never commit secrets, tokens or personal data.
   - CI keeps `permissions: contents: read`; only the tag/release job may use `contents: write`.
   - Use `pull_request`, never `pull_request_target`.
   - Releases stay drafts until the owner publishes them.
2. Reduced testing:
   - Keep every existing test, golden and driver script.
   - New tests: unit tests for new pure logic (pagination, CSV); at most 2 new driver scripts (export-html, install/uninstall); no new goldens; no fuzzing; the exported-HTML context is added to the XSS suite.
   - CI measurement gates: only M0, M1 and T4 typical warm, with the two-attempt retry rule (F-22 applies to these three only); every other measurement gate becomes report-only or is removed.
   - An existing driver script that is flaky on the runner may be marked report-only, with a one-line note.
3. Report ≤ 80 lines.
4. One full CI run at the end, plus at most one fix push.
```

2. **Amendment A-6 (fallback)** — add it to `docs/PROCESS.md`. It applies **only** if GitHub Actions cannot run when you reach §4:
```text
A-6 No-Actions fallback (owner decision): when GitHub Actions cannot run (minutes or billing):
- Push only with `[skip ci]`.
- Verify locally: fmt, clippy (native + cargo-xwin for x86_64-pc-windows-msvc incl. dev-tools), tests of the platform-independent crates, deny, spec-check, check-imports, size.
- Mark Windows-only tests, goldens, driver scripts and measurements "NOT RUN (A-6)".
- Build `YAMDV-1.0.0-rc1-win-x64.zip` locally (yamdv.exe, README.md, LICENSE, CHANGELOG.md, FUTURE.md) plus its SHA-256, and upload it with `gh release create v1.0.0-rc1 --draft --title "YAMDV 1.0.0 RC1"` (this uses no Actions minutes).
- Do not merge to `main` and do not tag `v1.0.0` until a CI run is green.
```

3. **The repository is public.** In prompt 008 §5, read "in the **private** repo" as "in this repository".

4. **The post-v1 issues already exist.** Prompt 009 created them: 67 issues in milestone `post-v1`, including FUT-01…FUT-50. Prompt 008 §5.2 is therefore reduced to one step: make `cargo xtask future-issues` (if it exists) skip existing ID prefixes, then run it once as a check; it should create 0. If it does not exist, do not build it; `tools/backlog/create_issues.py` is the tool.

5. **License:** issue DEC-01 is still open, so keep the LICENSE placeholder. The README says "License: to be decided (see issue DEC-01)".

---

## 2. Step 1 — State audit (short, no new code)

1. Commit any uncommitted work on the branch (`[skip ci]`).
2. Collect the state:
   - `git log --oneline -15`
   - `gh pr list`
   - `gh run list -L 3`
   - the existing tags
3. Write the audit table at the top of the report: one row per prompt-008 item, status done / partial / not started, evidence = a file or commit.

| Items to audit |
|---|
| F-22 (now: the 3 gates only) |
| §2.1 Export HTML |
| §2.2 CSV |
| §2.3 headless CLI |
| §3.1 print dialog-host kind |
| §3.2 pagination |
| §3.3 band printing |
| §3.4 Export PDF / `--export-pdf` |
| §3.5 trim after print |
| §3.6 Export menu |
| §4.1 `--install` |
| §4.2 `--uninstall` |
| §4.3 welcome-page install button |
| §4.4 version 1.0.0 + CHANGELOG |
| §5.1 README |
| §5.3 release workflow (tag job: zip + SHA-256 + draft release, no tests) |
| §6 tests (A-5 reduced set) |

If the prompt-009 report's "V1-D snapshot" is available to you, start from it and verify it quickly.

## 3. Step 2 — Finish the remaining items
Work through the items that are not done, in prompt-008 order (§2 → §3 → §4 → §5), with A-5 test reduction. Keep them small and simple: the specification is prompt 008, and nothing outside it is added.

## 4. Step 3 — Verify and release
1. Run all local gates (see A-6, list item 2), whether or not Actions works.
2. Make **one** CI run on the PR. GitHub-hosted standard runners are free for public repositories.
3. **If CI cannot start** (billing/quota): apply A-6 — build and upload RC1 as a draft, report, and **stop here**. The owner tests RC1 on a real PC and gives this prompt again later.
4. **If CI is green** (one fix push allowed):
   - merge the PR;
   - tag `phase-v1e` and `v1.0.0`;
   - the tag job creates the **draft** release `v1.0.0` with the zip and its SHA-256;
   - verify that the draft exists (`gh release view v1.0.0`) and that `post-v1` has ≥ 67 issues.

## 5. Report (≤ 80 lines)
1. The audit table (before → after).
2. AC table (§7).
3. Measurements: M0, M1, T1, T4 typical, B-SIZE.
4. Deviations.
5. What is left: point to issue numbers where they apply.
6. **The final owner QA list for the whole v1** (≤ 15 steps, English + Turkish): install; set as default app; double-click open; reading (images, tables, code, math, Mermaid); sidebar, themes, find, zoom; live reload; F2/F3 editing with autosave and a conflict; Turkish Q input; Export HTML / Export PDF / Print; CSV; uninstall.

## 6. Stop rule (quota)
If the Grok quota runs low at any point:
- commit and push the current state with `[skip ci]`;
- write `docs/reports/v1e_REPORT.md` containing the audit table and **"Next action: …"**;
- stop.

The owner will give this same prompt again.

---

## 7. Acceptance criteria
| ID | Criterion |
|---|---|
| AC-1 | A-5 and A-6 are in `PROCESS.md`; the audit table is in the report. |
| AC-2 | Every prompt-008 item is done (or, under A-6, every item that can be verified without Windows CI). |
| AC-3 | **Normal path:** CI green, merged, tags `phase-v1e` + `v1.0.0`, draft release with zip + SHA-256. **A-6 path:** draft `v1.0.0-rc1` with zip + SHA-256, nothing merged. |
| AC-4 | README is complete and links to `docs/FUTURE.md` and to milestone `post-v1`; the final owner QA list is in the report. |

**END OF PROMPT 010.** Start with §1 now.
