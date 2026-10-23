# ADR-0020: F2 reveal model (inline reveal, WYSIWYG Live Edit)

- Status: accepted (Phase V2-F, prompt 016 §2.1). Written before any F2 code of this phase.

Context: v1 F2 (prompt 006 §2.5) showed the whole active block as raw source in a code-tinted box. R-F2-02
(B.13.2) asks for Typora/Obsidian-style reveal: the page stays rendered, markers appear in `marker` color only
near the caret. The ADR-0007 document model (frozen `Document` + splice on leave) and the ADR-0007 rope buffer
stay; this ADR only decides how the **active block** is shown and edited.

Decision:
1. **Source map.** The active block is laid out as one *reveal paragraph* built by the pure
   `yamdv_core::doc::reveal` module from the parser events of the block's current text (standalone
   `Index` + `Detail` of that text). Every inline run becomes a styled segment whose source range is the
   run's byte range; the bytes no run covers are **markers**. The layout string is the concatenation of
   visible segments; the map is a `ParaText` (UTF-16 ↔ rope byte offset, verbatim segments are exact,
   non-verbatim segments — entities, escapes, emoji shortcodes, hidden list markers shown as `•`/`☐` —
   map to their whole source range). Offsets are block-relative and shifted by the block start.
2. **Reveal set.** A *construct* is a maximal stretch of inline-styled runs (strong, em, strike, code,
   link, highlight, math, image) on one line plus the marker gaps that bound and interleave it. Its markers
   are visible when the caret (either selection end) is inside the construct or touches its first/last
   byte. *Line markers* (the gap before the first run of a line: `#…`, `>`, `-`/`*`/`+`, `1.`, `- [ ]`,
   callout heads, indentation) are visible while a selection end is on that line. Non-verbatim runs
   (entities, escapes) show their source while a selection end touches them.
3. **Caret movement.** Moves operate on rope offsets (the F3 editor). Because a marker is always revealed
   when a selection end is in or next to its construct/line, the caret can enter every revealed marker and
   never rests inside a hidden one; positions inside a non-verbatim segment snap to its ends (atomic).
   CJK/emoji clusters stay whole because segments split only at run boundaries and caret moves are by
   grapheme cluster.
4. **Relayout scope.** A caret move that changes the reveal set re-lays out only the active block (one
   paragraph, at most the 2 paragraphs that lose/gain the caret when the caret leaves the block). An edit
   re-lays out only the active block; leaving it applies the ADR-0007 splice of the touched block only.
   The reveal set is cached by (text version, caret line, construct id) so unchanged moves skip relayout.
5. **Undo grouping.** Typing transforms (R-F2-03) are separate undo steps sealed before and after, so one
   `Ctrl+Z` restores the literal text; table operations and cross-block cut/delete are one step each.
6. **Block-level reveal (v1 behavior kept).** Code blocks (fences shown, monospace, highlighted),
   display math and Mermaid (source + live preview below), raw HTML blocks, front matter, footnote
   definitions and multi-block text (e.g. right after typing a blank line) show the block's source in
   place. Tables use cell editing (R-TBL), never pipes.

Consequences: the active block keeps body typography (heading size, weight, italics, code font, link
color) but block chrome (list indent boxes, quote bar) is approximated in the reveal paragraph; untouched
blocks are never re-serialized (R-F2-10); cost per keystroke is one parse + layout of the active block
(T5 F2 gate ≤ 40 ms on the 1 MB document).
