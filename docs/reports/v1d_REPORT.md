# YAMDV — Phase V1-D Report: Export, print, packaging, v1.0.0 RC1

## 0. Status
- Prompt 008 stored as `docs/prompts/008_V1D_EXPORT_PACKAGING_RELEASE.md`, SHA-256 `9483e4591ef463021bbe57c339aea144f1ad34f24f55995d6808d0fa1dab9662`. Rules: PROCESS A-4, owner addenda **A-5** (public repo, reduced testing, report ≤ 80 lines) and **A-6** (no CI until 2026-11-01), both recorded in `docs/PROCESS.md` via `PROCESS_AMENDMENTS.md` (spec-check green).
- Branch `phase/v1d-release` @ d912c88 (code) + this report; PR #49 **open, not merged**; tags `phase-v1d` / `v1.0.0` **not created** (A-6 §5). gh account `snipeTR`.
- **RC1 (draft, never published):** `YAMDV-1.0.0-rc1-win-x64.zip` (yamdv.exe, README.md, CHANGELOG.md, FUTURE.md), SHA-256 `f14fb8d73c7bb6bc420ba36806458a1d8c7530b48ef99f3a278349929ee02057`; `yamdv.exe` SHA-256 `70b990f14252bdc4033ff6bf60dd3182effc24130f2557706580c51a07f52eed`. Built locally with cargo-xwin from d912c88; draft release `v1.0.0-rc1` (no tag until published). Artifacts live in `target/rc1/` (not in git).
- Last CI evidence: PR run 39043473274 on 03a4775 (before A-5/A-6). Everything green **except** the V1-D check `--export-pdf`, which crashed with 0xC000041D (`T10` too). The fix (ADR-0018) is **unverified on Windows**.

## 1. Acceptance criteria
| AC | Status | Evidence |
|---|---|---|
| AC-1 | **partial (A-6)** | F-22 superseded by A-5: gates are only M0, M1 and T4 typical warm, each with one retry; the rest, B-SIZE included, is report-only. Draft RC1 with zip + SHA-256 exists. No `main` merge and no tags (A-6 §5) |
| AC-2 | **partial** | HTML export, CSV, headless CLI, install/uninstall: run 39043473274 checks green (exit codes 0/1/2, no `<script`, CSP, only `data:` images; install twice → exe, shortcut, Uninstall key, ProgID; uninstall removes all). XSS suite has the exported-HTML context (`s_test_02_xss_exported_html`, local pass). **PDF/print: crash on CI**, fix NOT RUN (A-6) |
| AC-3 | **pass** | 41 FUT issues #8–#48 = 41 rows in `FUTURE.md`, label `future`, milestone `post-v1`, no duplicate titles. README is complete |
| AC-4 | **pass** | This report (≤ 80 lines) with the RC1 QA list (§5) |

## 2. Local gates (A-6 §2), on d912c88
| Gate | Result |
|---|---|
| `cargo fmt --check`, clippy native `-D warnings`, clippy cargo-xwin x86_64-pc-windows-msvc `--all-targets` | pass |
| `cargo test --workspace` (platform-independent crates; pagination, CSV and XSS included) | pass |
| `cargo deny check`, `cargo xtask spec-check` | pass |
| `check-imports` on the cross-built `yamdv.exe` | OK (19 DLLs, no forbidden imports, hardening flags) |
| `size` (B-SIZE) | 2,783,744 B = 2.65 MiB ≤ 4.0 MiB |
| Windows unit tests (incl. new `print_job_file_round_trips`), goldens, 25 driver scripts, v1d-checks, measurements | **NOT RUN (A-6)** |

## 3. Measurements (A-5 table)
| ID | Result | Budget | Source |
|---|---|---|---|
| M0 PWS / commit | 2.28 / 3.00 MiB | ≤ 4.8 MiB | run 39043473274 (03a4775) |
| M1 PWS / commit (with images) | 4.65 / 5.97 MiB | ≤ 8.0 MiB | same |
| T1 first paint `typical.md` | 118 ms | ≤ 150 ms | same |
| T4 typical warm median p99 / cold | 14.83 / 15.00 ms | ≤ 16 ms | same |
| B-SIZE `yamdv.exe` | 2.65 MiB (RC1, local) | ≤ 4.0 MiB | local cargo-xwin |
RC1 itself has NOT been measured (A-6). The print-worker change does not touch the paths these numbers come from.

