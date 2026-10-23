//! Driver verbs (`ui nav-…`) and state keys of the V2-D features (dev-tools only).

use super::*;
use crate::window::driver::rel;

/// Temp tree of the `files-favorites` script.
fn make_tree(root: &std::path::Path) -> std::io::Result<()> {
    let _ = std::fs::remove_dir_all(root);
    for d in ["sub", "node_modules", ".hidden"] {
        std::fs::create_dir_all(root.join(d))?;
    }
    for (f, t) in [
        ("file10.md", "# Ten\n"),
        ("file2.md", "# Two\n"),
        ("notes.txt", "not markdown\n"),
        ("sub\\a.md", "# A\n"),
        ("node_modules\\x.md", "# skipped\n"),
        (".hidden\\h.md", "# hidden\n"),
    ] {
        std::fs::write(root.join(f), t)?;
    }
    // Developer tooling only: mark the folder hidden (the attribute, not just the dot name).
    let _ = std::process::Command::new("attrib")
        .arg("+h")
        .arg(root.join(".hidden"))
        .output();
    let link = root.join("junction");
    let target = root.join("sub");
    // Developer tooling only: a directory junction (reparse point) the tree must skip.
    let _ = std::process::Command::new("cmd")
        .args(["/c", "mklink", "/J"])
        .arg(&link)
        .arg(&target)
        .output();
    Ok(())
}

thread_local! {
    /// Last `nav-tab-walk`: visited stop kinds and whether their y never decreased.
    static WALK: std::cell::RefCell<(String, bool)> = const {
        std::cell::RefCell::new((String::new(), true))
    };
}

impl Win {
    /// `ui nav-<verb> <arg>`.
    pub(in crate::window) fn nav_ui(&mut self, sh: &Shared, verb: &str, arg: &str) -> String {
        match verb {
            "nav-tree" => {
                let root = std::env::temp_dir().join(arg);
                match make_tree(&root) {
                    Ok(()) => {
                        let f = root.join("file2.md").to_string_lossy().into_owned();
                        self.open(sh, &f, None, false);
                        String::new()
                    }
                    Err(e) => format!("FAIL ui nav-tree: {e}"),
                }
            }
            "nav-touch" => {
                let root = self
                    .nav
                    .files
                    .borrow()
                    .as_ref()
                    .map(|f| f.tree.root().to_string());
                match root {
                    Some(r) => match std::fs::write(format!("{r}\\{arg}"), "# new\n") {
                        Ok(()) => String::new(),
                        Err(e) => format!("FAIL ui nav-touch: {e}"),
                    },
                    None => "FAIL ui nav-touch: no Files tree".into(),
                }
            }
            "nav-drop-folder" => {
                let p = if arg.contains(':') {
                    arg.to_string()
                } else {
                    rel(arg)
                };
                self.files_open_folder(sh, &p);
                String::new()
            }
            "nav-wait-files" => {
                // Polls until the Files tab lists `arg` (FUT-33: updates within 1 s).
                if self.files_describe().contains(arg) {
                    String::new()
                } else {
                    crate::window::driver::again(50)
                }
            }
            "nav-tab" => {
                let t = arg.parse::<u8>().unwrap_or(0);
                self.side_select_tab(sh, t);
                String::new()
            }
            "nav-fav-add" => {
                let p = if arg.contains(':') {
                    arg.to_string()
                } else {
                    rel(arg)
                };
                app::with(|a| {
                    if !a.state.favorites.contains(&p) {
                        a.state.favorites.push(p.clone());
                    }
                });
                app::save_state();
                self.invalidate();
                String::new()
            }
            "nav-fav-drag" => {
                // "from to": drag favorite row `from` onto the slot before row `to` (0-based).
                let mut it = arg
                    .split_whitespace()
                    .filter_map(|x| x.parse::<usize>().ok());
                let (Some(from), Some(to)) = (it.next(), it.next()) else {
                    return "FAIL ui nav-fav-drag: from to".into();
                };
                let rows: Vec<Rect> = self
                    .side_rows(sh)
                    .into_iter()
                    .filter(|r| matches!(r.kind, crate::window::sidebar::RowKind::Fav(_)))
                    .map(|r| r.rect)
                    .collect();
                let (Some(a), Some(b)) = (rows.get(from), rows.get(to).or(rows.last())) else {
                    return "FAIL ui nav-fav-drag: no such row".into();
                };
                let ty = if to >= rows.len() {
                    b.y + b.h + 2.0
                } else {
                    b.y + 2.0
                };
                let _ = self.side_down(sh, a.x + 20.0, a.y + a.h / 2.0);
                self.side_move(sh, a.x + 20.0, a.y + a.h / 2.0 + 6.0);
                self.side_move(sh, a.x + 20.0, ty);
                self.mouse_up();
                String::new()
            }
            "nav-mind-click" | "nav-mind-dblclick" => {
                if self.mind_click_text(sh, arg, verb == "nav-mind-dblclick") {
                    String::new()
                } else {
                    format!("FAIL ui {verb}: no node {arg}")
                }
            }
            "nav-mind-mode" => {
                self.mind_mode(arg == "balanced");
                String::new()
            }
            "nav-tab-to" => {
                // "Kind [source text]": Tab until a stop of that kind in a block whose source
                // contains the text (at most 60 steps).
                let (kind, text) = arg.split_once(' ').unwrap_or((arg, ""));
                self.nav_kbd(true);
                for _ in 0..60 {
                    self.doc_focus_step(sh, 1);
                    let Some((k, _, b)) = self.focus_probe(sh) else {
                        continue;
                    };
                    let src = match &self.view {
                        View::Doc(d) => {
                            let r = d.content.range(b);
                            d.content
                                .source()
                                .get(r.start as usize..r.end as usize)
                                .unwrap_or("")
                                .to_string()
                        }
                        _ => String::new(),
                    };
                    if k == kind && src.contains(text) {
                        return String::new();
                    }
                }
                format!("FAIL ui nav-tab-to: no {arg}")
            }
            "nav-region" => {
                self.nav_kbd(true);
                self.nav.region = match arg {
                    "sidebar" => {
                        self.side_focus_first(sh);
                        Region::Sidebar
                    }
                    "titlebar" => Region::TitleBar,
                    "banners" => Region::Banners,
                    _ => Region::Document,
                };
                self.invalidate();
                String::new()
            }
            "nav-tab-walk" => {
                let n: usize = arg.parse().unwrap_or(1);
                self.nav_kbd(true);
                let (mut names, mut mono, mut last) = (Vec::new(), true, f64::MIN);
                for _ in 0..n {
                    self.doc_focus_step(sh, 1);
                    if let Some((k, y, _)) = self.focus_probe(sh) {
                        mono &= y + 4.0 >= last;
                        last = y;
                        names.push(k);
                    }
                }
                WALK.with(|w| *w.borrow_mut() = (names.join(">"), mono));
                String::new()
            }
            "nav-state-reload" => {
                // Restart check: the saved order is what a new process would read.
                let saved = app::reload_state_favorites();
                saved.join("|")
            }
            _ => format!("FAIL ui {verb}: unknown verb"),
        }
    }

