# 013_V2C_TRANSIENT_UI_VIEWERS — YAMDV Phase V2-C: CI back, v1.1.0, tooltips/popovers/banners, fullscreen viewers, folding

> **Prompt 013.** Phase **V2-C** (`docs/ROADMAP_V2.md`).
> - Branch `phase/v2c-viewers`, tag `phase-v2c`
> - Report `docs/reports/v2c_REPORT.md` (≤ 80 lines)
> - Store this file as received in `docs/prompts/`
>
> **Precedence:** `docs/PROCESS.md` (A-1…A-9) > this prompt > `docs/UI_SPEC.md` / `docs/SPEC.md` > the rest of `docs/*`.
> **Order:** §1 → §2 → §3 → §4. **Resumable:** if the quota runs low, follow §6.

---

## 1. Review of V2-B — accepted
- **CI:** run 39863513532 is green. All 32 driver scripts and every gate pass: M0 2.36 MiB, M1 4.84 MiB, T7 0.
- **Cancel fix accepted:** "Microsoft Print to PDF" writes the file after the job is cancelled, and this is now handled. Keep the cancel check in `export-progress`.
- **Repository:** the owner moved it to **github.com/abuzer1495/YAMDV**, where Actions minutes are available again. A-9 below ends A-8.

### 1.1 Owner notes (only if attached)
The owner may paste notes from his manual checks (`OWNER_TEST.md`) below this prompt.
- Each note becomes a fix item, numbered F-23, F-24, …
- Fix them first, on the branch `fix/v1.1.0-owner`: one CI run, then merge.
- They count toward no limits of this phase.
- If no notes are attached, skip this step.

---

## 2. Setup (by script; keep it cheap)
1. **A-9.** Add A-9 to `docs/PROCESS.md` through the amendment mechanism:
```text
A-9 CI is back (owner decision, 2026-10-23). The repository moved to https://github.com/abuzer1495/YAMDV, where GitHub Actions minutes are available. A-8 ends and A-7's CI rules apply again; where they differ from A-8, A-9 wins.
1. CI gates as in A-7: M0, M1, T4 typical warm and T7 (two-attempt retry rule). M0/M1 may grow by at most 0.3 MiB per phase. At most 3 new driver scripts and 4 new goldens per phase.
2. Minute and token economy: one full CI run at the end of a phase plus at most one fix push. WIP pushes use [skip ci].
3. Kept from A-8:
   - run-tests.cmd and the test-pack builder stay working, with no new features; build a test pack only when the owner asks;
   - new pure logic goes into platform-independent crates with unit tests.
4. Version tags are set when CI is green on main. The tag job creates a draft release; the owner tests the draft by hand and publishes it himself. Problems he reports go first in the next prompt.
5. Every PR run uploads the release exe as the artifact yamdv-exe (retention 7 days), so the owner can try a phase before its release.
6. Every report ends with an owner check list (≤ 8 steps, EN + TR) covering what CI cannot judge.
7. Security rules are unchanged:
   - no secrets in the repository;
   - workflows keep permissions: contents: read, and only the tag/release job may write;
   - pull_request, never pull_request_target;
   - releases stay drafts.
```
2. **New remote.**
   - Run `git remote set-url origin https://github.com/abuzer1495/YAMDV.git` and check that `gh repo view` works.
   - By script, replace `snipeTR/YAMDV` with `abuzer1495/YAMDV` in `README.md`, `docs/*` and `.github/*`. Leave `docs/prompts/` and `docs/reports/` unchanged; they are historical.
3. **Roadmap rules.** In `docs/ROADMAP_V2.md`, replace these two lines exactly:
   - `- Since V2-B there is no CI. The owner runs the Windows tests from a test pack (PROCESS A-8).` → `- From V2-C on, CI runs again (PROCESS A-9). The owner checks each release by hand.`
   - `- Version tags wait for the owner's OK.` → `- Version tags are set when CI is green; releases stay drafts until the owner publishes them.`
