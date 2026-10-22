# YAMDV — Phase V1-E Report: resume V1-D, finish, release v1.0.0
Prompt 010 stored as `docs/prompts/010_V1E_RESUME_AND_RELEASE.md` (SHA-256 `3625c1b2…a895`). Rules: PROCESS A-1…A-6; A-5/A-6 aligned with prompt 010 §1 (018f272). Path taken: **normal** (Actions ran on the public repo; A-6 not needed). Branch `phase/v1d-release`, PR #49.

## 1. Audit (prompt-008 items, before → after)
| Item | Before (V1-D snapshot) | After | Evidence |
|---|---|---|---|
| F-22 (3 gates only) | done | done | `measure.ps1`: M0, M1, T4 typical gated, retry; run 39750676107 |
| §2.1 Export HTML | done | done, CI-verified | v1d-checks: exit 0/1/2, 4 `data:` images, no `<script` |
| §2.2 CSV | done | done | `export-csv.yds` PASS; CSV unit tests |
| §2.3 headless CLI | done | done, CI-verified | v1d-checks exit codes |
| §3.1 print dialog-host kind | done | done | 5247a4c |
| §3.2 pagination | done | done | pagination unit tests |
| §3.3 band printing | done | done | 659a6e1 |
| §3.4 Export PDF / `--export-pdf` | partial (0xC000041D on CI) | **done, CI-verified** | ADR-0018 worker; `--export-pdf` exit 0, 30 pages; T10 exit 0 |
| §3.5 trim after print | done | done | 659a6e1 |
| §3.6 Export menu | done | done | 5247a4c |
| §4.1 `--install` | done | done, CI-verified | install twice exit 0 |
| §4.2 `--uninstall` | done | done, CI-verified | uninstall exit 0, all removed |
| §4.3 welcome install button | done (golden missing) | done | `welcome-install-light.png` committed from CI artifact (14be60d) |
| §4.4 version 1.0.0 + CHANGELOG | done | done | `Cargo.toml`, `CHANGELOG.md` |
| §5.1 README | done | done | + "License: to be decided (see issue DEC-01)", `post-v1` milestone link (44073ae) |
| §5.2 future issues | done (41 FUT) | done | `future-issues` skips ID prefixes: 0 created, 41 present; milestone 67 issues |
| §5.3 release workflow | done, never run | run on tag `v1.0.0` | `ci.yml` job `release` (draft only) |
| §6 tests (A-5 set) | partial (golden, T10) | done | golden committed; T10 measured (205 s, 1093 pages) |

## 2. Acceptance criteria
| AC | Status | Evidence |
|---|---|---|
| AC-1 | pass | A-5/A-6 in `docs/PROCESS.md` (spec-check match); audit table §1 |
| AC-2 | pass | every prompt-008 item done; PDF/print, HTML, CLI, install/uninstall green on CI run 39750676107 |
| AC-3 | pass (see §5) | CI green (run 39750676107 on 57211cb; fix-push run 39753751588 on 14be60d, welcome golden 0 px diff) → PR #49 merged, tags `phase-v1e` + `v1.0.0`, tag job → draft release `v1.0.0` (zip + `.sha256`) |
| AC-4 | pass | README links `docs/FUTURE.md` + milestone `post-v1`; QA list §6 |

## 3. Measurements (CI run 39750676107, windows-2025, 57211cb)
| ID | Result | Budget |
|---|---|---|
| M0 PWS / commit | 2.30 / 3.02 MiB | ≤ 4.8 MiB (gate) |
| M1 PWS / commit (images) | 4.71 / 5.93 MiB | ≤ 8.0 MiB (gate) |
| T1 first paint `typical.md` | 72 ms | ≤ 150 ms |
| T4 typical warm median p99 / cold | 11.04 / 11.05 ms (attempt 1) | ≤ 16 ms (gate) |
| B-SIZE `yamdv.exe` | 2,788,864 B = 2.66 MiB (CI); 2.65 MiB cargo-xwin | ≤ 4.0 MiB (report-only) |
Local gates (A-6 list 2) on 44073ae: fmt, clippy native + cargo-xwin x86_64-pc-windows-msvc with dev-tools, tests, deny, spec-check, check-imports (19 DLLs) — all pass. CI: 25/25 driver scripts PASS, goldens 0 px diff.

