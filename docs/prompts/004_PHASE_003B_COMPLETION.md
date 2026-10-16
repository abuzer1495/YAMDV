# 004_PHASE_003B_COMPLETION — YAMDV Phase 003b: Rendering completion

> **Prompt 004.** You are executing **Phase 003b** (a completion phase inserted between 003 and 004; the roadmap numbering of later phases does not change).
> Branch `phase/003b-completion`, tag `phase-003b`, plan `docs/plans/003b_PLAN.md`, report `docs/reports/003b_REPORT.md`.
> Store this file as received as `docs/prompts/004_PHASE_003B_COMPLETION.md` and report its SHA-256.
> This prompt is deliberately short. Where it says "per prompt 003 §x", read that section in `docs/prompts/003_FULL_RENDERING.md`; it is not repeated here.

**Precedence:** `docs/PROCESS.md` (incl. the new amendment A-3) > fix items (§2) > spec patches (§3) > tasks (§4) > `docs/*`.

---

## 1. Review of Phase 003

Phase 003 is **accepted as partial**. The report was honest and precise; the parts that are done are of high quality:
- dialog host: M6 commit −7.2 MiB with no trim
- 66 XSS vectors and 4 fuzz targets, which found 2 real bugs
- M1 2.59 MiB and warm T4 6.4 ms

Phase 003 was too large for one pass. This phase therefore finishes it before the app shell starts. **Goal: Phase 003's AC-4, AC-5 and AC-9 pass.**

**Answers to report section 8:**
1. *Band cap:* keep the 32-row floor; P-26 corrects the wording.
2. *Image decoder:* yes, it is the first task here (§4.1).
3. *ADS paths:* these are not network paths. Give them a separate reason, *invalid path* (also device names and NUL), never touch the file system, and show the missing-style placeholder with the text "Invalid image path" (P-27).

**Reviewer findings:**
- The goldens added **24 MB** in one phase. At that rate the repository bloats quickly, and git history keeps every old blob. The policy is tightened in P-28 and the set is pruned now (F-17).

---

## 2. Fix items

| ID | Fix | Done when |
|---|---|---|
| **F-16** | Apply P-26…P-29 and make sure `spec-check` stays green. | spec-check green |
| **F-17** | Prune the committed goldens to the P-28 policy in this phase. Delete anything the policy does not need, never rewrite history, and report the bytes before and after. | golden set ≤ 12 MB |
| **F-18** | Add amendment **A-3 "Token economy"** to `docs/PROCESS.md` with the text in §5 (through the amendment mechanism `spec-check` already knows), and follow it from now on. | spec-check green; A-3 present |
| **F-19** | Add a language coverage table to the report: every language and alias of R-CODE-01 → the lexer that handles it, plus a test name. Missing languages are mapped to the closest lexer (e.g. `jsonc` → json, `vue` → html) or implemented if small. | table in report; no unmapped name |

---

## 3. Spec patches
Apply them with `cargo xtask spec-patch` from `docs/spec-patches/003b.txt`. Each OLD text and each INSERT AFTER anchor line must match exactly once in the current docs (original + P-01…P-25). Log them in `SPEC_CHANGES.md`.

**P-26** — `docs/PERFORMANCE.md`, §D.4 rule 3. OLD:
```text
The band buffer is capped at 512 KiB regardless of window width (band height = clamp(524288 / (4 × width_px), 32, 128) rows).
```
NEW:
```text
The band buffer is ≤ 512 KiB for windows up to 4 096 px wide (band height = clamp(524288 / (4 × width_px), 32, 128) rows); wider windows use the 32-row minimum.
```

**P-27** — `docs/SPEC.md`, R-IMG-06. OLD:
```text
*blocked network path*, *too large*, *unsupported format*.
```
NEW:
```text
*blocked network path*, *invalid path* (device names, alternate data streams, NUL — the file system is never touched; shown like *missing* with the text "Invalid image path"), *too large*, *unsupported format*.
```

**P-28** — `docs/TESTING.md`, §H.4. OLD:
```text
Each report lists the number and total bytes of goldens added or changed.
```
NEW:
```text
Each report lists the number and total bytes of goldens added or changed. The committed golden set stays ≤ 12 MB in total: scale 1.5 only for `typical` and `unicode`; every golden covers at most the first 4 000 logical px of its document; regenerated goldens replace the old files (no per-phase copies).
```

**P-29** — `docs/ROADMAP.md`. INSERT AFTER the anchor line:
```text
| **003** | Full rendering | tables, code blocks + highlighter + toolbar, callouts, footnotes, task lists, images (WIC, lazy, cache, placeholders), front matter, safe HTML subset, highlight/sub/sup, emoji, autolink literals, wikilinks, details, link navigation + full link policy (R-LINK, S-LINK, S-RES) | full corpus renders; security matrices for links/paths pass; fuzzers for parser/HTML |
```
NEW line:
```text
| **003b** | Rendering completion | images end-to-end (WIC decoder on the worker, Windows `Fs` layer, cache, placeholders), tooltip/popover/banner components, link remainders, inline objects, table row laziness, in-window test driver | Phase 003 AC-4, AC-5 and AC-9 pass |
```

---

## 4. Tasks (the Phase 003 leftovers — nothing else)

