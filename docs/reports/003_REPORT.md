# YAMDV — Phase 003 Report: Full rendering, links & images

## 0. Fix items status
| F-ID | status | evidence |
|---|---|---|
| F-09 | DONE | `6c329f1`: `Ctrl+O` runs `yamdv.exe --dialog-host open <owner> <nonce>` by absolute path, WM_COPYDATA result protocol, in-process dialog and its trim removed (ADR-0013). CI run 38962744168 M6: PWS 2.55 MiB (2,674,688 B) ≤ M1 + 1 MiB, commit 3.56 MiB (3,731,456 B) ≤ M1 commit + 1.5 MiB, modules 36 before = 36 after, **no trim**. |
| F-10 | DONE | `3190b06`: idle-time font warm-up, one task per posted message. T4 cold/warm split in `measure.ps1`; typical cold p99 5.16 ms, large cold 6.21 ms. |
| F-11 | DONE | `b487a6a`: debug-build re-entrancy guard logging the message id; synchronous Win32 calls deferred (P-18, ADR-0007 amendment `aee8187`). |
| F-12 | DONE | `9738284`: `band_rows = clamp(524288 / (4 × w), 32, 128)`; tests at 1000/1920/3840/7680 px (`geom::band_tests`); memprobe JSON reports `band_bytes` (512,000 B on CI). See §8.1 for the 7680 px conflict. |
| F-13 | DONE | `27f1693`: inline code padding via `IDWriteTextLayout1::SetCharacterSpacing` (P-21); goldens regenerated (`ce5dfbf`). |
| F-14 | DONE | Previous phase: tag `phase-002` = merge commit `a444acdc5890340f5c581df52b5d2255fca89b24`. PR head commit: see §11. |
| F-15 | DONE | `1a00fae`: `docs/OWNER_KIT.md` step 7 (EN + TR) for links/Ask/blocked targets, images, tables, code toolbar, callouts, footnotes, details, dialog host, clipboard; `owner-kit` artifact now copies `typical-assets/` (`Copy-Item -Recurse`). |

## 1. Summary
Phase 003 delivers tables, syntax-highlighted code with toolbar, callouts, footnotes, front matter, definition lists, inline extensions, a safe HTML subset, full link navigation and policy, CF_HTML clipboard, the out-of-process dialog host and real clipping.
**Images are only partly done:** source resolution, the S-RES-02 network policy (mock-FS matrix), header probe, sizing and LRU exist and are tested in `yamdv_core::image`, and every image shows the P-25 chip. **WIC decoding and pixel display are NOT DONE** (see §3, §7).
CI windows-latest: M0 2.08 MiB, M1 (`typical.md`) 2.59 MiB, M2 5.46 MiB, M6 2.55 MiB without trim, T1 80 ms, warm T4 p99 6.36 / 4.39 ms, idle CPU 0, exe 1.82 MiB.
66 XSS vectors, 4 fuzz targets × 621 s; fuzzing found and fixed 2 real bugs (inline `<style>` text leak, URL host with `:`).

## 2. Environment
- Engineering host: Linux x86_64, 8 cores, 15 GiB RAM; rustc 1.99.0 (pinned), cargo 1.99.0; nightly for cargo-fuzz only; cross builds with `cargo-xwin` for `x86_64-pc-windows-msvc`.
- gh account: `snipeTR`.
- Can run Windows GUI: **no** locally; all Windows checks ran on GitHub Actions `windows-latest` (Windows Server 2025, Hyper-V 1024×768, no GPU).

