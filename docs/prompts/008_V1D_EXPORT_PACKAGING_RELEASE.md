# 008_V1D_EXPORT_PACKAGING_RELEASE — YAMDV Phase V1-D: Export, print, packaging, v1.0.0

> **Prompt 008.** Phase **V1-D**, the last feature phase of v1. Branch `phase/v1d-release`, tag `phase-v1d` and **`v1.0.0`**, report `docs/reports/v1d_REPORT.md` (A-4: ≤ 120 lines). Store this file as received in `docs/prompts/`.
> **Precedence:** `docs/PROCESS.md` (A-4) > this prompt > `docs/*`. Scope boundaries: FUT-21 (static HTML only) and FUT-39 (PDF through the print path). Build nothing from `docs/FUTURE.md`.

---

## 1. Review of V1-C — accepted
V1-C is **accepted**: native math and Mermaid work, both fuzz targets are clean, there are 23 driver scripts, and M3 stays below 3 MiB.

**One fix first:**

| ID | Fix |
|---|---|
| **F-22** | **Stop flaky reds on `main`.** Apply the F-21 rule (fail only when two consecutive attempts both exceed the budget) to **every** measurement gate: M0…M6, T1/T2, T4, T5, T7. The budgets themselves stay unchanged. `main` must be green at the `v1.0.0` tag. |

---

## 2. Export (R-EXP-02 static per FUT-21, R-EXP-04, R-EXP-05)

1. **Export HTML…** writes **one self-contained `.html` file**. Extend the existing CF_HTML writer in `yamdv-export`; do not start a second writer.
   - **Styles:** inline CSS generated from the current scheme's tokens, in the current mode.
   - **Content:**
     - a TOC list at the top
     - highlighted code as spans
     - callouts, tables, footnotes with back-links, task boxes (disabled checkboxes) and `<details>`
   - **Images** become `data:` URIs, but only for images that decoded successfully (S-RES-05).
   - **Math and Mermaid** become **2× PNG `data:` URIs**, using the V1-C block-PNG API.
     - Inline formulas: `vertical-align` from the math box's descent; `alt` = the LaTeX source.
     - Diagrams: `alt` = "Mermaid diagram".
   - **Security:**
     - **No script at all.**
     - CSP meta tag: `default-src 'none'; img-src data:; style-src 'unsafe-inline'; base-uri 'none'; form-action 'none'`.
     - External links get `rel="noopener noreferrer"`; every piece of text is escaped (S-EXP-01, S-EXP-02 without scripts).
   - **Save dialog:** the dialog-host `save` kind; the default name is `<doc stem>.html`.
2. **Tables → CSV** (R-EXP-04): right-clicking a table adds **Copy table as CSV** and **Export table as CSV…** to the menu. The file is UTF-8 with BOM, CRLF line endings and RFC 4180 quoting. The formula guard (S-EXP-04, `export.csv_formula_guard`, default on) applies.
3. **Headless CLI** (R-EXP-05):
   - `yamdv --export-html <in.md> [-o out.html] [--theme <scheme>] [--mode light|dark]`
   - `yamdv --export-pdf <in.md> [-o out.pdf]` — see §3, item 4.
   - Neither shows a window. Exit codes: 0 success, 1 error, 2 usage.

## 3. Print and PDF (R-EXP-03; PDF via FUT-39)

1. **Print… (`Ctrl+P`)**
   - The **dialog host** gains a `print` kind showing `PrintDlgExW`. It returns only serializable data: printer name, DEVMODE bytes, page range, copies. The main process creates its own printer DC.
2. **Pagination**
   - Lay out the document at the printable width: the page minus `export.pdf_margins_mm` (default 18 mm), using the printer DPI for the layout scale.
   - Page breaks fall between blocks. A block taller than the remaining space breaks at a line boundary. Table rows, code lines and diagrams never split; an element taller than a whole page is scaled down to fit.
   - With `export.pdf_page_numbers` on, the footer reads "n / N".
   - Paper size follows `export.pdf_page_size`: *auto* = Letter for en-US / en-CA, A4 otherwise.
