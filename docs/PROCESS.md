# YAMDV — Process (hard rules, methodology, report template)

> Sections 2, 3 and 7 below are copied programmatically from `docs/prompts/001_FOUNDATION_AND_SPIKE.md` by
> `cargo xtask spec-patch`; `cargo xtask spec-check` verifies them. Do not edit them by hand.

## 2. Hard rules (non-negotiable — apply in every phase)

**H1 — Location.** The whole project lives in **one folder named `YAMDV` directly inside your workspace folder** (`<workspace>/YAMDV/`). Every file you create (source, docs, scripts, test data, CI config, spike code, reports) goes inside it. Never write anywhere else except toolchain caches (`~/.cargo`, `~/.rustup`), the OS temp directory for build intermediates, and the `target/` directory inside `YAMDV/`.

**H2 — GitHub.** Use the GitHub account that is already authenticated in the `gh` CLI on this machine (`gh auth status`; record the account name in the report). Create a **private** repository named **`YAMDV`** and push to it.
- MUST NOT make the repo public, change its visibility, add collaborators, create public releases/packages, enable GitHub Pages, or publish anything anywhere.
- MUST NOT ask for, print, or store tokens/credentials in any file or commit.
- If `gh` is not authenticated or repo creation fails: complete all local work, skip remote steps, and report `BLOCKED: GitHub` with the exact error.

**H3 — Offline.** The shipped application performs **zero network I/O** of any kind (no HTTP, no DNS, no sockets, no update checks, no telemetry, no crash upload, no font/engine downloads, no remote images). Enforcement is described in Appendix C (`S-NET-*`) and is automated from this phase on.

**H4 — Technology.** Rust stable (pinned in `rust-toolchain.toml`, ≥ 1.85, edition 2024). Target `x86_64-pc-windows-msvc`. Raw Win32 + COM (DirectWrite etc.) via the `windows` crate.
**Forbidden** in the product (not even transitively): WebView/WebView2, wry, tao, Tauri, Electron, CEF, any HTML/CSS/JS engine, any JS runtime (QuickJS, Boa, V8, rquickjs…), GUI frameworks (egui, iced, slint, druid, xilem, makepad, dioxus, gtk, fltk, qt, sciter, nwg/native-windows-gui), windowing/graphics stacks (winit, sdl2, glutin, wgpu, glow, skia-safe, vulkano), async runtimes (tokio, async-std, smol), and every networking crate (list in `S-SUP-03`).

**H5 — Dependencies.** Keep them minimal. Pre-approved crates (use only when the phase needs them; pin exact minor versions):
`windows` (0.62.x, only the features you use), `pulldown-cmark` (0.13.x, `default-features = false`), `encoding_rs`, `chardetng`, `unicode-segmentation`, `ropey` (1.6.x), `tiny-skia` (only if chosen by the spike), `pdf-writer` + `subsetter` (export phase only).
Dev/test-only (never linked into `yamdv.exe`): `proptest`, `libfuzzer-sys`, `object` (for the PE import checker), `serde_json` (xtask output only), `tiny-skia` (xtask icon generation and spikes).
Anything else requires an ADR (`docs/adr/`) with: purpose, alternatives, binary-size delta, measured RAM delta, transitive dependency count, license, maintenance status. No git dependencies, no path dependencies outside `YAMDV/`, no build scripts that download anything.

**H6 — Budgets.** The memory/performance budgets in Appendix D are requirements. From Phase 002 on, every report includes the measurement table. Any regression > 10 % versus the previous phase must be explained.

**H7 — Security.** Appendix C applies to every line of code from day one.

**H8 — No scope creep.** Implement only the current phase. Architecture stubs needed for later phases are allowed only if trivial and marked `// PHASE-NNN: <what>`. Never report a stub as a feature.

