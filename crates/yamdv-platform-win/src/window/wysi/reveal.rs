//! Inline reveal of the F2 active block (prompt 016 §3 item 1).

use super::*;
use yamdv_core::doc::index::{BlockKind, Index};
use yamdv_core::doc::reveal::{self as rv, Analysis};

/// Per-buffer-version analysis of the active block.
#[derive(Debug, Default)]
pub(in crate::window) struct RevealCache {
    version: u64,
    start: usize,
    analysis: Option<Analysis>,
    /// Block-level reveal of a code / math / diagram block: monospace source.
    code: bool,
}

impl Win {
    /// Recomputes the reveal paragraph of the active F2 block for the current text and selection.
    /// True when the block's layout changed (the caller re-prepares).
    pub(in crate::window) fn live_reveal(&mut self) -> bool {
        let Some(r) = self.active_range() else {
            return false;
        };
        let View::Doc(d) = &mut self.view else {
            return false;
        };
        let opts = match &d.content {
            Content::Markdown(doc) => doc.opts,
            Content::Plain { .. } => return false,
        };
        let Some(e) = d.edit.as_deref_mut() else {
            return false;
        };
        let Some(a) = e.active else { return false };
        let text = e.ed.slice(r.clone());
        let fresh = e
            .reveal
            .as_ref()
            .is_some_and(|c| c.version == e.ed.version && c.start == r.start);
        if !fresh {
            let analysis = rv::analyze(&text, opts);
            let code = analysis.is_none() && {
                let idx = Index::build(&text, opts);
                idx.blocks.len() == 1
                    && idx
                        .blocks
                        .first()
                        .is_some_and(|b| matches!(b.kind, BlockKind::Code | BlockKind::Math))
            };
            e.reveal = Some(RevealCache {
                version: e.ed.version,
                start: r.start,
                analysis,
                code,
            });
        }
        let Some(c) = e.reveal.as_ref() else {
            return false;
        };
        let sel = e.ed.selection();
        let rel = |p: usize| u32::try_from(p.clamp(r.start, r.end) - r.start).unwrap_or(0);
        let reveal = c.analysis.as_ref().map(|an| {
            let set = rv::reveal_set(an, rel(sel.start)..rel(sel.end));
            yamdv_layout::reveal::Reveal {
                para: rv::build(an, &text, &set),
                heading: an.heading,
                quote: an.quote,
            }
        });
        let code = c.code || e.opts.code_font;
        let Some(sb) = d.viewer.source_block.as_mut() else {
            return false;
        };
        if sb.reveal == reveal && sb.code_font == code {
            return false;
        }
        sb.reveal = reveal;
        sb.code_font = code;
        d.viewer.invalidate_block(a.block);
        true
    }

    /// Source text of the markers revealed in the active block (dev-tools probe `revealed`).
    #[cfg(feature = "dev-tools")]
    pub(in crate::window) fn revealed_markers(&self) -> Vec<String> {
        match &self.view {
            View::Doc(d) => d
                .viewer
                .source_block
                .as_ref()
                .and_then(|s| s.reveal.as_ref())
                .map(|r| r.para.shown.clone())
                .unwrap_or_default(),
            _ => Vec::new(),
        }
    }
}
