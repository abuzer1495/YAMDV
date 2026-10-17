//! Two-level document model (P-07, ADR-0007).
//!
//! * [`index`] — the top-level block index and document maps, built by one full parse.
//! * [`detail`] — lazily built per-block detail trees (flat arena + inline runs), LRU-cached.
//! * [`text`] — layout string ↔ source mapping and text extraction for copy.

pub mod detail;
pub mod index;
pub mod inline;
pub mod slug;
pub mod text;

use pulldown_cmark::Options;

/// Reading options that change parsing (B.8.2 `reading.*`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ReadOpts {
    /// `reading.subscript_tilde` (R-MD-06 as patched by P-08a).
    pub subscript_tilde: bool,
    /// `reading.smart_punctuation`.
    pub smart_punctuation: bool,
    /// `reading.single_newline_break` (R-MD-15): soft breaks render as line breaks.
    pub single_newline_break: bool,
    /// `reading.code_line_numbers` (R-CODE-02 default; layout only).
    pub code_line_numbers: bool,
    /// `reading.code_wrap` (R-CODE-05 default; layout only).
    pub code_wrap: bool,
    /// `reading.front_matter` (R-REN-13; layout only): 0 collapsed, 1 expanded, 2 hidden.
    pub front_matter: u8,
    /// `reading.render_math` (R-MATH-*; layout only).
    pub render_math: bool,
    /// `reading.render_mermaid` (R-MMD-*; layout only).
    pub render_mermaid: bool,
}

/// pulldown-cmark options per Phase 002 §4.3 item 1.
pub fn parser_options(o: ReadOpts) -> Options {
    let mut opts = Options::ENABLE_TABLES
        | Options::ENABLE_FOOTNOTES
        | Options::ENABLE_STRIKETHROUGH
        | Options::ENABLE_TASKLISTS
        | Options::ENABLE_HEADING_ATTRIBUTES
        | Options::ENABLE_YAML_STYLE_METADATA_BLOCKS
        | Options::ENABLE_PLUSES_DELIMITED_METADATA_BLOCKS
        | Options::ENABLE_MATH
        | Options::ENABLE_GFM
        | Options::ENABLE_DEFINITION_LIST
        | Options::ENABLE_SUPERSCRIPT
        | Options::ENABLE_WIKILINKS;
    if o.subscript_tilde {
        opts |= Options::ENABLE_SUBSCRIPT;
    }
    if o.smart_punctuation {
        opts |= Options::ENABLE_SMART_PUNCTUATION;
    }
    opts
}

/// Nesting cap (S-PARSE-01): deeper content is attached at this level.
pub const MAX_DEPTH: usize = 64;

/// A loaded document: source text + index + cache of detail trees.
#[derive(Debug)]
pub struct Document {
    /// Source text (UTF-8, line endings as in the file).
    pub source: String,
    /// Parse options.
    pub opts: ReadOpts,
    /// Top-level block index and maps.
    pub index: index::Index,
    /// Detail tree cache.
    pub cache: detail::Cache,
}

impl Document {
    /// Parses `source` (one full pass) and returns the document.
    pub fn new(source: String, opts: ReadOpts) -> Self {
        let index = index::Index::build(&source, opts);
        Self {
            source,
            opts,
            index,
            cache: detail::Cache::new(detail::DEFAULT_BUDGET),
        }
    }

    /// Detail tree of top-level block `i` (built on demand, cached).
    pub fn detail(&mut self, i: usize) -> std::rc::Rc<detail::Detail> {
        self.cache
            .get_or_build(i, &self.source, &self.index, self.opts)
    }

    /// Detail tree built without touching the cache (text extraction of far-away blocks).
    pub fn detail_uncached(&self, i: usize) -> detail::Detail {
        detail::Detail::build(&self.source, &self.index, i, self.opts)
    }

    /// Incremental edit (ADR-0007 §4, prompt 006 §2.3): replaces the source bytes `edit` by
    /// `new`, re-parses only the affected top-level blocks and returns what changed. The detail
    /// cache is cleared (its trees hold absolute source ranges).
    pub fn apply_edit(&mut self, edit: std::ops::Range<usize>, new: &str) -> index::Splice {
        let edit = edit.start.min(self.source.len())..edit.end.min(self.source.len());
        self.source.replace_range(edit.clone(), new);
        let sp = self.index.splice(&self.source, edit, new.len(), self.opts);
        self.cache.clear();
        sp
    }

    /// Brings the document to `text` with one splice over the differing middle (common prefix
    /// and suffix are kept). Returns `None` when nothing changed.
    pub fn sync_to(&mut self, text: &str) -> Option<(std::ops::Range<usize>, index::Splice)> {
        let (a, b) = (self.source.as_bytes(), text.as_bytes());
        let mut p = a.iter().zip(b).take_while(|(x, y)| x == y).count();
        if p == a.len() && p == b.len() {
            return None;
        }
        let max_s = a.len().min(b.len()) - p;
        let mut s = a
            .iter()
            .rev()
            .zip(b.iter().rev())
            .take(max_s)
            .take_while(|(x, y)| x == y)
            .count();
        // Keep the cut on char boundaries of both texts.
        while p > 0 && !(self.source.is_char_boundary(p) && text.is_char_boundary(p)) {
            p -= 1;
        }
        while s > 0
            && !(self.source.is_char_boundary(a.len() - s) && text.is_char_boundary(b.len() - s))
        {
            s -= 1;
        }
        let old = p..a.len() - s;
        let new = text.get(p..b.len() - s).unwrap_or("").to_string();
        let sp = self.apply_edit(old.clone(), &new);
        Some((p..p + new.len(), sp))
    }

    /// Bytes owned by the model (source + index + cache), for `mem-stats`.
    pub fn heap_bytes(&self) -> (usize, usize, usize) {
        (
            self.source.capacity(),
            self.index.heap_bytes(),
            self.cache.bytes(),
        )
    }
}
