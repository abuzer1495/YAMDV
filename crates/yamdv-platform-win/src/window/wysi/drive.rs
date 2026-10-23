//! `ui v2f-*` driver verbs (dev-tools): no DirectWrite hit test under the test runner.

use super::*;

impl Win {
    /// `ui v2f-*` verbs.
    pub(in crate::window) fn v2f_ui(&mut self, sh: &Shared, verb: &str, arg: &str) -> String {
        let ok = match verb {
            // F2: opens the block holding `needle` and puts the caret `k` bytes into it
            // (`arg` = "k|needle"; selection with "k-j|needle").
            "v2f-caret" => {
                let (spec, needle) = arg.split_once('|').unwrap_or(("0", arg));
                let (a, b) = spec.split_once('-').unwrap_or((spec, spec));
                let (a, b) = (a.parse().unwrap_or(0usize), b.parse().unwrap_or(0usize));
                let at = match &self.view {
                    View::Doc(d) => d
                        .content
                        .source()
                        .find(needle)
                        .map(|p| (p, crate::window::edit::block_of(&d.content, p))),
                    _ => None,
                };
                at.is_some_and(|(p, bi)| {
                    self.enter_block(sh, bi, p + a);
                    if let Some(e) = self.edit_mut() {
                        e.ed.set_selection(p + a, p + b);
                    }
                    self.after_edit(sh, false);
                    true
                })
            }
            _ => false,
        };
        if ok {
            "ok".into()
        } else {
            format!("err {verb}")
        }
    }
}
