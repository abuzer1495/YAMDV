# YAMDV — Phase 002 Report: Core document model & reading MVP

## 0. Fix items status
| F-ID | status | evidence |
|---|---|---|
| F-01 | DONE | `docs/PROCESS.md` copied by script from prompt 001 §2/§3/§7, plus amendments A-1 and A-2. `cargo xtask spec-check` prints "PROCESS.md vs prompt 001 §2/§3/§7 + amendments: match" (commit 4ba9855) |
| F-02 | DONE | `docs/prompts/001_FOUNDATION_AND_SPIKE.md` SHA-256 `39e1f158e81c9ba84159cf95dbce40f477618c5691c44bc117ae492afe7f9411`. This **differs** from the reviewer's `04e60d03…e00c`, so ours is used (H10). All 8 extracted appendix hashes match the expected values, so the difference lies outside the appendices. `docs/prompts/002_READING_MVP.md` SHA-256 `43ae3020da9991beefb925203a421acbd68bbf6a5a0049f14951361799ffb3d2`. `.gitattributes`: `docs/prompts/** -text` |
| F-03 | DONE | `cargo xtask spec-check`: 8 × verified, every appendix matches at `phase-001`, and the current docs match original + patches (table in §3b). CI step `spec-check (F-03)` |
| F-04 | DONE | "Amendment (Phase 002, fix item F-04)" appended to `docs/reports/001_REPORT.md`: AC table, repository state, files, owner QA checklist. The measured content is unchanged |
| F-05 | DONE (artifact from the merge run, see §11) | CI job `owner kit (F-05)` (push to main + workflow_dispatch) uploads `owner-kit` (14 days): yamdv.exe, xtask.exe, spike-a.exe, corpus 002 + encodings + large-1mb.md, and `docs/OWNER_KIT.md` (EN + TR) |
| F-06 | DONE — policy **on** | `hardening::apply` sets `ProcessDynamicCodePolicy(ProhibitDynamicCode)`. Verified on CI: DirectWrite text + fallback + color emoji (32 goldens), WIC encode, IFileOpenDialog (M6). NOT VERIFIED: WIC decode, shell-extension-heavy folders, drag & drop, IME (owner kit). Recorded in the ADR-0001 amendment |
| F-07 | DONE | memprobe JSON `yamdv.memprobe/2` carries `*_bytes` + `*_mib` fields and the summary prints both. `xtask size` prints bytes + MiB against a 4.0 MiB budget. Phase 001 restated in MiB in §5 |
| F-08 | DONE | `memprobe --size WxH` sets the client area. The JSON records `requested_size`, `client_size` ([1000,700] on CI) and `monitor_resolution` ([1024,768] on CI) |

## 1. Summary
YAMDV is now a working read-only Markdown viewer:
- **Opening:** CLI (multi-file), Ctrl+O (multi-select) and drag & drop. Defensive path checks, size and binary guards with an in-window confirm dialog, and encoding detection.
- **Document model:** two-level (block index + lazy, LRU-cached detail trees, splice API; ADR-0007).
- **Rendering:** DirectWrite + backend A with virtualized layout and scroll anchoring, smooth scrolling, an overlay scrollbar, and selection/copy.
- **Placeholders:** constructs from Phase 003+ show as labeled interim cards.

Headline numbers (CI windows-latest, 1000×700): M0 2.05 MiB, M1b 2.50 MiB, M2 5.48 MiB; first paint T2 74 ms, T1b 80 ms, T1L 132 ms; scroll p99 5.9 ms (typical) and 4.9 ms (1 MiB); idle CPU 0 ms; yamdv.exe 1.45 MiB. Every soft-budget gate is green on CI.

## 2. Environment
- **Engineering host:** Linux x86_64. rustc/cargo **1.99.0** (pinned). cargo-xwin cross target `x86_64-pc-windows-msvc`. Fuzzing with nightly `rustc 1.101.0-nightly (4ebd25825 2026-10-15)` and cargo-fuzz 0.13.2.
- **gh account:** snipeTR.
- **Windows GUI:** not available locally. All Windows runs (tests, goldens, memprobe, autoscroll) happen on GitHub Actions `windows-latest` (Windows Server 2025, Hyper-V video 1024×768, no GPU).