**H9 — Quality gates (all MUST be green before you report "done"):**
`cargo fmt --all --check` · `cargo clippy --workspace --all-targets -- -D warnings` · `cargo test --workspace` · `cargo deny check` · `cargo xtask check-imports` · `cargo xtask size`. From Phase 002 on also `cargo xtask memprobe` against the budgets.

**H10 — Honesty.** Never claim something works unless you ran it and saw it work. If you could not run or verify something (e.g. no Windows host for GUI checks), write `NOT VERIFIED — <reason>`. Measurements are real numbers with the method stated; never present estimates as measurements.

**H11 — Docs live with code.** Update docs in the same commit as the code they describe. Keep `docs/TRACEABILITY.md` current. Every phase ends with `docs/reports/NNN_REPORT.md`.

**H12 — Git discipline.** Conventional Commits (`feat(render): … (R-REN-03)`), small focused commits. Work on branch `phase/NNN-<slug>` (this phase: `phase/001-foundation`), open a PR into `main`, merge only when all gates and CI are green, then tag `phase-NNN`. Never force-push `main`; never rewrite pushed history. Never commit secrets, personal paths, files > 1 MB (except deliberately generated test fixtures under `tests/corpus/large/`, which SHOULD be generated at test time instead), or third-party copyrighted content.

**H13 — Autonomy.** Do not stop to ask questions mid-phase unless truly blocked. Make reasonable decisions, record significant ones as ADRs, and list them in the report.

**H14 — Code rules.**
- English identifiers, comments, docs, commit messages.
- `unsafe` is forbidden (`#![forbid(unsafe_code)]`) in every crate except `yamdv-platform-win` (and `xtask` where needed for Win32 memory queries). Every `unsafe` block has a `// SAFETY:` comment explaining why it is sound.
- No `unwrap()` / `expect()` / `panic!` / unchecked indexing on data derived from files, user input, OS calls, or config. Allowed only for true invariants, marked `// INVARIANT: …`. Tests may unwrap freely.
- No `println!` debugging left behind. Use the internal logger (off by default).
- Every public item in pure crates has a doc comment.

**H15 — No fake progress.** No placeholder code presented as done; no tests that assert nothing; no disabled tests without a tracked reason.

## 3. Development methodology

**Model:** phase-gated, risk-driven, iterative–incremental development (Spiral-style risk analysis at the start of each phase, Scrum-style shippable increments, V-model verification at the end of each phase).

**Every phase follows this cycle:**
1. **Understand** – read the prompt and the referenced spec sections; restate the phase goals in your own words at the top of the plan.
2. **Plan** – write `docs/plans/NNN_PLAN.md` *before coding*: task breakdown, requirement IDs covered, top risks and how you'll retire them early, test plan, measurement plan.
3. **Build** – vertical slices, smallest useful increments, commit each; keep the build green.
4. **Verify** – automated tests (unit, integration, golden, fuzz where applicable), budget measurements, security checks, and a written manual QA checklist for the owner (who tests on a real Windows PC).
5. **Report** – `docs/reports/NNN_REPORT.md` using the template in §7. Your **final chat message is the full report text**.
6. **Gate** – the owner and a reviewer read the report; the next prompt may contain fix requests that take priority over new work.

**Traceability:** every requirement ID appears in `docs/TRACEABILITY.md` (`ID | title | phase | status | code location | tests`). Reference IDs in commit messages and in test names or doc comments (e.g. `#[test] fn r_ren_07_table_alignment()`).

**Definition of Done (per requirement):** implemented exactly as specified · tests written and passing · gates green · budget impact measured · docs + traceability updated · manual QA steps written.

**ADRs:** `docs/adr/NNNN-kebab-title.md` with sections *Context · Decision · Alternatives considered · Consequences · Measurements (if any)*. Number sequentially from `0000`.

## 7. Report template (`docs/reports/NNN_REPORT.md` — also your final chat message)

