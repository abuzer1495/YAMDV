//! Files tab model (FUT-33, prompt 014 §3.1): natural sort with Turkish-aware folding, skip
//! rules, and a lazily filled folder tree with entry / depth limits. Pure; the platform layer
//! enumerates folders on a worker thread and feeds [`Tree::fill`].

use std::cmp::Ordering;

/// Total entries kept in one tree (a muted "…more" row marks the cut).
pub const MAX_ENTRIES: usize = 5000;
/// Deepest folder level that may be expanded (root children are depth 0).
pub const MAX_DEPTH: u8 = 8;
/// Folders watched at most (root + expanded folders, P-43).
pub const MAX_WATCHED: usize = 32;
/// Folder names that are never listed.
pub const SKIP_DIRS: &[&str] = &[".git", "node_modules", "target", ".venv", "__pycache__"];

/// One directory entry as enumerated.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    /// File or folder name.
    pub name: String,
    /// Folder.
    pub dir: bool,
}

/// Attributes that decide whether an entry is listed.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Attrs {
    /// Folder.
    pub dir: bool,
    /// Hidden attribute (or a dot name).
    pub hidden: bool,
    /// System attribute.
    pub system: bool,
    /// Reparse point (junction, symlink, cloud placeholder).
    pub reparse: bool,
}

/// Turkish-aware case fold of one char (i/İ/ı/I equal, as in Find).
fn fold(c: char) -> char {
    crate::find::fold(c, false)
}

/// Natural order: digit runs compare by value, other chars by folded value; ties fall back to
/// the raw string so the order is total.
pub fn natural_cmp(a: &str, b: &str) -> Ordering {
    let (mut x, mut y) = (a.chars().peekable(), b.chars().peekable());
    loop {
        match (x.peek().copied(), y.peek().copied()) {
            (None, None) => return a.cmp(b),
            (None, Some(_)) => return Ordering::Less,
            (Some(_), None) => return Ordering::Greater,
            (Some(p), Some(q)) if p.is_ascii_digit() && q.is_ascii_digit() => {
                let (mut n, mut m) = (String::new(), String::new());
                while let Some(c) = x.peek().copied().filter(char::is_ascii_digit) {
                    n.push(c);
                    x.next();
                }
                while let Some(c) = y.peek().copied().filter(char::is_ascii_digit) {
                    m.push(c);
                    y.next();
                }
                let (tn, tm) = (n.trim_start_matches('0'), m.trim_start_matches('0'));
                let o = tn.len().cmp(&tm.len()).then_with(|| tn.cmp(tm));
                if o != Ordering::Equal {
                    return o;
                }
            }
            (Some(p), Some(q)) => {
                let o = fold(p).cmp(&fold(q));
                if o != Ordering::Equal {
                    return o;
                }
                x.next();
                y.next();
            }
        }
    }
}

/// Folders first, then natural order.
pub fn sort_entries(v: &mut [Entry]) {
    v.sort_by(|a, b| {
        b.dir
            .cmp(&a.dir)
            .then_with(|| natural_cmp(&a.name, &b.name))
    });
}

/// Markdown-family file name (by extension).
pub fn is_markdown(name: &str) -> bool {
    name.rsplit_once('.').is_some_and(|(stem, e)| {
        !stem.is_empty()
            && crate::link::MARKDOWN_EXT
                .iter()
                .any(|m| m.eq_ignore_ascii_case(e))
    })
}

/// The entry is not listed: hidden / system items, reparse points, skipped folders, and files
/// outside the Markdown family.
pub fn skipped(name: &str, a: Attrs) -> bool {
    if a.hidden || a.system || a.reparse || name.starts_with('.') || name == "." || name == ".." {
        return true;
    }
    if a.dir {
        SKIP_DIRS.iter().any(|s| s.eq_ignore_ascii_case(name))
    } else {
        !is_markdown(name)
    }
}

/// One tree node (the root is node 0 and is not shown as a row).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Node {
    /// Full path (`\` separated).
    pub path: String,
    /// Display name.
    pub name: String,
    /// Folder.
    pub dir: bool,
    /// Depth (root children are 0; the root itself is `u8::MAX` conceptually, stored as 0).
    pub depth: u8,
    /// Expanded (folders).
    pub expanded: bool,
    /// Children were enumerated at least once.
    pub loaded: bool,
    /// Children (node indexes, sorted).
    pub kids: Vec<usize>,
    /// The listing was cut by a limit ("…more").
    pub more: bool,
}

/// A visible row.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Row {
    /// A file or folder node.
    Node(usize),
    /// Muted "…more" row under a folder (node index), at a depth.
    More(usize, u8),
}

