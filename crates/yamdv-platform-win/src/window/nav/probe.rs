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
        v.extend(self.palette_state());
        v.extend(self.mind_state());
        v
    }
}
