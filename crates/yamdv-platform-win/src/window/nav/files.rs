//! Files tab (FUT-33): a lazy tree of the Markdown-family files under the document's folder.
//! Folders are listed only when expanded, on a worker thread; the root and expanded folders are
//! watched (≤ 32) and relisted within ~150 ms of a change. Pure rules: `yamdv_core::files`.

use super::*;
use crate::window::sidebar::RowKind;
use std::os::windows::fs::MetadataExt;
use std::sync::{Arc, Mutex};
use yamdv_core::files::{Attrs, Entry, MAX_ENTRIES, Row as FRow, Tree, is_markdown, skipped};

type Listing = (String, Vec<Entry>, bool);

/// Files tab state of one window.
pub(in crate::window) struct Files {
    pub tree: Tree,
    /// The root is the folder of the open document (re-rooted when it leaves the tree).
    from_doc: bool,
    watchers: Vec<(String, usize, crate::shell::Watcher)>,
    next_tag: usize,
    pending: Arc<Mutex<Vec<Listing>>>,
    dirty: Vec<usize>,
    hint: Option<Msg>,
}

/// UNC path or a mapped network drive.
fn is_network(path: &str) -> bool {
    use windows::Win32::Storage::FileSystem::GetDriveTypeW;
    if path.starts_with("\\\\") && !path.starts_with("\\\\?\\") && !path.starts_with("\\\\.\\") {
        return true;
    }
    let Some(drive) = path.get(..2).filter(|d| d.ends_with(':')) else {
        return false;
    };
    let root = HSTRING::from(format!("{drive}\\"));
    // SAFETY: valid NUL-terminated string.
    unsafe { GetDriveTypeW(&root) == 4 }
}

/// Lists one folder (runs on a worker thread): sub-folders and Markdown files, skip rules
/// applied, at most `MAX_ENTRIES + 1` names.
fn list_dir(path: &str) -> (Vec<Entry>, bool) {
    let mut out = Vec::new();
    let Ok(rd) = std::fs::read_dir(ypath::long_form(path)) else {
        return (out, false);
    };
    for e in rd.flatten() {
        if out.len() >= MAX_ENTRIES {
            return (out, true);
        }
        let Ok(m) = e.metadata() else { continue };
        let a = m.file_attributes();
        let attrs = Attrs {
            dir: a & 0x10 != 0,
            hidden: a & 0x2 != 0,
            system: a & 0x4 != 0,
            reparse: a & 0x400 != 0,
        };
        let name = e.file_name().to_string_lossy().into_owned();
        if skipped(&name, attrs) || (!attrs.dir && !is_markdown(&name)) {
            continue;
        }
        out.push(Entry {
            name,
            dir: attrs.dir,
        });
    }
    (out, false)
}

impl Files {
    fn new(root: &str, from_doc: bool) -> Self {
        Self {
            tree: Tree::new(root),
            from_doc,
            watchers: Vec::new(),
            next_tag: 0,
            pending: Arc::new(Mutex::new(Vec::new())),
            dirty: Vec::new(),
            hint: None,
        }
    }

    /// Starts listing `path` on a worker thread; the window gets [`WM_APP_FILES`].
    fn list(&self, hwnd: HWND, path: String) {
        let pending = self.pending.clone();
        let target = hwnd.0 as isize;
        let _ = std::thread::Builder::new()
            .name("yamdv-files".into())
            .stack_size(256 * 1024)
            .spawn(move || {
                let (entries, more) = list_dir(&path);
                if let Ok(mut p) = pending.lock() {
                    p.push((path, entries, more));
                }
                // SAFETY: posting to a window handle is safe even if it is gone.
                let _ = unsafe {
                    PostMessageW(
                        Some(HWND(target as *mut core::ffi::c_void)),
                        WM_APP_FILES,
                        WPARAM(0),
                        LPARAM(0),
                    )
                };
            });
    }

