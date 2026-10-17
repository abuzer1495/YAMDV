//! Undo/redo history (prompt 006 §2.1): typing coalesced per word or per 1 s pause, selection
//! restored, capped at 4 MiB of stored text (oldest groups dropped first).

/// History byte cap (4 MiB of deleted + inserted text).
pub const CAP_BYTES: usize = 4 * 1024 * 1024;
/// Typing pause that closes a coalesced group.
pub const PAUSE_MS: u64 = 1000;

/// One replacement: at byte `pos`, `del` was removed and `ins` inserted.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Change {
    /// Byte offset.
    pub pos: usize,
    /// Removed text.
    pub del: String,
    /// Inserted text.
    pub ins: String,
}

/// How a group may grow.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    /// Typed text (coalesces with the next typed text at its end).
    Typing,
    /// Backspace / Delete (coalesces with the next deletion at the same spot).
    Delete,
    /// Commands, paste, cut (never coalesce).
    Other,
}

/// One undo step.
#[derive(Debug, Clone)]
pub struct Group {
    /// Changes in application order.
    pub changes: Vec<Change>,
    /// Selection (anchor, caret) before the group.
    pub sel_before: (usize, usize),
    /// Selection after the group.
    pub sel_after: (usize, usize),
    kind: Kind,
    t_ms: u64,
    /// The last typed character was whitespace (a word ended).
    last_space: bool,
}

impl Group {
    fn bytes(&self) -> usize {
        self.changes.iter().map(|c| c.del.len() + c.ins.len()).sum()
    }
}

/// Undo and redo stacks.
#[derive(Debug, Default, Clone)]
pub struct History {
    undo: Vec<Group>,
    redo: Vec<Group>,
    bytes: usize,
    /// The next record may not coalesce (after undo/redo or an explicit break).
    sealed: bool,
}

impl History {
    /// Records a change; coalesces typing/deletion per the rules above.
    pub fn record(
        &mut self,
        c: Change,
        kind: Kind,
        now_ms: u64,
        sel_before: (usize, usize),
        sel_after: (usize, usize),
    ) {
        self.redo.clear();
        let space = c.ins.chars().last().is_some_and(char::is_whitespace);
        let starts_word = c.ins.chars().next().is_some_and(|ch| !ch.is_whitespace());
        if !self.sealed
            && let Some(g) = self.undo.last_mut()
            && g.kind == kind
            && kind != Kind::Other
            && now_ms.saturating_sub(g.t_ms) <= PAUSE_MS
            && let Some(last) = g.changes.last_mut()
        {
            let joins = match kind {
                // Typing continues at the end of the previous insertion and stays in the word.
                Kind::Typing => {
                    c.del.is_empty()
                        && last.del.is_empty()
                        && c.pos == last.pos + last.ins.len()
                        && !(g.last_space && starts_word)
                }
                // Backspace (ends where the last one started) or Delete (same position).
                Kind::Delete => {
                    c.ins.is_empty()
                        && last.ins.is_empty()
                        && (c.pos + c.del.len() == last.pos || c.pos == last.pos)
                }
                Kind::Other => false,
            };
            if joins {
                self.bytes += c.del.len() + c.ins.len();
                if kind == Kind::Typing {
                    last.ins.push_str(&c.ins);
                } else if c.pos == last.pos {
                    last.del.push_str(&c.del);
                } else {
                    last.del.insert_str(0, &c.del);
                    last.pos = c.pos;
                }
                g.sel_after = sel_after;
                g.t_ms = now_ms;
                g.last_space = space;
                self.trim();
                return;
            }
        }
        self.sealed = false;
        self.bytes += c.del.len() + c.ins.len();
        self.undo.push(Group {
            changes: vec![c],
            sel_before,
            sel_after,
            kind,
            t_ms: now_ms,
            last_space: space,
        });
        self.trim();
    }

    /// Appends a change to the current (open) group — multi-change commands.
    pub fn extend_last(&mut self, c: Change, sel_after: (usize, usize)) {
        self.bytes += c.del.len() + c.ins.len();
        if let Some(g) = self.undo.last_mut() {
            g.changes.push(c);
            g.sel_after = sel_after;
        }
        self.trim();
    }

    /// The next change starts a new group.
    pub fn seal(&mut self) {
        self.sealed = true;
    }

    fn trim(&mut self) {
        while self.bytes > CAP_BYTES && self.undo.len() > 1 {
            let g = self.undo.remove(0);
            self.bytes -= g.bytes();
        }
    }

    /// Pops the group to undo (moved to the redo stack).
    pub fn pop_undo(&mut self) -> Option<Group> {
        let g = self.undo.pop()?;
        self.bytes -= g.bytes();
        self.sealed = true;
        Some(g)
    }

    /// Pushes an undone group onto the redo stack.
    pub fn push_redo(&mut self, g: Group) {
        self.redo.push(g);
    }

    /// Pops the group to redo.
    pub fn pop_redo(&mut self) -> Option<Group> {
        self.redo.pop()
    }

    /// Pushes a redone group back onto the undo stack.
    pub fn push_undo(&mut self, g: Group) {
        self.bytes += g.bytes();
        self.undo.push(g);
        self.sealed = true;
        self.trim();
    }

    /// Stored bytes (≤ [`CAP_BYTES`] unless a single group is larger).
    pub fn bytes(&self) -> usize {
        self.bytes
    }

    /// Undo depth.
    pub fn undo_len(&self) -> usize {
        self.undo.len()
    }

    /// Redo depth.
    pub fn redo_len(&self) -> usize {
        self.redo.len()
    }
}