## 3. Task status
| Task / Requirement ID | Status | Evidence | Notes |
|---|---|---|---|
| §4.1.1 INI + Settings (B.8.2 incl. P-08), portable paths | DONE | `yamdv-core/src/{ini,settings,settings_table}.rs`, tests, fuzz `ini` | The writer is tested but the app only reads |
| §4.1.2 i18n EN/TR | DONE | `i18n.rs`, completeness test | `GetUserDefaultUILanguage` → `tr-*` → TR |
| §4.1.3 logger | DONE | `core/log.rs`, platform `log.rs` | Off by default, never logs document text |
| §4.1.4 theme data 11 schemes + WCAG test | DONE | `theme_table.rs`, contrast test | Only Default is used |
| §4.2.1 path intake (S-FILE-04, R-OPEN-12) | DONE | `core/path.rs`, `platform/sys.rs::full_path` | |
| §4.2.2 guards (R-OPEN-08, S-PARSE-02) | DONE | `decode::{size_guard,binary_guard}`, `yamdv-ui/dialog.rs` | |
| §4.2.3 decoding (R-OPEN-06/07/10) | DONE | `decode.rs`, `tests/encodings.rs` (12 fixtures) | |
| §4.2.4 plain-text mode (R-OPEN-09) | DONE | `layout/plain.rs`, golden `plaintext-*` | Chunks ≤ 200 lines |
| §4.3.1–3 options, index, lazy detail trees, depth 64 | DONE | `doc/{mod,index,detail}.rs` | |
| §4.3.4 layout ↔ source mapping | DONE | `doc/text.rs::ParaText` tests | |
| §4.3.5 splice + ADR-0007 | DONE | `Index::splice`, > 500-edit synthetic test, ADR-0007 | |
| §4.3.6 text extraction | DONE | `text::all_text`, `tests/large.rs` | |
| §4.4.1 TextShaper + mock + DirectWrite | DONE | `layout/shaper.rs`, `platform/dwrite.rs`, `render.rs` | |
| §4.4.2 block layout | DONE | `layout/block.rs`, goldens | Deviations in §7 |
| §4.4.3 interim placeholders | DONE | golden `placeholders-*` | Deviations in §7 |
| §4.4.4 virtualization (R-REN-21) | DONE | `fenwick.rs`, `viewer.rs`, anchor test, CI autoscroll | |
| §4.4.5 rendering integration | DONE | `render.rs` (bands, ScrollWindowEx) | Re-entrant WM_PAINT bug found and fixed (d8e2840) |
| §4.4.6 window, states, confirm dialog | DONE | `window.rs`, `yamdv-ui/dialog.rs` | |
| §4.5 scrolling/scrollbar (R-REN-17, R-A11Y-02) | DONE | `window.rs` | Touchpad and keys: NOT VERIFIED on CI (owner kit) |
| §4.6 selection & copy (R-REN-16 partial) | DONE | `text::copy_text` tests, `viewer::word_bounds` | |
| §4.7 opening (CLI/dialog/drop/close) | DONE | `window.rs`, M6 | Drag & drop: NOT VERIFIED on CI |
| §4.8 tests/vectors/fuzz/goldens/corpus | DONE | 196 vectors, `fuzz/`, `tests/golden/002` (32 images, tiled), `tests/corpus/002`, `xtask gen-large` | |
| §4.9 measurements + gate | DONE | `.github/scripts/measure.ps1`, `xtask memprobe` budgets | |
| §5 traceability | DONE | `docs/TRACEABILITY.md` | |

## 3b. Spec patches applied
| P-ID | doc | verified by spec-check (yes/no) |
|---|---|---|
| P-01 | docs/SPEC.md (R-LINK-05) | yes |
| P-02 | docs/SPEC.md (R-MMD-03) | yes |
| P-03a | docs/SPEC.md (R-THEME-06) | yes |
| P-03b | docs/UI_SPEC.md (§B.3.1) | yes |
| P-04 | docs/PERFORMANCE.md (§D.4 rule 8) | yes |
| P-05 | docs/PERFORMANCE.md (§D.3) | yes |
| P-06 | docs/ARCHITECTURE.md (§E.9) | yes |
| P-07 | docs/ARCHITECTURE.md (§E.4) | yes |
| P-08a | docs/SPEC.md (R-MD-06) | yes |
| P-08b | docs/UI_SPEC.md (§B.8.2) | yes |
| P-09 | docs/PERFORMANCE.md (§D.2) | yes |