/// Lazily filled folder tree.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Tree {
    /// Nodes; 0 is the root folder.
    pub nodes: Vec<Node>,
}

impl Tree {
    /// A tree rooted at folder `root` (expanded, not loaded yet).
    pub fn new(root: &str) -> Self {
        let root = root.trim_end_matches('\\').to_string();
        let name = crate::path::file_name(&root).to_string();
        Self {
            nodes: vec![Node {
                path: root,
                name,
                dir: true,
                depth: 0,
                expanded: true,
                loaded: false,
                kids: Vec::new(),
                more: false,
            }],
        }
    }

    /// Root folder path.
    pub fn root(&self) -> &str {
        self.nodes.first().map_or("", |n| n.path.as_str())
    }

    /// Node by path (case-insensitive, as on Windows).
    pub fn find(&self, path: &str) -> Option<usize> {
        let p = path.trim_end_matches('\\');
        self.nodes.iter().position(|n| {
            n.path.eq_ignore_ascii_case(p) || n.path.to_lowercase() == p.to_lowercase()
        })
    }

    fn live(&self) -> usize {
        let mut n = 0;
        let mut stack = vec![0usize];
        while let Some(i) = stack.pop() {
            if let Some(x) = self.nodes.get(i) {
                n += x.kids.len();
                stack.extend(x.kids.iter().copied());
            }
        }
        n
    }

    /// Sets the children of folder `at` from an enumeration (unsorted, unfiltered names are
    /// fine: [`skipped`] already ran in the lister). Keeps the expanded state of folders that are
    /// still there; caps the whole tree at [`MAX_ENTRIES`]. `more` = the lister stopped early.
    pub fn fill(&mut self, at: usize, mut entries: Vec<Entry>, more: bool) {
        let Some(parent) = self.nodes.get(at).cloned() else {
            return;
        };
        sort_entries(&mut entries);
        let old: Vec<usize> = parent.kids.clone();
        let base = self.live() - old.len();
        let room = MAX_ENTRIES.saturating_sub(base);
        let cut = entries.len() > room;
        entries.truncate(room);
        let depth = if at == 0 { 0 } else { parent.depth + 1 };
        let mut kids = Vec::with_capacity(entries.len());
        for e in entries {
            let reuse = old.iter().copied().find(|&k| {
                self.nodes
                    .get(k)
                    .is_some_and(|n| n.name == e.name && n.dir == e.dir)
            });
            if let Some(k) = reuse {
                kids.push(k);
                continue;
            }
            self.nodes.push(Node {
                path: format!("{}\\{}", parent.path, e.name),
                name: e.name,
                dir: e.dir,
                depth,
                expanded: false,
                loaded: false,
                kids: Vec::new(),
                more: false,
            });
            kids.push(self.nodes.len() - 1);
        }
        if let Some(p) = self.nodes.get_mut(at) {
            p.kids = kids;
            p.loaded = true;
            p.more = more || cut;
        }
    }

    /// Toggles folder `i`; returns `Some(i)` when it must be enumerated now.
    pub fn toggle(&mut self, i: usize) -> Option<usize> {
        let n = self.nodes.get_mut(i).filter(|n| n.dir && i != 0)?;
        n.expanded = !n.expanded;
        (n.expanded && !n.loaded && n.depth < MAX_DEPTH).then_some(i)
    }

    /// Sets the expanded state of folder `i` (← / →); returns `Some(i)` to enumerate.
    pub fn set_expanded(&mut self, i: usize, on: bool) -> Option<usize> {
        if self.nodes.get(i).is_some_and(|n| n.dir && n.expanded != on) {
            self.toggle(i)
        } else {
            None
        }
    }

    /// Parent folder of node `i`.
    pub fn parent(&self, i: usize) -> Option<usize> {
        self.nodes.iter().position(|n| n.kids.contains(&i))
    }

    /// Visible rows in order.
    pub fn rows(&self) -> Vec<Row> {
        let mut out = Vec::new();
        self.push_rows(0, &mut out);
        out
    }

    fn push_rows(&self, at: usize, out: &mut Vec<Row>) {
        let Some(n) = self.nodes.get(at) else { return };
        let depth = if at == 0 { 0 } else { n.depth + 1 };
        if at != 0 && n.depth >= MAX_DEPTH {
            out.push(Row::More(at, depth));
            return;
        }
        for &k in &n.kids {
            out.push(Row::Node(k));
            if self.nodes.get(k).is_some_and(|c| c.dir && c.expanded) {
                self.push_rows(k, out);
            }
        }
        if n.more {
            out.push(Row::More(at, depth));
        }
    }

