# YAMDV — Phase 003b Report: Rendering completion

## 0. Fix items status
| F-ID | Status | Evidence |
|---|---|---|
| F-16 | done | `docs/spec-patches/003b.txt` applied with `cargo xtask spec-patch`; `spec-check` green (ubuntu job) |
| F-17 | done | goldens 24,072,179 B / 112 files → 3,394,856 B / 44 files (now 3,477,862 B on disk incl. dir overhead); no history rewrite; `xtask golden` enforces P-28 |
| F-18 | done | A-3 "Token economy" added via the amendment mechanism; `spec-check` green |
| F-19 | done | §3c table; owner decision 2026-10-17: **no new lexers** — every R-CODE-01 name maps to an existing lexer; anything outside R-CODE-01 is "deferred (FUT-01)" |

## 1. Summary
Phase 003b is **partial (A-3 rule 9)**. Done: images end to end (Fs layer, worker, WIC, store + LRU, R-IMG-06 placeholders, P-27 invalid path), the ACG matrix on CI, the in-window test driver with 6 CI scripts (AC-5), Tab focus navigation with focus ring, Ctrl/middle-click → new window, `<kbd>` key caps. Not done: tooltip / popover / banner components (§4.2), inline objects for sup/sub/footnote refs, code-toolbar fade, table row laziness (§4.4), DIB-vs-DDB measurement, M1-with-images and T-IMG. Prompt 004 SHA-256: `3478375d70dfeddcb24b6ede3f656f196546f5284fd885ce88a00cb9d9606953`.

## 2. Environment
Linux host, Rust 1.99.0, `cargo-xwin` cross-gates; Windows checks on GitHub Actions `windows-latest` (Server 2025, Hyper-V 1024×768, no GPU).

## 3. Task status
| Task | Status | Notes |
|---|---|---|
| 4.1.1 Windows `Fs` + policy before FS | done | recording-mock test + Windows twin `s_res_02_blocked_before_fs_on_windows` |
| 4.1.2 worker (256 KiB, cancellable, 5 s idle exit), probe + WIC at display size | done | `images.rs` |
| 4.1.3 layout integration, scroll anchoring, viewport ± 1 screen pinned, LRU ≤ 1.5 MiB | done | `viewer::tests::resize_keeps_anchor_block`, `p_13_lru_budget` |
| 4.1.4 ACG matrix on CI | done | ADR-0015; ICO/WebP fail (reported, not gated) |
| 4.1.5 DIB section vs DDB | **not done** | ADR-0015 records the decision (heap PBGRA) without the 4-image numbers |
| 4.1.6 placeholders (block R-IMG-06, inline P-25 chip, SVG unsupported) | done | goldens `images-*` |
| 4.2 tooltip, link status tooltip, popover, banner | **not done** | Ask / Blocked / local-file confirmations still use the in-window dialog |
| 4.3 Ctrl+click / middle-click → new window | done | driver `new-window.yds` |
| 4.3 Tab / Shift+Tab, focus ring (B.16), auto-scroll, Enter | done | driver `tab-order.yds`; links only (no other interactive elements yet) |
| 4.3 heading anchor 16 px below top | done | `anchor.yds` |
| 4.3 inline objects (sup/sub/footref) | **not done** | still 0.75 em without baseline shift |
| 4.3 `<kbd>` key cap | done | 1 px border + 2 px bottom border |
| 4.3 `<details>` indent 16 px, collapsed = no space | done (already in 003) | `front::details` |
| 4.3 code toolbar 120 ms fade | **not done** | shows/hides instantly |
| 4.4 table row laziness | **not done** | |
| 4.5 in-window driver + CI scripts | done | `--script` (dev-tools), `tests/driver/*.yds`, CI step "in-window driver scripts" |
| 4.6 measurements | partial | §5 |

## 3b. Spec patches applied
| P-ID | Doc | Verified by spec-check |
|---|---|---|
| P-26 | PERFORMANCE.md §D.4 rule 3 | yes |
| P-27 | SPEC.md R-IMG-06 | yes |
| P-28 | TESTING.md §H.4 | yes |
| P-29 | ROADMAP.md (003b row) | yes |