    /// State keys for `assert` / `dump`.
    pub(in crate::window) fn nav_state(&self, sh: &Shared) -> Vec<(&'static str, String)> {
        let favs = app::with(|a| a.state.favorites.clone()).unwrap_or_default();
        let fav_names: Vec<String> = favs
            .iter()
            .map(|p| crate::window::sidebar::file_name(p))
            .collect();
        let region = match self.nav.region {
            Region::Document => "document",
            Region::TitleBar => "titlebar",
            Region::Banners => "banners",
            Region::Sidebar => "sidebar",
        };
        let rows = self.side_rows(sh);
        let side_focus = self
            .nav
            .side_focus
            .and_then(|i| rows.get(i))
            .map(|r| r.text.clone())
            .unwrap_or_else(|| "none".into());
        let kf = match &self.nav.kf {
            Some((b, s)) => format!("{b}:{s:?}"),
            None => "none".into(),
        };
        let mut v = vec![
            ("files", self.files_describe()),
            ("side_tab", self.tab().to_string()),
            ("side_tabs", self.side_tab_list(sh).len().to_string()),
            ("side_float", u8::from(sh.side_float).to_string()),
            (
                "side_rect",
                self.side_rect(sh).map_or("none".into(), |r| {
                    format!("{} {} {} {}", r.x, r.y, r.w, r.h)
                }),
            ),
            ("doc_area_w", self.doc_area(sh).1.to_string()),
            ("fav_order", fav_names.join("|")),
            ("region", region.into()),
            ("side_focus", side_focus),
            ("kfocus", kf),
        ];
        let toc_links = match &self.view {
            View::Doc(d) => match &d.content {
                Content::Markdown(doc) => (0..doc.index.blocks.len())
                    .filter(|&i| {
                        doc.index.blocks.get(i).is_some_and(|b| {
                            doc.source
                                .get(b.range.start as usize..b.range.end as usize)
                                .is_some_and(yamdv_core::doc::toc::is_marker)
                        })
                    })
                    .map(|i| doc.detail_uncached(i).links.len())
                    .sum::<usize>(),
                _ => 0,
            },
            _ => 0,
        };
        v.push(("toc_links", toc_links.to_string()));
        let (walk, mono) = WALK.with(|w| w.borrow().clone());
        v.push(("tab_walk", walk));
        v.push(("tab_walk_mono", u8::from(mono).to_string()));
        v.push(("kbd", u8::from(self.nav.kbd).to_string()));
        v.push((
            "clipboard",
            crate::sys::paste_text(self.hwnd)
                .unwrap_or_default()
                .chars()
                .take(120)
                .collect::<String>()
                .replace(['\r', '\n'], " "),
        ));
        v.extend(self.palette_state());
        v.extend(self.mind_state());
        v
    }
}