## 3. Task status
| Task / Requirement ID | Status | Evidence | Notes |
|---|---|---|---|
| M1 prompt, P-10…P-25 | DONE | `1dd6d34`, `docs/spec-patches/003.txt`, spec-check | |
| M2 F-10…F-13 | DONE | §0 | |
| M3 F-09 dialog host | DONE | §0 | T-DLG 534 ms > 400 ms soft (§9) |
| M4 clip stack, code hscroll (P-19) | DONE | `76d1596` | |
| M5 R-CODE-01…06 | DONE / PARTIAL | `c706a30`, `f9282ad` | PARTIAL: toolbar fade not animated; R-CODE-06 uses chunked paragraphs, not lazy shaping |
| M6 R-REN-07 tables | PARTIAL | `08be4db` | all rows laid out once per block (no row-level laziness); column measurement samples the first 1 000 rows; multi-line cells align the box, not each line; colspan/rowspan = 1 |
| M7 callouts, task boxes (R-MD-04, R-REN-06, B.6.4) | DONE | `646f5b7` | |
| M7 footnotes, front matter, deflists (R-REN-10, R-REN-13, R-MD-11) | DONE / PARTIAL | `5566d16` | PARTIAL: no footnote hover tooltip; refs are small link-coloured text, not raised inline objects |
| M8 inline (R-MD-03/05/06/12/13) | PARTIAL | `96745df` | sup/sub and footnote refs have no baseline shift (no `IDWriteInlineObject`); text with escapes/entities skips the inline pass; `==` cannot cross emphasis boundaries; kbd drawn as inline code (no key-cap border) |
| M9 safe HTML (R-MD-14, S-HTML-01…03) | DONE / PARTIAL | `6360ee1`, `7e67e28` | PARTIAL: named entities limited to a common subset (numeric all); cross-block details content not indented 16 px, hidden blocks keep their margins; abbr tooltip missing |
| M10 links (R-LINK-01…06, S-LINK-01…04, S-EXEC-04) | DONE (reduced) | `fa2414b` | PARTIAL: Ask/confirm are the in-window confirm dialog, not an anchored popover; no tooltip/status-tooltip/banner/popover components (MotW, elevation and lossy banners not shown); no Ctrl+click / middle-click new window; no Tab/Shift+Tab focus; heading anchors scroll to the block top |
| M11 images — resolution + policy (R-IMG-02, S-RES-01…04) | PARTIAL | `7e98b75`, `crates/yamdv-core/src/image.rs` tests `r_img_02_*`, `s_res_0*_*` | all path forms, data URIs, UNC/WebDAV/`\\?\UNC`/device/ADS/`file://host` blocked with **zero** FS calls (recording mock), mapped drive + reparse-to-network blocked, same-share exception, 50 MB limit. **Not wired**: no Windows `Fs` implementation; nothing opens image files yet |
| M11 header probe + limits (R-IMG-04, S-RES-04) | PARTIAL | `image::probe`, `within_limits` | PNG/JPEG/GIF/BMP/ICO/TIFF/WebP/SVG from bytes; no worker thread |
| M11 sizing (R-IMG-03) | PARTIAL | `display_size`, `obsidian_size`, `html_len`; `Detail::images` carries `w`/`h` | not used by layout yet (no pixels) |
| M11 LRU (P-13) | PARTIAL | `image::Lru` (test `p_13_lru_budget`) | not connected |
| M11 WIC decode, display-size decode, visible ± 1 screen, worker (E.10) | NOT DONE | — | carried over, see §14 |
| M11 ACG verification per format | NOT DONE | — | needs the decoder |
| M11 DIB vs DDB measurement ADR | NOT DONE | — | needs the decoder |
| M11 placeholders (R-IMG-06 + P-25) | PARTIAL | `block.rs` chip; golden `images-*` | every image (inline **and** block) is a P-25 chip (▣ + alt or file name, `code_bg`, radius 4, 12 px `text_muted`); block placeholders with icon/path/[Copy URL], kind-specific message, tooltip and click actions missing |
| M11 SVG placeholder | PARTIAL | chip | no "unsupported format" text yet |
| M12 clipboard (R-REN-16) | DONE / PARTIAL | `e1f16eb` | Copy as Markdown is marker-based, not AST-exact for reference links/autolinks; menu entry is Phase 004 |
| M13 corpus | DONE | `tests/corpus/002/`: typical.md (+ `typical-assets/one.png`, `two.png`, 800×600 generated), images, tables, callouts, details, code, footnotes, links, html, frontmatter, unicode, … | `code.md` is the Phase 002/003 file; not re-checked for every language |
| M13 goldens (P-23) | DONE | `ce5dfbf`: 19 docs × {1.0, 1.5} × {light, dark} = 76 renders, 112 tiles, 24,072,179 B, every file < 1 MB | from CI run 38962744168 |
| M13 XSS suite (S-TEST-02) | DONE | `crates/yamdv-export/tests/xss.rs`: 66 vectors × 3 contexts through model + CF_HTML, plus sanitizer token checks | |
| M13 fuzz (4 targets) | DONE | `fuzz/fuzz_targets/{parse_layout,html,url,highlight}.rs`, 621 s each | §4 AC-8 |
| M13 measurements | PARTIAL | §5 | T-IMG and T7-after-decode/tooltip not measurable (no decoder, no tooltip) |
| M13 owner kit (F-15) | DONE | §0 | |
| M13 TRACEABILITY | DONE | `8e15962` | |

