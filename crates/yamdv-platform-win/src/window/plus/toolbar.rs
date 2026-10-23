//! F2 format toolbar (prompt 015 §3.1, R-F2-04, B.13.2): placeholder (filled in next).

/// Toolbar commands.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::window) enum Cmd {
    Bold,
    Italic,
    Strike,
    Highlight,
    Code,
    Link,
    H(u8),
    Quote,
    Bullet,
    Numbered,
    Task,
    Clear,
}

impl super::Win {
    pub(super) fn toolbar_visible(&self, _sh: &super::Shared) -> bool {
        false
    }
}