3. **Rendering**
   - Each page is drawn with the existing band renderer and sent to the printer DC band by band (`StretchDIBits`). Never allocate a full-page bitmap.
   - Colors follow `export.print_colors` (default *light*: the scheme's light palette on white).
   - In v1, text in printed output is raster (FUT-39 records the native vector PDF writer).
4. **Export PDF… (`Ctrl+Shift+E`)**
   - Runs the same pipeline on the printer **"Microsoft Print to PDF"**, passing the output path in `DOCINFO.lpszOutput`, so no driver prompt appears. The path comes from the dialog-host `save` dialog; the default name is `<doc stem>.pdf`.
   - If that printer does not exist: in-window message "Microsoft Print to PDF is not installed — use Print… and choose a PDF printer."
   - `--export-pdf` uses the same path headless.
5. **Afterwards:** free every transient buffer and trim the working set **once** (D.4 rule 10).
6. **Menu:** **Export ▸** Export HTML… · Export PDF… `Ctrl+Shift+E` · Print… `Ctrl+P`.

## 4. Packaging (R-SYS-05, R-SYS-07)

1. **`yamdv --install`**
   - Copies the running exe to `%LOCALAPPDATA%\Programs\YAMDV\yamdv.exe`.
   - Registers per user (R-SYS-01) and creates a Start-menu shortcut (`IShellLinkW`).
   - Writes the HKCU `Uninstall\YAMDV` entry: DisplayName, DisplayIcon, DisplayVersion `1.0.0`, Publisher, EstimatedSize, InstallLocation, UninstallString `"<installed exe>" --uninstall`.
   - Starts the installed copy, which shows "Set as default app" on its welcome page.
   - Idempotent: running it again updates in place.
2. **`yamdv --uninstall`**
   - An in-window dialog asks whether to keep the settings.
   - Removes exactly the registry values it wrote (S-REG-02), the shortcut and the install folder.
   - The running exe deletes itself through a temp-copy helper that waits for the parent process to exit, then removes the folder.
3. **Welcome page:** when the exe runs from anywhere other than the install folder (and not in portable mode), show an **[Install YAMDV for this user]** button that runs `--install`.
4. **Version:** `1.0.0` in the workspace, the version resource and About. `CHANGELOG.md` gets a 1.0.0 entry summarizing v1.

## 5. Release

1. **README.md** (user-facing, in English, with a short Turkish section):
   - what YAMDV is, plus a screenshot taken from a golden
   - install / portable use / uninstall
   - first steps: F2, F3, F4, the context menu
   - a keyboard-shortcut summary
   - privacy ("never connects to the internet") and how to verify it
   - known limitations, linking `docs/FUTURE.md`
   - build from source
2. **Future-work issues:** `cargo xtask future-issues` reads `docs/FUTURE.md` and creates, idempotently and in the **private** repo:
   - the label `future`
   - the milestone `post-v1`
   - one issue per FUT row: title `FUT-xx: <feature>`; body = the "v1 instead" column + the spec IDs
   It skips titles that already exist. Run it once.
3. **Release build:** on the tag `v1.0.0`, CI builds `YAMDV-1.0.0-win-x64.zip` containing `yamdv.exe`, `README.md`, `LICENSE`, `CHANGELOG.md` and `FUTURE.md`, with its SHA-256 written next to it. CI creates a **draft** GitHub release `v1.0.0` in the private repo with the zip attached. Do not publish it — the owner decides.

---

## 6. Tests and measurements
**Driver scripts:**
- Export HTML: the file exists; it contains no `<script`; the CSP is present; images are `data:` URIs.
- CSV: the bytes match the expected output, including the formula guard.
- Headless `--export-pdf` on `typical.md` (skip with a clear note if the runner has no "Microsoft Print to PDF").
- `--install` then `--uninstall` against a temp `LOCALAPPDATA`/HKCU test hive (redirect `LOCALAPPDATA`; use the existing test ProgID approach for the registry), checking the keys and files before and after.

**Unit tests:** pagination (break rules, n/N), and the CSV quoting and guard.

**XSS suite:** add the exported HTML as a fourth context.

**Measurements:** the existing table only (A-4). Also report `T10 = --export-pdf` on `large-1mb.md` (time and page count) if the runner has the PDF printer.

---

## 7. Acceptance criteria
| ID | Criterion |
|---|---|
| AC-1 | F-22 done; `main` green; tags `phase-v1d` and `v1.0.0` exist; draft release with zip + SHA-256. |
| AC-2 | §2–§4 work; driver scripts pass; XSS suite passes for exported HTML. |
| AC-3 | FUT issues created (count = FUT rows in `FUTURE.md`); README complete. |
| AC-4 | Report ≤ 120 lines with the **final owner QA list (≤ 15 steps)** covering the whole v1: install, default app, double-click open, read (images/tables/code/math/Mermaid), sidebar/themes/find/zoom, live reload, F2/F3 editing + autosave + conflict, Turkish input, export HTML/PDF/print, CSV, uninstall. |

**END OF PROMPT 008.** Start with F-22 now.