## 3b. Spec patches applied
| P-ID | doc | verified by spec-check (yes/no) |
|---|---|---|
| P-10 | docs/PERFORMANCE.md (§D.2 M6) | yes |
| P-11 | docs/PERFORMANCE.md (§D.4 rule 10) | yes |
| P-12 | docs/PERFORMANCE.md (§D.4 rule 3) | yes |
| P-13 | docs/PERFORMANCE.md (§D.4 rule 4) | yes |
| P-14 | docs/SECURITY.md (S-PROC-07) | yes |
| P-15 | docs/SECURITY.md (after S-IPC-04) | yes |
| P-16 | docs/SECURITY.md (S-FILE-01) | yes |
| P-17 | docs/SPEC.md (R-REN-04) | yes |
| P-18 | docs/ARCHITECTURE.md (§E.10) | yes |
| P-19 | docs/ARCHITECTURE.md (§E.6) | yes |
| P-20 | docs/SHORTCUTS.md (§F.4) | yes |
| P-21 | docs/UI_SPEC.md (§B.6.3) | yes |
| P-22 | docs/ROADMAP.md (row 010) | yes |
| P-23 | docs/TESTING.md (§H.4) | yes |
| P-24 | docs/SPEC.md (R-MD-14) | yes |
| P-25 | docs/SPEC.md (R-IMG-06) | yes |

Applied by `cargo xtask spec-patch` from `docs/spec-patches/003.txt`, logged in `docs/SPEC_CHANGES.md`; `cargo xtask spec-check` → "PROCESS.md vs prompt 001 §2/§3/§7 + amendments: match", patches P-01…P-25 listed.

## 4. Acceptance criteria
| AC | PASS / FAIL / NOT VERIFIED | Evidence |
|---|---|---|
| AC-1 | PASS | §0; M6 met without trim (CI run 38962744168). |
| AC-2 | PASS | `cargo xtask spec-check` → "patches: P-01 … P-25". |
| AC-3 | PASS | fmt, clippy `-D warnings`, test, deny, check-imports, size, memprobe gate and goldens green on CI (§11); PR merged; tag `phase-003`. |
| AC-4 | **FAIL** | All corpus documents render and the goldens are committed and self-reviewed, but **images are interim chips** (no decoded pixels), which AC-4 does not allow. Other known deviations against `UI_SPEC.md`: see §3 PARTIAL rows. |
| AC-5 | **FAIL** (partial) | Policy matrix: `link.rs` tests (scheme × target × MotW × setting). **No automated in-window test** for anchor scroll, back/forward, blocked exe or the Ask dialog; manual steps are in OWNER_KIT step 7.1. The Ask UI is a dialog, not a popover. |
| AC-6 | PASS (policy level) | `image::tests::s_res_02_network_matrix_before_fs`: 15 forms, recording mock shows **0** FS calls; `s_res_02_mapped_drive_and_reparse`, `s_res_02_same_share_exception`. Caveat: the policy is not wired to a Windows FS layer yet. |
| AC-7 | PASS | `cargo test -p yamdv-export --test xss` → 4 passed (66 vectors; no `<script`, no `on*=`, no `javascript:`/`vbscript:`/non-image `data:` in markup). |
| AC-8 | PASS | `cargo +nightly fuzz run <t> -- -max_total_time=620`: parse_layout 142,793 runs / 621 s; highlight 9,822,139 / 621 s; html 142,393 / 621 s; url 2,978,368 / 621 s; 0 open crashes. Two earlier crashes were fixed and kept as regressions (§7). |
| AC-9 | **FAIL** (partial) | M/T table complete with MiB + bytes and deltas (§5); warm T4 ≤ 16 ms; T7 = 0 µs. **T-IMG and "T7 after tooltip / after decode" not measured**: no decoder and no tooltip component exist. |
| AC-10 | PASS | `docs/OWNER_KIT.md` step 7 EN + TR. |
| AC-11 | PASS | This file follows PROCESS §7 + A-2 (§0, §3b). |

