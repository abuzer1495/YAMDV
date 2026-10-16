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
