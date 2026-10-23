//! Rich paste and file drop (prompt 015 §3.5). Assets land under
//! `<doc folder>/<assets_dir>/<stem>/`; every directory step and every created file is checked
//! against the document folder after resolving its final path (S-FILE-05; `canonicalize` is
//! GetFinalPathNameByHandleW), and files are created with CREATE_NEW (`create_new`).

use super::*;
use std::io::Write as _;
use std::path::{Path, PathBuf};
use yamdv_core::paste::{self as yp, assets};

/// Largest clipboard bitmap read (bytes).
const MAX_DIB: usize = 256 * 1024 * 1024;
/// `CF_DIB` / `CF_DIBV5`.
const CF_DIB: u32 = 8;
const CF_DIBV5: u32 = 17;
/// Name attempts before giving up (`-1` … `-99`).
const TRIES: u32 = 100;

/// Where assets of the open document go.
struct Target {
    /// Resolved document folder (sandbox root).
    root: PathBuf,
    /// `assets/<stem>` relative to the document, `/`-separated.
    rel: String,
}

fn final_str(p: &Path) -> Option<String> {
    std::fs::canonicalize(p)
        .ok()
        .map(|f| f.to_string_lossy().into_owned())
}

/// Creates (or reuses) `rel` under `root` one component at a time, refusing any step whose
/// final path leaves `root` (junctions, symlinks). Returns the resolved directory.
fn sandboxed_dir(root: &Path, rel: &str) -> Option<PathBuf> {
    let root_s = final_str(root)?;
    let mut cur = PathBuf::from(&root_s);
    for c in rel.split('/').filter(|c| !c.is_empty()) {
        cur.push(c);
        if std::fs::symlink_metadata(&cur).is_err() {
            std::fs::create_dir(&cur).ok()?;
        }
        let f = final_str(&cur)?;
        if !assets::within(&root_s, &f) || !std::fs::metadata(&f).ok()?.is_dir() {
            return None;
        }
        cur = PathBuf::from(f);
    }
    Some(cur)
}

/// Writes `bytes` to a new file in `t`, trying `name(0)`, `name(1)` …; returns the file name.
fn save_new(t: &Target, name: &dyn Fn(u32) -> Option<String>, src: Src<'_>) -> Option<String> {
    let dir = sandboxed_dir(&t.root, &t.rel)?;
    let root_s = final_str(&t.root)?;
    for n in 0..TRIES {
        let nm = name(n)?;
        let p = dir.join(&nm);
        let f = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&p);
        let mut f = match f {
            Ok(f) => f,
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(_) => return None,
        };
        let ok = match src {
            Src::Bytes(b) => f.write_all(b).is_ok(),
            Src::File(from) => std::fs::File::open(from)
                .and_then(|mut r| std::io::copy(&mut r, &mut f))
                .is_ok(),
        };
        drop(f);
        // The file we just created must still resolve inside the document folder.
        if !ok || !final_str(&p).is_some_and(|fp| assets::within(&root_s, &fp)) {
            let _ = std::fs::remove_file(&p);
            return None;
        }
        return Some(nm);
    }
    None
}