4. **Release v1.1.0.**
   - **When to tag:** after §1.1 (if any) is merged and CI is green on `main`, tag `v1.1.0`. The tag job creates the **draft** release with the zip and its SHA-256.
   - **Version check script:** if `.github/scripts/v1d-checks.ps1` still expects `1.0.0`, make it read the version from the workspace first.
   - **Old test pack:** delete the draft pre-release `test-v2b` once the `v1.1.0` draft exists.
5. **Spec patches** (`docs/spec-patches/v2c.txt`, applied with `xtask spec-patch`):

**P-40** — `docs/SPEC.md`, R-MMD-02. OLD:
```text
Rendered in a card (Appendix B §B.6.12), scaled down to fit the column (never up);
```
NEW:
```text
Rendered in a card (Appendix B §B.6.12), scaled down to fit the column (never up), but never below the scale at which diagram text would be smaller than 10 px at 100 % zoom; a wider diagram scrolls horizontally inside its card;
```

**P-41** — `docs/UI_SPEC.md`, B.6.12. OLD:
```text
diagram centered and scaled to fit (never upscaled);
```
NEW:
```text
diagram centered and scaled to fit (never upscaled), not below the minimum scale of R-MMD-02; a wider diagram scrolls horizontally inside the card (thin overlay scrollbar on hover, `Shift+Wheel`, like code blocks);
```

**P-42** — `docs/UI_SPEC.md`, B.10.5. OLD:
```text
Used for: file changed on disk (with unsaved edits), file deleted, read-only, decoding fallback notice, recovery available.
```
NEW:
```text
Used for: file changed on disk (with unsaved edits), file deleted, read-only, decoding fallback notice, recovery available, Mark-of-the-Web (S-LINK-04), running elevated (S-PROC-06), export failure (R-EXP-07). At most 2 banners show at once (newest on top); the content moves down by their height and keeps its scroll anchor.
```
6. **Cleanup.** Remove the unused `Font` dialog-host kind (dialog_host and ipc).

---

## 3. V2-C tasks
Specifications:
- `docs/UI_SPEC.md` §B.6.12, §B.10.3–B.10.6, §B.11 and §B.16
- `docs/SPEC.md` R-READ-06, R-READ-08, R-IMG-05, R-MMD-02, R-REN-03, R-REN-10, R-MD-18
- `docs/SECURITY.md` S-UI-03, S-LINK-04, S-PROC-06

Issues: FUT-40, FUT-36, FUT-06, FUT-34, FUT-41, BUG-08.

1. **Tooltip component** (B.10.6; FUT-40)
   - **Look:** 500 ms delay, 12 px text, padding 4 × 8, radius 4, `tooltip_bg` / `tooltip_text`, max width 360, wrapped.
   - **Window:** one layered window, created on first use and reused.
     - It hides on mouse move-out, scroll, a key press or deactivation.
     - It also shows after 500 ms of keyboard focus on a link or button.
     - No timers run while nothing is pending.
   - **Uses:**
     - link titles (R-MD-18)
     - `<abbr title>` and image `title`
     - footnote references: a preview of the footnote text as plain text, up to about 300 characters (R-REN-10)
     - the word-count badge details (B.10.3)
     - the title-bar buttons (Minimize, Maximize/Restore, Close, Settings, More)
     - the fullscreen-viewer toolbar buttons
   - **Text:** EN/TR.
   - **Link status:** also add the B.10.4 link status tooltip if it is not there yet.
2. **Anchored popover component** (B.10.6; FUT-40)
   - **Base:** generalize the V2-B color-picker popover.
     - Size and look: width 360, radius 8, `menu_bg`, `shadow`.
     - Anchored to its target, and it flips to stay on the monitor.
     - Focus stays inside it; `Esc` or an outside click closes it.
   - **First new user:** the external-link *Ask*, which replaces the in-window confirm for external links.
     - It shows the URL in the code font (12 px, wrapped).
     - It keeps every existing link-policy warning (IDN, credentials, …).
     - Buttons: [Open] · [Copy link] · [Cancel].
     - Mark-of-the-Web still forces *Ask* (S-LINK-04).
   - **Later:** the link editor and the language picker come with the F2 phases.