```markdown
# YAMDV — Phase NNN Report: <phase title>

## 1. Summary
<max 6 lines: what was achieved, headline numbers, chosen decisions>

## 2. Environment
OS / arch / RAM · rustc · cargo · targets · gh account (login only) · can run Windows GUI: yes/no/how

## 3. Task status
| Task / Requirement ID | Status (DONE / PARTIAL / NOT DONE / BLOCKED) | Evidence (file, test, commit) | Notes |

## 4. Acceptance criteria
| AC | PASS / FAIL / NOT VERIFIED | Evidence (command + short output excerpt) |

## 5. Measurements
<tables; machine; method; links to raw JSON in repo or CI artifacts>

## 6. Decisions (ADRs)
<ADR number — one-line decision>

## 7. Deviations from the spec
<what, why, impact>

## 8. Conflicts, spec defects, questions for the reviewer
<numbered list; each with your proposed resolution>

## 9. Risks discovered
| Risk | Severity (H/M/L) | Mitigation proposal |

## 10. Dependencies
| Crate | Version | Product or dev-only | Why | ADR |

## 11. Repository state
branch · last commit hash · tag · PR link · CI run link + status

## 12. Files added/changed
<tree, trimmed>

## 13. Manual QA checklist for the owner (Windows PC)
<numbered steps, each with the expected result>

## 14. Proposed next steps
<short; do not start them>

## 15. Özet (Türkçe)
<5–10 lines in Turkish summarizing results, blockers and decisions for the owner>
```

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
A-5 Owner addendum (V1-D until v1.0.0):
1. Visibility: the owner made the repository PUBLIC (overrides H2's "private"). Agents never change visibility or any repository settings. Never commit secrets, tokens or personal data. CI keeps `permissions: contents: read`; only the tag/release job may use `contents: write`. Use `pull_request`, never `pull_request_target`. Releases stay drafts until the owner publishes them.
2. Reduced testing: (a) keep every existing test, golden and driver script — delete none. (b) New tests this phase: unit tests for new pure logic only (pagination, CSV); at most 2 new driver scripts (export-html, install/uninstall); no new goldens, no fuzzing; the exported-HTML context is added to the existing XSS suite. (c) CI measurement gates: only M0, M1 and T4 typical warm remain gates, with the retry rule (F-22 applies to these three only); every other measurement is report-only. (d) A flaky existing driver script may be marked report-only with a one-line note in the report.
3. Report ≤ 80 lines: status, AC table, measurements (M0, M1, T1, T4 typical, B-SIZE only), deviations, what is left, final owner QA list.
4. CI: one full run at the end of the phase plus at most one fix push.
```

**A-6 (Phase V1-D, owner addendum 2026-10-17; GitHub Actions unavailable until the minutes reset on 2026-11-01; supersedes the CI steps of A-5).**
```text
A-6 No-CI mode (V1-D until 2026-11-01):
1. Push nothing that triggers CI: every commit carries `[skip ci]` (or the push is branch-only). Do not re-run or wait for CI runs.
2. Verify locally: cargo fmt, clippy (native + cargo-xwin x86_64-pc-windows-msvc), tests of all platform-independent crates, deny, spec-check, check-imports and size on the cross-compiled exe.
3. Windows-only tests, goldens, driver scripts and measurements are deferred and marked "NOT RUN (A-6)" in the report; never simulated.
4. Test build: release exe built with cargo-xwin, zipped as `YAMDV-1.0.0-rc1-win-x64.zip` (yamdv.exe, README.md, CHANGELOG.md, FUTURE.md) plus its SHA-256 file, uploaded with `gh release create v1.0.0-rc1 --draft --title "YAMDV 1.0.0 RC1"`. Stays a DRAFT. Build artifacts stay out of git.
5. PR #49 is not merged and `phase-v1d` / `v1.0.0` are not tagged: merge only after the owner confirms RC1 on a real PC; `v1.0.0` after green CI on 2026-11-01.
6. The report (≤ 80 lines, A-5) includes a step-by-step manual QA list for RC1, replacing CI evidence.
```