## 4. Deviations and what is left
- **PDF/print crash → ADR-0018.** On CI, `--export-pdf` died with 0xC000041D, an exception inside a kernel→user callback, typical of an in-process printer driver under StrictHandleCheck/ACG. Fix: `--export-pdf` and the new internal `--print-job <file>` use `hardening::apply_for_print()` (no StrictHandleCheck, no ACG). The GUI keeps the full hardening and runs Print… / Export PDF… in that worker through a temp job file. The root cause is a hypothesis until it is checked on a real PC (QA steps 11–12).
- `export-csv.yds` is a third new driver script, written before A-5 (limit 2). It was kept, not deleted (A-5 2a).
- The welcome golden was renamed `welcome-install-light.png` (install button). Commit it from the next CI artifact.
- The release CI job no longer refuses a public repo (A-5); it still creates drafts only.
- **Left:** owner QA of RC1 → merge PR #49; on 2026-11-01 one full CI run (A-5 §4) → `phase-v1d` + `v1.0.0` tags → CI draft release `v1.0.0`; commit the welcome golden; T10 (PDF of `large-1mb.md`).

## 5. RC1 manual QA (Windows 10/11 PC)
1. Check `YAMDV-1.0.0-rc1-win-x64.zip` with `Get-FileHash -Algorithm SHA256`; it must match §0. Unzip and run `yamdv.exe`. The welcome page shows **[Install YAMDV for this user]**. Click it: the installed copy starts and shows "Set as default app".
2. Install check: Start menu ▸ YAMDV exists; Settings ▸ Apps lists YAMDV 1.0.0. Run `--install` again: nothing breaks.
3. Set as default app, then double-click a `.md` in Explorer: it opens in YAMDV. Double-click a second `.md`: same instance, new window.
4. Read a rich document: local images, tables, highlighted code, `$…$`/`$$…$$` math and Mermaid flowchart/sequence render. Missing images show placeholder cards.
5. Sidebar (TOC/History/Favorites), theme and dark-mode switch, Ctrl+F find, Ctrl+wheel zoom.
6. Live reload: edit the open file in Notepad and save. YAMDV updates and keeps the scroll position.
7. F3 split edit and F2 in-place edit: type, Ctrl+Z / Ctrl+Y, autosave. Change the file outside YAMDV during an edit: the conflict dialog appears.
8. Turkish input: type `ğüşıöç ĞÜŞİÖÇ` and AltGr characters (`@ € { }`) in F2 and F3. Save, reopen: the bytes, encoding and line endings are unchanged.
9. Export ▸ Export HTML…: open the `.html` in a browser. It looks like the app (TOC, code, math/diagrams as images), and View Source shows no `<script>`.
10. Right-click a table ▸ Copy table as CSV / Export table as CSV…: Excel opens it correctly. A cell `=1+1` stays text.
11. **Export ▸ Export PDF… (Ctrl+Shift+E):** a PDF is created with no driver prompt, page numbers "n / N", and nothing split mid-row. From a terminal, `yamdv --export-pdf doc.md -o out.pdf; $LASTEXITCODE` gives 0. **Crash or failure here = RC1 blocker, report it.**
12. **Print… (Ctrl+P):** pick a real printer or "Microsoft Print to PDF", choose a page range: it prints and YAMDV does not crash.
13. Help/About shows 1.0.0. With no network (airplane mode), everything above works; `Resource Monitor ▸ Network` shows no yamdv.exe traffic.
14. Uninstall from Settings ▸ Apps and choose whether to keep settings. The install folder, Start-menu shortcut and registry entries are gone; `.md` files reopen with the previous app.

## 6. Özet (Türkçe)
V1-D kod olarak tamamlandı: HTML dışa aktarma, CSV, yazdırma/PDF, kurulum/kaldırma, sürüm 1.0.0, README, CHANGELOG ve 41 FUT issue'su hazır. Son CI çalışmasında her şey yeşildi, yalnızca `--export-pdf` çöktü. Muhtemel neden, yazıcı sürücüsünün sertleştirme (StrictHandleCheck/ACG) altında çalışması. Çözüm olarak yazdırma artık ayrı bir işçi süreçte, gevşetilmiş sertleştirmeyle çalışıyor (ADR-0018), ama bu Windows'ta henüz denenmedi. A-6 gereği CI yok: yerel kapıların hepsi yeşil. Taslak RC1 yüklendi. PR #49 birleştirilmedi, etiket atılmadı. RC1'i §5'teki adımlarla dene, özellikle 11. ve 12. adımları (PDF ve yazdırma).