## 3c. F-19 language coverage (R-CODE-01)
Test for every row: `yamdv-highlight` `r_code_01_every_name_maps_to_lexer` plus the per-family test named.
| R-CODE-01 names | Lexer | Extra aliases | Family test |
|---|---|---|---|
| bash, sh, shell, zsh, console | Bash | shellsession | `clike::shells` |
| powershell, ps1 | PowerShell | pwsh, ps | `clike::shells` |
| bat, cmd | Bat | batch | `clike::shells` |
| c | C | | `clike::c_and_cpp` |
| cpp, c++, h, hpp | Cpp | cc, cxx, hxx | `clike::c_and_cpp` |
| csharp, cs | CSharp | c# | `clike::csharp_java_kotlin` |
| java | Java | | `clike::csharp_java_kotlin` |
| kotlin, kt | Kotlin | kts | `clike::csharp_java_kotlin` |
| go | Go | golang | `clike::go_python` |
| rust, rs | Rust | | `clike::rust` |
| python, py | Python | py3, python3 | `clike::go_python` |
| javascript, js, mjs, cjs, jsx | JavaScript | | `clike::js_ts` |
| typescript, ts, tsx | TypeScript | mts, cts | `clike::js_ts` |
| json, jsonc, json5 | Json | | `misc::json` |
| yaml, yml | Yaml | | `misc::yaml` |
| toml | Toml | | `misc::toml_ini` |
| ini, conf, properties | Ini | cfg, editorconfig, gitconfig | `misc::toml_ini` |
| xml, html, svg, xaml, vue | Xml (markup) | htm, xhtml, plist | `markup::xml_html` |
| css, scss, less | Css | sass | `markup::css_family` |
| sql | Sql | mysql, postgresql, psql, sqlite | `clike::sql_php_ruby` |
| markdown, md | Markdown | mdx | `markup::markdown` |
| diff, patch | Diff | udiff | `misc::diff_regex` |
| dockerfile | Dockerfile | docker, containerfile | `clike::config_languages` |
| makefile | Makefile | make, mk, mak | `clike::config_languages` |
| php, ruby/rb | Php, Ruby | | `clike::sql_php_ruby` |
| swift, lua, dart, r | Swift, Lua, Dart, R | | `clike::swift_lua_dart_r` |
| scala, perl, haskell/hs, elixir/ex | Scala, Perl, Haskell, Elixir | sc, pl, pm, exs | `clike::scala_perl_haskell_elixir` |
| nginx, graphql/gql, protobuf/proto | Nginx, GraphQl, Protobuf | nginxconf | `clike::config_languages` |
| regex | Regex | regexp, re | `misc::diff_regex` |

Unmapped R-CODE-01 names: **none**. Languages outside R-CODE-01 (e.g. zig, nim, ocaml, fortran): **deferred (FUT-01)**, rendered as plain text. No new lexer was written.

## 4. Acceptance criteria
| AC | Status | Evidence |
|---|---|---|
| AC-1 | pass | §0 |
| AC-2 | pass | §3b |
| AC-3 | pass | CI run 38977144180 green; PR #4 merged; tag `phase-003b` |
| AC-4 | pass | goldens 44/44 OK on CI; local images show pixels; block placeholders follow R-IMG-06; math/Mermaid/SVG remain placeholders as allowed |
| AC-5 | pass | 6/6 driver scripts on CI (anchor, navigation + scroll restore, blocked .exe, http Ask, Ctrl+click → 2nd window, Tab order); "popover" state is the in-window dialog until §4.2 |
| AC-6 | **partial** | ADR-0015 has the ACG matrix with results; DIB/DDB numbers missing |
| AC-7 | **partial** | table below; M1-with-images, T-IMG and T7-after-components missing |
| AC-8 | pass | this report |

## 5. Measurements
CI run 38976168917 (branch head before the last test-only commit), `windows-latest`, client 1000×700, medians of 5 runs.
| ID | Result | Budget | Δ vs 003 |
|---|---|---|---|
| M0 PWS / commit | 2.07 MiB (2,170,880 B) / 2.72 MiB | ≤ 4.8 MiB | −0.01 |
| M1 PWS / commit (`typical.md`, no images) | 2.57 MiB (2,695,168 B) / 3.59 MiB | ≤ 8.0 MiB | −0.02 |
| M2 PWS / commit | 5.38 MiB (5,636,096 B) / 6.83 MiB | ≤ 16 MiB | −0.08 |
| M6 PWS / commit | 2.55 / 3.56 MiB | ≤ M1 + 1 / + 1.5 MiB | = |
| T2 / T1 / T1L first paint | 81 / 87 / 168 ms | 100 / 150 / 300 | 0 / +7 / +27 |
| T4 typical warm median p99 (runs) | 14.24 ms (14.13 / 14.24 / 18.56) | ≤ 16 ms | **+7.9 ms** |
| T4 typical cold p99 | 13.96 ms | report | +8.8 |
| T4 large warm median p99 / cold | 5.58 / 4.73 ms | ≤ 16 ms | +1.2 |
| T7 idle CPU 30 s (empty, `typical.md`) | 0 µs / 0 µs | ≤ 1 ms | = |
| T-DLG | 506 ms | ≤ 400 soft | −28 (still over) |
| B-SIZE yamdv.exe | 1.89 MiB (1,983,488 B) | ≤ 4.0 MiB | +0.07 MiB |
| ACG matrix | PNG, JPEG, GIF, BMP, TIFF yes; ICO, WebP no | — | new |
| M1 with decoded images, image cache bytes | NOT MEASURED | | |
| T-IMG | NOT MEASURED | ≤ 100 ms soft | |
| T7 after tooltip/popover/banner | N/A (components not built) | | |
| T7 after an image decode | NOT MEASURED | | |
| Goldens | 44 files, 3,394,856 B (P-28 ≤ 12 MB); 0 added this run | | −20.7 MB |