## 5. Measurements
**Setup:** CI run [38962744168](https://github.com/snipeTR/YAMDV/actions/runs/38962744168), artifact `measurements`. windows-latest, Hyper-V 1024×768, client 1000×700. `xtask memprobe`, QueryWorkingSet shared-bit counting, median of 5 runs (M0/M1/M2) or 3 (M6); release build with ACG on. M1 now uses `typical.md` (35,901 B; replaces M1b).

| ID | Result | Bytes | Budget | Gate | Δ vs Phase 002 |
|---|---|---|---|---|---|
| M0 PWS | 2.08 MiB | 2,183,168 | ≤ 4.8 MiB | pass | +0.03 MiB |
| M1 PWS (`typical.md`) | 2.59 MiB | 2,711,552 | ≤ 8.0 MiB | pass | vs M1b 2.50: +0.09 MiB (larger doc) |
| M2 PWS | 5.46 MiB | 5,722,112 | ≤ 16 MiB | pass | −0.02 MiB |
| M6 PWS / commit | 2.55 MiB / 3.56 MiB | 2,674,688 / 3,731,456 | ≤ M1 + 1 MiB / ≤ M1 commit + 1.5 MiB | pass | commit −7.2 MiB vs 002's 10.77 MiB (dialog host) |
| M7 commit ÷ PWS | M0 2.74 MiB (2,871,296) = 1.32×; M1 3.61 MiB (3,788,800) = 1.40×; M2 6.86 MiB (7,196,672) = 1.26× | | ≤ 3× | pass | |
| T2 first paint, empty | 81 ms | | ≤ 100 ms | pass | +7 ms |
| T1 first paint, `typical.md` | 80 ms | | ≤ 150 ms | pass | = T1b 80 ms |
| T1L first paint, large-1mb | 141 ms | | ≤ 300 ms | pass | +9 ms |
| T4 typical, warm median p99 (runs) | 6.36 ms (6.48 / 4.99 / 6.36) | | ≤ 16 ms | pass | +0.46 ms |
| T4 typical, cold p99 | 5.16 ms | | report (≤ 33 ms target) | — | 002 cold was 18–45 ms |
| T4 large-1mb, warm median p99 | 4.39 ms (4.39 / 4.26 / 4.61) | | ≤ 16 ms | pass | −0.51 ms |
| T4 large-1mb, cold p99 | 6.21 ms | | report | — | |
| T7 idle CPU 30 s | 0 µs (empty and `typical.md`) | | ≤ 1 ms | pass | = |
| T7 after tooltip / after decode | NOT VERIFIED — no tooltip, no decoder | | ≤ 1 ms | — | |
| T-IMG | NOT VERIFIED — no decoder | | ≤ 100 ms soft | — | |
| T-DLG | 534 ms | | ≤ 400 ms soft | over (report only) | new |
| B-SIZE yamdv.exe | 1.82 MiB | 1,907,200 | ≤ 4.0 MiB | pass | +0.37 MiB (+25 %, see note) |
| band buffer | 0.49 MiB | 512,000 | ≤ 512 KiB (P-12) | pass | = |

Notes:
- **B-SIZE +25 % (> 10 %, H6):** new code paths: 30 hand-written lexers (`yamdv-highlight`), the gemoji table (1 913 aliases, ADR-0014), HTML tokenizer/sanitizer, URL parser/IDN, table/callout/front-matter layout, the dialog host and the CF_HTML writer. No new crates in the product.
- **T-DLG 534 ms** includes `CreateProcessW` of a second `yamdv.exe` plus `IFileOpenDialog` start-up on a cold runner.
- `GetProcessTimes` has ~15.6 ms granularity, so 0 µs means "below one tick".

**Subsystem breakdown** (`mem-stats` build; Rust-side sizes measured; native = commit growth minus Rust-side, **estimated**):
| Subsystem | M1 (`typical.md`) | M2 |
|---|---|---|
| source text | 35,901 B | 1,053,090 B (1.00 MiB) |
| block index + maps | 27,412 B | 1,205,180 B (1.15 MiB) |
| detail-tree cache (incl. HTML model, `Detail::images`) | 6,378 B | 6,378 B |
| layout cache (18 layouts, 30 paras; model estimate) | 6,640 B | 6,640 B |
| highlighter state | not separately counted (per-line state lives in the layout cache entries) | same |
| image cache | 0 B (no decoder yet) | 0 B |
| band buffer | 512,000 B | 512,000 B |
| DirectWrite + other native (estimated) | 1,635,797 B (1.56 MiB) | 2,680,776 B (2.56 MiB) |
| commit growth since start | 2,224,128 B | 5,464,064 B |

## 6. Decisions (ADRs)
- ADR-0013 — Open dialog in an out-of-process `--dialog-host` (F-09).
- ADR-0014 — Committed gemoji shortcode table generated by `tools/gen_emoji.py`.
- ADR-0007 (amended) — re-entrancy rule and deferred synchronous Win32 calls (F-11).
- No DIB-vs-DDB ADR: the decoder is not built (§3).

## 7. Deviations from the spec
1. **Images:** no WIC decode, no worker thread, no pixels; every image is a P-25 chip, including block images, which should get the large R-IMG-06 placeholder or the picture. The policy, probe and sizing code is tested but not wired.
2. **Links:** the Ask "popover" is an in-window dialog. There are no tooltip, popover or banner components (affects link title, abbr and footnote tooltips, and the MotW/elevation/lossy banners).
3. Tables have no row-level laziness. Inline objects (baseline-shifted sup/sub, footnote refs) are missing. Toolbar fade is not animated. The rest of the PARTIAL items are in §3.
4. Fuzz regressions fixed in this phase:
   - Inline `<style>`/`<script>` split across Markdown inline events leaked its text content into the model. Fixed with `inline_drop` in `detail.rs`.
   - `url_allowed` now rejects active schemes anywhere in the value, not only as a prefix.
   - Hosts with a stray `:` broke the serialize→parse round trip. Fixed in `link.rs`, test `fuzz_url_host_colon_rejected`.
   - Inputs are kept in `tests/regressions/`.
5. The CF_HTML XSS check scans **markup only** for schemes. Escaped document text, such as a code span reading `javascript:alert(1)`, is inert and allowed. Raw `<script` is checked over the whole output.
6. Goldens stay in `tests/golden/002/` (the directory name is kept so the Phase 002 tooling works unchanged).

## 8. Conflicts, spec defects, questions for the reviewer
1. **Band cap (P-12) conflict:** `clamp(524288 / (4 × w), 32, 128)` hits the 32-row floor above 4 096 px. At 7 680 px a band is 7680 × 32 × 4 = **983,040 B**, which exceeds the 512 KiB cap that P-12 states. The two rules cannot both hold. Proposal: keep the 32-row floor (fewer than 32 rows makes ScrollWindowEx banding visibly slow) and amend P-12 to "≤ 512 KiB for widths ≤ 4 096 px; 32 rows above". Alternative: drop the floor to 16 rows.
2. AC-4 vs scope: should Phase 004 open with the image decoder as a fix item (F-16), since AC-4 depends on it?
3. R-IMG-06 says "blocked network path" is a separate placeholder. Should ADS paths (`a.png:stream`) be reported as "blocked network path" or as "invalid"? They are currently classed as Network/Remote (blocked before any FS access).

## 9. Risks discovered
| Risk | Severity | Mitigation proposal |
|---|---|---|
| Image decoder still missing; WIC under ACG unverified per format | H | Make it the first fix item of Phase 004. Verify each codec under ACG on CI before the layout work. |
| T-DLG 534 ms on CI | M | Measure on owner hardware. Consider pre-spawning the host on first idle (watch the RAM cost). |
| exe +25 % | L | The emoji table and lexers dominate. Check `cargo bloat` in Phase 004. |
| No automated in-window link test | M | Add a dev-tools `--script` driver (click at hot rect, assert scroll/title) next phase. |

## 10. Dependencies
| Crate | Version | Product or dev-only | Why | ADR |
|---|---|---|---|---|
| (no new crates in Phase 003) | | | | |
| libfuzzer-sys | 0.4 | fuzz only (not in workspace) | 4 fuzz targets | ADR-0012 |

## 11. Repository state
- Branch `phase/003-full-rendering` → `main`, PR https://github.com/snipeTR/YAMDV/pull/3.
- PR head commit and merge commit: recorded in the final chat message and by tag `phase-003`.
- Previous phase: `phase-002` = `a444acdc5890340f5c581df52b5d2255fca89b24`.
- Golden source CI run: 38962744168 (only the golden gate failed before regeneration).

## 12. Files added/changed
```
crates/yamdv-core/src/image.rs               (new: R-IMG/S-RES policy, probe, sizing, LRU)
crates/yamdv-core/src/{html,link}.rs, doc/{detail,inline,index,text}.rs
crates/yamdv-highlight/                       (new lexers)
crates/yamdv-layout/src/{block,code,table,callout,front,viewer,geom,warm}.rs
crates/yamdv-export/src/html.rs, tests/xss.rs (new)
crates/yamdv-platform-win/src/{window,render,dwrite,dialog_host,reentry}.rs
fuzz/fuzz_targets/{html,url,highlight}.rs     (new)
tests/corpus/002/{typical,images,tables,callouts,details}.md, typical-assets/ (new)
tests/golden/002/                             (112 tiles)
tests/regressions/                            (fuzz inputs)
docs/{OWNER_KIT,TRACEABILITY,SPEC_CHANGES}.md, docs/adr/ADR-0013, ADR-0014, docs/plans/003_PLAN.md, docs/reports/003_REPORT.md
.github/scripts/measure.ps1, .github/workflows/ci.yml
```

## 13. Manual QA checklist for the owner (Windows PC)
Follow `docs/OWNER_KIT.md` steps 1–7. The Phase 003 essentials are:
1. `links.md`: `#anchor` click → smooth scroll. Relative `.md` → same window. Alt+← / Alt+→ restore the scroll position. An http link shows the Ask dialog with the real URL. The `.exe` link is blocked with [Show in Explorer]. `javascript:` → "Blocked link type" toast.
2. `images.md`: every image is a grey ▣ chip. Nothing is fetched: Resource Monitor shows no network activity for `yamdv.exe`.
3. `tables.md`: alignment, zebra rows and rounded border. Ctrl+C then paste into Excel gives tab-separated text.
4. `code.md`: colours, hover toolbar, Copy, and horizontal scroll with Shift+wheel.
5. `callouts.md`: five colours and icons. The folded Obsidian callout opens on click.
6. `footnotes.md`: `[1]` → note, `↩` → back.
7. `details.md`, `html.md`: details toggle, centred header, no script/style text visible.
8. Ctrl+O: a second `yamdv.exe` appears in Task Manager only while the dialog is open.
9. Ctrl+C of formatted text pasted into Word keeps bold/italic/links. Ctrl+Shift+C gives Markdown.
10. Memory step 3 with `typical.md`: send the JSON files.

## 14. Proposed next steps
- Fix item: WIC decoder on a worker thread with the Windows `Fs` layer wired to `image::resolve_policy`. Then display-size decode, visible ± 1 screen + LRU, block placeholders, the ACG matrix per format, the DIB-vs-DDB ADR, T-IMG and T7-after-decode.
- Tooltip/popover/banner components (unblocks the link Ask popover, footnote/abbr tooltips, MotW banner).
- Automated in-window link test (AC-5).
- Then Phase 004 scope.

## 15. Özet (Türkçe)
Faz 003'te şunlar eklendi:
- Tablolar, sözdizimi renklendirmeli kod blokları ve araç çubuğu, callout'lar, dipnotlar, front matter, tanım listeleri ve satır içi uzantılar.
- Güvenli HTML alt kümesi ve tam bağlantı gezintisi/politikası.
- CF_HTML panosu, ayrı süreçte çalışan Aç penceresi (F-09) ve gerçek kırpma.

CI ölçümleri:
- Bellek: M0 2,08 MiB, M1 2,59 MiB, M2 5,46 MiB.
- M6: kırpma olmadan 2,55 MiB.
- İlk çizim 80 ms, sıcak kaydırma p99 6,4 ms, boşta CPU 0. Exe 1,82 MiB.

**Görseller yarım kaldı:**
- Yol çözümleme, ağ yolu engelleme (sahte dosya sistemiyle, hiç dosya erişimi olmadan), başlık okuma ve boyutlandırma test edildi.
- Ancak WIC ile çözme ve gerçek görsel gösterimi yapılmadı; şimdilik tüm görseller küçük ▣ etiketi olarak görünüyor.

Bu yüzden AC-4, AC-5 (pencere içi otomatik bağlantı testi yok) ve AC-9 (T-IMG ölçülemedi) FAIL. Güvenlik tarafında 66 XSS vektörü geçiyor. 4 fuzz hedefi 10'ar dakika çalıştı ve bulunan 2 gerçek hata düzeltildi. Bant sınırı çelişkisi (7680 px'te 983 040 B) §8.1'de.