3. **Banner component** (B.10.5 + P-42; S-UI-03)
   - **Look:** per B.10.5, at the top of the document area under the title bar: icon, text, right-aligned text buttons, × when dismissible.
   - **Move these existing notices into banners**, keeping their actions:
     - the file changed on disk while there are unsaved edits (the existing conflict actions)
     - the file was deleted
     - the file is read-only
     - decoding fallback
     - Mark-of-the-Web ("This file came from the internet — links always ask")
     - running elevated ("Running as administrator is not recommended")
     - export failure (BUG-11 used the in-window message until now)
   - **Not here:** recovery comes with V2-E.
4. **Fullscreen viewers** (B.11; R-READ-08, R-IMG-05, R-MMD-02; FUT-36, FUT-06)
   - **Badges:**
     - A ⛶ badge appears on hover over images and Mermaid cards.
     - Tables get the read-mode hover pill **[⛶] [CSV]** (B.6.8).
     - Double-clicking an image or diagram opens the viewer too.
   - **Overlay:** inside the window, with the `scrim` backdrop.
     - The content opens at "fit" (≤ 92 % of the window).
     - The toolbar pill has [−] [100 %] [+] [fit] [1:1] [copy] [×].
   - **Controls:**
     - the wheel zooms at the cursor, 10 %–800 %, ×1.1 per notch;
     - dragging pans;
     - a double-click toggles fit / 100 %;
     - `+`, `-`, `0`;
     - `Esc` closes.
   - **Image:**
     - The image is decoded only at the size needed (WIC scaler). Above that size, only the visible region is decoded, bounded by the viewport.
     - [copy] puts the image on the clipboard.
   - **Mermaid:**
     - The diagram is re-rendered at the zoom scale for the visible region, so it stays vector-sharp.
     - [copy] gives a PNG through the V1-C block-PNG API.
   - **Table:**
     - The table sits on a page-colored card with a 28 px header ("Table · R × C" + [Export CSV]). The header moves the card, and a resize grip sits at the bottom-right.
     - Columns keep their natural widths, with no wrapping, and the card has scrollbars.
     - [copy] copies the table as CSV text.
   - **Memory:** the viewer is built on demand and freed on close.
5. **Heading folding** (R-READ-06; FUT-34)
   - **Toggle:** hovering the left gutter of H1–H3 shows ▸ / ▾.
     - A click folds or unfolds the section, up to the next heading of the same or a higher level.
     - `Alt+click` folds or unfolds every heading of that level.
     - It works in read mode and in F2, and with the keyboard: `Tab` reaches the toggle and `Enter` activates it.
   - **Rendering:**
     - Folded blocks are skipped in layout (top-level block index), so they cost nothing.
     - The heading keeps its screen position.
     - The fold animation takes 120 ms; it is instant with reduced motion.
   - **Find** auto-expands a folded section when a match is inside it (R-READ-05).
   - **State:** session-only; it is cleared when the document is closed.
   - **Exports and print** always include folded content.
6. **Inline objects for sup/sub/footnote refs** (R-REN-03; FUT-41)
   - **What:** superscript, subscript and footnote reference marks become inline objects with a real baseline shift, like inline math (`IDWriteInlineObject`).
   - **Typography:** the font's `sups` / `subs` features are used when it has them; otherwise the text is synthesized at 75 % size with a shift of about 0.33 em.
   - **Lines:** the line height does not change unless a shift would collide.
   - **Footnote refs:** they stay clickable (jump to the note) and show the tooltip from item 1.
   - **Selection and copy** work on the text as before.
7. **BUG-08 — wide diagrams** (P-40/P-41)
   - **Scale:** diagrams are never scaled below the minimum, which keeps diagram text ≥ 10 px at 100 % zoom.
   - **Scrolling:** a wider card scrolls horizontally like a code block.
   - **Fixture:** the 9-node LR flowchart.

