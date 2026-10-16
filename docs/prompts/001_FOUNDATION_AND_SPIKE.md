# 001_FOUNDATION_AND_SPIKE — YAMDV (Yet Another Markdown Viewer)

> **Prompt 001 of a chained series.** You are executing **Phase 001 only**.
> Read this entire document (including all appendices) before you run a single command.
> When you finish, you produce a report (template in §7). The project owner will hand that report to a reviewer, and the next prompt (`002_...`) will be built from it. Do **not** start work that belongs to later phases.

---

## 0. How to use this document

- **§1–§7** are the instructions for *this* phase.
- **Appendices A–H** are the permanent project specification. In this phase you copy them **verbatim** into the repository (`docs/…`, mapping in §4.3). From then on they are the single source of truth. Future prompts will reference their requirement IDs (e.g. `R-REN-07`, `S-NET-02`, `M1`).
- **Precedence when instructions conflict:** Hard rules (§2) > this phase's tasks (§4) > appendices > your own judgment. If you find a real conflict, do not guess silently: do the non-conflicting work, choose the safest interpretation for the rest, and list the conflict in the report (§7, section 8).
- **MUST / MUST NOT / SHOULD / MAY** are used as in RFC 2119. "MAY" items are optional and are only implemented when a phase prompt explicitly asks for them.

---

## 1. Your role and the product

You are the **sole engineer** of **YAMDV**, a native Windows Markdown viewer and WYSIWYG editor written **from scratch in Rust**.

**What it is:** the user double-clicks a `.md` file and it opens instantly, rendered like a clean web page (reading-first, almost no chrome). `F2` turns the rendered page into an in-place WYSIWYG editor (Obsidian/Typora-style "live preview"); `F3` opens a split view (source left, live preview right). Its only job is to **show and edit Markdown files** — extremely fast and extremely light.