### 4.1 Images end to end
Do these in order, per prompt 003 §4.5 and P-13/P-25/P-27:
1. **Windows `Fs` layer** implementing the trait already used by `yamdv_core::image`, wired to `image::resolve_policy`. The policy decision happens **before** any file call (the recording-mock test stays and gets a Windows twin).
2. **Worker thread** (E.10; 256 KiB stack; cancellable; posts results; exits after 5 s idle). It runs the header probe (existing `image::probe`) and **WIC decode at display size**, in device pixels with high-quality scaling.
3. **Layout integration:**
   - Probed sizes replace the 4:3 estimate, with scroll anchoring.
   - Sizing (`display_size`, HTML `width`/`height`, `|300`, centering) is used for real.
   - Images within visible ± 1 screen stay decoded; the rest go into `image::Lru` (≤ 1.5 MiB).
4. **ACG matrix on CI** (PNG, JPEG, GIF first frame, BMP, ICO, TIFF, WebP): decode each format under ACG; record the results in an ADR.
5. **DIB section vs. DDB** measurement with 4 large images (PWS + commit), with the decision in the same ADR.
6. **Placeholders:**
   - **Block images** get the full R-IMG-06 placeholders (icon, message per reason, path or URL, [Copy URL] for remote) or the picture.
   - **Inline images** keep the P-25 chip.
   - **SVG** shows "unsupported format".

### 4.2 UI components (per prompt 003 §4.1 item 5; `yamdv-ui`, token-driven, keyboard-operable, EN/TR)
- **Tooltip** (500 ms delay): link `title`, `abbr`, footnote preview (≤ 400 px, 6 lines), image placeholder details.
- **Link status tooltip** (B.10.4).
- **Popover** anchored to its target (B.10.6). It replaces the in-window dialog for the external-link *Ask* and for "Blocked for safety" / local-file confirmations. Keep the in-window dialog only for document-level guards.
- **Banner** (B.10.5): Mark-of-the-Web, running elevated, lossy decoding.

Layered popup windows (E.6). Timers only during delays and fades. T7 must hold after showing and dismissing each.

### 4.3 Links and inline remainders
- `Ctrl+click` and middle-click on Markdown links → new window in the same process.
- `Tab`/`Shift+Tab` focus navigation through links and interactive elements, with the focus ring (B.16), auto-scroll into view, and `Enter` to activate.
- Heading anchors scroll so the heading sits 16 px below the top of the document area.
- **Inline objects** (`IDWriteInlineObject`) for `^sup^`, `~sub~` (when enabled) and footnote references: 0.75 em with a baseline shift, one caret/selection unit, copying their text form.
- `<kbd>` gets the key-cap look (B.6.3: 1 px border with a 2 px bottom border).
- `<details>` content is indented 16 px; hidden (collapsed) blocks take no space.
- The code toolbar fades in and out over 120 ms (B.15).

### 4.4 Tables
Row-level laziness per prompt 003 §4.3 item 3: only visible rows (± 1 screen) are laid out; other rows use estimated heights; column widths come from the existing sample.

### 4.5 In-window test driver (closes AC-5)
- **Driver:** a dev-tools-only `--script <file>` that injects events internally (no `SendInput`): `click x y`, `click-link "<text>"`, `key <chord>`, `wheel <delta>`, `wait-idle`, `assert <field> <op> <value>` over a JSON state dump (current document, scroll_y, focus, open popover/banner, navigation stack), `dump`.
- **Scripts on CI:** anchor scroll, relative navigation + back/forward with scroll restore, a blocked `.exe` link (popover shown, nothing launched), the http Ask popover, `Ctrl+click` → second window, the Tab order.

### 4.6 Measurements
Repeat the Phase 003 table (§4.11 of prompt 003), now including:
- **M1 with decoded images.** Report the image cache bytes in the breakdown. If M1 exceeds 5.0 MiB, explain what it would take to meet the final budget.
- **T-IMG.**
- **T7 after tooltip, popover and banner, and after an image decode.**

### 4.7 Out of scope
Everything of Phase 004 and later (title bar, sidebar, menus, settings UI, schemes, find, zoom, live reload, viewers, editing, export).

---

## 5. Amendment A-3 text (for `docs/PROCESS.md`, F-18)
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

---

## 6. Acceptance criteria
| ID | Criterion |
|---|---|
| AC-1 | F-16…F-19 done. |
| AC-2 | P-26…P-29 applied by script; `spec-check` green. |
| AC-3 | All gates green on CI; PR merged; tag `phase-003b`. |
| AC-4 (= 003 AC-4) | Every corpus document renders without interim placeholders except math, Mermaid and SVG. Local images show pixels; block placeholders follow R-IMG-06. Goldens follow P-28. |
| AC-5 (= 003 AC-5) | The §4.5 driver scripts pass on CI. |
| AC-6 | ACG + DIB/DDB ADR written with numbers. |
| AC-7 (= 003 AC-9) | Full measurement table, including M1 with images, T-IMG and T7 after the new components and after a decode. |
| AC-8 | Report ≤ 250 lines, PROCESS.md template incl. §0 and §3b. |

**END OF PROMPT 004.** Start with §2 now.