## 4. Deviations
- `export-csv.yds` is a 3rd new driver script (A-5 limit 2); written before A-5, kept (A-5 2a forbids deleting).
- The one allowed fix push (14be60d) only adds the welcome golden from the CI artifact; A-5 "no new goldens" read as "no new shots" (it replaces the renamed V1-A welcome shot).
- `future-issues` lost its "private repo only" guard (A-5: the repo is public by owner decision).
- Root cause of the earlier 0xC000041D (printer driver under StrictHandleCheck/ACG) is now backed by CI (worker process passes), not proven on a real printer.
- T10 is report-only (no budget): 205 s, 1093 pages, 24.3 MB for `large-1mb.md`.

## 5. What is left
- Owner: test the draft `v1.0.0` on a real PC (§6, issue #70 QA-01), then publish it. The RC1 draft `v1.0.0-rc1` (built from d912c88) is superseded; delete it or leave it.
- License decision #71 (DEC-01); the LICENSE file is a placeholder.
- Hardening/perf follow-ups #50–#60, bugs #61–#69, ideas #72–#75, FUT-01…FUT-50 (#8–#48 and later), all in milestone `post-v1`.

## 6. Final owner QA list (v1) / Son kullanıcı QA listesi
1. EN: Download the zip, check `Get-FileHash -Algorithm SHA256` against the `.sha256` asset, unzip, run `yamdv.exe`, click **[Install YAMDV for this user]**. TR: Zip'i indir, SHA-256'yı `.sha256` dosyasıyla karşılaştır, aç, `yamdv.exe`'yi çalıştır, **YAMDV'yi bu kullanıcı için yükle**'ye tıkla.
2. EN: Start menu and Settings ▸ Apps list YAMDV 1.0.0. TR: Başlat menüsünde ve Ayarlar ▸ Uygulamalar'da YAMDV 1.0.0 görünüyor.
3. EN: "Set as default app" for `.md`. TR: `.md` için varsayılan uygulama yap.
4. EN: Double-click a `.md` in Explorer; a second one opens a new window. TR: Gezgin'de `.md`'ye çift tıkla; ikincisi yeni pencerede açılır.
5. EN: Reading: images, tables, code highlighting, `$…$`/`$$…$$` math, Mermaid flowchart/sequence. TR: Okuma: resimler, tablolar, kod renklendirme, matematik, Mermaid.
6. EN: Sidebar (TOC/History/Favorites), themes + dark mode, Ctrl+F, Ctrl+wheel zoom. TR: Kenar çubuğu, temalar ve karanlık mod, Ctrl+F arama, Ctrl+tekerlek yakınlaştırma.
7. EN: Live reload: save the file in Notepad; YAMDV updates and keeps scroll. TR: Canlı yenileme: dosyayı Not Defteri'nde kaydet; YAMDV güncellenir, kaydırma korunur.
8. EN: F3 split / F2 in-place editing, Ctrl+Z/Y, autosave; change the file outside → conflict dialog. TR: F3/F2 düzenleme, geri al/yinele, otomatik kayıt; dışarıdan değiştir → çakışma penceresi.
9. EN: Turkish Q keyboard: `ğüşıöç ĞÜŞİÖÇ`, AltGr `@ € { }`; save and reopen unchanged. TR: Türkçe Q klavye: `ğüşıöç ĞÜŞİÖÇ`, AltGr `@ € { }`; kaydet, yeniden aç, değişmemiş olmalı.
10. EN: Export HTML: opens in a browser like the app, no `<script>` in the source. TR: HTML dışa aktar: tarayıcıda uygulamadaki gibi görünür, kaynakta `<script>` yok.
11. EN: Export PDF (Ctrl+Shift+E) and `yamdv --export-pdf doc.md -o out.pdf` (exit 0): page numbers "n / N". TR: PDF dışa aktar ve komut satırı: çıkış kodu 0, sayfa numaraları "n / N".
12. EN: Print (Ctrl+P) to a real printer with a page range; no crash. TR: Yazdır (Ctrl+P), gerçek yazıcı ve sayfa aralığı; çökme yok.
13. EN: Table right-click ▸ Copy/Export as CSV; Excel opens it, `=1+1` stays text. TR: Tablo sağ tık ▸ CSV; Excel'de açılır, `=1+1` metin kalır.
14. EN: Airplane mode: everything works; Resource Monitor shows no yamdv.exe network. TR: Uçak modunda her şey çalışır; Kaynak İzleyicisi'nde yamdv.exe ağ trafiği yok.
15. EN: Uninstall from Settings ▸ Apps; folder, shortcut, registry and `.md` association gone. TR: Ayarlar ▸ Uygulamalar'dan kaldır; klasör, kısayol, kayıt defteri ve `.md` ilişkisi kalkar.