**Reference product:** "mdview" (https://github.com/mdview2026/mdview, https://www.mdview.top). YAMDV reproduces mdview's **feature set and visual design, including all of its paid "Pro" features** (Mermaid, math, Live Edit F2, Split Edit F3, mind map, table tools …), but with a completely different architecture:

| | mdview (reference) | YAMDV (us) |
|---|---|---|
| Rendering | WebView2 (Chromium) + Tailwind CSS | **Native**: Win32 + DirectWrite (backend chosen in this phase) |
| Server | axum/tokio HTTP server on localhost + SSE | **None.** No server, no sockets, no IPC listeners |
| Markdown | md4c (C) | pulldown-cmark (Rust) + our own extensions |
| Mermaid / KaTeX | JavaScript engines downloaded on first use | **Native Rust implementations, built in, never downloaded** |
| Network | update checks, startup report, payments, feedback | **Zero network I/O. Ever.** |
| Licensing | Community/Pro, trial counter, device ID | **None.** Every feature is always available |
| RAM | WebView process tree (tens to hundreds of MB) | **Target: 3–5 MB private working set** |
| Platforms | Windows, macOS, Linux, Android | **Windows 10 (1809+) / 11, x64 only** |

**Do not copy** anything from mdview: no code (none is public anyway), no text, no icons, no logo, no name. Screenshots and descriptions are visual/functional references only. Our name is **YAMDV**; our icon is our own design (Appendix B §B.14).

---

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

---

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

---

## 4. Phase 001 — tasks

**Phase goal:**
(a) a professional repository, workspace, documentation set, tooling and CI;
(b) **prove, with measurements, that the 3–5 MB architecture is feasible** by spiking the rendering backend candidates and choosing one;
(c) a minimal, correct native window skeleton that later phases build on.

### 4.1 Environment discovery (record everything in the report)
- Host OS + version, CPU architecture, RAM, available disk.
- `rustc -V`, `cargo -V`, installed targets, `gh --version`, `gh auth status` (account login only — never the token).
- Can you **run** Windows GUI binaries here? (native Windows / Wine / no).
- If the host is **not Windows**: cross-compile for `x86_64-pc-windows-msvc` using `cargo-xwin` (preferred) or, if that fails, `x86_64-pc-windows-gnu` for compile checks only (record the deviation). Run pure-crate tests natively. Run Windows-only tests, the spike measurements and `memprobe` on GitHub Actions `windows-latest` and publish their outputs as workflow artifacts; mark local GUI checks `NOT VERIFIED`.

### 4.2 Repository and workspace
1. Create `<workspace>/YAMDV/`, `git init -b main`, create the private GitHub repo `YAMDV` (`gh repo create YAMDV --private --source . --remote origin`), push an initial commit (README + .gitignore), then create branch `phase/001-foundation` for the rest.
2. Create the workspace layout from Appendix E §E.2 (all crates exist; crates not needed yet contain only `lib.rs` with a crate-level doc comment and `#![forbid(unsafe_code)]` where applicable — no fake code).
3. Root `Cargo.toml`: `[workspace]` with `resolver = "3"`, `[workspace.package]` (edition 2024, `rust-version`, license placeholder, repository URL), `[workspace.dependencies]` (all versions pinned here only), `[workspace.lints]` (see Appendix E §E.9).
4. Profiles:
   ```toml
   [profile.release]
   opt-level = "s"          # revisit in Phase 013 with measurements
   lto = "fat"
   codegen-units = 1
   panic = "abort"
   strip = "symbols"
   debug = false
   overflow-checks = true   # security over micro-speed
   incremental = false

   [profile.release-dbg]    # for profiling/analysis
   inherits = "release"
   strip = "none"
   debug = "line-tables-only"
   ```
5. `.cargo/config.toml`: for the msvc target set `rustflags = ["-C", "target-feature=+crt-static", "-C", "control-flow-guard"]` and linker args `/DYNAMICBASE /HIGHENTROPYVA /NXCOMPAT /CETCOMPAT /OPT:REF /OPT:ICF` (verify each is accepted by the toolchain; drop any that isn't and record why).
6. `rust-toolchain.toml` pinned to the current stable you have (record in ADR-0001), components `rustfmt`, `clippy`, target `x86_64-pc-windows-msvc`.
7. Files: `.gitignore`, `.gitattributes` (`* text=auto eol=lf`; `tests/corpus/** -text` so CRLF/BOM/UTF-16 fixtures keep their exact bytes; `*.png *.ico binary`), `.editorconfig`, `rustfmt.toml`, `clippy.toml`, `deny.toml` (bans per `S-SUP-03`, license allowlist per `S-SUP-04`, `unknown-registry = "deny"`, `unknown-git = "deny"`), `LICENSE` (placeholder: `Copyright (c) 2026 <repo owner>. All rights reserved.` — final license is an open question for the owner), `README.md` (short: what YAMDV is, status, how to build), `CHANGELOG.md` (Keep a Changelog format).

### 4.3 Documentation (copy appendices verbatim)
| Appendix | File |
|---|---|
| A — Product specification | `docs/SPEC.md` |
| B — UI & visual specification | `docs/UI_SPEC.md` |
| C — Security specification | `docs/SECURITY.md` |
| D — Performance & memory budgets | `docs/PERFORMANCE.md` |
| E — Architecture | `docs/ARCHITECTURE.md` |
| F — Keyboard, mouse & CLI | `docs/SHORTCUTS.md` |
| G — Roadmap | `docs/ROADMAP.md` |
| H — Testing strategy & corpus | `docs/TESTING.md` |

Also create: `docs/THREAT_MODEL.md` (skeleton per `S-TEST-04`), `docs/TRACEABILITY.md` (every `R-*`, `S-*`, `M*`, `T*`, `B-*` ID with status `planned`, phase from Appendix G), `docs/adr/0000-record-architecture-decisions.md`, `docs/adr/0001-toolchain-and-build-profile.md`, `docs/plans/001_PLAN.md`, `docs/reports/` (your report goes here), `docs/spike/` (spike results). If you find a typo or contradiction inside an appendix, copy it verbatim anyway and list it in the report (§7 section 8) — the reviewer fixes the spec, not you.

### 4.4 Tooling (`xtask` crate, run as `cargo xtask <cmd>`)
1. `check-imports [<exe>]` — parse the PE import table (and delay-load table) of the release `yamdv.exe` (use the `object` crate, dev-only). **Fail** if any forbidden DLL or function from `S-NET-03` appears. Print the full DLL list. (Rust `std` should not import `ws2_32.dll` when `std::net` is unused; if it does, report the referencing symbols and the cause — never weaken the check.)
2. `size [<exe>]` — print exe size in bytes and KiB; fail if above the current budget (Appendix D `B-SIZE`).
3. `memprobe <exe> [--doc <file>] [--settle-ms 3000] [--runs 3] [--json out.json]` — launch the app, wait until its main window exists and the message queue is idle, wait `settle-ms`, then measure (Appendix D §D.3: private working set via `QueryWorkingSet` shared-bit counting, `PrivateUsage`, `WorkingSetSize`, `PeakWorkingSetSize`, handle count, GDI/USER object counts, loaded module list), then close it with `WM_CLOSE` and verify exit. Median of `--runs`. Output human-readable + JSON.
4. `ci` — runs every gate from H9 in order and stops at the first failure.
5. `gen-icons` — generates the original app/document icons (Appendix B §B.14) as multi-resolution `.ico` from code (vector drawing with tiny-skia in xtask only, or hand-written pixel code). No external image assets.
Unit-test the PE import checker (positive and negative case).

### 4.5 CI (`.github/workflows/ci.yml`)
- Triggers: `push` to `main`, `pull_request`, plus `workflow_dispatch` for manual runs (fuzzing, long measurements). Private repos have limited Actions minutes and Windows minutes count double — no cron, no matrix explosion, cache aggressively, aim for < 10 min per run.
- Job `windows` on `windows-latest`: checkout, cache cargo, fmt, clippy, test, `cargo deny` (install via `cargo install --locked cargo-deny` or the official action), build release, `check-imports`, `size`, `memprobe` for the skeleton window (and the spike binaries, §4.7), upload JSON + screenshots as artifacts.
- Job `pure` on `ubuntu-latest` (fast): fmt, clippy and tests for the platform-independent crates only.
- Do **not** add any workflow that publishes, releases, or deploys.

### 4.6 Platform skeleton (`yamdv-platform-win` + `yamdv-app`)
Implement only this — nothing more:
1. `yamdv.exe`, GUI subsystem (`#![windows_subsystem = "windows"]`), embedded application manifest: `PerMonitorV2` DPI awareness, `longPathAware=true`, `requestedExecutionLevel=asInvoker`, `supportedOS` Windows 10/11, `heapType=SegmentHeap` **only if** the measurement in §4.7 shows it lowers private working set (record result), plus a version resource and the generated icon.
2. **Startup hardening first thing in `main`** (`S-PROC-01..05`): `SetDefaultDllDirectories(LOAD_LIBRARY_SEARCH_SYSTEM32)`, `SetDllDirectoryW("")`, `SetSearchPathMode(BASE_SEARCH_PATH_ENABLE_SAFE_SEARCHMODE | BASE_SEARCH_PATH_PERMANENT)`, `HeapSetInformation(HeapEnableTerminationOnCorruption)`, process mitigation policies listed in `S-PROC-04` (each call's failure is logged, never fatal).
3. One top-level window, class `YAMDV.Window`, initial client size 1100×760 logical px, centered on the monitor that contains the mouse cursor; correct `WM_DPICHANGED` handling (use the suggested rect); minimum size 480×360 logical px.
4. Window background = theme token `bg` (light `#ffffff`, dark `#1e1e1e`) following the Windows app mode (`HKCU\Software\Microsoft\Windows\CurrentVersion\Themes\Personalize\AppsUseLightTheme`), switching live on `WM_SETTINGCHANGE` with `lParam == "ImmersiveColorSet"`. Use `DWMWA_USE_IMMERSIVE_DARK_MODE` for the (temporary) system title bar. The custom title bar comes in Phase 004 — do not build it now.
5. Esc and Ctrl+W close the window. No flicker on resize (handle `WM_ERASEBKGND`, paint in `WM_PAINT` only). Message loop is `GetMessageW`-based; **no timers, no polling** — idle CPU must be 0.
6. CLI: `--help` and `--version` print to the parent console (`AttachConsole(ATTACH_PARENT_PROCESS)`; if there is none, show nothing and exit 0). Unknown flags → short error on the parent console, exit code 2.
7. Panic hook: writes a short crash note (version, location, message — **no document content**) to `%LOCALAPPDATA%\YAMDV\logs\crash-<timestamp>.txt`, then aborts. (Recovery of unsaved text is added with the editor in Phase 006.)

### 4.7 Rendering backend spike — the most important task of this phase
Build the same **reference scene** with each candidate backend as separate spike binaries under `YAMDV/spikes/` (a workspace member that is **excluded from the product build**; keep it in the repo for reproducibility). Measure, compare, decide.

**Reference scene** (logical px; test at 100 % and 150 % DPI):
1. Heading `Aurora Notes — Başlık İıŞşĞğ` — Segoe UI, 32 px, weight 600, color `#333333`.
2. A paragraph, 16 px, line height 1.6, wrapped at 760 px, containing Latin, Turkish (`ğüşıöçİ`), CJK (`中文 日本語 한국어`), Arabic RTL (`مرحبا بالعالم`), color emoji with modifiers and a ZWJ sequence (`👍🏽 🚀 👩‍💻`), plus a **bold** span, an *italic* span and a link-colored span (`#2563eb`).
3. A code block: rounded rectangle (radius 8, 1 px border `#e0e0e0`, fill `#f5f5f5`) containing 6 lines of monospace text (Cascadia Mono → Consolas fallback, 14 px) with 4 syntax colors.
4. A 4-column × 3-row table grid: 1 px lines `#dddddd`, header row fill `#f5f5f5`, bold header text.
5. Anti-aliased primitives: a circle (r = 6), a 1.5 px diagonal line, a filled triangular arrowhead, a cubic Bézier curve (needed later for Mermaid), a 3 px vertical bar (blockquote).
6. A selection highlight over part of the paragraph (`#2563eb` at 25 % alpha).
7. A generated 400×300 PNG test image (create it in code; no third-party images) drawn scaled to 200×150 with high-quality interpolation (decode via WIC; record the WIC initialization cost separately).
8. The scene repeated 200× vertically; mouse-wheel smooth scrolling.

**Candidates:**
- **A — DirectWrite + GDI interop, banded.** `IDWriteGdiInterop::CreateBitmapRenderTarget` with a reused band buffer (full width × 64–128 px) that is rendered band by band and blitted to the window DC; shapes drawn into the same DIB (tiny-skia on the DIB bits — mind BGRA vs RGBA and premultiplication — or hand-written AA primitives); color emoji via `IDWriteFactory4::TranslateColorGlyphRun`; scrolling via `ScrollWindowEx` + repainting only the exposed strip.
- **B — Direct2D software DC render target, banded.** `ID2D1DCRenderTarget` with `D2D1_RENDER_TARGET_TYPE_SOFTWARE` bound to a band-sized memory DC; DirectWrite text with `D2D1_DRAW_TEXT_OPTIONS_ENABLE_COLOR_FONT`.
- **C — Direct2D hardware (reference).** `ID2D1DeviceContext` on a DXGI flip-model swap chain (D3D11 device). Expected to be the heaviest; it is the quality/performance yardstick.
- **D — (optional) pure GDI + Uniscribe** (`ExtTextOutW` / `ScriptString*`) as a minimal-memory baseline.

**Measure for each candidate** (release build, same machine, median of 3 runs, document the machine):
| Metric | How |
|---|---|
| Private working set after 3 s idle at scene top | `memprobe` (Appendix D §D.3) |
| Private working set after scrolling to the end and idling 3 s | same |
| Peak working set, `PrivateUsage` (commit) | same |
| Loaded DLL count + list | `EnumProcessModules` |
| Exe size | bytes |
| Startup → first frame | process creation time (`GetProcessTimes`) → end of first `WM_PAINT` (`QueryPerformanceCounter`/`GetSystemTimePreciseAsFileTime`), written to a small log line |
| Frame time while scrolling (avg, p99) at 1920×1080 | instrumented paint timing |
| Correctness of all 8 scene elements incl. color emoji, RTL, fallback | screenshot review |
| Text quality | screenshots at 100 % and 150 % (ClearType/grayscale) saved as PNG to `docs/spike/` |
| Heap variant | repeat the private-working-set measurement of the skeleton window and of the two lightest candidates with and without the manifest `heapType=SegmentHeap` |

**Decision rule:** pick the backend with the **lowest steady private working set** that renders **all** scene elements correctly (including color emoji and RTL) with **p99 frame time ≤ 16 ms** at 1080p. Ties → smaller exe. Write `docs/adr/0002-rendering-backend.md` containing the full table, screenshots links and reasoning.
If **no** candidate reaches ≤ 5 MB for the scene, do not hide it and do **not** "fix" it with periodic working-set trimming. Report the numbers and analyze the top contributors: enumerate private regions (`VirtualQuery`), heap summaries (`HeapSummary` / `HeapWalk`), per-module deltas (measure before/after each init step: DWrite factory, first text layout, D2D factory, WIC/COM init, first image decode). Put the breakdown in the ADR — it drives the next phases.

### 4.8 Tests in this phase
- xtask: PE import checker (pass + fail fixtures), size check logic, JSON output schema.
- `yamdv-app`: `#[cfg(windows)]` integration test that runs `yamdv.exe --version` and checks output and exit code; unknown flag → exit code 2.
- Lint configuration proven: CI fails on a deliberately introduced `unwrap()` in a pure crate (verify locally once, do not commit the violation).

### 4.9 Explicitly out of scope for 001
Markdown parsing, layout, document rendering, custom title bar, sidebar, menus, themes beyond the background color, file opening, settings, editing. **Do not implement them.**

---

## 5. Acceptance criteria (Phase 001)

| ID | Criterion |
|---|---|
| AC-1 | Private GitHub repo `YAMDV` exists on the authenticated account; `main` contains the merged phase branch; tag `phase-001` exists. |
| AC-2 | `docs/` contains all appendix files verbatim, `THREAT_MODEL.md` (skeleton), `TRACEABILITY.md`, ADR-0000/0001/0002, `plans/001_PLAN.md`, `reports/001_REPORT.md`, `spike/` results. |
| AC-3 | `cargo xtask ci` passes locally (where the host allows) and the CI workflow is green on `windows-latest`. |
| AC-4 | `yamdv.exe` opens a window that follows system light/dark live, closes with Esc/Ctrl+W, `--version`/`--help` work, `check-imports` shows no forbidden DLLs/functions, size is reported. |
| AC-5 | Spike table complete with real measurements (or explicit `NOT VERIFIED` with reason), screenshots committed, backend chosen in ADR-0002. |
| AC-6 | Skeleton window steady private working set measured (goal ≤ 2.5 MB) and reported. |
| AC-7 | Report delivered per §7 (file + final chat message). |

---

## 6. If you get blocked
Continue with every task that does not depend on the blocker. In the report, mark the item `BLOCKED`, paste the exact (trimmed) error output, list what you tried, and propose the smallest decision the owner could make to unblock you.

---

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

---

# APPENDIX A — Product Specification (`docs/SPEC.md`)

## A.0 Vision and principles
YAMDV is a reading-first Markdown viewer with in-place WYSIWYG editing for Windows. Double-click a `.md` file → it is on screen, rendered like a clean web page, in well under a second. `Esc` closes it like a photo viewer. `F2` edits in place, `F3` edits side by side.

- **P1 Reading first** — the document is the UI. No toolbars, no status bars, no panels the user didn't ask for.
- **P2 Instant** — open, scroll, switch, type: no perceptible delay.
- **P3 Tiny** — 3–5 MB private working set, ≤ 3 MB exe (Appendix D).
- **P4 Offline & private** — no network, no accounts, no telemetry. Files never leave the machine.
- **P5 Never surprise-modify files** — the app writes only what the user edits, only where expected, byte-identical elsewhere.
- **P6 Keyboard-complete** — everything reachable by keyboard; shortcuts rebindable.
- **P7 Faithful rendering** — CommonMark + GFM + the common extensions people actually use (callouts, math, Mermaid, front matter, wikilinks, highlight).

**Non-goals (v1):** vault/knowledge-base features, sync, plugins/extension API, tabs inside a window, collaboration, DOCX/EPUB/LaTeX export, non-Markdown formats (except `.txt` as plain text), spell checking, macOS/Linux/Android, auto-update, licensing/trial, feedback/telemetry, Microsoft Store packaging.

**Explicitly excluded mdview features (they are online or commercial):** update checks and auto-update, anonymous startup report, Pro licensing/trial counter/"Maybe Later" prompt, device ID, invite codes, payment, in-app feedback form and reply cards, cloud-synced read state, engine downloads (our engines are built in), VS Code/Sublime live-preview plugins (possible future work, local-only), Android/macOS/Linux builds, Microsoft Store edition.

Requirement keywords: **MUST** (v1), **SHOULD** (v1 unless a phase prompt defers it), **MAY** (only when a phase prompt asks).

---

## A.1 Markdown dialect (R-MD)
- **R-MD-01 MUST** CommonMark 0.31 complete (via pulldown-cmark).
- **R-MD-02 MUST** GFM: tables with column alignment, task lists, strikethrough (`~~x~~`), footnotes, GFM alerts (`> [!NOTE]`, `[!TIP]`, `[!IMPORTANT]`, `[!WARNING]`, `[!CAUTION]`).
- **R-MD-03 MUST** GFM *autolink literals* (`https://…`, `http://…`, `www.…`, bare e-mail addresses) following the GFM spec's trailing-punctuation rules. pulldown-cmark does not do this — implement a linkify pass over text events (never inside code, links, or HTML).
- **R-MD-04 MUST** Obsidian-style callouts: `> [!type] Optional title` with optional fold marker `[!type]-` (collapsed) / `[!type]+` (expanded). Type mapping (case-insensitive) → 5 visual styles:
  - *note*: note, info, abstract, summary, tldr, todo, quote, cite
  - *tip*: tip, hint, success, check, done, example
  - *important*: important, question, help, faq
  - *warning*: warning, attention
  - *caution*: caution, danger, error, failure, fail, missing, bug
  Unknown type → *note* style showing the type name capitalized as title.
- **R-MD-05 MUST** `==highlight==` (custom inline extension; not inside code).
- **R-MD-06 MUST** Superscript `^x^` and subscript `~x~` (pulldown-cmark options; `~~x~~` stays strikethrough).
- **R-MD-07 MUST** Math: inline `$…$`, display `$$…$$` (pulldown-cmark `ENABLE_MATH`), and ` ```math ` fences. `\$` is a literal dollar.
- **R-MD-08 MUST** Mermaid: ` ```mermaid ` fences.
- **R-MD-09 MUST** Front matter: YAML (`---` … `---`) and TOML (`+++` … `+++`) at the very start of the file. Never rendered as headings/rules; shown as a metadata card (R-REN-13).
- **R-MD-10 MUST** Heading IDs: GitHub-compatible slugs (Unicode-lowercase; remove characters that are not letters, numbers, spaces, `-`, `_`; spaces → `-`; keep non-ASCII letters such as `ç ğ ı ö ş ü`; duplicates get `-1`, `-2`, …) and explicit `{#custom-id}` heading attributes.
- **R-MD-11 SHOULD** Definition lists (pulldown-cmark option).
- **R-MD-12 MUST** Wikilinks: `[[Page]]`, `[[Page|alias]]`, `[[Page#Heading]]`, `[[#Heading]]`, and embeds `![[image.png]]`, `![[image.png|300]]` (width). Resolution: R-LINK-03.
- **R-MD-13 SHOULD** Emoji shortcodes `:smile:` (GitHub set, compile-time table; not inside code/URLs).
- **R-MD-14 MUST** Raw HTML is never executed. A **safe subset** is rendered natively; everything else is sanitized (rules in `S-HTML-*`). Allowed tags: `b strong i em u ins s del strike mark sub sup small kbd code var samp abbr br hr p div span center h1–h6 blockquote ul ol li dl dt dd table thead tbody tfoot tr th td pre a img details summary picture source(srcset ignored) figure figcaption`. Allowed attributes: `align` (p, div, h1–h6, img, td, th), `width`/`height` (img), `src`/`alt`/`title` (img), `href`/`title` (a), `open` (details), `colspan`/`rowspan` (td, th), `style` restricted to `color`, `background-color`, `text-align`. HTML comments are hidden (including multi-line). Entities decoded. README patterns like `<p align="center"><img …></p>` MUST render centered.
- **R-MD-15 MUST** Hard breaks: two trailing spaces, backslash. Setting "Single newline as line break" (default **off**).
- **R-MD-16 SHOULD** Smart punctuation (setting, default off).
- **R-MD-17 MUST** Indented code blocks; tabs expand to 4 columns inside code.
- **R-MD-18 MUST** Reference-style links/images; link titles shown as tooltips.
- **R-MD-19 MAY** `[TOC]` / `[[_TOC_]]` placeholder renders an inline table of contents.

## A.2 Opening files (R-OPEN)
- **R-OPEN-01 MUST** After association, double-clicking a `.md`/`.markdown` file in Explorer opens it rendered.
- **R-OPEN-02 MUST** CLI `yamdv <path> [<path>…]`: one window per path (Appendix F).
- **R-OPEN-03 MUST** `Ctrl+O` open dialog. Filters: *Markdown* (`*.md;*.markdown;*.mdown;*.mkd;*.mkdn;*.mdwn;*.mdtxt;*.mdtext`), *Text* (`*.txt`), *All files*. Multi-select: first file replaces the current document if the window shows the welcome page, otherwise each opens in its own window.
- **R-OPEN-04 MUST** Drag & drop files onto a window: first file opens **in place**, others in new windows. In edit modes, dropping an image/file inserts it (R-EDIT-08). Dropping a folder SHOULD open the Files tab rooted at it.
- **R-OPEN-05 MUST** Optional Explorer context-menu entry "Open with YAMDV" (R-SYS-03).
- **R-OPEN-06 MUST** Encoding detection: BOM (UTF-8, UTF-16LE/BE) → that encoding; else valid UTF-8 → UTF-8; else detect with `chardetng` (expect windows-1254 Turkish, windows-1252, GB18030/GBK, Shift_JIS, EUC-KR, Big5, windows-1251, ISO-8859-x) and decode with `encoding_rs`. The detected encoding is shown in the document info (word-count badge tooltip / About-document info). SHOULD offer "Reopen with encoding ▸" in the menu.
- **R-OPEN-07 MUST** Line endings (LF / CRLF / mixed) are detected and preserved on save; newly inserted lines use the file's dominant ending.
- **R-OPEN-08 MUST** Size guards: > 20 MB → prompt "Large file — open as plain text (fast) / render anyway / cancel"; > 256 MB → refuse with message. NUL bytes in the first 8 KB → warn "This looks like a binary file" with "Open anyway".
- **R-OPEN-09 MUST** Any extension passed explicitly (CLI/drag/dialog) is rendered as Markdown; `.txt` SHOULD render as plain text with a "Render as Markdown" toggle.
- **R-OPEN-10 MUST** Read-only files open normally; edit modes are disabled with a "Read-only" pill and "Save a copy as…".
- **R-OPEN-11 MUST** If the open file is deleted/moved: banner "File was deleted or moved" with [Close] and, if there are unsaved edits, [Save as…]. If it reappears (atomic-save editors), reload silently.
- **R-OPEN-12 MUST** Long paths (> 260 chars) and UNC paths that the **user** opened directly are supported; displayed without `\\?\` / `\\?\UNC\` prefixes.
- **R-OPEN-13 MUST** Every successful open is recorded in History (R-NAV-08).

## A.3 Windows and instances (R-WIN)
- **R-WIN-01 MUST** One process, multiple top-level windows (one document per window). A second launch forwards its arguments to the running instance (`WM_COPYDATA`, validated per `S-IPC-*`) and exits within 50 ms.
- **R-WIN-02 MUST** One window per file: opening an already-open file activates its window (restore if minimized; `FlashWindowEx` if focus stealing is prevented).
- **R-WIN-03 MUST** Window geometry (size, position, maximized, monitor) is remembered; restored geometry is validated against current monitors (never off-screen; falls back to the primary monitor). Additional windows cascade by 28 px.
- **R-WIN-04 MUST** Close with `Esc` (read mode, no overlay/popup open) or `Ctrl+W`. Unsaved changes are never lost (autosave or prompt).
- **R-WIN-05 MUST** Title bar text: file name (default) or full path (setting) + `●` when dirty + mode pill (`Live Edit`, `Split Edit`, `Read-only`).
- **R-WIN-06 MUST** `F11` distraction-free fullscreen (hides title bar and sidebar; `Esc`/`F11` exits).
- **R-WIN-07 MUST** The process exits when the last window closes; all pending state writes are flushed first.

## A.4 Rendering (R-REN) — visual details in Appendix B
- **R-REN-01 MUST** Reading view: one centered column, web-page-like, no chrome.
- **R-REN-02 MUST** Headings H1–H6 (styles per theme), anchors per R-MD-10.
- **R-REN-03 MUST** Paragraphs, emphasis, strong, strikethrough, highlight, sub/superscript, inline code, `<kbd>`.
- **R-REN-04 MUST** Lists: bullets by depth `•` `◦` `▪` (repeat); ordered lists honor the start number (decimal at all depths); tight vs loose spacing; task-list checkboxes (rendered, not interactive in read mode unless the setting allows).
- **R-REN-05 MUST** Blockquotes, nested (one bar per level).
- **R-REN-06 MUST** Callouts (R-MD-02/04) with icon + title row; foldable ones toggle on title click.
- **R-REN-07 MUST** Tables: alignment, header style, borders, inline formatting in cells, wrapping long cell text (break anywhere if needed — never overflow the column), optional zebra rows, hover toolbar with ⛶ fullscreen (R-READ-08) and CSV export (R-EXP-04).
- **R-REN-08 MUST** Code blocks: syntax highlighting (R-CODE), hover toolbar.
- **R-REN-09 MUST** Horizontal rules.
- **R-REN-10 MUST** Footnotes: superscript reference `[n]` → jumps to note; notes section at the end with `↩` back-links. SHOULD show a hover tooltip with the footnote text.
- **R-REN-11 MUST** Images per R-IMG.
- **R-REN-12 MUST** Links styled per theme; hovering shows the destination in a status tooltip at the window's bottom-left (like a browser).
- **R-REN-13 MUST** Front matter card: collapsed by default (one muted line "Front matter · N fields"), expandable to a key/value list (setting: collapsed / expanded / hidden).
- **R-REN-14 MUST** `<details>/<summary>` render as collapsible sections.
- **R-REN-15 MUST** Math (R-MATH) and Mermaid (R-MMD) blocks render natively.
- **R-REN-16 MUST** Text selection: drag, Shift+click, double-click word, triple-click paragraph, `Ctrl+A`; selection spans blocks. `Ctrl+C` puts plain text **and** HTML (`CF_HTML`) on the clipboard; menu "Copy as Markdown" copies the source slice.
- **R-REN-17 MUST** Scrolling: wheel and precision touchpad (fractional deltas), `PgUp`/`PgDn`/`Space`/`Shift+Space`, `Home`/`End`, arrows; smooth (setting); overlay auto-hiding scrollbar (Appendix B §B.5.6). Middle-click autoscroll MAY.
- **R-REN-18 MUST** Incremental re-render: only changed blocks are re-parsed/re-laid-out; no full-document flicker.
- **R-REN-19 MUST** Bidi: paragraph direction from the first strong character; RTL paragraphs right-aligned; mixed runs correct.
- **R-REN-20 MUST** Color emoji, emoji modifiers and ZWJ sequences; font fallback for every script (DirectWrite system fallback).
- **R-REN-21 MUST** Virtualized layout: only visible blocks (plus a small margin) are laid out; the rest are height estimates refined lazily, so 1 MB documents open instantly.

## A.5 Code blocks (R-CODE)
- **R-CODE-01 MUST** Built-in highlighter (no downloads, no runtime grammar files): small hand-written lexers for at least: `bash/sh/shell/zsh/console`, `powershell/ps1`, `bat/cmd`, `c`, `cpp/c++/h/hpp`, `csharp/cs`, `java`, `kotlin/kt`, `go`, `rust/rs`, `python/py`, `javascript/js/mjs/cjs`, `typescript/ts`, `jsx`, `tsx`, `json/jsonc/json5`, `yaml/yml`, `toml`, `ini/conf/properties`, `xml/html/svg/xaml/vue (template only)`, `css/scss/less`, `sql`, `markdown/md`, `diff/patch`, `dockerfile`, `makefile`, `php`, `ruby/rb`, `swift`, `lua`, `dart`, `r`, `scala`, `perl`, `haskell/hs`, `elixir/ex`, `nginx`, `graphql/gql`, `protobuf/proto`, `regex`. Unknown → plain. Token classes: keyword, string, number, comment, function, type, constant/builtin, operator, punctuation, tag, attribute, variable, meta/preprocessor, diff-add, diff-del, heading (md).
- **R-CODE-02 MUST** Hover toolbar (top-right inside the block): **Copy** (raw code → clipboard, toast "Copied"), **Line numbers** toggle, **Wrap** toggle, **language label**. Per-block toggles; global defaults in Settings. Line numbers are drawn, never part of copied text.
- **R-CODE-03 MUST** Unwrapped long lines scroll horizontally per block (Shift+wheel, touchpad, thin always-visible scrollbar when overflowing).
- **R-CODE-04 MUST** Whitespace preserved; tab width 4; final newline trimmed.
- **R-CODE-05 MUST** `diff` blocks: `+` lines green background, `-` lines red background, `@@` lines muted.
- **R-CODE-06 MUST** Very long blocks (> 5 000 lines) are virtualized like the document.

## A.6 Images (R-IMG)
- **R-IMG-01 MUST** Formats: PNG, JPEG, GIF (first frame; animation behind a setting, default off), BMP, ICO, TIFF, WebP (if the OS codec exists; otherwise placeholder "WebP codec not installed"). SVG: SHOULD, via a small native, sandboxed renderer decided by ADR in the phase that implements it (no external references, no scripts, bounded size).
- **R-IMG-02 MUST** Path resolution: relative to the document's folder (`./`, `../` allowed), absolute local paths, `file:///C:/…` (local only), percent-decoding (`%20`), backslashes accepted. Remote (`http/https`) images are **never fetched** (placeholder). Network locations are blocked per `S-RES-02`. `data:` URIs allowed for image MIME types within limits (`S-RES-04`).
- **R-IMG-03 MUST** Size: 1 image pixel = 1 logical px, scaled down to the column width (aspect preserved); HTML `width`/`height` and `![[x.png|300]]` honored; never upscaled beyond 100 % automatically.
- **R-IMG-04 MUST** Lazy: header-only probe for dimensions at layout time; full decode only when visible, **decoded at display size**; LRU cache with a byte budget (Appendix D).
- **R-IMG-05 MUST** Hover ⛶ badge → fullscreen image viewer (R-READ-08).
- **R-IMG-06 MUST** Placeholders: *missing* (icon + alt + path), *blocked remote* ("Remote image not loaded — YAMDV works offline" + [Copy URL]), *blocked network path*, *too large*, *unsupported format*.

## A.7 Links (R-LINK)
- **R-LINK-01 MUST** `#anchor` links smooth-scroll to the heading/footnote/`id`.
- **R-LINK-02 MUST** Relative Markdown links (`./a.md`, `../b.md#section`, percent-encoded or Unicode names) open **in the current window**, pushing a navigation history entry; `Alt+←` / `Alt+→` and mouse X1/X2 navigate back/forward with per-entry scroll restore. `Ctrl+click` opens in a new window. Missing target → toast "Link target not found: <name>".
- **R-LINK-03 MUST** Wikilink resolution: `Page` → `Page.md` (then `Page.markdown`) in the document's folder, then recursively in subfolders (max depth 4, first match by shortest path); `#Heading` resolved by slug or case-insensitive heading text.
- **R-LINK-04 MUST** `http`, `https`, `mailto` links follow the setting **Open external links**: *Ask* (default — popover showing the full URL with [Open in browser] [Copy link] [Cancel]), *Open directly*, *Never (copy only)*. Opening uses `ShellExecuteExW` with the validated URL only.
- **R-LINK-05 MUST** Other local targets: folders → "Show in Explorer?"; document types on the allowlist in `S-EXEC-02` → "Open with default app?" confirmation; everything else (executables, scripts, shortcuts…) is **never** launched — show "Blocked for safety" with [Show in Explorer] (which selects the file, never runs it).
- **R-LINK-06 MUST** Any other scheme (`javascript:`, `file://host/`, `vbscript:`, `data:` as link, `ms-*`, `search-ms:`, custom protocols…) is inert: tooltip "Blocked link type".
- **R-LINK-07 MUST** Right-click on a link → "Copy link address".

## A.8 Sidebar / navigation (R-NAV)
- **R-NAV-01 MUST** Sidebar with tabs **Contents · History · Favorites · Files**. History, Favorites and Files are individually hideable (Settings); if all three are hidden, the tab row disappears (pure outline).
- **R-NAV-02 MUST** Positions: *Left docked* (default), *Right docked*, *Left floating*, *Right floating* (floating = card overlaying the page with shadow, 12 px from edges).
- **R-NAV-03 MUST** Toggle via the chevron button on the sidebar edge and `Ctrl+\`; visibility remembered globally.
- **R-NAV-04 MUST** Resizable width by dragging its inner edge (180–480 px), remembered.
- **R-NAV-05 MUST** *Contents*: heading tree (levels 1..`toc_max_depth`, default 4). Click → smooth scroll to the heading. **Scroll spy**: the heading whose section contains the reading line (top of viewport + 80 px) is highlighted; the list auto-scrolls to keep it visible. Title flattening: if the document has exactly one H1 and it is the first heading, its H2 children are shown at the same indent as the H1.
- **R-NAV-06 MUST** Collapsing: per-node toggle (hover an item with children → ▸/▾); "Collapse one level / Expand one level" buttons in the tab's header area on hover; state remembered per document.
- **R-NAV-07 MUST** TOC font size adjustable (11–18 px, default 13).
- **R-NAV-08 MUST** *History*: groups **Current Session** (files opened since launch; the current file tagged "Current"), **Today**, **Yesterday**, then one group per date (locale short date). Item = file name; tooltip = full path; hover shows × (remove); group header hover shows "Clear". Click opens in the current window. Missing files shown muted; clicking → toast. Max 500 entries.
- **R-NAV-09 MUST** *Favorites*: add/remove via `Ctrl+D` or menu; hover × to remove; drag to reorder (SHOULD); empty state: centered hint "Press Ctrl+D to favorite this file" with a star icon.
- **R-NAV-10 MUST** *Files*: Markdown files under the current document's folder as a lazy explorer tree (folders first, natural sort, current file highlighted). Click switches the document **in place without flicker**. Updates on directory changes. Limits: 5 000 entries, depth 8; skips hidden/system items and `.git`, `node_modules`, `target`, `.venv`, `__pycache__`.
- **R-NAV-11 MUST** Keyboard: `↑/↓` move, `Enter` open, `←/→` collapse/expand, `Home/End`; `Ctrl+Shift+1…4` switch tabs (rebindable).

## A.9 Reading features (R-READ)
- **R-READ-01 MUST** **Live reload**: the open file is watched (directory watch, so atomic-rename saves are caught). External change → re-render within 100 ms, only changed blocks, scroll anchored to the same block, no flicker. Deleted → R-OPEN-11. If there are unsaved in-app edits → conflict banner (R-EDIT-03).
- **R-READ-02 MUST** **Reading position memory** per file (block index + offset within block, fallback scroll ratio), restored on reopen; LRU 2 000 entries; setting to disable. Following a link and coming back restores that document's own position (never carries over).
- **R-READ-03 MUST** **Font zoom**: `Ctrl+wheel`, `Ctrl+=`/`Ctrl+-`/`Ctrl+0`, `Ctrl+↑/↓`; 10–32 px, step 1 px; real reflow (not bitmap scaling); transient zoom indicator; persisted globally.
- **R-READ-04 MUST** **Column width**: `Alt+wheel`, `Alt+↑/↓`, `Alt+0` reset; 30–100 % of client width, step 5 %, default 65 %; indicator; persisted.
- **R-READ-05 MUST** **Find** `Ctrl+F` (Appendix B §B.9): incremental, highlights all matches, "n/m" count, `Enter`/`Shift+Enter` in the find bar and `Ctrl+G`/`Ctrl+Shift+G` anywhere (`F3` is reserved for Split Edit), toggles *Match case* and *Whole word*; searches rendered text (including tables, code, footnotes, collapsed sections — auto-expanded on hit; excluding hidden markup); case-insensitive mode uses Unicode case folding **plus Turkish equivalence (`i/İ/ı/I` match each other)**; `Esc` closes; highlights are never written to the file.
- **R-READ-06 MUST** **Heading folding**: hovering the left gutter of H1–H3 shows ▸/▾; click folds/unfolds the section (read mode and F2). `Alt+click` folds/unfolds all headings of that level (SHOULD). Session-only state.
- **R-READ-07 MUST** **Word-count badge** (off by default): bottom-right, "1,234 words · 7,890 chars"; with a selection adds "· 12 selected". CJK ideographs count as one word each. Tooltip: lines, reading time (230 wpm), encoding, line endings.
- **R-READ-08 MUST** **Fullscreen viewers** (overlay in the same window; `Esc` closes): image, table (natural width, no wrapping, horizontal scroll, resizable/movable card), Mermaid diagram (vector-sharp at any zoom), mind map (R-MAP). Common controls: wheel zoom at cursor (10 %–800 %), drag to pan, double-click toggles fit/100 %, `+`/`-`/`0`, toolbar (zoom out, %, zoom in, fit, 1:1, close).
- **R-READ-09 MUST** Toasts for transient feedback (Copied, Saved, Reloaded, errors).
- **R-READ-10 MUST** "Single newline as line break" toggle in the menu (⏎ icon with check state).
- **R-READ-11 MUST** Copy, Copy as Markdown, Select All in the context menu.
- **R-READ-12 MAY** Quick heading jump palette (`Ctrl+Shift+O`).

## A.10 Themes and typography (R-THEME)
- **R-THEME-01 MUST** Mode: *Follow System* (default) / *Light* / *Dark*; live switch on system change.
- **R-THEME-02 MUST** 11 color schemes: **Default, GitHub, Glacier, Forest, Sunset, Typewriter, Editorial, Violet, Lychee, Mint, Neon**, each with light and dark palettes (exact tokens in Appendix B §B.3). Schemes change colors only (fonts stay user-selected).
- **R-THEME-03 MUST** Scheme picker as **cards** (mini preview: page color, heading line, text lines, accent bar); hover = live preview, click = apply + persist; leaving without clicking reverts.
- **R-THEME-04 MUST** Fonts: body font (any installed family; searchable list; each name previewed in its own face) and code font (monospace families listed first). Defaults: body `Segoe UI`, code `Cascadia Mono` → `Consolas`. Settings show the font actually resolved.
- **R-THEME-05 MUST** Corner style: *Rounded* (default) / *Sharp* — applies to code blocks, tables, callouts, math/Mermaid cards, menus, cards, exported HTML.
- **R-THEME-06 MUST** Custom colors editor: 11 tokens (`bg`, `text`, `heading`, `link`, `accent`, `quote_bar`, `code_bg`, `border`, `table_header_bg`, `zebra_bg`, `selection`) per mode, with an in-app color picker (hue/saturation square, hue slider, hex input); applies instantly; reset per token / all.
- **R-THEME-07 MUST** `custom-theme.ini` in the config folder (documented keys = the tokens above in `[light]` / `[dark]`), hot-reloaded on save; invalid values ignored with a toast.
- **R-THEME-08 MUST** Windows High Contrast: use system colors, disable custom colors and scheme tinting.
- **R-THEME-09 MUST** Table zebra striping toggle (default off).
- **R-THEME-10 MUST** All chrome (title bar, menus, dialogs, settings, toasts, viewers, scrollbars) follows the active mode and scheme.

## A.11 Settings (R-SET) — panel layout in Appendix B §B.8
- **R-SET-01 MUST** Open via ⚙ title-bar button, menu, `Ctrl+,`.
- **R-SET-02 MUST** In-window overlay panel, draggable by its header, section navigation on the left; MAY have a search box.
- **R-SET-03 MUST** Changes apply instantly (no OK/Apply) and persist atomically.
- **R-SET-04 MUST** "Reset section" and "Reset all".
- **R-SET-05 MUST** Storage: `%APPDATA%\YAMDV\settings.ini`, `state.ini` (window, history, favorites, positions, TOC states), `custom-theme.ini`, `%LOCALAPPDATA%\YAMDV\{logs,recovery}`. **Portable mode**: a file `yamdv.portable` next to the exe → all data in `.\YAMDV-data\` beside the exe, and no registry writes unless the user explicitly registers.
- **R-SET-06 MUST** Settings keys and defaults: Appendix B §B.8.2 (that table is normative).

## A.12 Editing — common (R-EDIT)
- **R-EDIT-01 MUST** The **source text is the single source of truth**. Every edit (F2 or F3) is a text operation on the source buffer; no lossy HTML↔Markdown round-trips. Bytes outside edited ranges stay identical on save.
- **R-EDIT-02 MUST** Saving: autosave (default on; 800 ms after the last edit, on mode exit, on window deactivation, on close) and `Ctrl+S`. Atomic write (`S-FILE-03`). Preserve encoding, BOM, line endings and final-newline state. Dirty `●` in the title.
- **R-EDIT-03 MUST** Conflict detection: before writing, compare the file's current size/mtime/hash with the last known state. If it changed externally: banner "Changed on disk" with [Overwrite] [Reload (discard mine)] [Save mine as…].
- **R-EDIT-04 MUST** Undo/redo `Ctrl+Z`, `Ctrl+Y`, `Ctrl+Shift+Z`; typing coalesced (word boundary or 1 s pause); restores selection; history capped by memory (4 MB) with oldest dropped; shared between F2 and F3 for the same document.
- **R-EDIT-05 MUST** Clipboard: cut/copy/paste; `Ctrl+Shift+V` paste as plain text; pasting `CF_HTML` from browsers SHOULD convert to Markdown (sanitized, bounded); pasting an image → R-EDIT-08.
- **R-EDIT-06 MUST** **Tab snippets** (type trigger, press `Tab`): `h1`…`h6`, `table` (3×3), `tableRxC` (e.g. `table2x3`), `code`, any known language name (`python`, `rust`, `js`, …) → fenced block with that language, `link`, `img`, `line`/`hr`, `quote`, `task`, `note`/`tip`/`important`/`warning`/`caution`/`red` (red = caution) → callout, `math`, `mermaid`, `flow` (flowchart template), `seq` (sequence template), `gantt`, `date` (today ISO), `time`, `details`, `footnote`. Snippets place the caret at the first placeholder; `Tab` jumps to the next placeholder.
- **R-EDIT-07 MUST** Markdown-aware keys: `Enter` continues lists/tasks/quotes (empty item ends the list); `Tab`/`Shift+Tab` indent/outdent list items; `Ctrl+B` bold, `Ctrl+I` italic, `Ctrl+Shift+X` strikethrough, `Ctrl+Shift+H` highlight, ``Ctrl+` `` inline code, `Ctrl+K` link, `Ctrl+Shift+K` code block, `Ctrl+1…6` heading level, `Ctrl+Shift+0` paragraph, `Ctrl+Shift+Q` quote, `Ctrl+Shift+8` bullet list, `Ctrl+Shift+7` numbered list, `Ctrl+Shift+9` task list, `Ctrl+Enter` toggle task checkbox, `Ctrl+T` insert table, `Alt+↑/↓` move line/block, `Ctrl+Shift+D` duplicate line, `Ctrl+L` select line. Formatting toggles apply to the selection or the word at the caret. Auto-pairing of brackets/quotes/markers: setting, default off.
- **R-EDIT-08 MUST** Paste/drop image or file in edit modes → saved into `<doc folder>/<assets_dir>/<doc stem>/` (default `assets_dir = "assets"`), images as PNG named `image-YYYYMMDD-HHMMSS[-n].png`, files keep their sanitized original name (dedupe `-1`, `-2`); never overwrite; inserted as `![](assets/<stem>/<name>)` / `[name](assets/<stem>/<name>)` with forward slashes and `%20` for spaces. Files > 50 MB ask for confirmation. Rules `S-FILE-05`.
- **R-EDIT-09 MUST** IME: inline composition rendering, candidate window positioned at the caret (CJK, Korean, etc.); dead keys and AltGr layouts (Turkish Q/F) work.
- **R-EDIT-10 MUST** Find & Replace `Ctrl+H` (replace, replace all — one undo step); case / whole word. Regex MAY (only with a linear-time engine, ADR).
- **R-EDIT-11 MUST** Mode switching keeps caret and scroll on the same content: Read ↔ F2 ↔ F3 directly; `Esc` returns to read mode; pressing the active mode's key again also exits.
- **R-EDIT-12 MUST** Editing disabled (with reason) for read-only files and for files that could not be decoded losslessly (offer "Convert to UTF-8 and edit").
- **R-EDIT-13 MUST** Crash/power-loss safety: unsaved buffers are journaled to `%LOCALAPPDATA%\YAMDV\recovery\` (only when autosave is off or a save failed); offered for restore at next launch; deleted after successful save or discard.
- **R-EDIT-14 MUST** Grapheme-aware caret movement/deletion; `Ctrl+←/→` word movement (Unicode word boundaries); `Home`/`End` respect wrapped lines.

## A.13 Split edit — F3 (R-F3)
- **R-F3-01 MUST** Source editor + live preview side by side; swap sides via ⇄ button / menu (remembered); draggable splitter (20–80 %, default 50 %, remembered, scales with window; layout preserved on maximize/restore).
- **R-F3-02 MUST** Editor: line-number gutter (toggle), soft wrap (default on), current-line highlight (subtle), editor font = body font (default, matching the reference) or code font (setting); optional Markdown syntax tint (setting, default off).
- **R-F3-03 MUST** Preview updates ≤ 50 ms after typing stops (debounce ~30 ms, incremental).
- **R-F3-04 MUST** Scroll sync by block/section in both directions.
- **R-F3-05 MUST** Click in preview → caret to the matching source position; selection in preview selects the matching source range.
- **R-F3-06 MUST** "?" help button bottom-right of the editor pane → cheat-sheet overlay (syntax, shortcuts, snippets).
- **R-F3-07 MUST** Find works in the editor pane; matches mirrored in the preview (SHOULD).

## A.14 Live Edit — F2 WYSIWYG (R-F2)
- **R-F2-01 MUST** `F2` makes the rendered page editable in place; title pill "Live Edit"; reading position kept.
- **R-F2-02 MUST** **Reveal model** (Obsidian Live Preview / Typora style): the document stays rendered with the same typography; Markdown markers become visible (muted color) only where the caret is:
  - inline markers (`**`, `*`, `~~`, `==`, `` ` ``, `[`…`](url)`, `$`) appear when the caret is inside or adjacent to that span;
  - line/block markers (`## `, `> `, `- `, `1. `, `- [ ] `) appear when the caret is on that line;
  - code blocks: fence lines (```` ```lang ````) appear when the caret is in the block; the code is edited as monospace text with live highlighting;
  - math: caret inside `$…$`/`$$…$$` shows the source with a live-rendered preview popover below;
  - Mermaid: caret inside the block shows the source with the live diagram below;
  - images: caret on the image line shows `![alt](src)` above the still-visible image;
  - tables are **never** shown as pipes — they use cell editing (R-TBL).
- **R-F2-03 MUST** Typing transforms immediately: `# `…`###### ` at line start → heading; `- `/`* `/`+ ` → bullet; `1. ` → numbered; `[ ] `/`[x] ` in a list → task; `> ` → quote; ```` ``` ```` or ```` ```lang ```` + `Enter` → code block with caret inside (no stray blank lines); `---` + `Enter` → rule; `$$` + `Enter` → math block; a pipe header row + `Enter` → table with delimiter row.
- **R-F2-04 MUST** Floating format toolbar above a non-empty selection (setting to disable; `Esc` dismisses): Bold, Italic, Strikethrough, Highlight, Inline code, Link, H1, H2, H3, Quote, Bullet list, Numbered list, Task list, Clear formatting.
- **R-F2-05 MUST** Context menu adds: Cut/Copy/Paste/Paste as plain text, **Format ▸**, **Insert ▸** (Table…, Code block, Callout ▸ 5 types, Math block, Mermaid diagram, Horizontal rule, Image…, Link…, Footnote), **Table ▸** (when inside a table).
- **R-F2-06 MUST** Cross-block selection: copy/cut/delete across paragraphs, lists, tables and code in one step (one undo step); clipboard gets Markdown + HTML + plain text; pasting multi-block Markdown creates the blocks.
- **R-F2-07 MUST** Lists: `Enter`/`Tab`/`Shift+Tab` per R-EDIT-07; clicking a checkbox toggles `[ ]`↔`[x]`.
- **R-F2-08 MUST** Links: click places the caret; `Ctrl+click` follows (R-LINK); `Ctrl+K` opens an inline popover (text, URL, title).
- **R-F2-09 MUST** Code block language label is editable (click → input with autocomplete of known languages).
- **R-F2-10 MUST** Blocks the user did not touch are never re-serialized.
- **R-F2-11 MAY** Visual Mermaid editing for flowcharts (double-click node → edit label; written back to source).
- **R-F2-12 MAY** Image resize handles (write `|width` / HTML `width`).

## A.15 Table editing (R-TBL)
- **R-TBL-01 MUST** In-cell editing; `Tab`/`Shift+Tab` next/previous cell; `Tab` in the last cell adds a row; `Enter` → cell below (adds a row at the end); `Shift+Enter` inserts `<br>`; empty cells keep row height so the caret can be placed.
- **R-TBL-02 MUST** Quiet hover toolbar (top-right of the table): add row above/below, add column left/right, delete row/column, align column (left/center/right/none), sort column ascending/descending (natural, locale-aware; one undo step), move row up/down, move column left/right, delete table, ⛶ fullscreen, export CSV.
- **R-TBL-03 MUST** Drag handles: column grip (on header hover) and row grip (left edge) to reorder, with a live drop indicator.
- **R-TBL-04 MUST** `\|` escaping inside cells handled both ways.
- **R-TBL-05 MUST** Re-serialization of an edited table pretty-aligns its columns (setting "Auto-format tables", default on). Other tables are untouched.
- **R-TBL-06 MUST** Insert-table picker: grid up to 10×10 with live "R × C" label.
- **R-TBL-07 SHOULD** Pasting TSV/CSV (e.g. from Excel) into a table expands into cells; pasting TSV outside a table offers "Convert to table".
- **R-TBL-08 MUST** Inserting a table adjacent to another table never merges them (blank-line handling in source).

## A.16 Math (R-MATH)
- **R-MATH-01 MUST** Native TeX-math layout engine (no JS, no download) using the OpenType `MATH` table of the system font **Cambria Math** (present on every Windows 10/11), read via DirectWrite.
- **R-MATH-02 MUST** KaTeX-compatible subset, at least: `^ _` (with limits rules), `\frac \dfrac \tfrac \cfrac \binom \dbinom \tbinom`, `\sqrt[n]{}`, big operators `\sum \prod \coprod \int \iint \iiint \oint \bigcup \bigcap \bigoplus \bigotimes \lim \limsup \liminf \max \min \sup \inf \det \gcd`, `\limits \nolimits`, delimiters `\left \right \middle` and `\big \Big \bigg \Bigg`, environments `matrix pmatrix bmatrix Bmatrix vmatrix Vmatrix smallmatrix cases dcases aligned align align* gathered gather split array{…} equation`, `\\` and `&`, accents `\hat \widehat \bar \overline \underline \vec \overrightarrow \overleftarrow \dot \ddot \tilde \widetilde \acute \grave \breve \check \mathring`, `\overbrace{}^{} \underbrace{}_{}`, `\overset \underset \stackrel`, fonts `\mathrm \mathbf \mathit \mathsf \mathtt \mathcal \mathbb \mathfrak \mathscr \boldsymbol \bm \text \textbf \textit \textrm \operatorname`, all Greek letters and variants, ≈ 500 common symbols/relations/arrows/operators (KaTeX "supported functions" core list), spacing `\, \: \; \! \quad \qquad \hspace{}`, `\not`, `\cancel`, `\boxed`, `\color{name|#hex}{…}`, `\textcolor`, `\tag{}` (right-aligned number), `\displaystyle \textstyle \scriptstyle`, `\phantom`, `\mod \bmod \pmod`, `\ldots \cdots \vdots \ddots`, primes `'`.
- **R-MATH-03 MUST** Errors never crash: the source is rendered in the error color (`#cc0000` light / `#ff6b6b` dark) with a tooltip containing the message and position.
- **R-MATH-04 MUST** Copying a formula copies its LaTeX source with delimiters.
- **R-MATH-05 MUST** Display math centered; too-wide formulas scroll horizontally within the block.
- **R-MATH-06 MUST** Colors follow the theme; `\color` overrides.
- **R-MATH-07 SHOULD** Per-document `\newcommand`, `\renewcommand`, `\def` (simple, bounded expansion — `S-PARSE-05`).

## A.17 Mermaid (R-MMD)
- **R-MMD-01 MUST** Native parser + layout + renderer (no JS). Diagram types: **flowchart/graph (MUST), sequenceDiagram (MUST), gantt (MUST), pie (MUST)**, stateDiagram/stateDiagram-v2 (SHOULD), classDiagram (SHOULD), erDiagram (SHOULD), mindmap (SHOULD), timeline, journey, gitGraph, quadrantChart (MAY). Unsupported types → shown as a code block with badge "Diagram type not supported".
- **R-MMD-02 MUST** Rendered in a card (Appendix B §B.6.12), scaled down to fit the column (never up); hover ⛶ / double-click → fullscreen viewer.
- **R-MMD-03 MUST** Light and dark palettes (Appendix B §B.6.12); `%%{init: {"theme": "…"}}%%` directive honors `default`, `neutral`, `dark`, `forest`, `base` and a subset of `themeVariables` (primaryColor, primaryTextColor, primaryBorderColor, lineColor, secondaryColor, tertiaryColor, fontSize) (SHOULD).
- **R-MMD-04 MUST** Syntax errors → card shows the source plus "Mermaid syntax error at line L, column C: <message>".
- **R-MMD-05 MUST** Limits per `S-PARSE-06`; over limit → placeholder "Diagram too large (N nodes)" with [Render anyway] (still capped by the hard limit).
- **R-MMD-06 MUST Flowchart:** directions `TB TD BT LR RL`; node shapes `[]` rect, `()` round, `([])` stadium, `[[]]` subroutine, `[()]` cylinder, `(())` circle, `((()))` double circle, `>]` asymmetric, `{}` rhombus, `{{}}` hexagon, `[/ /]` parallelogram, `[\ \]` alt-parallelogram, `[/ \]` trapezoid, `[\ /]` alt-trapezoid; edges `-->` `---` `-.->` `-.-` `==>` `===` `--o` `--x` `<-->` `o--o` `x--x`, extra-length variants (`--->`, `-.-->`), labels `-->|text|` and `-- text -->`, chaining `A --> B --> C`, `A & B --> C & D`; nested `subgraph id [title] … end` with optional `direction`; `classDef`, `class`, `:::cls`, `style` (fill, stroke, color, stroke-width, stroke-dasharray), `linkStyle` (basic); `%%` comments; quoted labels with entities; `click`/`callback` lines are parsed and **ignored** (security). Layout: layered (Sugiyama) — cycle breaking, layer assignment, crossing reduction (barycenter, a few sweeps), coordinate assignment, orthogonal-ish or spline edges with arrowheads, label placement.
- **R-MMD-07 MUST Sequence:** `participant`/`actor` with `as` alias, messages `->` `-->` `->>` `-->>` `-x` `--x` `-)` `--)`, activations (`+`/`-`, `activate`/`deactivate`), `Note left of|right of|over A[,B]`, blocks `loop alt else opt par and critical option break rect`, `autonumber`, `title`, participant boxes at top and bottom.
- **R-MMD-08 MUST Gantt:** `title`, `dateFormat` (common Day.js tokens `YYYY MM DD HH mm`), `axisFormat` (common strftime tokens), `excludes weekends`, `section`, tasks with tags `done active crit milestone`, ids, `after id`, start dates, durations `d w h m`, today marker (local clock).
- **R-MMD-09 MUST Pie:** `title`, `showData`, `"label" : value`; legend; palette in Appendix B.
- **R-MMD-10 SHOULD** Context menu on a diagram: "Copy as PNG" (clipboard), "Copy source".

## A.18 Mind map view (R-MAP)
- **R-MAP-01 MUST** Menu "View as mind map" (`Ctrl+Shift+M`): fullscreen overlay; root = document title (first H1 or file name); headings → branches by level; list items → leaves (text truncated to 80 chars); right-expanding tree layout (SHOULD offer balanced two-sided layout); curved connectors; branch colors from the scheme accent family; theme-aware.
- **R-MAP-02 MUST** Interactions: wheel zoom, drag pan, click a node to collapse/expand, double-click → close and jump to that heading; `Esc` closes. "Copy as PNG" SHOULD.

## A.19 Export and print (R-EXP)
- **R-EXP-01 MUST** **Export PDF** (native writer — `pdf-writer` + `subsetter`, no external tools): vector, selectable/searchable text with subset-embedded fonts; images embedded; colors per setting `pdf_colors` (*Follow theme* default, or *Light*), backgrounds kept (dark themes do not turn white unless *Light* is chosen); page size *Auto* (Letter for en-US/en-CA locales, else A4) / A4 / Letter; margins 18 mm; optional footer page numbers "n / N"; PDF outline (bookmarks) from headings; internal links clickable; external links as link annotations; code wraps in PDF; avoid breaking inside short blocks and table rows; never emit a blank trailing page. Default file name `<doc stem>.pdf`. Progress toast and a completion toast with [Show in folder].
- **R-EXP-02 MUST** **Export single-file HTML**: one `.html` with inline CSS for the current theme, images embedded as `data:` URIs (only files that decoded successfully as images, within limits), math and Mermaid as inline SVG generated from our own layouts, highlighted code as spans, TOC sidebar, collapsible H1–H3, light/dark toggle, in-page find — implemented with a tiny inline script protected by a hash-based CSP (`S-EXP-02`). No external references of any kind.
- **R-EXP-03 MUST** **Print** `Ctrl+P`: system print dialog (`PrintDlgExW`); same pagination engine as PDF; colors per `print_colors` (*Light* default / *Follow theme*).
- **R-EXP-04 MUST** **Export table as CSV** (from table toolbar or fullscreen table): UTF-8 with BOM, CRLF, RFC 4180 quoting; formatting stripped to plain text.
- **R-EXP-05 MUST** Headless CLI export: `yamdv --export-html <in.md> [-o <out.html>]`, `yamdv --export-pdf <in.md> [-o <out.pdf>]`, options `--theme <scheme>`, `--mode light|dark`; no window; non-zero exit code on error.
- **R-EXP-06 SHOULD** Copy whole document as rich HTML; export a Mermaid diagram or the mind map as PNG.

## A.20 External editor (R-EXT)
- **R-EXT-01 MUST** `F4` / `Ctrl+E` opens the file in an external editor at the line of the selection start, or of the topmost visible block.
- **R-EXT-02 MUST** Auto-detection (first found): VS Code (`Code.exe --goto "<file>:<line>:<col>"`), VSCodium, Cursor (same CLI as VS Code), Sublime Text (`subl.exe "<file>:<line>:<col>"`), Notepad++ (`notepad++.exe -n<line> -c<col> "<file>"`), EmEditor (`EmEditor.exe /l <line> /cl <col> "<file>"`), Typora (no line), Notepad (`notepad.exe "<file>"`). Detection uses known install folders and the read-only `App Paths` registry keys; it resolves to real `.exe` files only.
- **R-EXT-03 MUST** Custom editor: absolute path to an existing `.exe` plus an argument template with `{file}`, `{line}`, `{col}` placeholders; arguments are tokenized and quoted per Windows rules and passed to `CreateProcessW` — **never through a shell** (`S-EXEC-03`).
- **R-EXT-04 MUST** Changes saved by the external editor appear via live reload.

## A.21 System integration (R-SYS)
- **R-SYS-01 MUST** Per-user registration only (HKCU, no admin): ProgID `YAMDV.Markdown` (friendly type name, `DefaultIcon` = document icon in the exe, `shell\open\command` = `"<exe>" "%1"`), `Software\YAMDV\Capabilities` (FileAssociations for all Markdown extensions) + `Software\RegisteredApplications\YAMDV`, `OpenWithProgids` entries for `.md .markdown .mdown .mkd .mkdn .mdwn .mdtxt .mdtext`, `Applications\yamdv.exe` (SupportedTypes); then `SHChangeNotify(SHCNE_ASSOCCHANGED)`.
- **R-SYS-02 MUST** Opening files never changes the system default. "Set as default app" (welcome page and Settings) **never writes `UserChoice`**; it opens Windows Settings for the app (`ms-settings:defaultapps?registeredAppUser=YAMDV` on Windows 11; `ms-settings:defaultapps` + an instruction toast on Windows 10). Association status (default / not default) is queried with `AssocQueryStringW` / `IApplicationAssociationRegistration` and shown on the welcome page and in Settings.
- **R-SYS-03 MUST** Optional Explorer context-menu verb "Open with YAMDV" for the Markdown extensions (HKCU `SystemFileAssociations`), toggled in Settings or via CLI.
- **R-SYS-04 MUST** CLI per Appendix F §F.4.
- **R-SYS-05 MUST** Self-install (no third-party installer): `yamdv --install` copies the exe to `%LOCALAPPDATA%\Programs\YAMDV\`, registers per R-SYS-01, creates a Start-menu shortcut, writes an HKCU `Uninstall` entry (DisplayName, DisplayIcon, DisplayVersion, Publisher, EstimatedSize, InstallLocation, UninstallString `"<exe>" --uninstall`), and offers "Set as default app". `--uninstall` removes every key/shortcut it created and the install folder (self-deletion via a temp copy helper that waits for the parent to exit), asking whether to keep settings.
- **R-SYS-06 MUST** Portable mode per R-SET-05.
- **R-SYS-07 MUST** Version resource; original icons (app + document) embedded.
- **R-SYS-08 MAY** User-supplied custom app/document icon.
- **R-SYS-09 MAY** Taskbar jump list with recent files.

## A.22 Internationalization (R-I18N)
- **R-I18N-01 MUST** UI languages **English** and **Türkçe**; setting *Follow System* (default: `tr-*` → Turkish, otherwise English) / English / Türkçe; switching is live.
- **R-I18N-02 MUST** All UI strings are compile-time `&'static str` tables keyed by an enum (no runtime files); a test fails if any key is missing in a language. Dates via `GetDateFormatEx` with the user locale; numbers with locale digit grouping.
- **R-I18N-03 MUST** Document content in any script; grapheme-correct editing.
- **R-I18N-04 MUST** Locale-independent internal processing: file extensions, URL schemes, setting keys, language tags compared **ASCII-case-insensitively** (never culture-sensitive — avoids the Turkish-İ bug). User-facing find uses R-READ-05 rules.

## A.23 Accessibility (R-A11Y)
- **R-A11Y-01 MUST** Every UI element is keyboard-operable with a visible focus ring (2 px accent).
- **R-A11Y-02 MUST** High Contrast (R-THEME-08); reduced motion (`SPI_GETCLIENTAREAANIMATION` off → no animations, instant scrolling).
- **R-A11Y-03 SHOULD** UI Automation: window, buttons, menus, tabs, list items, settings controls with names/roles; the document exposed as a Document control with TextPattern (read-only text) — Phase 012.
- **R-A11Y-04 MUST** Contrast: body text ≥ 4.5:1, links ≥ 4.5:1, headings ≥ 3:1 against the page background in every palette (unit test computes WCAG ratios for all tokens).

## A.24 Reliability (R-REL)
- **R-REL-01 MUST** No input file can crash, hang (> 2 s UI block) or exhaust memory: fuzzed parsers, depth/size limits (`S-PARSE-*`), time budgets with graceful placeholders.
- **R-REL-02 MUST** State files are written atomically and debounced (≤ 1 write per 2 s); corrupt state files are renamed `*.corrupt-<ts>` and defaults used.
- **R-REL-03 MUST** All OS failures (fonts missing, WIC codec missing, clipboard busy, file locked) degrade gracefully with a toast/placeholder — never a crash.
- **R-REL-04 MUST** Works without admin rights, on locked-down machines, from read-only media (portable mode then falls back to `%APPDATA%`).

---

# APPENDIX B — UI & Visual Specification (`docs/UI_SPEC.md`)

All sizes are **logical px** (DIPs at 96 DPI); multiply by `dpi / 96`. Snap 1 px lines and rectangle edges to device pixels so borders stay crisp at every scale. Colors are sRGB hex; `rgba()` values are alpha-blended over what is beneath.

## B.1 The look in one paragraph (reference description)
A plain, bright page that feels like a well-made documentation website. On the left, a quiet full-height sidebar (260 px, same background as the page, separated only by a 1 px hairline) with small text tabs at the top — **Contents · History · Favorites · Files** — the active tab in the accent blue with a 2 px underline. Below, the heading outline in 13 px text; the current section is blue. A tiny chevron sits on the sidebar's hairline, vertically centered, to hide it. The document column is centered in the remaining space, ~65 % of the window wide, with generous top padding. Headings are dark and semibold (32 / 24 / 20 px), body text is 16 px Segoe UI with 1.6 line height in `#333`. Code blocks are light-gray rounded cards with a hairline border and a hover-only toolbar (copy, line numbers, wrap, language). Tables have hairline borders and a light-gray header row. Callouts are tinted cards with a 3 px colored left bar and an icon + title row. In dark mode the page becomes `#1e1e1e` with `#d4d4d4` text and slightly lighter cards. There are no toolbars, no status bar, no ribbon: just the document. The title bar is custom-drawn in the page color, so the window looks like one seamless sheet.

## B.2 Typography
| Role | Family (fallbacks) | Size | Weight | Line height |
|---|---|---|---|---|
| Body | setting `body_font` (default `Segoe UI`; DirectWrite system fallback for other scripts) | `font_size` (default 16) | 400 | `line_height` (default 1.6) |
| H1 / H2 / H3 / H4 / H5 / H6 | body family | 2.0 / 1.5 / 1.25 / 1.125 / 1.0 / 0.875 em | 600 | 1.25 |
| Code (blocks & inline) | setting `code_font` (default `Cascadia Mono` → `Consolas` → `Courier New`) | blocks 0.875 em (14 px); inline 0.875 em | 400 | blocks 1.6 |
| Math | `Cambria Math` | 1.21 em | — | — |
| UI chrome (title, tabs, menus, settings, toasts) | `Segoe UI Variable Text` (Win 11) → `Segoe UI` | 12–13 px (per component) | 400 / 600 | 1.4 |
| UI icons | `Segoe Fluent Icons` (Win 11) → `Segoe MDL2 Assets` (Win 10) → vector fallback | per component | — | — |

Text rendering: ClearType when the system uses it, grayscale on rotated/transparent layers, DirectWrite natural symmetric rendering mode; honor the user's ClearType settings.

## B.3 Color tokens

### B.3.1 Document tokens — light palettes
| Scheme | `bg` | `text` | `heading` | `link` | `accent` | `quote_bar` | `quote_text` | `code_bg` | `inline_code_bg` | `table_header_bg` |
|---|---|---|---|---|---|---|---|---|---|---|
| Default | `#ffffff` | `#333333` | `#333333` | `#2563eb` | `#2563eb` | `#dddddd` | `#666666` | `#f5f5f5` | `#f0f0f0` | `#f5f5f5` |
| GitHub | `#ffffff` | `#1f2328` | `#1f2328` | `#0969da` | `#0969da` | `#d1d9e0` | `#59636e` | `#f6f8fa` | `#eff1f3` | `#f6f8fa` |
| Glacier | `#f6f9fe` | `#1a2538` | `#2563eb` | `#2563eb` | `#2563eb` | `#2563eb` | `#6e7684` | `#e8f0fe` | `#e6edfc` | `#e8f0fe` |
| Forest | `#f5faf5` | `#1a2e1a` | `#008a2e` | `#00822b` | `#008a2e` | `#008a2e` | `#6d7c6d` | `#e8f8e8` | `#e2f1e6` | `#e8f8e8` |
| Sunset | `#fef8f0` | `#2c1a0a` | `#b45309` | `#b45309` | `#b45309` | `#b45309` | `#7c6e61` | `#faf0e0` | `#f8ecde` | `#faf0e0` |
| Typewriter | `#f5f0e8` | `#2c2416` | `#8b6914` | `#896714` | `#8b6914` | `#8b6914` | `#787166` | `#ede5d8` | `#ede5d8` | `#ede5d8` |
| Editorial | `#faf8f5` | `#2c2416` | `#8b4513` | `#8b4513` | `#8b4513` | `#8b4513` | `#7a746b` | `#f5f0e8` | `#f1eae3` | `#f5f0e8` |
| Violet | `#faf5fe` | `#271b30` | `#7209b7` | `#7209b7` | `#7209b7` | `#7209b7` | `#776e7f` | `#f4eafb` | `#efe3f8` | `#f4eafb` |
| Lychee | `#fff5f5` | `#2b2726` | `#e63946` | `#db1c2a` | `#e63946` | `#e63946` | `#7c7575` | `#ffeeee` | `#fde6e7` | `#ffeeee` |
| Mint | `#f5fbf8` | `#1a3a2a` | `#0ea271` | `#0b815a` | `#10b981` | `#10b981` | `#6d8378` | `#e8f8f0` | `#e3f6ef` | `#e8f8f0` |
| Neon | `#f0f4f8` | `#1a1a2e` | `#0077b6` | `#0072ae` | `#0077b6` | `#0077b6` | `#6b6d7b` | `#e2e8f0` | `#ddeaf3` | `#e2e8f0` |

Shared light tokens: `code_border #e0e0e0`, `table_border #dddddd`, `border_subtle #eeeeee`, `zebra_bg = mix(bg, table_header_bg, 50%)`, `text_muted #7a7a7a`, `marker #b0b0b0` (revealed Markdown syntax in F2), `line_number #b8b8b8`, `selection = accent @ 20 %`, `highlight_bg #fff3a3`, `find_match_bg #fff3a3`, `find_current_bg #ffb547`, `error #cc0000`.
GitHub scheme extras: H1 and H2 get a 1 px bottom border `#d1d9e0` with 0.3 em padding-bottom.
(The values were sampled from the reference screenshots; `link` and Mint `heading` were minimally darkened to meet R-A11Y-04.)

### B.3.2 Document tokens — dark palettes
Base for every scheme except GitHub: `bg #1e1e1e`, `text #d4d4d4`, `quote_text #aaaaaa`, `inline_code_bg #3a3a3a`, `code_border #3a3a3a`, `table_border #3a3a3a`, `border_subtle #333333`, `text_muted #9a9a9a`, `marker #6a6a6a`, `line_number #5a5a5a`, `selection = accent @ 30 %`, `highlight_bg #665c1e` (text `#f5f0d0`), `find_match_bg #5c5427`, `find_current_bg #9e6a03`, `error #ff6b6b`.

| Scheme | `heading` | `link` / `accent` | `quote_bar` | `code_bg` | `table_header_bg` |
|---|---|---|---|---|---|
| Default | `#d4d4d4` | `#4da5ec` | `#555555` | `#2d2d2d` | `#2d2d2d` |
| Glacier | `#608df0` | `#608df0` | `#4966a6` | `#31353d` | `#323740` |
| Forest | `#5ff290` | `#5ff290` | `#48a868` | `#313d35` | `#324137` |
| Sunset | `#f29e5f` | `#f29e5f` | `#a87148` | `#3d3631` | `#413832` |
| Typewriter | `#e9c467` | `#e9c467` | `#a28a4d` | `#3c3932` | `#403c33` |
| Editorial | `#ea9d66` | `#ea9d66` | `#a3714d` | `#3c3632` | `#403833` |
| Violet | `#b85ff2` | `#b85ff2` | `#8248a8` | `#38313d` | `#3b3241` |
| Lychee | `#ec656f` | `#ec656f` | `#a44c53` | `#3c3132` | `#403334` |
| Mint | `#5ff1c1` | `#5ff1c1` | `#48a788` | `#313d39` | `#32413c` |
| Neon | `#5fbff2` | `#5fbff2` | `#4887a8` | `#31393d` | `#323c41` |

GitHub dark: `bg #0d1117`, `text #e6edf3`, `heading #e6edf3`, `link/accent #4493f8`, `quote_bar #3d444d`, `quote_text #9198a1`, `code_bg #151b23`, `inline_code_bg #262c36`, `code_border #3d444d`, `table_border #3d444d`, `table_header_bg #151b23`, H1/H2 bottom border `#3d444d`.

### B.3.3 Callout tokens
| Type | Light color (bar, icon, title) | Light bg | Dark color | Dark bg |
|---|---|---|---|---|
| note | `#0969da` | `#ddf4ff` | `#4493f8` | `#232c38` |
| tip | `#1a7f37` | `#e8faf0` | `#3fb950` (title `#57ab5a`) | `#202d22` |
| important | `#8250df` | `#f6f0ff` | `#ab7df8` | `#2f2938` |
| warning | `#9a6700` | `#fffbe8` | `#d29922` | `#342d1e` |
| caution | `#cf222e` | `#ffebe9` | `#f85149` | `#382423` |
Callout body text uses `quote_text`.

### B.3.4 Syntax highlighting palettes
| Token class | Light | Dark |
|---|---|---|
| default text | `#24292e` | `#c9d1d9` |
| comment | `#6a737d` (italic) | `#8b949e` (italic) |
| keyword, meta/preprocessor | `#d73a49` | `#ff7b72` |
| string, regex | `#032f62` | `#a5d6ff` |
| number, constant, builtin, attribute | `#005cc5` | `#79c0ff` |
| function, class/title | `#6f42c1` | `#d2a8ff` |
| type, variable (special) | `#e36209` | `#ffa657` |
| tag | `#22863a` | `#7ee787` |
| operator, punctuation | `#24292e` | `#c9d1d9` |
| diff-add (line bg / text) | `#e6ffec` / `#22863a` | `#12261e` / `#aff5b4` |
| diff-del (line bg / text) | `#ffebe9` / `#b31d28` | `#25171c` / `#ffdcd7` |
| markdown heading in code | `#005cc5` bold | `#79c0ff` bold |
| code toolbar icons / language label | `#bcbcbc` (hover `#7a7a7a`) | `#5f5f5f` (hover `#9a9a9a`) |

### B.3.5 Chrome tokens
| Token | Light | Dark |
|---|---|---|
| `chrome_bg` (title bar, sidebar) | = `bg` | = `bg` |
| `chrome_border` (sidebar hairline, splitter, cards) | `#e0e0e0` | `#333333` |
| `tab_inactive` | `#7a7a7a` | `#9a9a9a` |
| `history_group` | `#9e9e9e` | `#7a7a7a` |
| `chevron` | `#858585` | `#8a8a8a` |
| `hover` / `pressed` overlay | `rgba(0,0,0,.05)` / `rgba(0,0,0,.09)` | `rgba(255,255,255,.06)` / `rgba(255,255,255,.10)` |
| `menu_bg` / `menu_border` / `menu_hover` / `separator` | `#ffffff` / `#e0e0e0` / `#f0f0f0` / `#eeeeee` | `#2b2b2b` / `#3a3a3a` / `#383838` / `#3a3a3a` |
| `shadow` | `0 8 24 rgba(0,0,0,.14)` | `0 8 24 rgba(0,0,0,.45)` |
| `tooltip_bg` / `tooltip_text` | `#333333` / `#ffffff` | `#f0f0f0` / `#1e1e1e` |
| `toast_bg` / `toast_text` | `rgba(32,32,32,.92)` / `#ffffff` | same |
| `caption_close_hover` | `#c42b1c` (glyph `#ffffff`) | same |
| `gutter_bg` / `gutter_border` (F3) | `#fafafa` / `#f0f0f0` | `#252526` / `#2d2d2d` |
| `scrim` (fullscreen viewers) | `rgba(0,0,0,.82)` | same |
| `focus_ring` | `accent`, 2 px | `accent`, 2 px |
| `scrollbar_thumb` / hover | `rgba(0,0,0,.28)` / `.42` | `rgba(255,255,255,.28)` / `.42` |
Chrome accents (active tab, active TOC item, toggles, focus) use the scheme's `accent`.

### B.3.6 Mermaid palettes
| Element | Light | Dark |
|---|---|---|
| card bg / border | `#f5f5f5` / `#e0e0e0` (follows `code_bg`/`code_border` for colored schemes) | `#2d2d2d` / `#3a3a3a` |
| node fill / stroke / text | `#eff6ff` / `#93c5fd` / `#333333` | `#1e3a5f` / `#60a5fa` / `#e5e7eb` |
| edge / arrowhead | `#94a3b8` | `#94a3b8` |
| edge label bg / text | card bg / `#333333` | card bg / `#e5e7eb` |
| subgraph fill / stroke / title | `#f8fafc` / `#cbd5e1` / `#475569` | `#262b33` / `#475569` / `#cbd5e1` |
| sequence actor box / lifeline / message | `#eff6ff`+`#93c5fd` / `#94a3b8` / `#333333` | `#1e3a5f`+`#60a5fa` / `#64748b` / `#e5e7eb` |
| note fill / stroke | `#fff8c5` / `#e3c34d` | `#3b3415` / `#a88f2a` |
| gantt bar / done / active / crit / milestone / grid / today | `#93c5fd` / `#cbd5e1` / `#60a5fa` / `#f87171` / `#2563eb` / `#e5e7eb` / `#ef4444` | `#3b82f6` / `#475569` / `#60a5fa` / `#ef4444` / `#93c5fd` / `#374151` / `#f87171` |
| pie / categorical series | `#60a5fa #34d399 #fbbf24 #f87171 #a78bfa #f472b6 #22d3ee #a3e635 #fb923c #94a3b8` | same hues at 85 % lightness-adjusted for dark bg |
Font: body family, 14 px; node padding 12 × 8; corner radius 6 for rect nodes (sharp style → 0).

## B.4 Window frame and title bar
- Custom-drawn title bar (`WM_NCCALCSIZE` extends the client area over the caption), **height 32 px**, background `chrome_bg` (seamless with the page). When the document is scrolled away from the top, a 1 px `border_subtle` line appears under the title bar.
- Left → right: 12 px padding · app icon 16 px · 8 px · title text 12 px (`text`; inactive window → 55 % opacity), ellipsized in the middle for long paths · 8 px · mode pill (11 px, padding 2 × 8, radius 9, bg `accent @ 12 %`, text `accent`; "Read-only" uses warning colors) · dirty `●` (`text_muted`).
- Right → left: Close, Maximize/Restore, Minimize (each **46 × 32**, glyphs 10 px from the icon font), then 4 px gap, **⋯ More** (32 × 32, glyph 14 px), **⚙ Settings** (32 × 32, glyph 14 px). Hover/pressed overlays per B.3.5; Close hover `#c42b1c` with white glyph.
- Behavior: drag to move; double-click toggles maximize (single click never restores); dragging a maximized window restores it under the cursor; right-click / `Alt+Space` → system menu; `WM_NCHITTEST` returns `HTMAXBUTTON` over Maximize so Windows 11 **Snap Layouts** appear; resize borders 6 px (outside the visible frame on Windows 11); DWM shadow and Windows 11 rounded corners kept (`DWMWA_WINDOW_CORNER_PREFERENCE`).
- Fullscreen (`F11`): title bar hidden; moving the mouse to the top 4 px reveals it temporarily.

## B.5 Sidebar
### B.5.1 Frame
Width 260 (180–480, drag the inner edge; cursor `IDC_SIZEWE` over a 6 px grip). Background `chrome_bg`; 1 px `chrome_border` on its inner edge. Floating positions: card 12 px from the window edges and 8 px below the title bar, radius 12, `shadow`, 1 px `chrome_border`.
### B.5.2 Tabs row
Height 38; tabs left-aligned starting at x = 4; each tab: 13 px text, horizontal padding 10; inactive `tab_inactive`, hover `text`, active `accent` with a 2 px `accent` underline spanning the tab width; 1 px `chrome_border` under the row. Hidden tabs are removed; if only Contents remains, the row is hidden.
### B.5.3 Contents (outline)
List top padding 12; row height `round(toc_font_size × 1.95)` (25 at 13 px); text `toc_font_size` px, color `text`, single line with ellipsis + tooltip; left padding 20 + 12 px per depth level (after title flattening, R-NAV-05); H1 rows weight 600; **active row** color `accent`; hover overlay radius 4 (inset 4 px horizontally). Collapsible rows show a muted 8 px chevron at x = (indent + 6) — ▾ expanded / ▸ collapsed. Top fade: when the list is scrolled, a 16 px gradient from `chrome_bg` to transparent overlays the top.
### B.5.4 History
Group header: 11 px `history_group`, 14 px top margin, row height 24; hover shows "Clear" (11 px link) at the right. Item rows: 13 px `text`, height 29, left padding 12; current file tagged "Current" (11 px `accent`, right-aligned 34 px from the sidebar's inner edge); hover shows × (12 px) at the right; missing files 50 % opacity.
### B.5.5 Favorites & Files
Favorites: same row style as History items (star 12 px muted at left). Empty state: centered star icon 24 px muted + hint text 12 px muted. Files: tree rows 13 px, height 26, folder chevron + folder glyph 14 px, file glyph 14 px, indent 16 per level, current file `accent`.
### B.5.6 Edge toggle and scrollbars
Toggle: a 16 × 32 hit area centered vertically **on the sidebar's inner hairline**, glyph chevron 12 px `chevron` ("‹" when open on the left, "›" when closed — then it sits 4 px from the window's left edge). Hover: `hover` overlay radius 4, glyph `text`. It follows the edge while resizing. Mirror for right positions.
Overlay scrollbars (document, sidebar lists, panels): 8 px wide (12 px on hover), 2 px from the edge, thumb radius = width / 2, min thumb length 32, track invisible (hover: `hover` overlay). Shown on scroll/hover, hidden after 1 000 ms idle with a 300 ms fade (setting off → always visible). Thumb drag; track click pages.

## B.6 Document layout and elements
### B.6.1 Column
Available area = client area minus docked sidebar. Column width = `clamp(column_width_pct % × client_width, min(360, available − 48), available − 48)`, centered within the available area, always ≥ 20 px from the sidebar. Top padding 48 (below the title bar), bottom padding 120. Cursor: I-beam over text, hand over links/buttons, arrow elsewhere.
### B.6.2 Blocks and spacing
Paragraph margin-bottom `paragraph_spacing` (16). Headings: H1 margin 0 0 16 (top margin 0 if first block, else 32); H2 24 0 16; H3 24 0 12; H4–H6 20 0 8; H6 color `text_muted`. Strong = weight 600 (DirectWrite chooses the nearest available weight). Strikethrough line in `text`. `==highlight==` → `highlight_bg` with 2 px horizontal padding.
### B.6.3 Inline code and kbd
Inline code: code font 0.875 em, bg `inline_code_bg`, padding 0.15 em × 0.4 em, radius 4 (sharp 0). `<kbd>`: code font 0.8125 em, bg `code_bg`, 1 px `code_border` with a 2 px bottom border, radius 4, padding 1 × 5.
### B.6.4 Lists
Content starts 40 px right of the list's left edge; the marker sits right-aligned in that box with an 8 px gap. Bullet: filled disc (depth 1), ring (depth 2), square (depth 3+), size 0.35 em, color `text`. Ordered: "1." in `text`, tabular figures. Tight lists: no extra spacing; loose lists: 8 px between items. Task items: 16 × 16 checkbox at the marker position, radius 4, 1.5 px border `#b0b0b0` (dark `#6a6a6a`); checked → `accent` fill with a white check.
### B.6.5 Blockquote
Margin 16 0; 3 px bar `quote_bar` at the left; content starts 16 px after the bar; text `quote_text`; nested quotes add another bar 16 px further in.
### B.6.6 Callout
Margin 16 0; bg per B.3.3; 3 px bar in the callout color; padding 12 16 12 16 (measured from the bar); radius 0 6 6 0 (sharp 0). Title row: 14 px icon glyph + 6 px + title 14 px weight 600 in the callout color; 8 px gap to the body; foldable callouts have a chevron at the far right of the title row. Default titles (localized): Note, Tip, Important, Warning, Caution.
### B.6.7 Code block
Margin 16 0; bg `code_bg`; 1 px `code_border`; radius 8 (sharp 0). A **28 px header strip** inside the top + 16 px padding before the first line; left/right padding 32; bottom padding 16. Code text per B.2; line numbers (when on): right-aligned in a gutter `digits × digit_width + 16`, color `line_number`, text starts 12 px after the gutter. Toolbar inside the header strip, right-aligned, 8 px from the right: [copy] [line numbers] [wrap] buttons 24 × 24 (glyph 13 px) + language label 11 px; colors per B.3.4; visible on hover (120 ms fade) or keyboard focus; active toggles shown in `text` color (never blue). Copy → glyph becomes a check for 1.2 s + toast "Copied". Overflow: a 6 px horizontal scrollbar at the block's bottom, always visible while overflowing.
### B.6.8 Table
Margin 16 0; auto layout (shrink-to-content up to the column width; min column 48 px; then wrap, breaking anywhere if a token is too long). 1 px `table_border` grid; header bg `table_header_bg`, header weight 600; cell padding 8 × 8; alignment per column; zebra (setting) on even body rows with `zebra_bg`; rounded style clips outer corners at radius 6. Hover toolbar: a small pill above the table's top-right corner (28 px tall, icons 16 px, 60 % opacity → 100 % on hover): read mode [⛶] [CSV]; F2 adds the editing tools (R-TBL-02).
### B.6.9 Horizontal rule, footnotes, images, front matter, details
- HR: 2 px `border_subtle`, margin 24 0.
- Footnote reference: superscript 0.75 em `link` text `[n]`. Footnotes section: margin-top 32, 1 px `border_subtle` top border, padding-top 16, 14 px `text_muted`, numbered; `↩` back-link in `link`.
- Images: block images margin 8 0, left-aligned (centered when the HTML says `align="center"` or inside a centered `<p>/<div>`); inline images sit on the baseline. Hover badge ⛶: 28 × 28 circle `toast_bg` with a white 13 px glyph, 8 px inside the top-right corner. Placeholder: 1 px dashed `code_border`, padding 12, icon 16 px + text 13 px `text_muted`.
- Front matter card: margin 0 0 16, bg `code_bg`, radius 6, padding 8 × 12; collapsed: 13 px `text_muted` line "Front matter · N fields" + chevron; expanded: 2-column grid, keys in code font 13 px `text_muted`, values 13 px `text`.
- `<details>`: summary row with 12 px chevron and weight 600; content indented 16; margin 8 0.
### B.6.10 Links
`link` color, no underline; underline on hover; hand cursor; status tooltip (B.10.4).
### B.6.11 Math
Inline: baseline-aligned, 1.21 em. Display: centered, margin 16 0, no card; overflow scrolls horizontally; `\tag` numbers right-aligned in `text_muted`. Errors in `error` color.
### B.6.12 Mermaid card
Margin 16 0; bg/border per B.3.6; radius 8 (sharp 0); padding 24; diagram centered and scaled to fit (never upscaled); hover ⛶ badge like images.
### B.6.13 Selection and find highlights
Selection fill `selection`, drawn beneath text, per line box (no gaps between lines). Find: all matches `find_match_bg`, current `find_current_bg`, radius 2.

## B.7 Menus
### B.7.1 Style
Popup windows (layered, owner-drawn), width 260 (min; grows for long labels up to 360), padding 4, radius 8 (sharp 0), bg `menu_bg`, 1 px `menu_border`, `shadow`. Item: height 32, padding 0 12, 20 px icon column (16 px glyph), label 13 px, shortcut right-aligned 12 px `text_muted`, submenu chevron; hover/keyboard-selected: `menu_hover` radius 4; disabled 40 % opacity; check/radio marks in the icon column; separators 1 px `separator` with 4 px vertical margin. Opens at the cursor (or under the ⋯ button), flips to stay on-screen and on the same monitor. Submenus open after 150 ms hover or with `→`/`Enter`; `←`/`Esc` closes one level; `↑/↓/Home/End` navigate; typing a letter jumps to the next item starting with it. Appear: 80 ms fade.
### B.7.2 Document context menu (read mode) — also shown by the ⋯ button
1. **Copy** `Ctrl+C` *(selection only)* · **Copy as Markdown** *(selection only)* · **Copy link address** *(on a link)* · **Select all** `Ctrl+A`
2. ─
3. **Appearance ▸** — Theme mode ▸ (◉ Follow system / Light / Dark) · **Color scheme…** (opens the card flyout, B.7.3) · **Layout settings…** (Settings → Typography & Layout) · ✓ Show outline `Ctrl+\` · ✓ ⏎ Single newline as line break
4. **Edit ▸** — Live Edit `F2` · Split Edit `F3` · Open in external editor `F4`
5. **Export ▸** — Export PDF… `Ctrl+Shift+E` · Export HTML… · Print… `Ctrl+P` · View as mind map `Ctrl+Shift+M`
6. **☆ Add to favorites** `Ctrl+D` / **★ Remove from favorites**
7. **File ▸** — Open… `Ctrl+O` · Reload `F5` · Show in Explorer · Copy file path · Reopen with encoding ▸ · Document info…
8. ─
9. **Settings…** `Ctrl+,` · **Keyboard shortcuts** `F1` · **About YAMDV**

In **F2 / F3** the menu starts with: Cut · Copy · Paste · Paste as plain text · ─ · **Format ▸** · **Insert ▸** · **Table ▸** *(inside a table)* · ─ and then continues with items 3–9.
### B.7.3 Color-scheme flyout
Grid of 11 cards, 3 columns, card 112 × 84 (radius 8): top 56 px preview drawn in that scheme (page bg; a 40 %-wide 6 px heading bar in `heading`; two 4 px text lines in `text` @ 35 %; a 3 px `accent` bar; a tiny code rect in `code_bg`), name 12 px below. Hover = live preview of the whole window + 1 px `accent` ring; selected = 2 px `accent` ring + check badge. `Esc` / leaving the flyout reverts the preview.

## B.8 Settings panel
### B.8.1 Layout
In-window modal panel 720 × 560 (clamped to window − 48), centered, radius 12, `menu_bg`, `shadow`, backdrop `rgba(0,0,0,.25)`; draggable by its 48 px header (title "Settings" 15 px 600, close ×). Left navigation 180 px: rows 32 px, 13 px, active row `menu_hover` + 3 px `accent` bar. Right content: scrollable, padding 24; section headings 14 px 600, 1 px `separator` between groups; each setting row: label 13 px + optional 12 px `text_muted` description on the left, control on the right. Controls (all custom-drawn, keyboard-operable): toggle switch 36 × 20 (on = `accent`), segmented control (height 28), dropdown (opens a menu-styled list), slider (with live value label), text input (height 32, radius 6, 1 px `chrome_border`, focus ring), buttons (primary: `accent` bg + white text; secondary: 1 px border), color swatch (24 × 24, radius 6) opening the color picker popover, shortcut capture field ("Press a key combination…", `Esc` cancels, `Backspace` clears, conflicts shown inline in `error`).
Sections: **General · Appearance · Typography & Layout · Sidebar · Reading · Editing · Shortcuts · External editor · Export & Print · Files & Integration · Privacy & Security · About**.

### B.8.2 Settings table (normative — `settings.ini`)
| Section / key | Type | Default | Range / values |
|---|---|---|---|
| `general.language` | enum | `system` | `system`, `en`, `tr` |
| `general.title_format` | enum | `filename` | `filename`, `fullpath` |
| `general.open_external_links` | enum | `ask` | `ask`, `direct`, `never` |
| `general.restore_reading_position` | bool | `true` | |
| `general.remember_window_geometry` | bool | `true` | |
| `appearance.mode` | enum | `system` | `system`, `light`, `dark` |
| `appearance.scheme` | enum | `default` | 11 schemes (R-THEME-02) |
| `appearance.corner_style` | enum | `rounded` | `rounded`, `sharp` |
| `appearance.table_zebra` | bool | `false` | |
| `appearance.use_custom_colors` | bool | `true` | applies `custom-theme.ini` if present |
| `typography.body_font` | string | `Segoe UI` | installed family |
| `typography.code_font` | string | `Cascadia Mono` | installed family (fallback chain B.2) |
| `typography.font_size` | int | `16` | 10–32 |
| `typography.line_height` | float | `1.6` | 1.2–2.2, step 0.05 |
| `typography.paragraph_spacing` | int | `16` | 0–48 |
| `typography.column_width_pct` | int | `65` | 30–100, step 5 |
| `sidebar.visible` | bool | `true` | |
| `sidebar.position` | enum | `left` | `left`, `right`, `left-floating`, `right-floating` |
| `sidebar.width` | int | `260` | 180–480 |
| `sidebar.toc_font_size` | int | `13` | 11–18 |
| `sidebar.toc_max_depth` | int | `4` | 1–6 |
| `sidebar.show_history` / `show_favorites` / `show_files` | bool | `true` | |
| `reading.single_newline_break` | bool | `false` | |
| `reading.smart_punctuation` | bool | `false` | |
| `reading.code_line_numbers` | bool | `false` | |
| `reading.code_wrap` | bool | `false` | |
| `reading.front_matter` | enum | `collapsed` | `collapsed`, `expanded`, `hidden` |
| `reading.render_math` / `render_mermaid` / `render_html` / `emoji_shortcodes` | bool | `true` | |
| `reading.animate_gifs` | bool | `false` | |
| `reading.smooth_scrolling` | bool | `true` | |
| `reading.scrollbar_autohide` | bool | `true` | |
| `reading.word_count_badge` | bool | `false` | |
| `reading.task_toggle_in_read_mode` | bool | `false` | |
| `editing.autosave` | bool | `true` | |
| `editing.autosave_delay_ms` | int | `800` | 300–5000 |
| `editing.assets_dir` | string | `assets` | sanitized relative folder name |
| `editing.editor_font` | enum | `body` | `body`, `code` |
| `editing.editor_line_numbers` | bool | `true` | |
| `editing.editor_soft_wrap` | bool | `true` | |
| `editing.editor_syntax_tint` | bool | `false` | |
| `editing.split_ratio` | float | `0.5` | 0.2–0.8 |
| `editing.split_swapped` | bool | `false` | |
| `editing.format_toolbar` | bool | `true` | |
| `editing.auto_format_tables` | bool | `true` | |
| `editing.snippets` | bool | `true` | |
| `editing.auto_pair` | bool | `false` | |
| `editing.list_indent` | enum | `2` | `2`, `4`, `tab` |
| `shortcuts.preset` | enum | `default` | `default`, `typora` (Appendix F §F.6) |
| `shortcuts.<action_id>` | chord | — | overrides (Appendix F) |
| `external_editor.mode` | enum | `auto` | `auto`, `custom` |
| `external_editor.custom_exe` | path | `""` | absolute `.exe` |
| `external_editor.custom_args` | string | `{file}` | placeholders `{file}` `{line}` `{col}` |
| `export.pdf_colors` | enum | `theme` | `theme`, `light` |
| `export.pdf_page_size` | enum | `auto` | `auto`, `a4`, `letter` |
| `export.pdf_margins_mm` | int | `18` | 5–40 |
| `export.pdf_page_numbers` | bool | `false` | |
| `export.print_colors` | enum | `light` | `light`, `theme` |
| `export.html_include_toc` | bool | `true` | |
| `export.csv_formula_guard` | bool | `true` | S-EXP-04 |
| `integration.context_menu` | bool | `false` | mirrors registry state |
| `privacy.diagnostic_log` | bool | `false` | local file only, no content/paths unless enabled |
"Files & Integration" also shows: default-app status + [Set as default app], config folder path + [Open folder], [Edit custom-theme.ini], portable-mode indicator. "Privacy & Security" shows the offline statement ("YAMDV never connects to the internet; it has no network code") and how to verify it (firewall test). "About" shows version, build commit, license, exe path (click to copy), config path.

## B.9 Find bar
Anchored top-right of the document area: 12 px below the title bar, 16 px from the right (left of the scrollbar). 380 × 40, radius 8, `menu_bg`, 1 px `menu_border`, `shadow`. Contents: search glyph · input (flex) · count "3/17" 12 px `text_muted` ("No results" in `error`) · [↑] [↓] 24 × 24 · [Aa] match case · [ab] whole word · [×]. In edit modes a [▸] toggles a replace row (height becomes 76): input · [Replace] · [Replace all]. Opening pre-fills with the selection; `Enter` next, `Shift+Enter` previous, `Esc` closes and keeps the current match selected.

## B.10 Transient UI
### B.10.1 Toasts
Bottom-center, 24 px above the bottom edge; height 32, padding 0 14, radius 16, `toast_bg`, 13 px `toast_text`, optional 14 px icon and an action link (`#8ab4ff`). Max 3 stacked (8 px apart). Duration 1 600 ms (errors 4 000 ms); fade in 120 ms, out 200 ms.
### B.10.2 Zoom indicator
Window center, 140 × 48, radius 10, `toast_bg`, 15 px text ("18 px", "Column 70 %"), visible 900 ms.
### B.10.3 Word-count badge
Bottom-right, 12 px from the bottom and from the scrollbar; height 24, padding 0 10, radius 12, `chrome_bg` @ 90 % with 1 px `chrome_border`, 12 px `text_muted`; tooltip with details (R-READ-07).
### B.10.4 Link status tooltip
Bottom-left, 8 px inset; height 22; max width 60 % of the window; 12 px; `tooltip_bg`/`tooltip_text`; radius 4; middle-ellipsis for long URLs/paths.
### B.10.5 Banners
Top of the document area (under the title bar), full available width, height 36, bg = warning or note callout bg, 13 px text with a leading icon, text buttons right-aligned, × when dismissible. Used for: file changed on disk (with unsaved edits), file deleted, read-only, decoding fallback notice, recovery available.
### B.10.6 Popovers and dialogs
Popover (external link "Ask", link editor, language picker, color picker): width 360, radius 8, `menu_bg`, `shadow`, anchored to its target; URL shown in code font 12 px, wrapped. In-window confirm dialog: width 420, title 15 px 600, body 13 px, buttons right-aligned (primary last), `Enter` = primary, `Esc` = cancel, backdrop `rgba(0,0,0,.25)`. Native dialogs are used only for open/save files and printing. Tooltips: 500 ms delay, 12 px, padding 4 × 8, radius 4.

## B.11 Fullscreen viewers
Overlay inside the window. Image / table / Mermaid: backdrop `scrim`; content centered at "fit" (≤ 92 % of the window); toolbar pill top-right (12 px inset), height 36, bg `rgba(32,32,32,.9)`, white 14 px glyphs: [−] [100 %] [+] [fit] [1:1] [copy] [×]. Wheel zooms at the cursor (10 %–800 %, ×1.1 per notch), drag pans (hand cursor), double-click toggles fit/100 %, `+ - 0` keys, `Esc` closes. Table viewer: the table on a page-colored card (radius 8) with a 28 px header ("Table · R × C" + [Export CSV]) that moves the card, a 14 px resize grip at the bottom-right, natural column widths, scrollbars. Mind map: opaque `bg` instead of the scrim, same toolbar plus [Right | Balanced] layout toggle.

## B.12 Welcome page (no document open)
Sidebar hidden. Centered vertical stack (max width 560): app icon 64 px · 16 px · "YAMDV" 28 px 600 · subtitle "Read instantly. Edit in place." 14 px `text_muted` · 32 px · **drop zone** 560 × 200, 2 px dashed `chrome_border`, radius 12 (on drag-over: border `accent`, bg `accent @ 6 %`), containing a 32 px document glyph, "Drop a Markdown file here" 15 px, "or" 12 px `text_muted`, primary button "Open file…  Ctrl+O" · 24 px · **Recent** (12 px `text_muted` label; up to 8 rows of 32 px: file name 13 px + folder 12 px `text_muted`, middle-ellipsized; hover ×) · 24 px · **default-app status**: if not default → warning-tinted pill "YAMDV is not your default Markdown app" + [Set as default]; if default → `text_muted` "✓ Default app for .md files". Footer 16 px from the bottom: "v<version> · Shortcuts (F1) · Settings (Ctrl+,)" 12 px `text_muted`, the last two are links.

## B.13 Edit modes
### B.13.1 Split Edit (F3)
Title pill "Split Edit". Editor pane (left by default): bg `bg`; gutter `gutter_bg` with 1 px `gutter_border` on its right, width `max(3, digits) × digit_width + 20`, line numbers 11 px `line_number` right-aligned (current line `text_muted`); text area left padding 8, top padding 48 (aligned with the preview's first block); editor font per `editor_font`, size = body size, line height = body line height. Caret: 1.5 px wide (2 px at ≥ 150 % scale), color `text`, blink at the system rate, **blinking stops after 5 s without input** (keeps idle CPU at 0). Current line: `hover` overlay. Splitter 6 px `chrome_border` (hover `accent @ 40 %`, cursor `IDC_SIZEWE`); a 24 px ⇄ swap button appears at the splitter's top on hover. Preview pane: normal reading layout scaled into its width; the sidebar toggle chevron sits at the preview's inner edge. "?" help button: 32 px circle at the editor pane's bottom-right (16 px inset), bg `menu_hover`, glyph 14 px `text_muted`.
### B.13.2 Live Edit (F2)
Title pill "Live Edit". Revealed markers in `marker` color, same size as the surrounding text. Caret as in F3. Format toolbar: height 36, radius 8, `menu_bg`, `shadow`, 28 × 28 icon buttons (14 px glyphs) in groups separated by 1 px `separator`, placed 8 px above the selection (below if there is no room). Table tools per B.6.8; row/column grips 12 × 20 with ⋮⋮ dots in `text_muted`, drop indicator 2 px `accent`. Code-block language label becomes an input on click. Inline popovers per B.10.6.

## B.14 Icons and logo (original design — generated by `cargo xtask gen-icons`)
- **App icon:** rounded square (corner radius 22 % of size) with a vertical gradient `#2563eb → #1d4ed8`; a white geometric "Y" built from three strokes occupying the left ~60 %, and a white downward arrow (stem + chevron) in the right ~40 % ("Y↓"). Sizes 16, 20, 24, 32, 40, 48, 64, 128, 256 (256 stored as PNG inside the ICO). At 16 px use a simplified drawing (thicker strokes, no gradient).
- **Document icon:** white page with a folded top-right corner and a `#d0d0d0` outline; the app badge (Y↓, blue) centered in the lower half.
- **UI glyphs:** from the system icon font (B.2). Suggested code points (verify each with `GetGlyphIndices` at startup; draw a simple vector fallback if missing): Settings `E713`, More `E712`, ChevronLeft `E76B`, ChevronRight `E76C`, ChevronDown `E70D`, ChevronUp `E70E`, Copy `E8C8`, Search `E721`, Close `E8BB`, Minimize `E921`, Maximize `E922`, Restore `E923`, FavoriteStar `E734`, FavoriteStarFill `E735`, FullScreen `E740`, BackToWindow `E73F`, ZoomIn `E8A3`, ZoomOut `E71F`, Delete `E74D`, Folder `E8B7`, Document `E8A5`, History `E81C`, Edit `E70F`, Link `E71B`, Print `E749`, Save `E74E`, Info `E946`, Warning `E7BA`, Help `E897`, Add `E710`, Remove `E738`, CheckMark `E73E`, Sort `E8CB`, Lightbulb `EA80`, Code `E943`, BulletedList `E8FD`. Callout icons: note → Info, tip → Lightbulb, important → a filled-circle "!" (vector), warning → Warning, caution → an octagon "!" (vector).

## B.15 Motion
| Animation | Duration | Easing |
|---|---|---|
| Smooth scroll (wheel notch, keys, TOC jump) | 120–180 ms (TOC jump up to 300 ms) | ease-out cubic |
| Sidebar show/hide | 160 ms | ease-out |
| Menu / popover appear | 80 ms fade | linear |
| Toast in / out | 120 / 200 ms | ease-out |
| Scrollbar fade-out | 300 ms after 1 000 ms idle | linear |
| Code toolbar fade | 120 ms | linear |
| Heading fold / TOC collapse | 120 ms | ease-out |
With reduced motion (R-A11Y-02) every animation is instant. Animation timers run **only while something animates**.

## B.16 Interaction states
Hover → `hover` overlay; pressed → `pressed` overlay; keyboard focus → 2 px `focus_ring` with 1 px offset, shown only after keyboard interaction (like `:focus-visible`); disabled → 40 % opacity, no hover. All clickable targets ≥ 24 × 24 px.

## B.17 Visual acceptance
Every UI phase delivers golden PNGs (Appendix H §H.4) of: welcome page; reading view with the sidebar (Default light, Default dark, and every scheme in light); each element of B.6; menus; settings (every section); find bar; viewers; F2 and F3. The owner compares them with the reference descriptions above.

---

# APPENDIX C — Security Specification (`docs/SECURITY.md`)

**Threat model in one line:** the adversary controls the *content* of Markdown files (and the files next to them) that the user opens — e.g. a README in a cloned repository, an e-mail attachment, an AI-generated file — and may also control other files the user drags in or pastes. Other processes of the *same* user are semi-trusted; other users and the network are untrusted. Assets to protect: the user's files and their integrity, the user's credentials (including NTLM hashes), the system's integrity, and the user's privacy.

## C.1 Network (S-NET)
- **S-NET-01 MUST** No networking code in the product: no socket, HTTP, DNS, TLS or WebSocket APIs or crates (bans in S-SUP-03).
- **S-NET-02 MUST** Nothing referenced by a document is ever fetched from a network: remote images, fonts, includes, iframes → placeholders. Never pass document-derived strings to APIs that can fetch (URL monikers, `URLDownloadToFile*`, `SHCreateStreamOnFileEx` with URLs, WIC decoders created from URLs, `LoadImage` on UNC paths).
- **S-NET-03 MUST** Automated binary check (`cargo xtask check-imports`): the import and delay-import tables of `yamdv.exe` MUST NOT contain `ws2_32.dll`, `wsock32.dll`, `mswsock.dll`, `winhttp.dll`, `wininet.dll`, `urlmon.dll`, `dnsapi.dll`, `iphlpapi.dll`, `webio.dll`, `httpapi.dll`, `netapi32.dll`, `mpr.dll`, `rasapi32.dll`, `wlanapi.dll`, `winhttpcom.dll`, nor functions named `URLDownloadToFile*`, `URLOpenStream*`, `Internet*`, `WinHttp*`, `socket`, `WSA*`, `connect`, `getaddrinfo`, `GetAddrInfo*`. (System DLLs loaded *at runtime* by Windows itself — e.g. shell extensions inside the file dialog — are outside our control; `memprobe` lists loaded modules so they are visible.)
- **S-NET-04 MUST** The only outward hand-offs, both user-initiated and leaving our process: (a) a validated `http`/`https`/`mailto` URL passed to the default handler via `ShellExecuteExW` (R-LINK-04), (b) the hard-coded `ms-settings:defaultapps…` URI (R-SYS-02).
- **S-NET-05 MUST** Verification step in every release QA: with an outbound-block firewall rule for `yamdv.exe` nothing changes, and `netstat -ano` / Resource Monitor show no sockets owned by the process.

## C.2 Process hardening (S-PROC)
- **S-PROC-01 MUST** First statements in `main`: `SetDefaultDllDirectories(LOAD_LIBRARY_SEARCH_SYSTEM32)`; any `LoadLibraryExW` uses `LOAD_LIBRARY_SEARCH_SYSTEM32` with a bare system DLL name. Never load DLLs from the exe folder, the current directory or a document folder (DLL planting is the #1 risk for a portable exe in `Downloads`).
- **S-PROC-02 MUST** `SetDllDirectoryW("")`, `SetSearchPathMode(BASE_SEARCH_PATH_ENABLE_SAFE_SEARCHMODE | BASE_SEARCH_PATH_PERMANENT)`.
- **S-PROC-03 MUST** `HeapSetInformation(NULL, HeapEnableTerminationOnCorruption, …)`.
- **S-PROC-04 MUST** `SetProcessMitigationPolicy`: `ProcessImageLoadPolicy` (NoRemoteImages, NoLowMandatoryLabelImages, PreferSystem32Images), `ProcessStrictHandleCheckPolicy`. **SHOULD** `ProcessDynamicCodePolicy` (ProhibitDynamicCode) if DirectWrite, the renderer, WIC, IMEs, file dialogs and printing all keep working (test and record in an ADR). **MUST NOT** enable `ProcessExtensionPointDisablePolicy` (breaks IMEs/accessibility), `ProcessSignaturePolicy` (breaks printer drivers/IMEs), `ProcessFontDisablePolicy` (breaks per-user fonts).
- **S-PROC-05 MUST** Build hardening: ASLR + high-entropy VA, DEP/NX, Control Flow Guard, CET shadow-stack compatible, static CRT, no RWX sections, `overflow-checks = true`, `panic = "abort"`. Verified by xtask (PE header flags) in CI.
- **S-PROC-06 MUST** Manifest `asInvoker`; never request elevation. If running elevated, show a banner "Running as administrator is not recommended".
- **S-PROC-07 MUST** Child processes only for: the external editor (user action), self-install/uninstall helper, nothing else. Showing a file in Explorer uses `SHOpenFolderAndSelectItems` (no process launch by us).
- **S-PROC-08 MUST** Security-relevant behavior never depends on environment variables or `PATH` (no `MD_EDITOR`-style variables; executables are resolved to absolute paths from known locations / `App Paths`).

## C.3 File system (S-FILE)
- **S-FILE-01 MUST** Writes happen only to: the open document (explicit save / autosave **in edit modes only**), the assets folder (S-FILE-05), export targets chosen in a Save dialog or via CLI `-o`, the YAMDV config/state/log/recovery folders, and the install folder during `--install`. Read mode never writes the document.
- **S-FILE-02 MUST** Reads happen only from: the opened document, resources per S-RES, YAMDV config files, OS-provided fonts/codecs, and directory listings (names/attributes only) for the Files tab and wikilink resolution.
- **S-FILE-03 MUST** Atomic save: write a hidden temp file in the same folder (`.~<name>.yamdv-<random>.tmp`, `CREATE_NEW`), `FlushFileBuffers`, then `ReplaceFileW` (keeps ACLs/attributes), falling back to `MoveFileExW(MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH)`. On any failure the original stays intact, the temp file is removed, the buffer stays dirty and an error is shown. Never clear the read-only attribute. If the document path is a symlink, write the final target (`GetFinalPathNameByHandleW`), never replace the link itself.
- **S-FILE-04 MUST** Path validation for every path that comes from a document, IPC, CLI or config: normalize with `GetFullPathNameW`; reject device namespace paths (`\\.\`, `\\?\GLOBALROOT`), reserved device names (`CON PRN AUX NUL COM1–9 LPT1–9`, with or without extension, any case), alternate data streams (`:` beyond the drive letter), and embedded NULs. Long paths are handled internally with `\\?\`.
- **S-FILE-05 MUST** Assets sandbox: the target folder `<doc_dir>/<assets_dir>/<doc_stem>/` must, after creation and `GetFinalPathNameByHandleW`, still be inside `<doc_dir>` (rejects `..`, absolute values, junction/symlink escapes). File names are sanitized (remove `<>:"/\|?*` and control characters, trim trailing dots/spaces, avoid reserved names, max 120 UTF-16 units) and created with `CREATE_NEW` (never overwrite).
- **S-FILE-06 MUST** Temporary files only in the document folder (atomic save) or `%TEMP%\YAMDV-<random>\` (export staging), created with `CREATE_NEW`, deleted on success, on failure and at exit.
- **S-FILE-07 MUST** Config/state parsing is defensive: ≤ 4 MB per file, ≤ 64 KB per line, unknown keys ignored, values clamped to their ranges, nothing ever executed; corrupt files renamed to `*.corrupt-<timestamp>` and defaults used.
- **S-FILE-08 MUST** File watching covers only the open document's folder (non-recursive) and the config folder; buffer overflow → full reload of the document.
- **S-FILE-09 MUST** `.lnk`, `.url` and other shell-link files are never followed or resolved automatically.
- **S-FILE-10 MUST** Data integrity: never normalize line endings, never strip/add a BOM, never re-format untouched blocks, never write in read mode (also R-EDIT-01/02).

## C.4 Resources referenced by documents (S-RES)
- **S-RES-01 MUST** Allowed image sources: relative paths, absolute local paths on local fixed/removable drives, `file:///X:/…` (local), and `data:image/(png|jpeg|gif|bmp|webp|x-icon|tiff);base64,…`.
- **S-RES-02 MUST** **Network-location blocking (prevents NTLM credential leaks):** block UNC paths (`\\server\share\…`, `//server/share/…`, `\\?\UNC\…`), `file://host/…`, WebDAV forms (`\\host@SSL\…`, `\\host@port\…`), and paths on mapped network drives (`GetDriveTypeW == DRIVE_REMOTE`) — **before** any `CreateFileW`/WIC call. Exception: if the user opened the document itself from a network location, resources on the **same server and share** are allowed. Additionally open local resources with `FILE_FLAG_OPEN_REPARSE_POINT` first; if the final component is a symlink/junction whose target is a network path, block it.
- **S-RES-03 MUST** Never fetch remote (`http`, `https`, `ftp`, …) resources; show the offline placeholder.
- **S-RES-04 MUST** Limits: image file ≤ 50 MB; `data:` URI ≤ 10 MB decoded; dimensions ≤ 16 384 × 16 384 and ≤ 64 megapixels (checked from the header **before** decoding); animated GIF ≤ 1 000 frames; decoded-image cache within the Appendix D budget.
- **S-RES-05 MUST** Referenced files are only ever *decoded as images* — never opened with another app automatically. Exports embed only files that decoded successfully as images (so `![](C:/Users/me/.ssh/id_rsa)` can never be exfiltrated into an exported HTML/PDF).
- **S-RES-06 MUST** Fonts come only from the system/user font collections via DirectWrite; documents cannot load fonts.

## C.5 Links (S-LINK)
- **S-LINK-01 MUST** External schemes allowlist: `http`, `https`, `mailto` (ASCII-case-insensitive). Every other scheme is inert (R-LINK-06).
- **S-LINK-02 MUST** Before hand-off: the URL is parsed and re-serialized; rejected if it contains control characters, unescaped spaces or quotes, or exceeds 2 048 characters. The *Ask* popover shows the real destination; internationalized host names are shown **with their punycode form** (homograph protection); a credential part (`user:pass@`) is highlighted as suspicious.
- **S-LINK-03 MUST** Local link targets: Markdown family → open in YAMDV; folders → `SHOpenFolderAndSelectItems` after confirmation; "open with default app" only after confirmation and only for this extension allowlist: `png jpg jpeg gif bmp webp tif tiff ico svg pdf txt log csv tsv json yaml yml toml xml docx xlsx pptx odt ods odp mp3 wav ogg flac mp4 webm mkv mov` — everything else (including `exe com scr pif bat cmd ps1 psm1 vbs vbe js jse wsf wsh hta cpl msc msi msp msu reg lnk url scf inf jar appref-ms application settingcontent-ms library-ms search-ms searchconnector-ms theme themepack diagcab chm hlp iso img vhd vhdx dll sys docm xlsm pptm` and extension-less files) is **blocked** (R-LINK-05). Confirmation dialogs focus **Cancel** by default.
- **S-LINK-04 MUST** Mark-of-the-Web: if the document has a `Zone.Identifier` stream with `ZoneId` ≥ 3, show a banner "This file came from the internet" and always *Ask* before any external link or local file launch, regardless of settings.

## C.6 HTML inside Markdown (S-HTML)
- **S-HTML-01 MUST** There is no HTML engine. Raw HTML is tokenized by a small bounded tokenizer and mapped to the allowlist (R-MD-14). Unknown elements are dropped but their text content is kept; these containers are dropped **with** their content: `script style iframe frame frameset object embed applet form input button select textarea template noscript link meta base title head svg math` (inline SVG/MathML → placeholder).
- **S-HTML-02 MUST** Only allowlisted attributes survive; all `on*` attributes and `javascript:`/`vbscript:`/`data:` (except image data URIs in `img src`) values are removed; `style` accepts only `color`, `background-color`, `text-align` with a strict value grammar (hex, `rgb()`, named colors; `left|center|right|justify`); every `href`/`src` goes through S-LINK / S-RES.
- **S-HTML-03 MUST** Limits: nesting depth 64, 100 000 tags per document, 8 KB per attribute value.

## C.7 Execution (S-EXEC)
- **S-EXEC-01 MUST** No eval, scripting, macros, plugins, or dynamic code loading of any kind.
- **S-EXEC-02 MUST** Launching local files: only per S-LINK-03.
- **S-EXEC-03 MUST** Processes are started only with `CreateProcessW` with `lpApplicationName` = absolute path to an existing `.exe` (verified not to be a network path or a reparse point to one), arguments quoted per the MSVC/`CommandLineToArgvW` rules (unit-tested round-trip). Never `cmd.exe`, PowerShell, `.bat`/`.cmd`/`.ps1` targets (the "BatBadBut" class of injection).
- **S-EXEC-04 MUST** `ShellExecuteExW` is used only for: validated external URLs, the constant `ms-settings:` URI, and allowlisted local files after confirmation. Never pass `SEE_MASK_NOZONECHECKS`.

## C.8 Single instance / IPC (S-IPC)
- **S-IPC-01 MUST** Single instance via a named mutex `Local\YAMDV-<hash of user SID>`; arguments forwarded with `WM_COPYDATA` to the primary window, `dwData` = magic constant, payload = versioned UTF-16 structure (magic, version, count, paths/flags), ≤ 64 KB.
- **S-IPC-02 MUST** The receiver verifies the sender's process runs as the same user (token SID comparison) and ignores anything else.
- **S-IPC-03 MUST** Forwarded paths get exactly the validation of user-opened paths; IPC can only *open documents* — never export, install, register, or change settings.
- **S-IPC-04 MUST** No named pipes, sockets, shared memory sections, COM servers, DDE, or listening endpoints of any kind.

## C.9 Registry (S-REG)
- **S-REG-01 MUST** Writes only to: `HKCU\Software\Classes\YAMDV.Markdown`, `HKCU\Software\Classes\<ext>\OpenWithProgids`, `HKCU\Software\Classes\Applications\yamdv.exe`, `HKCU\Software\Classes\SystemFileAssociations\<ext>\shell\YAMDV`, `HKCU\Software\YAMDV\Capabilities`, `HKCU\Software\RegisteredApplications` (value `YAMDV`), `HKCU\Software\Microsoft\Windows\CurrentVersion\Uninstall\YAMDV` (install only). Never HKLM, never `UserChoice`, never `Explorer\FileExts`, never policy keys.
- **S-REG-02 MUST** Every written key/value is listed in one place in code; `--unregister` / `--uninstall` remove exactly those; registration is idempotent.
- **S-REG-03 MUST** Registration happens **only on explicit user action** (Settings button, context-menu toggle, `--register`, `--install`) — never on a normal launch.
- **S-REG-04 MUST** Registry reads are limited to: theme/accessibility settings, `App Paths`, editor install keys, association queries.

## C.10 Parser robustness / DoS (S-PARSE)
- **S-PARSE-01 MUST** No panic, hang, or unbounded allocation on any input; all recursion has explicit depth limits (visual nesting of quotes/lists capped at 64 levels — deeper content is laid out at level 64).
- **S-PARSE-02 MUST** Limits: file 256 MB hard / 20 MB soft prompt; a single paragraph > 1 MB uses a plain-text fast path; tables ≤ 1 000 columns and ≤ 100 000 cells (else shown as text); TOC ≤ 10 000 headings; ≤ 10 000 footnotes; ≤ 100 000 link reference definitions.
- **S-PARSE-03 MUST** Time budgets: any single block whose layout exceeds 200 ms is replaced by a "Content too complex" placeholder with [Render anyway]; the UI thread never blocks > 2 s.
- **S-PARSE-04 MUST** Only linear-time algorithms on user-controlled input; no backtracking regular expressions.
- **S-PARSE-05 MUST** Math: ≤ 10 000 chars per formula, depth ≤ 64, macro expansion ≤ 10 000 steps and ≤ 100 000 output tokens per formula, ≤ 5 000 rendered formulas per document (the rest shown as source).
- **S-PARSE-06 MUST** Mermaid: source ≤ 100 KB per diagram; soft limit 500 nodes / 1 500 edges (placeholder + [Render anyway]); hard limit 2 000 nodes / 6 000 edges; bounded layout iterations; ≤ 100 rendered diagrams per document.
- **S-PARSE-07 MUST** Checked or saturating arithmetic for sizes/offsets from input; `overflow-checks` stay on in release.
- **S-PARSE-08 MUST** Invalid encodings → U+FFFD for viewing; editing disabled when decoding was lossy (R-EDIT-12).

## C.11 Privacy (S-PRIV)
- **S-PRIV-01 MUST** No telemetry, analytics, usage counters, device IDs or crash upload.
- **S-PRIV-02 MUST** Logging off by default; when enabled: local only, rotated (≤ 1 MB × 3 files), never containing document text; file paths only when "diagnostic log" is enabled.
- **S-PRIV-03 MUST** Crash notes contain no document content.
- **S-PRIV-04 MUST** Settings offer "Clear history", "Clear favorites" and "Forget reading positions".
- **S-PRIV-05 MUST** The clipboard is read only on an explicit paste.
- **S-PRIV-06 MUST** Exports contain no personal metadata (PDF `Producer` = "YAMDV <version>", `Title` = first H1 or file name; no author/user name).

## C.12 Supply chain (S-SUP)
- **S-SUP-01 MUST** `Cargo.lock` committed; `cargo deny check` (advisories, bans, licenses, sources) in CI; builds use `--locked`.
- **S-SUP-02 MUST** Each dependency has an ADR (H5); reports list dependency changes with `cargo tree -e normal` deltas.
- **S-SUP-03 MUST** Banned crates (in `deny.toml`): `reqwest hyper hyper-util h2 h3 ureq isahc curl curl-sys surf attohttpc minreq tokio async-std smol mio socket2 native-tls openssl openssl-sys rustls webpki webpki-roots quinn tungstenite tokio-tungstenite websocket lettre trust-dns-resolver hickory-resolver wry tao tauri webview2 webview2-com winit egui eframe iced slint druid gtk glib fltk wgpu glutin sdl2 skia-safe rquickjs quick-js boa_engine v8 deno_core mlua rlua pyo3 libloading`.
- **S-SUP-04 MUST** License allowlist: MIT, Apache-2.0, Apache-2.0 WITH LLVM-exception, BSD-2-Clause, BSD-3-Clause, ISC, Zlib, Unicode-3.0, Unicode-DFS-2016, BSL-1.0, CC0-1.0. Everything else denied unless an ADR approves it.
- **S-SUP-05 MUST** No network access in build scripts; resource embedding (manifest, icons, version info) via a pure-Rust `.res` writer or a crate approved by ADR.
- **S-SUP-06 MUST** GitHub Actions pinned by full commit SHA, `permissions: contents: read`, no secrets, only well-known actions (`actions/checkout`, `actions/cache`, `actions/upload-artifact`, `EmbarkStudios/cargo-deny-action`, a toolchain installer).

## C.13 Export safety (S-EXP)
- **S-EXP-01 MUST** Exports are written atomically, only to the user-chosen path.
- **S-EXP-02 MUST** HTML export: `<meta http-equiv="Content-Security-Policy" content="default-src 'none'; img-src data:; style-src 'unsafe-inline'; script-src 'sha256-<hash of our inline script>'; base-uri 'none'; form-action 'none'">`; all text escaped; images only as `data:` URIs of decoded images; external links get `rel="noopener noreferrer"`; no external resource of any kind.
- **S-EXP-03 MUST** PDF: no JavaScript, launch actions, embedded files or forms; only URI annotations for `http`/`https`/`mailto` and internal GoTo links.
- **S-EXP-04 MUST** CSV export guards against formula injection: cells starting with `= + - @` or a tab/CR are prefixed with `'` (setting `export.csv_formula_guard`, default on).

## C.14 UI-level safety (S-UI)
- **S-UI-01 MUST** The real destination of every link is visible before it is followed (status tooltip + Ask popover).
- **S-UI-02 MUST** Dangerous confirmations default to Cancel; nothing is auto-accepted.
- **S-UI-03 MUST** Banners for: Mark-of-the-Web documents, elevated process, decoding fallback.

## C.15 Security verification (S-TEST)
- **S-TEST-01 MUST** Fuzz targets (`cargo-fuzz`): Markdown → block tree → layout (mock shaper), inline extensions, HTML sanitizer, math parser + layout, each Mermaid parser + layout, INI parser, encoding detection, IPC payload decoder. Each target runs ≥ 10 minutes in the phase that introduces or changes it (locally or in a manual CI workflow); small seed corpora are committed.
- **S-TEST-02 MUST** Security unit-test matrices: link policy (scheme × target type × MotW × setting), path validator (UNC variants, WebDAV forms, device names, ADS, traversal, reparse points), assets sandbox, HTML sanitizer (a list of ≥ 50 known XSS vectors must yield no executable output in exported HTML), CSV guard, argument quoting round-trip, IPC validation.
- **S-TEST-03 MUST** CI binary checks: forbidden imports (S-NET-03), PE `DllCharacteristics` contain `DYNAMIC_BASE`, `HIGH_ENTROPY_VA`, `NX_COMPAT`, `GUARD_CF`; CET compatibility flag present; no section is both writable and executable.
- **S-TEST-04 MUST** `docs/THREAT_MODEL.md` (STRIDE per entry point: document content, dropped files, clipboard, CLI, IPC, config files) is created as a skeleton in Phase 001 and completed in Phase 012.

---

# APPENDIX D — Performance & Memory Budgets (`docs/PERFORMANCE.md`)

## D.1 Principles
Memory is a feature. Everything is lazy, virtualized and measured. "Fast and light" is verified with numbers in every phase, never assumed.

## D.2 Budgets
**Final** budgets become hard gates in Phase 013 (optimization). During Phases 002–012 the **soft** budget (≈ 1.6 × final) applies; exceeding a soft budget requires an explanation and a plan in the report. Reference machine: x64, 4+ cores, SSD, Windows 11, 1920×1080 at 100 % and 150 %, window 1280×800.

| ID | Scenario | Final | Soft |
|---|---|---|---|
| **B-SIZE** | `yamdv.exe` release size | ≤ 3.0 MB (stretch 2.0 MB) | ≤ 4.0 MB |
| **M0** | Welcome page, idle 3 s — private working set (PWS) | ≤ 3.0 MB | ≤ 4.8 MB |
| **M1** | `typical.md` (~30 KB README-like: headings, lists, 3 code blocks, 1 table, 2 local 800×600 PNGs, 1 callout) — PWS after open + 3 s | ≤ 5.0 MB | ≤ 8.0 MB |
| **M2** | `large-1mb.md` (generated, 1 MB) — PWS | ≤ 10 MB | ≤ 16 MB |
| **M3** | `math-heavy.md` (50 formulas) / `mermaid-heavy.md` (5 diagrams) — PWS | ≤ 6.0 MB | ≤ 9.6 MB |
| **M4** | Each additional window with `typical.md` (same process) — PWS delta | ≤ +1.5 MB | ≤ +2.4 MB |
| **M5** | F3 and F2 after 200 edits on `typical.md` — PWS | ≤ 6.0 MB | ≤ 9.6 MB |
| **M6** | After closing the Open-file dialog — PWS returns to ≤ M1 + 1 MB within 2 s | required | required |
| **M7** | Commit (`PrivateUsage`) | ≤ 2.5 × PWS | ≤ 3 × PWS |
| **T1** | Process start → first paint of `typical.md` (warm file cache) | ≤ 80 ms | ≤ 150 ms |
| **T2** | Process start → welcome page painted | ≤ 60 ms | ≤ 100 ms |
| **T3** | Second launch → file visible in the existing instance | ≤ 50 ms | ≤ 80 ms |
| **T4** | Scroll frame time p99 at 1080p / 4K | ≤ 16 / 25 ms | same |
| **T5** | Typing latency p99 (key → caret painted), 1 MB doc, F3 / F2 | ≤ 16 / 25 ms | ≤ 25 / 40 ms |
| **T6** | Live reload after an external write completes | ≤ 100 ms | ≤ 150 ms |
| **T7** | Idle CPU over 30 s (no input, no animation) | CPU time delta ≤ 1 ms; zero active timers | same |
| **T8** | Theme / scheme switch repaint | ≤ 50 ms | ≤ 80 ms |
| **T9** | Find, 1 MB doc, per keystroke | ≤ 30 ms | ≤ 50 ms |
| **T10** | PDF export of 100 pages | ≤ 3 s; transient memory ≤ +30 MB, released afterwards | same |

## D.3 Measurement method (normative)
- **Private working set (PWS)** = what Task Manager shows as "Memory (active private working set)". Compute it with `QueryWorkingSet` (psapi): count the working-set pages whose `Shared` bit is 0, × page size. Cross-check with `PROCESS_MEMORY_COUNTERS_EX2::PrivateWorkingSetSize` where available.
- Also record: `PrivateUsage` (commit), `WorkingSetSize`, `PeakWorkingSetSize`, handle count (`GetProcessHandleCount`), GDI and USER objects (`GetGuiResources`), loaded modules.
- Protocol: release build · machine idle · window 1280×800 · 3 runs, report the **median** · settle 3 s after the scenario's last action · repeat at 100 % and 150 % scale for M0/M1 · record OS build, CPU, GPU, display scale.
- Startup times: process creation time (`GetProcessTimes`) → timestamp logged at the end of the first `WM_PAINT` that shows the target content.
- `cargo xtask memprobe` implements the scenarios; CI runs M0 and M1 on `windows-latest` (runner variance noted, trends matter more than absolute values there).

## D.4 Memory design rules
1. **Source once.** The document text is stored once (UTF-8 `String`, later a rope in edit modes). The block tree stores byte ranges (`u32`), not copies.
2. **Virtualized layout.** Lay out only visible blocks ± one screen. Unseen blocks keep only an estimated height. Paragraph layout objects are cached LRU for ≤ 3 screens.
3. **No full-window framebuffers in our process** if the chosen backend allows it (banded rendering / DWM-owned surfaces). Scrolling reuses pixels (`ScrollWindowEx`) and paints only exposed strips.
4. **Images** decoded at display size; LRU cache ≤ 1.5 MB decoded pixels; off-screen images evicted; re-decoded on demand.
5. **Glyph caching** is left to DirectWrite/the OS. No own glyph atlas.
6. **Lazy subsystems:** WIC/COM only when the first image appears; math engine and Cambria Math only when math exists; Mermaid only when diagrams exist; font enumeration only when the font picker opens; History/Files tab data only when the tab is shown.
7. **Threads:** UI thread + one file-watcher thread + at most one worker for heavy jobs (export, large layout); explicit 256 KB stacks; no thread pools, no async runtime.
8. **Allocator:** the system heap (default). No arena/segment allocators that reserve large regions. `shrink_to_fit` long-lived buffers after load.
9. **Compact data:** `u32` offsets/indices, small enums, interned style runs, no per-glyph or per-character heap objects.
10. **Transient spikes** (file dialogs, printing, PDF export) are allowed; afterwards transient buffers are freed, and a working-set trim (`SetProcessWorkingSetSizeEx(-1, -1)`) MAY be performed **once, right after the transient operation** and on minimize. Periodic or timer-based trimming is **forbidden** — it fakes the metric.
11. Static data (highlighter keyword tables, emoji table, strings) lives in read-only sections (`&'static`), never copied to the heap at startup.

## D.5 Tooling
- Feature `mem-stats` (dev only): a counting global allocator with subsystem tags; `Ctrl+Shift+Alt+M` overlay HUD showing PWS, commit, heap bytes per subsystem, cache sizes.
- `cargo xtask memprobe` scenarios M0–M7; `cargo xtask size` with a per-crate size breakdown (via `cargo bloat` if installed; dev tool only).
- Every report: table of M0–M7, B-SIZE and the applicable T-metrics, with deltas versus the previous phase.

## D.6 Startup path
Before the first paint only: parse arguments → single-instance check → read `settings.ini` → create the window → read + decode the file → parse → lay out the first screen → paint. Everything else (history/state loading, file watcher, sidebar data, Files tab scan, font enumeration, image decoding outside the first screen) is deferred until after the first paint.

---

# APPENDIX E — Architecture (`docs/ARCHITECTURE.md`)

## E.1 Overview
```
 bytes on disk ──decode (encoding_rs)──► Source text (String / rope in edit modes)
                                              │  pulldown-cmark + YAMDV extensions
                                              ▼
                                        Block tree (flat arena, byte ranges into source)
                                              │  lazy, per visible block
                                              ▼
           TextShaper (trait) ◄──────── Layout engine ──────► Display list (draw ops)
           impl: DirectWrite                 │                        │
                                             │                        ▼
                         Math / Mermaid / Highlight            Renderer backend (ADR-0002)
                         (pure crates → display ops)                  │
                                                                      ▼
                                                                   Window
 Edits: Action → editor ops on Source → incremental reparse of affected blocks → relayout → partial repaint
```
Platform-independent logic lives in pure crates (testable on any OS, `#![forbid(unsafe_code)]`); all Win32/COM code lives in `yamdv-platform-win`.

## E.2 Repository layout
```
YAMDV/
├─ Cargo.toml                 # workspace, pinned deps, lints, profiles
├─ Cargo.lock
├─ rust-toolchain.toml
├─ .cargo/config.toml
├─ deny.toml  rustfmt.toml  clippy.toml  .editorconfig  .gitignore  .gitattributes
├─ README.md  CHANGELOG.md  LICENSE
├─ .github/workflows/ci.yml
├─ docs/
│  ├─ SPEC.md  UI_SPEC.md  SECURITY.md  PERFORMANCE.md  ARCHITECTURE.md
│  ├─ SHORTCUTS.md  ROADMAP.md  TESTING.md  THREAT_MODEL.md  TRACEABILITY.md
│  ├─ adr/       0000-…md, 0001-…md, …
│  ├─ plans/     001_PLAN.md, …
│  ├─ reports/   001_REPORT.md, …
│  └─ spike/     backend measurements + screenshots
├─ crates/
│  ├─ yamdv-core/         # source, encoding, md parsing adapter + extensions, block tree,
│  │                      # slugs, outline, link/resource policy (pure), stats, config model,
│  │                      # INI parser, i18n tables, action/keymap model
│  ├─ yamdv-layout/       # TextShaper trait, block layout, virtualization, hit testing,
│  │                      # display list, pagination (for PDF/print)
│  ├─ yamdv-highlight/    # lexers + token classes
│  ├─ yamdv-math/         # TeX parser, MATH-table reader, math layout → display ops
│  ├─ yamdv-mermaid/      # parsers, models, layouts → display ops
│  ├─ yamdv-editor/       # buffer, selections, undo, commands, snippets, table model
│  ├─ yamdv-ui/           # chrome widgets (title bar, sidebar, menus, settings, find, toasts,
│  │                      # viewers) as platform-independent state + paint + input handling
│  ├─ yamdv-export/       # HTML writer, PDF writer, CSV
│  ├─ yamdv-platform-win/ # ALL unsafe/Win32/COM: window, DPI, theme, DirectWrite shaper,
│  │                      # renderer backend, WIC, clipboard, dialogs, file I/O, watcher,
│  │                      # registry, shell/process, single instance, printing, IME, memory
│  └─ yamdv-app/          # bin "yamdv": composition root, app state, command dispatch, CLI
├─ xtask/                 # dev tasks: ci, check-imports, size, memprobe, gen-icons, golden
├─ spikes/                # Phase 001 backend spikes (not part of the product build)
├─ fuzz/                  # cargo-fuzz targets (added when the first parser lands)
└─ tests/
   ├─ corpus/             # Markdown test documents (Appendix H)
   └─ golden/             # golden PNGs for visual regression
```

## E.3 Dependency rules
```
yamdv-app ─► yamdv-ui ─► yamdv-layout ─► yamdv-core
    │            │            ├────────► yamdv-highlight
    │            │            ├────────► yamdv-math
    │            │            └────────► yamdv-mermaid
    │            └──────────► yamdv-editor ─► yamdv-core
    ├──────────► yamdv-export ─► yamdv-layout
    └──────────► yamdv-platform-win (implements traits defined in core/layout/ui)
```
- Pure crates never depend on `yamdv-platform-win` or the `windows` crate.
- Platform services are reached through traits (`TextShaper`, `ImageDecoder`, `Clipboard`, `FileSystem`, `Clock`, `Shell`…) so pure logic is unit-testable with fakes.
- No cyclic dependencies; no "utils" grab-bag crate.

## E.4 Document model
- `Document { source, encoding, line_endings, bom, path, disk_state(size, mtime, hash) }`.
- `BlockTree`: a flat `Vec<Block>` arena; each block has `kind`, `source_range: Range<u32>`, `parent`, `children` range, and kind-specific data (heading level + slug, list info, code lang, table alignments, callout kind…). Inline content is parsed lazily per block into compact runs referencing source ranges (owned strings only where unescaping requires).
- Extensions (autolink literals, `==highlight==`, callout detection, wikilinks, emoji, front matter, sanitized HTML) run as passes on the pulldown-cmark event stream (`into_offset_iter`) so every element keeps its source range — required for F2/F3 mapping.
- **Incremental reparse** (designed in Phase 002, used from Phase 006): after an edit, reparse from the start of the top-level block containing the edit until the new block boundaries re-synchronize with the old ones; splice the arena; global constructs (link reference definitions, footnote definitions, front matter) trigger an inline re-resolution pass rather than a full block reparse.

## E.5 Layout
- `TextShaper` builds paragraph layouts from styled runs and a width, and answers metrics + hit tests; the DirectWrite implementation wraps `IDWriteTextLayout` (wrapping, bidi, fallback, hit testing, inline objects for images/math/checkboxes).
- Block layout computes boxes top-down; heights of unseen blocks are estimates refined when laid out; a Fenwick (binary indexed) tree over block heights maps scroll offset ↔ block in O(log n). Scroll anchoring keeps the top visible block stable when heights above it change.
- Output is a **display list**: `Rect`, `RoundRect`, `Line`, `Path` (lines/quad/cubic), `Text(layout handle, origin, color overrides)`, `GlyphRun` (math), `Image(handle, rect)`, `PushClip/PopClip`, `Opacity`. The same display list feeds the screen renderer, the print path and the PDF writer (via pagination).

## E.6 Rendering
Backend per ADR-0002. Requirements regardless of backend: dirty-rect invalidation; scrolling by pixel reuse + exposed-strip painting; overlays (find bar, toasts, badges, viewers) drawn as top layers of the same window; menus, popovers and tooltips as separate layered popup windows; correct per-monitor DPI; no work when nothing changed.

## E.7 UI layer and commands
- Small retained model for chrome components (title bar, sidebar, panels, popups) with explicit state, painting into display lists, and input handling (hover, press, capture, focus, keyboard navigation).
- **Every user-visible operation is an `Action`** (enum with a stable snake_case id, e.g. `toggle_sidebar`, `live_edit`, `export_pdf`). Menus, shortcuts, buttons and CLI map to Actions. The keymap = preset + user overrides; conflicts detected; Actions have enable/visibility predicates. This makes rebinding (Appendix F) and testing trivial.

## E.8 Platform layer (`yamdv-platform-win`) modules
`window` (class, custom frame, hit testing, DPI) · `theme` (app mode, high contrast, reduced motion, accent) · `text` (DirectWrite factory, font collection, `TextShaper` impl, color glyph runs, MATH table access) · `render` (backend) · `image` (WIC decode at size, probing) · `clipboard` (`CF_UNICODETEXT`, `CF_HTML`, `CF_DIB`/PNG) · `dialogs` (`IFileOpenDialog`, `IFileSaveDialog`) · `fs` (read, atomic save, path validation helpers, MotW read) · `watch` (`ReadDirectoryChangesW` on the watcher thread, overlapped I/O, clean cancellation) · `registry` · `shell` (`ShellExecuteExW`, `CreateProcessW`, `SHOpenFolderAndSelectItems`) · `instance` (mutex + `WM_COPYDATA`) · `print` (`PrintDlgExW`, GDI printing) · `ime` (Imm32) · `memory` (working-set queries, trim) · `timers` (only while animating) · `uia` (Phase 012).

## E.9 Lints and code standards (workspace)
```toml
# Pure crates use `[lints] workspace = true`. Cargo does not allow partial overrides, so
# yamdv-platform-win, xtask and spikes declare their own complete [lints] tables with
# unsafe_code = "deny" and allow it only in the modules that need it (every block has a SAFETY comment).
[workspace.lints.rust]
unsafe_code = "forbid"
missing_docs = "warn"
unused_must_use = "deny"

[workspace.lints.clippy]
unwrap_used = "deny"
expect_used = "deny"
panic = "deny"
todo = "deny"
unimplemented = "deny"
dbg_macro = "deny"
print_stdout = "deny"             # allowed only in the CLI output module
print_stderr = "deny"
undocumented_unsafe_blocks = "deny"
indexing_slicing = "warn"
cast_possible_truncation = "warn"
cast_sign_loss = "warn"
large_stack_arrays = "warn"
large_types_passed_by_value = "warn"
```
Tests may opt out locally with `#[allow(...)]` at the test module level.

## E.10 Threading
- **UI thread:** everything interactive.
- **Watcher thread:** blocks on `ReadDirectoryChangesW`; posts a custom window message on change (debounced 50 ms on the UI side).
- **Worker thread (on demand):** PDF/HTML export, printing preparation, very large layouts; results posted back; cancellable; created when needed and exits when idle for 5 s.
- Communication via `PostMessageW` + a lock-free or mutex-protected queue. No async runtime.

## E.11 Errors and logging
Own error enums per crate (no `anyhow`/`thiserror` in product crates unless an ADR approves). User-facing errors become toasts/banners with localized messages. Internal logger: compile-time levels, off by default, writes to `%LOCALAPPDATA%\YAMDV\logs\` when enabled.

## E.12 Persistence formats
- **INI dialect** (own parser, ADR): UTF-8 (BOM tolerated), `[section]`, `key = value`, `#`/`;` comments, quoted strings with `\" \\ \n \t \u{…}` escapes, booleans `true/false`, repeated keys for lists. Unknown keys preserved on rewrite where possible.
- `settings.ini` per Appendix B §B.8.2.
- `state.ini`: `[window]` geometry; `[history]` entries `item = <unix_ms>|<path>`; `[favorites]` `item = <path>`; `[positions]` `item = <unix_ms>|<block_index>|<offset_ppm>|<path>`; `[toc]` `item = <path>|<collapsed ids…>`. Caps per R-NAV/R-READ; LRU eviction.
- All writes atomic (S-FILE-03), debounced (R-REL-02).

## E.13 Internationalization
`enum Msg { … }` + `fn tr(lang, Msg) -> &'static str` generated from two tables (EN, TR) in source; a unit test asserts completeness; placeholders `{0}`; plural helper for "1 word / 2 words" (Turkish has no plural agreement after numbers: "2 kelime").

## E.14 Future portability
Only `yamdv-platform-win` and the thin wiring in `yamdv-app` are Windows-specific. A future macOS/Linux port would add another platform crate implementing the same traits; no pure crate may assume Windows.

---

# APPENDIX F — Keyboard, Mouse & CLI (`docs/SHORTCUTS.md`)

Every binding maps to an `Action` id (E.7) and is rebindable in Settings → Shortcuts. macOS-style alternatives are not needed (Windows only). "Read" = read mode, "Edit" = F2 or F3.

## F.1 Global
| Action id | Default binding | Scope / notes |
|---|---|---|
| `open_file` | `Ctrl+O` | all |
| `close_window` | `Ctrl+W`; `Esc` | `Esc` only in read mode with no overlay/popup open |
| `reload` | `F5` | all (edit: asks if dirty) |
| `toggle_sidebar` | `Ctrl+\` | all |
| `sidebar_contents` / `sidebar_history` / `sidebar_favorites` / `sidebar_files` | `Ctrl+Shift+1` / `2` / `3` / `4` | all |
| `toggle_favorite` | `Ctrl+D` | all |
| `find` | `Ctrl+F` | all |
| `find_next` / `find_prev` | `Enter` / `Shift+Enter` in the find bar; `Ctrl+G` / `Ctrl+Shift+G` anywhere | all |
| `font_bigger` / `font_smaller` / `font_reset` | `Ctrl+=` (`Ctrl+Num+`), `Ctrl+↑`, `Ctrl+Wheel↑` / `Ctrl+-`, `Ctrl+↓`, `Ctrl+Wheel↓` / `Ctrl+0` | all |
| `column_wider` / `column_narrower` / `column_reset` | `Alt+↑`, `Alt+Wheel↑` / `Alt+↓`, `Alt+Wheel↓` / `Alt+0` | read (in edit, `Alt+↑/↓` = move line; wheel variants still work) |
| `live_edit` | `F2` | toggles |
| `split_edit` | `F3` | toggles |
| `external_editor` | `F4`, `Ctrl+E` | all |
| `export_pdf` | `Ctrl+Shift+E` | all |
| `export_html` | — | all |
| `print` | `Ctrl+P` | all |
| `mind_map` | `Ctrl+Shift+M` | all |
| `settings` | `Ctrl+,` | all |
| `shortcuts_help` | `F1` | all — overlay listing current bindings |
| `fullscreen` | `F11` | all |
| `nav_back` / `nav_forward` | `Alt+←`, mouse X1 / `Alt+→`, mouse X2 | all |
| `copy` / `copy_markdown` / `select_all` | `Ctrl+C` / `Ctrl+Shift+C` / `Ctrl+A` | all |
| `heading_jump` (MAY) | `Ctrl+Shift+O` | all |
| `toggle_word_count`, `toggle_single_newline_break`, `toggle_line_numbers_all`, `toggle_wrap_all` | unbound | menu/Settings |

## F.2 Read-mode navigation
`↑/↓` 40 px · `PgUp/PgDn`, `Space/Shift+Space` one viewport minus 40 px · `Home/End`, `Ctrl+Home/End` start/end · `Tab`/`Shift+Tab` move focus through links and interactive elements (code toolbars, table toolbars, fold toggles) · `Enter` activates the focused element.

## F.3 Edit modes (F2 and F3)
| Action id | Binding |
|---|---|
| `save` | `Ctrl+S` |
| `undo` / `redo` | `Ctrl+Z` / `Ctrl+Y`, `Ctrl+Shift+Z` |
| `cut` / `paste` / `paste_plain` | `Ctrl+X` / `Ctrl+V` / `Ctrl+Shift+V` |
| `bold` / `italic` / `strikethrough` / `highlight` / `inline_code` | `Ctrl+B` / `Ctrl+I` / `Ctrl+Shift+X` / `Ctrl+Shift+H` / ``Ctrl+` `` |
| `link` / `image` | `Ctrl+K` / `Ctrl+Shift+I` |
| `code_block` | `Ctrl+Shift+K` |
| `heading_1` … `heading_6` / `paragraph` | `Ctrl+1` … `Ctrl+6` / `Ctrl+Shift+0` |
| `quote` / `bullet_list` / `numbered_list` / `task_list` | `Ctrl+Shift+Q` / `Ctrl+Shift+8` / `Ctrl+Shift+7` / `Ctrl+Shift+9` |
| `toggle_task` | `Ctrl+Enter` |
| `insert_table` | `Ctrl+T` |
| `move_line_up` / `move_line_down` | `Alt+↑` / `Alt+↓` |
| `duplicate_line` / `select_line` | `Ctrl+Shift+D` / `Ctrl+L` |
| `replace` | `Ctrl+H` |
| `indent` / `outdent` / `snippet_expand` | `Tab` / `Shift+Tab` / `Tab` (snippet takes precedence when the word before the caret is a trigger) |
| `exit_edit` | `Esc` |
Table context (R-TBL): `Tab`/`Shift+Tab` next/previous cell, `Enter` cell below, `Shift+Enter` `<br>`, `Ctrl+Alt+↑/↓` move row, `Ctrl+Alt+←/→` move column, `Ctrl+Shift+Backspace` delete row.

## F.4 Command line
```
yamdv [<file> …]                 open files (one window each; forwarded to a running instance)
yamdv <file> --line <n>          open and scroll to source line n
yamdv --help | --version
yamdv --settings                 open the settings panel
yamdv --register | --unregister  per-user file-type registration (never changes the default app)
yamdv --set-default              open Windows "Default apps" for YAMDV
yamdv --context-menu on|off      Explorer "Open with YAMDV" entry
yamdv --install | --uninstall    per-user self-install / removal (R-SYS-05)
yamdv --export-html <in.md> [-o <out.html>] [--theme <scheme>] [--mode light|dark]
yamdv --export-pdf  <in.md> [-o <out.pdf>]  [--theme <scheme>] [--mode light|dark] [--page-size a4|letter]
```
Dev builds only (feature `dev-tools`, never in release): `--render-png <in.md> -o <out.png> [--width 1200] [--scale 1.0|1.5] [--theme …] [--mode …]`, `--mem-hud`.
Exit codes: `0` success · `1` runtime error · `2` usage error. Console output only when started from a console (`AttachConsole`).

## F.5 Mouse
Wheel scroll (smooth) · `Shift+Wheel` horizontal in code blocks/tables · `Ctrl+Wheel` font size · `Alt+Wheel` column width · double-click selects a word, triple-click a paragraph · `Ctrl+click` / middle-click on a link → new window (Markdown targets) · X1/X2 back/forward · right-click → context menu · hover the left gutter of H1–H3 → fold toggle · hover image/diagram/table → ⛶ badge · double-click image/diagram → fullscreen viewer · drag files onto the window → open (or insert in edit modes) · drag the sidebar edge / splitter → resize.

## F.6 Presets
- `default` — the tables above.
- `typora` — differences: `split_edit` = `Ctrl+/`, `toggle_sidebar` = `Ctrl+Shift+L`, `inline_code` = ``Ctrl+Shift+` ``, `strikethrough` = `Alt+Shift+5`, `bullet_list` = `Ctrl+Shift+]`, `numbered_list` = `Ctrl+Shift+[`, `paragraph` = `Ctrl+0` (then `font_reset` = `Ctrl+Shift+Alt+0`).
User overrides apply on top of the selected preset; the Shortcuts section shows conflicts and allows "Reset to preset".

---

# APPENDIX G — Roadmap (`docs/ROADMAP.md`)

One prompt per phase. A phase starts only when the previous report has been reviewed. Fix requests from a review always come first in the next phase. The plan may be re-cut by the reviewer based on reports.

| Phase | Title | Scope (main IDs) | Exit criteria (summary) |
|---|---|---|---|
| **001** | Foundation & rendering spike | repo, docs, tooling, CI, window skeleton, backend spike (ADR-0002) | AC-1…AC-7 of prompt 001 |
| **002** | Core document & reading MVP | open via CLI/drag/`Ctrl+O` (R-OPEN-01…04, 06–09), encodings, block tree + extensions skeleton, text layout via DirectWrite, paragraphs/headings/emphasis/lists/quotes/inline code/rules/links (styling only), virtualized smooth scrolling + overlay scrollbar, selection + copy, Default light/dark tokens, golden-image tool | `typical-basic.md` renders like the reference; M0/M1 soft budgets; T1/T4 measured |
| **003** | Full rendering | tables, code blocks + highlighter + toolbar, callouts, footnotes, task lists, images (WIC, lazy, cache, placeholders), front matter, safe HTML subset, highlight/sub/sup, emoji, autolink literals, wikilinks, details, link navigation + full link policy (R-LINK, S-LINK, S-RES) | full corpus renders; security matrices for links/paths pass; fuzzers for parser/HTML |
| **004** | App shell & theming | custom title bar, sidebar (all tabs, positions, resize, toggle), context/⋯ menus, 11 schemes + cards, modes, fonts, corner style, custom colors + `custom-theme.ini`, settings panel + `settings.ini`/`state.ini`, welcome page, toasts, EN/TR | every B.4–B.8/B.12 element present; golden PNGs for all schemes |
| **005** | Reading features & system integration | single instance + multi-window, geometry memory, live reload, reading position, find, zoom/column width, heading folding, fullscreen viewers (image, table), word count, external editor, registration/context menu/default-app flow, CLI, portable mode, MotW/elevation banners | R-WIN, R-READ (except mind map), R-EXT, R-SYS-01…04/06 done; T3/T6/T7 met |
| **006** | Editor core + Split Edit (F3) | buffer (rope), selections, undo/redo, IME, clipboard, Markdown-aware keys, snippets, image/file paste, autosave + atomic save + conflict detection + recovery, find & replace, incremental reparse, scroll/selection sync | R-EDIT, R-F3 done; T5 (F3) met; no data-loss tests pass |
| **007** | Live Edit (F2) | reveal model, typing transforms, format toolbar, edit context menu, cross-block selection/clipboard, lists/tasks, link popover, code block editing, byte-identical untouched blocks | R-F2-01…10 done; T5 (F2) met |
| **008** | Table editing & advanced editing | R-TBL complete, HTML-paste → Markdown, TSV paste, insert dialogs | R-TBL done |
| **009** | Math engine | TeX parser, MATH-table layout with Cambria Math, R-MATH complete, F2 math popover | `math-heavy.md` matches KaTeX output closely; M3 met |
| **010** | Mermaid engine & mind map | flowchart, sequence, gantt, pie (+ SHOULD types as time allows), fullscreen diagram viewer, mind map view | `mermaid-heavy.md` renders; S-PARSE-06 enforced; M3 met |
| **011** | Export & print | PDF writer + pagination, single-file HTML export, print, CSV, headless CLI export | R-EXP done; S-EXP tests pass; T10 met |
| **012** | Hardening | full security review against Appendix C, fuzz campaign, THREAT_MODEL complete, accessibility (UIA basics, High Contrast, keyboard audit), R-REL edge cases, error-path polish | zero open High/Medium findings |
| **013** | Optimization | final budgets become hard gates (Appendix D), size/startup/memory tuning with measurements | all M/T/B budgets met (final column) |
| **014** | Packaging & release | `--install`/`--uninstall` polish, icons, version info, release checklist, user guide (EN/TR), a **draft** release inside the private repo only when the owner asks | release checklist complete |

---

# APPENDIX H — Testing Strategy & Corpus (`docs/TESTING.md`)

## H.1 Test levels
| Level | What | Where | When |
|---|---|---|---|
| Unit | pure logic (parsing adapters, extensions, slugs, link/path policy, layout with a mock shaper, highlighter, math, Mermaid, editor ops, INI, i18n completeness, contrast ratios) | each crate, any OS | every commit |
| Integration | app-level flows through Actions with fake platform services (open, navigate, edit, save, conflict, undo) | `yamdv-app` tests | every commit |
| Windows integration | real DirectWrite/WIC/registry (HKCU test keys under a unique test ProgID, cleaned up)/file I/O/clipboard | `#[cfg(windows)]` tests | CI `windows-latest` |
| Golden images | rendered PNGs vs approved goldens (H.4) | Windows CI + local Windows | every UI-affecting commit |
| Fuzz | targets per S-TEST-01 | `fuzz/` | phases touching parsers; ≥ 10 min/target |
| Performance & memory | `memprobe` scenarios, startup/frame/typing timers | xtask | every phase report; CI for M0/M1 |
| Security | matrices per S-TEST-02, binary checks per S-TEST-03 | unit + xtask | every commit |
| Manual QA | checklist written in each report, executed by the owner on a real PC | owner | end of every phase |

## H.2 Rules
- Tests are deterministic (no wall-clock dependence; inject `Clock`), hermetic (temp dirs, no network, no user profile writes except under a unique test folder), and fast (unit suite < 30 s).
- Every bug fix comes with a regression test named after the bug (`regression_<short_name>`).
- Requirement IDs appear in test names or doc comments for traceability.
- Never delete or weaken a failing test to get green; fix the code or record the reason in the report.

## H.3 Corpus (`tests/corpus/`, all content original or generated — no third-party copyrighted text)
- `typical-basic.md` (Phase 002 subset) and `typical.md` (M1 scenario: README-like ~30 KB with headings, lists, 3 code blocks, 1 table, 2 generated 800×600 PNGs in `typical-assets/`, 1 callout, footnotes).
- `commonmark/` (examples from the CommonMark spec as test vectors), `gfm.md`, `callouts.md` (all GFM + Obsidian types, folding, custom titles), `tables.md` (alignment, wide, inline formatting, escaped pipes, empty cells, 1 000 columns), `code.md` (every language in R-CODE-01, diff, very long lines, tabs, 6 000-line block), `images.md` (relative, `../`, absolute, `file:///`, percent-encoded, missing, remote, UNC, WebDAV, mapped-drive, data URI, oversized dimensions, corrupt file, WebP, GIF), `links.md` (anchors incl. Turkish/CJK headings, relative `.md`, wikilinks, external, dangerous schemes, blocked extensions, IDN/homograph, credential URLs), `unicode.md` (Turkish, CJK, Arabic/Hebrew bidi, emoji ZWJ/modifiers, combining marks, zero-width chars), `html.md` (allowed tags, README centering patterns, ≥ 50 XSS vectors, comments incl. multi-line), `frontmatter.md` (YAML, TOML, malformed), `footnotes.md`, `math.md`, `math-heavy.md` (50 formulas covering R-MATH-02), `mermaid.md`, `mermaid-heavy.md` (5 diagrams), `details.md`.
- Encodings & bytes: `utf8-bom.md`, `utf16le-bom.md`, `utf16be-bom.md`, `cp1254-turkish.md`, `cp1252.md`, `gb18030.md`, `shift_jis.md`, `invalid-utf8.md`, `crlf.md`, `mixed-endings.md`, `no-final-newline.md`, `nul-bytes.bin.md`.
- Pathological (generated at test time by a helper, not committed when large): 10 000 nested `>`; 100 000 list items; 50 000-cell table; 1 MB single line; emphasis bombs (`*a**a***…`); 100 000 unclosed `[`; unclosed fences; 10 000 headings; 10 000 footnotes; deeply nested HTML; Mermaid with 5 000 nodes; math with 1 000 nested braces; `large-1mb.md` (M2).

## H.4 Golden images
- Dev tool `yamdv --render-png` (feature `dev-tools`) renders a document headlessly at a given width, scale, scheme and mode.
- Goldens live in `tests/golden/<phase>/<name>@<scale>-<scheme>-<mode>.png`; comparison uses a per-pixel tolerance plus a maximum differing-pixel ratio (fonts differ slightly across Windows builds; CI uses a fixed runner image). `cargo xtask golden --update` refreshes them **only** with a note in the report explaining each visual change.
- Each report links the new/changed goldens so the owner can review the look without building.

## H.5 Manual QA checklist format (in every report)
Numbered steps written for a non-developer on Windows: what to download/build, what to click/press, what should happen, and what to send back if it doesn't (screenshot + the step number).

---

**END OF PROMPT 001.** Start with §4.1 now. Remember: Phase 001 only, measured facts only, report per §7 at the end.