    /// Watches the root and expanded folders (≤ 32), dropping the rest.
    fn sync_watchers(&mut self, hwnd: HWND) {
        let want = self.tree.watched();
        self.watchers
            .retain(|(p, _, _)| want.iter().any(|w| w == p));
        for w in want {
            if self.watchers.iter().any(|(p, _, _)| *p == w) {
                continue;
            }
            let tag = self.next_tag;
            self.next_tag += 1;
            if let Some(x) = crate::shell::watch_dir(&w, hwnd, FILES_TAG + tag) {
                self.watchers.push((w, tag, x));
            }
        }
    }
}

/// Folder of a document path.
fn folder_of(path: &str) -> Option<String> {
    path.rsplit_once('\\').map(|(d, _)| d.to_string())
}

/// `path` is inside folder `root` (case-insensitive).
fn under(path: &str, root: &str) -> bool {
    let (p, r) = (
        path.to_lowercase(),
        root.trim_end_matches('\\').to_lowercase(),
    );
    p.len() > r.len() && p.starts_with(&r) && p.as_bytes().get(r.len()) == Some(&b'\\')
}

impl Win {
    fn doc_path(&self) -> Option<String> {
        match &self.view {
            View::Doc(d) => Some(d.path.clone()),
            _ => None,
        }
    }

    /// Builds the tree for the current document when there is none (or it left the tree).
    fn files_ensure(&self) {
        let doc = self.doc_path();
        let mut slot = self.nav.files.borrow_mut();
        let stale = match (&*slot, &doc) {
            (None, Some(_)) => true,
            (Some(f), Some(p)) => f.from_doc && !under(p, f.tree.root()),
            _ => false,
        };
        if !stale {
            return;
        }
        let Some(root) = doc.as_deref().and_then(folder_of) else {
            return;
        };
        // The document's own share is allowed; nothing else on the network is reached.
        let mut f = Files::new(&root, true);
        f.list(self.hwnd, root);
        f.sync_watchers(self.hwnd);
        *slot = Some(f);
    }

    /// Rows of the Files tab: (kind, text, indent).
    pub(in crate::window) fn files_rows(&self, _sh: &Shared) -> Vec<(RowKind, String, f32)> {
        self.files_ensure();
        let slot = self.nav.files.borrow();
        let Some(f) = slot.as_ref() else {
            return vec![(RowKind::Hint(Msg::FilesNoFolder), String::new(), 0.0)];
        };
        if let Some(m) = f.hint {
            return vec![(RowKind::Hint(m), String::new(), 0.0)];
        }
        let cur = self.doc_path().unwrap_or_default().to_lowercase();
        let mut out = Vec::new();
        for r in f.tree.rows() {
            match r {
                FRow::Node(i) => {
                    let Some(n) = f.tree.nodes.get(i) else {
                        continue;
                    };
                    out.push((
                        RowKind::File {
                            node: i,
                            dir: n.dir,
                            open: n.expanded,
                            current: !n.dir && n.path.to_lowercase() == cur,
                        },
                        n.name.clone(),
                        f32::from(n.depth) * 12.0,
                    ));
                }
                FRow::More(_, depth) => {
                    out.push((
                        RowKind::Hint(Msg::FilesMore),
                        String::new(),
                        f32::from(depth) * 12.0,
                    ));
                }
            }
        }
        let loaded = f.tree.nodes.first().is_some_and(|n| n.loaded);
        if out.is_empty() && loaded {
            out.push((RowKind::Hint(Msg::FilesEmpty), String::new(), 0.0));
        }
        out
    }

    /// Click / Enter on a node: folders expand or collapse, files open in place.
    pub(in crate::window) fn files_click(&mut self, sh: &Shared, node: usize) {
        let hwnd = self.hwnd;
        let open = {
            let mut slot = self.nav.files.borrow_mut();
            let Some(f) = slot.as_mut() else { return };
            let Some(n) = f.tree.nodes.get(node) else {
                return;
            };
            if n.dir {
                if let Some(i) = f.tree.toggle(node)
                    && let Some(p) = f.tree.nodes.get(i).map(|n| n.path.clone())
                {
                    f.list(hwnd, p);
                }
                f.sync_watchers(hwnd);
                None
            } else {
                Some(n.path.clone())
            }
        };
        if let Some(p) = open {
            self.open_from_side(sh, &p);
        }
        self.invalidate();
    }