**Out of scope:** the Files tab, the floating sidebar, the heading palette and the mind map (all V2-D); everything later.

**Tests (A-7/A-9):**
- **Driver scripts (3):**
  - `tooltips-banners`:
    - hovering a titled link shows its tooltip after ≥ 500 ms;
    - hovering a footnote reference shows its preview;
    - open a fixture that has a `Zone.Identifier` stream with ZoneId=3 (the script writes it): the MotW banner shows;
    - clicking an external link opens the Ask popover with the URL, and `Esc` closes it;
    - a decoding-fallback fixture shows its banner.
  - `viewers`:
    - double-clicking an image opens the viewer at fit;
    - wheel zoom keeps the point under the cursor fixed;
    - `0` resets the zoom, and `Esc` closes the viewer;
    - the table ⛶ shows "Table · R × C";
    - Mermaid ⛶ opens the diagram viewer;
    - after closing, PWS is back within +0.2 MiB, and T7 is 0 with an idle viewer open.
  - `folding`:
    - clicking the H2 toggle hides the section and makes the document shorter;
    - a find hit inside the fold expands it;
    - `Alt+click` folds every H2;
    - reopening the document resets the state.
- **Goldens (4):**
  - a footnote tooltip and sup/sub text (light)
  - the MotW banner + the Ask popover (dark)
  - the image viewer with its toolbar (light)
  - the wide flowchart at minimum scale, scrollable, with a folded H2 above it (light)
- **Unit tests (pure):**
  - fold range computation
  - viewer zoom math: fixed point at the cursor, clamping, fit
  - banner stacking
  - sup/sub metrics
  - BUG-08 minimum scale

**Gates (A-7/A-9):**
- M0 and M1 at most +0.3 MiB vs V2-B (2.36 / 4.84 MiB).
- T4 typical warm.
- T7, also with a viewer open and idle.
- Report the exe-size delta; viewer-open PWS is report-only.

---

## 4. Merge and report
- **PR and CI:** one CI run at the end, plus at most one fix push.
- **Issues:** the PR body closes FUT-40, FUT-36, FUT-06, FUT-34, FUT-41 and BUG-08 with "Closes #n".
- **After CI is green:** merge, then tag `phase-v2c`.
- **Report (≤ 80 lines):**
  - the AC table
  - the gates
  - the exe delta
  - deviations
  - the `v1.1.0` draft-release link and its SHA-256
  - the owner check list (≤ 8 steps, EN + TR), which must cover:
    - the `v1.1.0` draft: export colors with `colors.md` and the progress card;
    - tooltips;
    - the MotW banner and the Ask popover;
    - the three viewers;
    - folding;
    - a wide diagram.

## 5. Acceptance criteria
| ID | Criterion |
|---|---|
| AC-1 | §1.1 is done (if notes were attached); A-9, the new remote, the roadmap lines, P-40…P-42 (`spec-check` green) and the Font-kind cleanup are done; `v1.1.0` is tagged and its draft release has the zip + SHA-256; `test-v2b` is deleted. |
| AC-2 | §3 items 1–7 work per the listed specs; TRACEABILITY is updated by script. |
| AC-3 | The 3 driver scripts pass on CI; the 4 goldens are committed and self-reviewed; the unit tests pass; the PR artifact `yamdv-exe` exists. |
| AC-4 | The gates are green within the growth limits; the PR is merged with the issues closed; `phase-v2c` is tagged; the report follows §4. |

## 6. Stop rule (quota)
If the Grok quota or the Actions minutes run low at any point:
1. commit and push with `[skip ci]`;
2. write `docs/reports/v2c_REPORT.md` with a done / partial / not-started table for §1–§4 and **"Next action: …"**;
3. stop.

The owner will give this same prompt again.

**END OF PROMPT 013.** Start with §1.1 (or §2 if no notes are attached).
