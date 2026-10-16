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