T4 typical regressed (still under the 16 ms gate); the dev build loads `windowscodecs.dll` and the scroll path now blits decoded images. Investigate in Phase 004 before more content is added.

## 6. Decisions (ADRs)
- ADR-0015 image decoding under ACG and bitmap storage (heap PBGRA in the banded DIB; DIB/DDB comparison open).
- Driver design (one line): handlers are called directly on the UI thread, `ShellExecute`/Explorer are counted instead of launched (`sys::set_launch_dry_run`), exit code 0/3 — no ADR needed.

## 7. Deviations from the spec
- Ctrl+click new window ignores the `#fragment` of the target (opens at top).
- Tab order covers links only; code-toolbar buttons and fold toggles are not yet in the focus order.
- ICO/WebP decode fails under ACG on the runner (reported, not gated).

## 8. Conflicts, spec defects, questions for the reviewer
1. Can §4.2 (tooltip/popover/banner), inline objects and table laziness move to a 003c or into Phase 004's chrome work? They are the remaining 003 items.
2. WebP: the runner may lack the codec; treat as "codec not installed" (R-IMG-01) rather than an ACG failure?

## 9. Risks discovered
- T4 typical warm p99 +7.9 ms (14.2 ms of 16 ms).
- The driver's first CI run caught a real script/behaviour interaction: `click-link` scrolls the link into view before clicking, so scroll restore must be asserted from the click position.

## 10. Dependencies
None added.

## 11. Repository state
Branch `phase/003b-completion` → `main` via PR #4 (merge commit); tag `phase-003b`. Code head green on CI run 38977144180 (all jobs; 6/6 driver scripts PASS).

## 12. Files added/changed (this session)
`crates/yamdv-core/src/devscript.rs` (new), `crates/yamdv-platform-win/src/window/driver.rs` (new), `window.rs`, `sys.rs`, `yamdv-layout/src/block.rs`, `yamdv-app/src/{cli,main}.rs`, `tests/driver/*`, `.github/workflows/ci.yml`, `docs/plans/003b_PLAN.md`, this report.

## 13. Manual QA checklist for the owner (Windows PC)
1. Open a document with local PNG/JPEG images: pixels appear, scrolling stays smooth.
2. A missing image and `![x](C:\a.png:ads)` show the R-IMG-06 cards ("Invalid image path" for the second).
3. Tab through links: focus ring visible, the page scrolls to follow, Enter opens.
4. Ctrl+click and middle-click a link to another `.md`: a second window opens.
5. `<kbd>Ctrl</kbd>` renders as a key cap.

## 14. Proposed next steps
Finish the remaining 003 items (§4.2 components, inline objects, toolbar fade, table laziness, DIB/DDB, M1-with-images, T-IMG, T7 after decode), then Phase 004. The v1 scope decision (FUT-01…29) will be applied in prompt 005.

## 15. Özet (Türkçe)
Faz 003b kısmen tamamlandı. Bitenler: görseller uçtan uca (WIC çözücü, işçi iş parçacığı, önbellek, yer tutucular), CI'da ACG matrisi, pencere içi test sürücüsü ve 6 betiğin hepsinin CI'da geçmesi (AC-5), Tab ile bağlantılar arasında gezinme ve odak halkası, Ctrl+tık ve orta tık ile yeni pencere, `<kbd>` tuş görünümü. F-16…F-19 bitti: F-19 için yeni lexer yazılmadı, R-CODE-01'deki her ad mevcut bir lexer'a eşleniyor, dışındakiler "deferred (FUT-01)". Eksik kalanlar: ipucu, popover ve banner bileşenleri; satır içi nesneler; araç çubuğu geçişi; tablo satır tembelliği; DIB/DDB ölçümü; görselli M1, T-IMG ve çözme sonrası T7. Kaydırmada p99 14,2 ms'ye çıktı (sınır 16 ms), bu izlenmeli.