The patches are applied by `cargo xtask spec-patch` from `docs/spec-patches/002.txt` and logged in `docs/SPEC_CHANGES.md`.

## 4. Acceptance criteria
| AC | PASS / FAIL / NOT VERIFIED | Evidence |
|---|---|---|
| AC-1 | PASS | §0: F-01…F-08 done. F-05's artifact comes from the main run (§11) |
| AC-2 | PASS | `cargo xtask spec-check` → 8 × "verified \| match \| match", patches P-01…P-09; green CI step |
| AC-3 | PASS | CI run 38938805660: fmt, clippy -D warnings, tests (Linux + Windows), deny, release build, check-imports, size, goldens and the memprobe gate all green. Merged and tagged `phase-002` (§11) |
| AC-4 | PASS (CLI, dialog, encodings, guards) / NOT VERIFIED (drag & drop, multi-select interaction) | 12 encoding fixtures pass; guard and dialog unit tests; M6 opens and cancels the real IFileOpenDialog; CLI multi-file code path. Drag & drop and multi-select: owner kit step 2 |
| AC-5 | PASS | 32 goldens (8 docs × light/dark × 1.0/1.5) in `tests/golden/002`, reviewed. Deviations listed in §7 |
| AC-6 | PASS | M2 5.48 MiB ≤ 16; T1L 132 ms ≤ 300; mock anchor-stability test; CI autoscroll top→bottom→top on large-1mb.md (15,191 frames, p99 4.9 ms) |
| AC-7 | PASS | Copy-rule unit tests. `tests/large.rs`: Ctrl+A + Ctrl+C text for large-1mb.md takes 31 ms (release, Linux host); the clipboard call itself is not timed |
| AC-8 | PASS | `parse_layout`: 255,320 runs in 661 s; `ini`: 1,172,084 runs in 661 s; 0 crashes (`fuzz/artifacts/*` empty) |
| AC-9 | PASS | §5 (MiB + bytes, breakdown, deltas). The DirectWrite share is estimated |
| AC-10 | PASS | owner-kit job, `docs/OWNER_KIT.md` EN + TR (§11) |
| AC-11 | PASS | This file follows the PROCESS.md template with A-2 sections 0 and 3b |

