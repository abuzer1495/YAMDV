//! Inline reveal of the F2 active block (prompt 016 §3 item 1) and the block-level source with
//! its live math / Mermaid preview (item 2).

use super::*;
use yamdv_core::doc::fence::{self, FenceKind};
use yamdv_core::doc::reveal::{self as rv, Analysis};
use yamdv_layout::reveal::{Preview, PreviewKind};

/// Mermaid preview debounce (prompt 016 §3 item 2).
const MERMAID_MS: u32 = 150;

/// Per-buffer-version analysis of the active block.
#[derive(Debug, Default)]
pub(in crate::window) struct RevealCache {
    version: u64,
    start: usize,
    analysis: Option<Analysis>,
    /// Fenced (code / math / diagram) block: monospace source.
    fence: Option<fence::Fence>,
    /// Table block: cell grid (prompt 016 §3 item 8).
    grid: Option<yamdv_editor::table::Grid>,
}

impl Win {
    /// Recomputes the reveal paragraph of the active F2 block for the current text and selection.
    /// True when the block's layout changed (the caller re-prepares).
    pub(in crate::window) fn live_reveal(&mut self) -> bool {
        let Some(r) = self.active_range() else {
            return false;
        };
        let hwnd = self.hwnd;
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
            let fence = if analysis.is_none() {
                fence::parse(&text)
            } else {
                None
            };
            let grid = if analysis.is_none() && fence.is_none() {
                yamdv_editor::table::grid(&text)
            } else {
                None
            };
            e.reveal = Some(RevealCache {
                version: e.ed.version,
                start: r.start,
                analysis,
                fence,
                grid,
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
                image: an.image.clone(),
            }
        });
        let table = c
            .grid
            .as_ref()
            .map(|g| super::table::table_reveal(g, &text, opts, rel(sel.start)..rel(sel.end)));
        let code = c.fence.is_some() || (e.opts.code_font && table.is_none());
        let want = c.fence.as_ref().and_then(|f| {
            let body = text.get(f.body.clone()).unwrap_or("").to_string();
            match f.kind {
                FenceKind::Math => Some(Preview {
                    kind: PreviewKind::Math,
                    text: body,
                }),
                FenceKind::Mermaid => Some(Preview {
                    kind: PreviewKind::Mermaid,
                    text: body,
                }),
                FenceKind::Code => None,
            }
        });
        let Some(sb) = d.viewer.source_block.as_mut() else {
            return false;
        };
        let mut preview = sb.preview.clone();
        match want {
            Some(p) if p.kind == PreviewKind::Mermaid => {
                let shown = preview
                    .as_ref()
                    .is_some_and(|q| q.kind == PreviewKind::Mermaid);
                if preview.as_ref() != Some(&p) && yamdv_mermaid::parse(&p.text).is_ok() {
                    if shown {
                        // Debounced; the last good diagram stays meanwhile.
                        self.wysi.pending = Some(p);
                        // SAFETY: valid hwnd.
                        unsafe { SetTimer(Some(hwnd), super::TIMER_F2_PREVIEW, MERMAID_MS, None) };
                    } else {
                        preview = Some(p);
                    }
                }
            }
            other => preview = other,
        }
        if sb.reveal == reveal && sb.code_font == code && sb.preview == preview && sb.table == table
        {
            return false;
        }
        sb.table = table;
        sb.reveal = reveal;
        sb.code_font = code;
        sb.preview = preview;
        d.viewer.invalidate_block(a.block);
        true
    }

    /// `TIMER_F2_PREVIEW`: applies the pending (debounced) Mermaid preview.
    pub(in crate::window) fn f2_preview_tick(&mut self, sh: &Shared) {
        // SAFETY: valid hwnd.
        let _ = unsafe { KillTimer(Some(self.hwnd), super::TIMER_F2_PREVIEW) };
        let Some(p) = self.wysi.pending.take() else {
            return;
        };
        let View::Doc(d) = &mut self.view else { return };
        let Some(sb) = d.viewer.source_block.as_mut() else {
            return;
        };
        if sb.preview.as_ref().is_some_and(|q| q.kind == p.kind) {
            sb.preview = Some(p);
            let b = sb.block;
            d.viewer.invalidate_block(b);
            self.prepare_now(sh);
            self.invalidate();
        }
    }

    /// Fence of the active block (block-level source), with the block start.
    pub(in crate::window) fn active_fence(&self) -> Option<(usize, fence::Fence)> {
        let e = self.edit()?;
        let c = e.reveal.as_ref()?;
        (c.version == e.ed.version)
            .then(|| c.fence.clone().map(|f| (c.start, f)))
            .flatten()
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

    /// Kind of the live preview under the active block (`math`, `mermaid` or empty).
    #[cfg(feature = "dev-tools")]
    pub(in crate::window) fn preview_kind(&self) -> &'static str {
        match &self.view {
            View::Doc(d) => match d
                .viewer
                .source_block
                .as_ref()
                .and_then(|s| s.preview.as_ref())
            {
                Some(p) if p.kind == PreviewKind::Math => "math",
                Some(_) => "mermaid",
                None => "",
            },
            _ => "",
        }
    }
}