#[derive(Clone, Copy)]
enum Src<'a> {
    Bytes(&'a [u8]),
    File(&'a str),
}

fn stamp() -> assets::Stamp {
    use windows::Win32::System::SystemInformation::GetLocalTime;
    // SAFETY: GetLocalTime has no preconditions.
    let t = unsafe { GetLocalTime() };
    assets::Stamp {
        y: t.wYear,
        mo: t.wMonth,
        d: t.wDay,
        h: t.wHour,
        mi: t.wMinute,
        s: t.wSecond,
    }
}

impl Win {
    /// Asset target of the open document; `None` (after a toast) for a buffer without a file.
    fn asset_target(&mut self) -> Option<Target> {
        let (path, virt) = match &self.view {
            View::Doc(d) => (d.path.clone(), d.virt),
            _ => return None,
        };
        if virt || path.is_empty() {
            self.toast(Msg::SaveFirst);
            return None;
        }
        let long = ypath::long_form(&path);
        let p = Path::new(&long);
        let stem = p.file_stem()?.to_string_lossy().into_owned();
        let rel = assets::asset_dir(&app::get("editing.assets_dir"), &stem);
        let (Some(root), Some(rel)) = (p.parent(), rel) else {
            self.toast(Msg::AssetFailed);
            return None;
        };
        Some(Target {
            root: root.to_path_buf(),
            rel,
        })
    }

    /// Ctrl+V in the edit modes: clipboard HTML → Markdown, bitmap → PNG asset, else text.
    pub(in crate::window) fn paste_rich(&mut self, sh: &Shared) {
        if !self.typing() {
            return;
        }
        let html = sys::html_format();
        if let Some(raw) = sys::clip_bytes(self.hwnd, html, yp::MAX_HTML + 4096) {
            let raw = String::from_utf8_lossy(&raw).into_owned();
            let frag = yp::cf_html_fragment(&raw).to_string();
            if let Some(md) = self.html_markdown(&frag)
                && !md.trim().is_empty()
            {
                self.with_ed(sh, |ed| ed.paste(&md));
                return;
            }
        } else if sys::paste_text(self.hwnd).is_none()
            && let Some(dib) = sys::clip_bytes(self.hwnd, CF_DIBV5, MAX_DIB)
                .or_else(|| sys::clip_bytes(self.hwnd, CF_DIB, MAX_DIB))
        {
            self.paste_bitmap(sh, &dib);
            return;
        }
        self.edit_paste(sh);
    }

    /// HTML → Markdown; `data:` images become assets (none without a document file).
    fn html_markdown(&mut self, frag: &str) -> Option<String> {
        let t = if frag.contains("data:image/") {
            self.asset_target()
        } else {
            None
        };
        let ts = stamp();
        let (mut used, mut failed) = (0u32, false);
        let mut save = |bytes: &[u8], ext: &'static str| -> Option<String> {
            let t = t.as_ref()?;
            let start = used;
            let name = |n: u32| Some(assets::image_name(ts, start + n, ext));
            let nm = save_new(t, &name, Src::Bytes(bytes));
            used += 1;
            failed |= nm.is_none();
            nm.map(|nm| format!("{}/{nm}", t.rel))
        };
        let md = yp::html2md::convert(frag, &mut save);
        if failed {
            self.toast(Msg::AssetFailed);
        }
        md
    }

    fn paste_bitmap(&mut self, sh: &Shared, dib: &[u8]) {
        let Some((w, h, px)) = yp::dib::to_bgra(dib) else {
            self.toast(Msg::AssetFailed);
            return;
        };
        let Some(t) = self.asset_target() else {
            return;
        };
        let ts = stamp();
        let name = |n: u32| Some(assets::image_name(ts, n, "png"));
        let saved = crate::snapshot::encode_png(w, h, &px)
            .ok()
            .and_then(|png| save_new(&t, &name, Src::Bytes(&png)));
        match saved {
            Some(nm) => {
                let md = assets::image_markdown(&t.rel, &nm);
                self.with_ed(sh, |ed| ed.paste(&md));
            }
            None => self.toast(Msg::AssetFailed),
        }
    }

    /// Ctrl+Shift+V: plain text only.
    pub(in crate::window) fn paste_plain(&mut self, sh: &Shared) {
        self.edit_paste(sh);
    }

    /// Files dropped on an edit mode: asks above [`assets::ASK_ABOVE`], else copies.
    pub(in crate::window) fn drop_into_doc(&mut self, sh: &Shared, files: Vec<String>) {
        let big = files.iter().any(|f| {
            std::fs::metadata(ypath::long_form(f)).is_ok_and(|m| m.len() > assets::ASK_ABOVE)
        });
        if big {
            self.ask(
                sh,
                Msg::DropLargeTitle,
                Msg::DropLargeBody,
                vec![(Msg::DropCopy, BTN_DROP_COPY), (Msg::Cancel, BTN_CANCEL)],
                Pending::Drop(files),
            );
        } else {
            self.drop_copy(sh, &files);
        }
    }

    /// Copies dropped files into the asset folder and inserts `[name](…)` links.
    pub(in crate::window) fn drop_copy(&mut self, sh: &Shared, files: &[String]) {
        let Some(t) = self.asset_target() else {
            return;
        };
        let mut md = String::new();
        let mut failed = false;
        for f in files {
            let long = ypath::long_form(f);
            let base = Path::new(&long)
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .and_then(|n| assets::sanitize_name(&n));
            let Some(base) = base else {
                failed = true;
                continue;
            };
            let (stem, ext) = match base.rsplit_once('.') {
                Some((s, e)) if !s.is_empty() => (s.to_string(), format!(".{e}")),
                _ => (base.clone(), String::new()),
            };
            let name = |n: u32| {
                Some(if n == 0 {
                    base.clone()
                } else {
                    format!("{stem}-{n}{ext}")
                })
            };
            match save_new(&t, &name, Src::File(&long)) {
                Some(nm) => {
                    if !md.is_empty() {
                        md.push('\n');
                    }
                    md.push_str(&assets::file_markdown(&t.rel, &nm));
                }
                None => failed = true,
            }
        }
        if failed {
            self.toast(Msg::AssetFailed);
        }
        if !md.is_empty() {
            self.with_ed(sh, |ed| ed.paste(&md));
        }
    }
}