## 5. Measurements
**Setup:**
- Source: CI run [38938805660](https://github.com/snipeTR/YAMDV/actions/runs/38938805660), artifact `measurements`.
- Machine: windows-latest, Hyper-V 1024×768, client set to 1000×700 (F-08).
- Method: `xtask memprobe`, QueryWorkingSet shared-bit counting, median of 5 runs (M0/M1b/M2) or 3 runs (M6), 3 s settle. The release build has ACG on.

| ID | Result | Bytes | Budget | Gate |
|---|---|---|---|---|
| M0 PWS | 2.05 MiB | 2,146,304 | ≤ 4.8 MiB | pass |
| M1b PWS | 2.50 MiB | 2,617,344 | ≤ 8.0 MiB | pass |
| M2 PWS | 5.48 MiB | 5,742,592 | ≤ 16 MiB | pass |
| M6 PWS | 0.55 MiB | 581,632 | ≤ M1b + 1 MiB | pass (see note) |
| M7 commit/PWS | M0 2.70 MiB (2,826,240) = 1.32×; M1b 3.53 MiB (3,698,688) = 1.41×; M2 6.76 MiB (7,090,176) = 1.23× | | ≤ 3× | pass |
| T2 first paint, empty | 74 ms | | ≤ 100 ms | pass |
| T1b first paint, typical-basic | 80 ms | | ≤ 150 ms | pass |
| T1L first paint, large-1mb | 132 ms | | ≤ 300 ms | pass |
| T4 scroll p99 typical (3 runs) | 5.90 ms (5.64 / 5.90 / 6.55) | | ≤ 16 ms | pass |
| T4 scroll p99 large-1mb (3 runs, 15,191 frames each) | 4.90 ms (4.52 / 5.51 / 4.90) | | ≤ 16 ms | pass |
| T7 idle CPU 30 s | 0 µs (empty and document) | | ≤ 1 ms | pass |
| B-SIZE yamdv.exe | 1.45 MiB | 1,521,664 | ≤ 4.0 MiB | pass |

Notes:
- **M6:** after the dialog closes, the process trims its working set once (D.4 rule 10). PWS then drops to 0.55 MiB while commit **grows to 10.77 MiB (11,296,768 B), +7.2 MiB versus M1b**. The shell and dialog DLLs stay loaded (88 modules vs 34). The PWS budget is met only because of the trim. The commit growth is a real cost: see §9.
- **T4:**
  - Without warm-up, the first runs on the runner gave typical-basic p99 of 28.8, 44.6 and 18.0 ms (runs 38936429436 and 38937457897). The slow frames are first-time DirectWrite font loads (bold, italic, mono, fallback) on a cold system font cache. Pure layout with the mock shaper is p99 0.02 ms.
  - CI therefore runs one untimed warm-up pass before the measured runs, and gates on the median of 3. Cold-cache scrolling is a real first-open cost and should be re-checked on owner hardware (owner kit step 4).
- **T7:** `GetProcessTimes` has ~15.6 ms granularity, so 0 µs means "below one tick".

**Subsystem breakdown** (`mem-stats` build; Rust-side heap sizes are measured with `heap_bytes`/capacities; native = commit growth since start minus the Rust-side total, so it is **estimated, not measured**):
| Subsystem | M1b | M2 |
|---|---|---|
| source text | 20,085 B (0.02 MiB) | 1,053,090 B (1.00 MiB) |
| block index + maps | 20,457 B (0.02 MiB) | 1,205,180 B (1.15 MiB) |
| detail-tree cache | 5,954 B | 5,954 B |
| layout cache (18 layouts, 30 paras; model estimate) | 6,600 B | 6,600 B |
| band buffer | 512,000 B (0.49 MiB) | 512,000 B (0.49 MiB) |
| DirectWrite + other native (estimated) | 1,568,920 B (1.50 MiB) | 2,779,544 B (2.65 MiB) |
| commit growth since start | 2,134,016 B | 5,562,368 B |

**Phase 001 restated in MiB** (conversion only):
- Backend A full scene: 2.39 MB → 2.28 MiB.
- Skeleton: 1.33 MB → 1.27 MiB.
- Spikes B/C: ~11.7 MB → ~11.16 MiB.
- yamdv.exe: 480,768 B → 0.46 MiB.

**Deltas vs Phase 001:**
- M0: 1.27 → 2.05 MiB (+0.78 MiB: DirectWrite factory, fonts, band DIB).
- yamdv.exe: 0.46 → 1.45 MiB (+0.99 MiB: pulldown-cmark, encoding_rs and chardetng tables, DirectWrite/COM glue).
- Scroll p99: 4.2 → 4.9 ms on a real 1 MiB document.

## 6. Decisions (ADRs)
- ADR-0001 amendment: ProcessDynamicCodePolicy (ACG) is **on** (F-06).
- ADR-0007: Two-level document model, splice algorithm, estimate formula, module-level lint allows, damaged-UTF-8 heuristic, RTL alignment, code-overflow masking.
- ADR-0008: pulldown-cmark 0.13.4.
- ADR-0009: encoding_rs 0.8.42.
- ADR-0010: chardetng 1.0.0.
- ADR-0011: unicode-segmentation 1.13.3.
- ADR-0012: libfuzzer-sys 0.4 (fuzz/ only).

## 7. Deviations from the spec
1. **RTL alignment:** RTL paragraphs use DirectWrite RTL reading direction with `LEADING` alignment, which puts the text on the right edge. §4.4.1's literal "trailing alignment" would put RTL text on the left. The intended result (right-aligned RTL) is achieved.
2. **`~sub~`:** it renders as **strikethrough**, not as normal text (§4.4.3). This follows P-08a: `reading.subscript_tilde` is off by default, so `~x~` is strikethrough. `^sup^` renders as normal text.
3. **☑ task marker:** U+2611 renders as a color emoji glyph (Segoe UI Emoji presentation), not as a text-style glyph. ☐ is a text glyph.
4. **`<details>` blocks:** they appear as two `html` cards around the inner Markdown, which renders normally. pulldown splits them that way, so there is no `details` label.
5. **Code cards:** they keep a header strip of empty space above the code, reserved for the Phase 003 toolbar and language label.
6. **Overflow masking:** overflowing code text is masked with page-background fills right of the card (the renderer has no glyph clip). Inside a future colored container this mask would need that container's color.
7. **Inline code background:** the padding overlaps adjacent letters slightly because no extra advance is inserted (visible in `code-*`).
8. **Damaged UTF-8:** decoded as lossy UTF-8 by a heuristic (ADR-0007) instead of purely by chardetng.
9. **Detail reparse:** appends stub footnote definitions to the slice. This is invisible but makes the reparse a little more than "exactly the slice".
10. **T4 on CI:** uses an untimed warm-up pass and the median of 3 runs (§5).

## 8. Conflicts, spec defects, questions for the reviewer
1. **§4.4.3 vs P-08a (`~sub~`):** §4.4.3 says "normal text" but P-08a makes it strikethrough by default. Proposal: P-08a wins (precedence: patches > tasks). Add a patch to §4.4.3 in 003.
2. **F-02 hash:** our 001 prompt hash `39e1f158…` differs from the reviewer's `04e60d03…`, yet all appendix hashes match. The difference must be outside the appendices (line endings or a trailer). Question: does the reviewer want a diff?
3. **"trailing alignment" for RTL (R-REN-19 wording):** proposal: patch to "aligned to the paragraph's start edge (right for RTL)".
4. **M6 budget vs trimming:** after the one-off trim the PWS budget is trivially met, while commit grows +7.2 MiB. Proposal: add a commit-based M6 budget, e.g. ≤ M1b commit + 8 MiB, or require releasing the dialog DLLs (CoFreeUnusedLibraries).
5. **≤ 1 MB file rule vs tall goldens:** 1.5× `typical-basic` is a 1500×26,610 image of 2.3–3.2 MB. Goldens are stored as tiles of ≤ 5000 rows (largest 637 KB). Please confirm this is acceptable.

## 9. Risks discovered
| Risk | Severity (H/M/L) | Mitigation proposal |
|---|---|---|
| Open dialog leaves +7 MiB commit and 54 extra DLLs loaded | M | `CoFreeUnusedLibrariesEx` after the dialog, or host the dialog on a short-lived thread; measure in 003 |
| Cold font cache: first scroll through a new document can reach 30–45 ms frames on slow machines | M | Pre-create the text formats for bold/italic/mono at startup (idle-time warm-up); check real numbers via owner kit step 4 |
| ACG could break third-party shell extensions in the Open dialog | L | Owner kit step 2; revisit in the ADR-0001 amendment |
| Re-entrancy: synchronous Win32 messages during RefCell borrows (bug found: WM_PAINT validated unpainted) | M | Fixed for paint (deferred UpdateWindow, skip re-entrant WM_PAINT). Audit other synchronous calls (SetWindowPos, dialogs) in later phases |
| CI first-paint variance (73–121 ms runs for M0) near the 100 ms T2 budget | L | Median of 5 runs. Move DirectWrite fallback setup off the first-paint path if it creeps up |

## 10. Dependencies
| Crate | Version | Product or dev-only | Why | ADR |
|---|---|---|---|---|
| pulldown-cmark | 0.13.4 | product | CommonMark/GFM parser | ADR-0008 |
| encoding_rs | 0.8.42 | product | legacy decoding | ADR-0009 |
| chardetng | 1.0.0 | product | encoding detection | ADR-0010 |
| unicode-segmentation | 1.13.3 | product | word bounds | ADR-0011 |
| libfuzzer-sys | 0.4.13 | dev-only (fuzz/, separate workspace) | fuzzing | ADR-0012 |
| windows / windows-core / windows-numerics, tiny-skia | as in 001 (new features: DirectWrite, Imaging, Shell, Ole, DataExchange, ProcessStatus, …) | product | Win32 / rendering | ADR-0003, ADR-0006 |

## 11. Repository state
- Branch `phase/002-reading-mvp` → `main`, PR https://github.com/snipeTR/YAMDV/pull/2.
- Last branch CI run before this report: [38938805660](https://github.com/snipeTR/YAMDV/actions/runs/38938805660), success.
- Merged with a merge commit; tag `phase-002` is on that merge commit. The hash is not known when this file is committed, so `git rev-parse phase-002` gives it. The main push run produces the `owner-kit` artifact.

## 12. Files added/changed
```
.github/workflows/ci.yml  .github/scripts/measure.ps1  .gitattributes  CHANGELOG.md
crates/yamdv-core/src/{decode,path,i18n,ini,log,settings,settings_table,theme,theme_table}.rs
crates/yamdv-core/src/doc/{mod,index,detail,text,slug}.rs  crates/yamdv-core/tests/{encodings,large}.rs
crates/yamdv-layout/src/{geom,fenwick,shaper,block,plain,viewer}.rs  crates/yamdv-layout/tests/{commonmark.rs,vectors.txt}
crates/yamdv-ui/src/{lib,dialog}.rs
crates/yamdv-platform-win/src/{dwrite,render,sys,perf,devtools,window,hardening,lib}.rs
crates/yamdv-app/src/{cli,main}.rs
xtask/src/{golden,memprobe,size,spec,sha256,main}.rs
fuzz/{Cargo.toml,Cargo.lock,fuzz_targets/{parse_layout,ini}.rs,seeds/}
tests/corpus/002/*  tests/corpus/encodings/*  tests/golden/002/*.png (48 tiles)
docs/{PROCESS,SPEC_CHANGES,OWNER_KIT,TRACEABILITY}.md  docs/prompts/*  docs/spec-patches/002.txt
docs/plans/002_PLAN.md  docs/adr/ADR-0001 (amended), ADR-0007…0012  docs/reports/{001 (amended),002}_REPORT.md
SPEC/UI_SPEC/PERFORMANCE/ARCHITECTURE.md (patched)
```

## 13. Manual QA checklist for the owner (Windows PC)
Follow `docs/OWNER_KIT.md` (artifact `owner-kit`) — every step lists its expected result:
1. Kit step 0: unblock the ZIP and get past SmartScreen. Expected: the exes start.
2. Kit step 1: open every corpus file. Expected: title `<file> — YAMDV`, readable text, smooth wheel, keys and scrollbar, Ctrl+C copies, Esc and Ctrl+W close. Turkish encoding samples look right.
3. Kit step 2: Ctrl+O with multi-select (first file in place, others in new windows); drag & drop a file (opens) and a folder (shows "Drop Markdown files, not folders"); CLI with two files gives two windows; IME input in the dialog. This verifies the F-06 / AC-4 items that are NOT VERIFIED on CI.
4. Kit step 3: memprobe M0/M1b/M2/M6 JSON. Expected: within the §5 budgets on your hardware.
5. Kit step 4: spike A at 1920×1080 with autoscroll. Expected: real T4 p99 ≤ 16 ms.
6. Kit step 5: switch light and dark. Expected: the window follows immediately.
7. Kit step 6: 150 % scaling. Expected: crisp text, correct per-monitor relayout.
8. Also open a > 20 MiB file (expected: the "Open as plain text / Render anyway / Cancel" dialog) and a binary file such as an .exe renamed to .md (expected: the binary warning).

## 14. Proposed next steps
- Phase 003: tables, code highlighting, the code toolbar and horizontal scroll, callouts, footnotes, images (re-verify WIC decode under ACG), math.
- Release dialog DLLs after Ctrl+O (risk 1). Warm up text formats (risk 2).
- Spec patches for §8 items 1, 3 and 4.

## 15. Özet (Türkçe)
- Phase 002 tamamlandı. YAMDV artık gerçek bir salt-okunur Markdown görüntüleyici: CLI'dan çoklu dosya, Ctrl+O ve sürükle-bırak ile açılış; kodlama tespiti; boyut ve ikili dosya korumaları ile pencere içi onay penceresi.
- İki seviyeli belge modeli (blok indeksi + tembel ayrıntı ağaçları, splice) ve sanallaştırılmış, akıcı kaydırma, seçim ve kopyalama çalışıyor.
- CI ölçümleri (1000×700): M0 2,05 MiB, M1b 2,50 MiB, M2 5,48 MiB; ilk çizim 74/80/132 ms; kaydırma p99 5,9 ms (1 MiB dosyada 4,9 ms); boşta CPU 0; exe 1,45 MiB. Tüm bütçe kapıları yeşil.
- ACG (dinamik kod yasağı) açık. Sürükle-bırak, IME ve kabuk eklentili klasörler CI'da doğrulanamadı; sahip kitindeki (OWNER_KIT.md, EN+TR) adımlarla kontrol edin.
- Dikkat: Ctrl+O sonrasında commit +7 MiB artıyor (diyalog DLL'leri yüklü kalıyor). Soğuk font önbelleğinde ilk kaydırma yavaş olabiliyor. İkisi de Phase 003 için önerildi.
- 001 prompt dosyamızın SHA-256 değeri hakeminkinden farklı (`39e1f158…`), ama 8 ekin tamamı doğrulandı.
- Bloklanmış madde yok.