    /// Parent folder row of `node` (not the hidden root).
    pub(in crate::window) fn files_parent(&self, node: usize) -> Option<usize> {
        let slot = self.nav.files.borrow();
        slot.as_ref()?.tree.parent(node).filter(|p| *p != 0)
    }

    /// Listings arrived from worker threads.
    pub(in crate::window) fn files_collect(&mut self, _sh: &Shared) {
        let hwnd = self.hwnd;
        let mut slot = self.nav.files.borrow_mut();
        let Some(f) = slot.as_mut() else { return };
        let got: Vec<Listing> = f
            .pending
            .lock()
            .map(|mut p| std::mem::take(&mut *p))
            .unwrap_or_default();
        for (path, entries, more) in got {
            if let Some(i) = f.tree.find(&path) {
                f.tree.fill(i, entries, more);
            }
        }
        f.sync_watchers(hwnd);
        drop(slot);
        self.invalidate();
    }

    /// A watched folder changed (tag offset); relisted after the debounce.
    pub(in crate::window) fn files_dirty(&mut self, tag: usize) {
        if let Some(f) = self.nav.files.borrow_mut().as_mut()
            && !f.dirty.contains(&tag)
        {
            f.dirty.push(tag);
        }
    }

    /// Relists the folders that changed.
    pub(in crate::window) fn files_relist(&mut self) {
        let hwnd = self.hwnd;
        let mut slot = self.nav.files.borrow_mut();
        let Some(f) = slot.as_mut() else { return };
        let tags = std::mem::take(&mut f.dirty);
        let paths: Vec<String> = f
            .watchers
            .iter()
            .filter(|(_, t, _)| tags.contains(t))
            .map(|(p, _, _)| p.clone())
            .collect();
        for p in paths {
            f.list(hwnd, p);
        }
    }

    /// A dropped folder: the Files tab rooted at it (FUT-33).
    pub(in crate::window) fn files_open_folder(&mut self, sh: &Shared, folder: &str) {
        let mut f = Files::new(folder, false);
        if is_network(folder) && !self.doc_path().is_some_and(|p| under(&p, folder)) {
            f.hint = Some(Msg::FilesNetwork);
        } else {
            f.list(self.hwnd, folder.trim_end_matches('\\').to_string());
            f.sync_watchers(self.hwnd);
        }
        *self.nav.files.borrow_mut() = Some(f);
        if !matches!(self.view, View::Doc(_)) {
            self.view = View::Empty {
                note: Some(Msg::FilesPick),
            };
        }
        app::with(|a| a.state.sidebar_tab = 3);
        app::save_state();
        self.side.scroll = 0.0;
        if !sh.side_visible {
            self.actions.push(Action::Command(app::OUTLINE));
        }
        self.invalidate();
    }

    /// Driver / tests: visible Files rows as text.
    #[cfg(feature = "dev-tools")]
    pub(in crate::window) fn files_describe(&self) -> String {
        let Some(sh) = shared() else {
            return String::new();
        };
        self.files_rows(&sh)
            .into_iter()
            .map(|(k, t, _)| match k {
                RowKind::File {
                    dir: true, open, ..
                } => format!("{}{t}", if open { "v " } else { "> " }),
                RowKind::File { current: true, .. } => format!("*{t}"),
                RowKind::Hint(m) => format!("({})", tr(Lang::En, m)),
                _ => t,
            })
            .collect::<Vec<_>>()
            .join("|")
    }
}

#[cfg(test)]
mod tests {
    use super::{folder_of, under};

    #[test]
    fn under_and_folder() {
        assert!(under(r"C:\a\b\x.md", r"C:\A"));
        assert!(!under(r"C:\ab\x.md", r"C:\a"));
        assert!(!under(r"C:\a", r"C:\a"));
        assert_eq!(folder_of(r"C:\a\x.md").as_deref(), Some(r"C:\a"));
    }
}