    /// Folders to watch: the root and expanded, loaded folders (at most [`MAX_WATCHED`]).
    pub fn watched(&self) -> Vec<String> {
        let mut out = vec![self.root().to_string()];
        for r in self.rows() {
            if let Row::Node(i) = r
                && let Some(n) = self.nodes.get(i)
                && n.dir
                && n.expanded
                && n.loaded
                && out.len() < MAX_WATCHED
            {
                out.push(n.path.clone());
            }
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn e(name: &str, dir: bool) -> Entry {
        Entry {
            name: name.into(),
            dir,
        }
    }

    #[test]
    fn natural_sort_folders_first() {
        let mut v = vec![
            e("file10.md", false),
            e("file2.md", false),
            e("Zeta", true),
            e("alpha", true),
            e("File1.md", false),
            e("ırmak.md", false),
            e("Ilık.md", false),
        ];
        sort_entries(&mut v);
        let names: Vec<&str> = v.iter().map(|x| x.name.as_str()).collect();
        assert_eq!(
            names,
            [
                "alpha",
                "Zeta",
                "File1.md",
                "file2.md",
                "file10.md",
                "Ilık.md",
                "ırmak.md"
            ]
        );
        assert_eq!(natural_cmp("a02", "a2"), "a02".cmp("a2"));
        assert_eq!(natural_cmp("v1.10", "v1.9"), Ordering::Greater);
    }

    #[test]
    fn skip_rules() {
        let dir = Attrs {
            dir: true,
            ..Attrs::default()
        };
        for s in SKIP_DIRS {
            assert!(skipped(s, dir), "{s}");
        }
        assert!(skipped("Node_Modules", dir));
        assert!(skipped(".hidden", dir));
        assert!(!skipped("docs", dir));
        assert!(skipped(
            "docs",
            Attrs {
                reparse: true,
                ..dir
            }
        ));
        assert!(skipped(
            "docs",
            Attrs {
                hidden: true,
                ..dir
            }
        ));
        assert!(skipped(
            "a.md",
            Attrs {
                system: true,
                ..Attrs::default()
            }
        ));
        assert!(!skipped("a.md", Attrs::default()));
        assert!(!skipped("B.MARKDOWN", Attrs::default()));
        assert!(skipped("a.txt", Attrs::default()));
        assert!(skipped(".md", Attrs::default()));
    }

    #[test]
    fn tree_lazy_rows_and_limits() {
        let mut t = Tree::new("C:\\p\\");
        assert_eq!(t.root(), "C:\\p");
        t.fill(
            0,
            vec![e("b.md", false), e("sub", true), e("a.md", false)],
            false,
        );
        let rows = t.rows();
        assert_eq!(rows.len(), 3);
        let sub = t.find("C:\\p\\sub").unwrap_or(0);
        assert_eq!(rows.first(), Some(&Row::Node(sub)));
        assert_eq!(t.toggle(sub), Some(sub));
        t.fill(sub, vec![e("x.md", false)], true);
        assert_eq!(t.rows().len(), 5);
        assert!(matches!(t.rows().get(2), Some(Row::More(s, 1)) if *s == sub));
        assert_eq!(
            t.watched(),
            vec!["C:\\p".to_string(), "C:\\p\\sub".to_string()]
        );
        // Re-listing keeps the expanded folder and its children.
        t.fill(0, vec![e("sub", true), e("c.md", false)], false);
        assert_eq!(t.rows().len(), 4);
        assert_eq!(t.toggle(sub), None);
        assert_eq!(t.rows().len(), 2);
        assert_eq!(t.parent(sub), Some(0));
        let many: Vec<Entry> = (0..MAX_ENTRIES + 10)
            .map(|i| e(&format!("f{i}.md"), false))
            .collect();
        let mut big = Tree::new("C:\\q");
        big.fill(0, many, false);
        assert_eq!(big.rows().len(), MAX_ENTRIES + 1);
        assert!(matches!(big.rows().last(), Some(Row::More(0, 0))));
    }

    #[test]
    fn depth_limit_shows_more() {
        let mut t = Tree::new("C:\\d");
        let mut at = 0;
        for d in 0..=MAX_DEPTH {
            t.fill(at, vec![e(&format!("l{d}"), true)], false);
            let k = t
                .nodes
                .get(at)
                .and_then(|n| n.kids.first().copied())
                .unwrap_or(0);
            let load = t.toggle(k);
            assert_eq!(load.is_some(), d < MAX_DEPTH, "depth {d}");
            at = k;
        }
        assert!(matches!(t.rows().last(), Some(Row::More(_, _))));
    }
}
