//! `cargo xtask golden` (Appendix H §H.4) and `cargo xtask gen-large`.
//!
//! `golden --exe <yamdv.exe built with dev-tools> [--out <dir>] [--update]` renders every golden
//! document through `yamdv --render-png` at width 1000, scales 1.0/1.5, light/dark, and compares it
//! with `tests/golden/002/` (per-channel tolerance 16, at most 0.5 % differing pixels); `--update`
//! rewrites the goldens instead.

use std::path::{Path, PathBuf};

/// Per-channel tolerance.
pub const TOLERANCE: u8 = 16;
/// Maximum share of differing pixels.
pub const MAX_DIFF_SHARE: f64 = 0.005;
/// Goldens are stored in horizontal tiles of at most this many rows (keeps every file < 1 MB).
pub const TILE_ROWS: u32 = 5000;

/// Golden documents (corpus file names in `tests/corpus/002/`).
pub const DOCS: [(&str, &str); 18] = [
    ("typical-basic", "typical-basic.md"),
    ("headings", "headings.md"),
    ("lists", "lists.md"),
    ("quotes", "quotes.md"),
    ("code", "code.md"),
    ("unicode", "unicode.md"),
    ("placeholders", "placeholders.md"),
    ("footnotes", "footnotes.md"),
    ("frontmatter", "frontmatter.md"),
    ("deflist", "deflist.md"),
    ("inline", "inline.md"),
    ("html", "html.md"),
    ("links", "links.md"),
    ("images", "images.md"),
    ("tables", "tables.md"),
    ("callouts", "callouts.md"),
    ("details", "details.md"),
    ("plaintext", "plain.txt"),
];

/// Result of comparing two RGBA images.
#[derive(Debug, Clone, PartialEq)]
pub struct Diff {
    /// Pixels where any channel differs by more than [`TOLERANCE`] (all pixels if sizes differ).
    pub differing: usize,
    /// Total pixels.
    pub total: usize,
    /// Sizes match.
    pub same_size: bool,
}

impl Diff {
    /// Within the thresholds.
    #[allow(clippy::cast_precision_loss)]
    pub fn ok(&self) -> bool {
        self.same_size && (self.differing as f64) <= MAX_DIFF_SHARE * self.total as f64
    }
}

/// Compares RGBA buffers of `(w, h)`.
pub fn compare(a: (u32, u32, &[u8]), b: (u32, u32, &[u8])) -> Diff {
    if a.0 != b.0 || a.1 != b.1 || a.2.len() != b.2.len() {
        let total = (a.0 as usize) * (a.1 as usize);
        return Diff {
            differing: total,
            total,
            same_size: false,
        };
    }
    let differing =
        a.2.as_chunks::<4>()
            .0
            .iter()
            .zip(b.2.as_chunks::<4>().0.iter())
            .filter(|(p, q)| {
                p.iter()
                    .zip(q.iter())
                    .any(|(x, y)| x.abs_diff(*y) > TOLERANCE)
            })
            .count();
    Diff {
        differing,
        total: a.2.len() / 4,
        same_size: true,
    }
}

fn load(p: &Path) -> Result<tiny_skia::Pixmap, String> {
    tiny_skia::Pixmap::load_png(p).map_err(|e| format!("{}: {e}", p.display()))
}

/// Runs the golden command.
pub fn run(root: &Path, args: &[String]) -> Result<(), String> {
    let mut exe = None;
    let mut out = root.join("target").join("golden-out");
    let mut update = false;
    let mut from = None;
    let mut it = args.iter();
    while let Some(a) = it.next() {
        match a.as_str() {
            "--exe" => exe = it.next().map(PathBuf::from),
            "--out" => out = it.next().map(PathBuf::from).ok_or("--out needs a value")?,
            "--update" => update = true,
            "--from" => from = it.next().map(PathBuf::from),
            s => return Err(format!("unknown option {s}")),
        }
    }
    if exe.is_none() && from.is_none() {
        return Err("usage: golden (--exe <yamdv.exe (dev-tools)> | --from <dir of rendered PNGs>) [--out dir] [--update]".into());
    }
    std::fs::create_dir_all(&out).map_err(|e| e.to_string())?;
    let gold = root.join("tests").join("golden").join("002");
    if update {
        std::fs::create_dir_all(&gold).map_err(|e| e.to_string())?;
    }
    let mut failures = Vec::new();
    for (name, file) in DOCS {
        for theme in ["light", "dark"] {
            for scale in ["1.0", "1.5"] {
                let png = format!("{name}-{theme}-{scale}.png");
                let target = match &from {
                    Some(dir) => dir.join(&png),
                    None => out.join(&png),
                };
                if let Some(exe) = &exe {
                    let input = root.join("tests").join("corpus").join("002").join(file);
                    let status = std::process::Command::new(exe)
                        .arg("--render-png")
                        .arg(&target)
                        .args(["--width", "1000", "--scale", scale, "--theme", theme])
                        .arg(&input)
                        .status()
                        .map_err(|e| format!("{}: {e}", exe.display()))?;
                    if !status.success() {
                        failures.push(format!("{png}: render failed ({status})"));
                        continue;
                    }
                }
                let stem = png.trim_end_matches(".png").to_string();
                let tiles = tile(&load(&target)?)?;
                if update {
                    for old in std::fs::read_dir(&gold)
                        .map_err(|e| e.to_string())?
                        .flatten()
                    {
                        let n = old.file_name().to_string_lossy().to_string();
                        if n.starts_with(&format!("{stem}.t")) {
                            let _ = std::fs::remove_file(old.path());
                        }
                    }
                    for (i, t) in tiles.iter().enumerate() {
                        t.save_png(gold.join(format!("{stem}.t{i}.png")))
                            .map_err(|e| e.to_string())?;
                    }
                    println!("updated {png} ({} tiles)", tiles.len());
                    continue;
                }
                let mut d = Diff {
                    differing: 0,
                    total: 0,
                    same_size: true,
                };
                for (i, t) in tiles.iter().enumerate() {
                    let px = t.width() as usize * t.height() as usize;
                    let g = gold.join(format!("{stem}.t{i}.png"));
                    if !g.exists() {
                        d.same_size = false;
                        d.total += px;
                        d.differing += px;
                        continue;
                    }
                    let b = load(&g)?;
                    let x = compare(
                        (t.width(), t.height(), t.data()),
                        (b.width(), b.height(), b.data()),
                    );
                    d.differing += x.differing;
                    d.total += x.total;
                    d.same_size &= x.same_size;
                }
                if gold.join(format!("{stem}.t{}.png", tiles.len())).exists() {
                    d.same_size = false;
                }
                println!(
                    "{png}: {} / {} px differ{} — {}",
                    d.differing,
                    d.total,
                    if d.same_size { "" } else { " (size differs)" },
                    if d.ok() { "OK" } else { "FAIL" }
                );
                if !d.ok() {
                    failures.push(png);
                }
            }
        }
    }
    if failures.is_empty() {
        println!("golden: all OK");
        Ok(())
    } else {
        Err(format!("golden failures: {}", failures.join(", ")))
    }
}

/// Splits an image into tiles of at most [`TILE_ROWS`] rows.
pub fn tile(p: &tiny_skia::Pixmap) -> Result<Vec<tiny_skia::Pixmap>, String> {
    let row = p.width() as usize * 4;
    let mut out = Vec::new();
    let mut y = 0;
    while y < p.height() {
        let h = TILE_ROWS.min(p.height() - y);
        let mut t = tiny_skia::Pixmap::new(p.width(), h).ok_or("empty tile")?;
        let start = y as usize * row;
        let src = p
            .data()
            .get(start..start + h as usize * row)
            .ok_or("tile out of range")?;
        t.data_mut().copy_from_slice(src);
        out.push(t);
        y += h;
    }
    Ok(out)
}

/// Generates `large-1mb.md` (≥ 1 MiB, deterministic, original content) from the typical sample.
pub fn gen_large(root: &Path, args: &[String]) -> Result<(), String> {
    let out = args
        .first()
        .map(PathBuf::from)
        .unwrap_or_else(|| root.join("target").join("large-1mb.md"));
    let base = std::fs::read_to_string(root.join("tests/corpus/002/typical-basic.md"))
        .map_err(|e| e.to_string())?;
    let text = large_text(&base, 1024 * 1024);
    std::fs::write(&out, &text).map_err(|e| e.to_string())?;
    println!("wrote {} ({} bytes)", out.display(), text.len());
    Ok(())
}

/// Repeats `base` with renumbered headings until at least `min` bytes.
pub fn large_text(base: &str, min: usize) -> String {
    let mut s = String::with_capacity(min + base.len());
    let mut part = 0u32;
    while s.len() < min {
        part += 1;
        for line in base.lines() {
            if let Some(rest) = line.strip_prefix("# ") {
                s.push_str(&format!("# Part {part}: {rest}\n"));
            } else if line.starts_with('[') && line.contains("]: ") {
                // Keep reference labels unique per part.
                s.push_str(&line.replacen("[", &format!("[p{part}-"), 1));
                s.push('\n');
            } else {
                s.push_str(&line.replace("][ref", &format!("][p{part}-ref")));
                s.push('\n');
            }
        }
        s.push('\n');
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compare_thresholds() {
        let a = vec![100u8; 1000 * 4];
        let mut b = a.clone();
        assert!(compare((10, 100, &a), (10, 100, &b)).ok());
        // 5 pixels within tolerance, 5 beyond: 0.5 % of 1000 = 5 → still OK.
        for i in 0..5 {
            b[i * 4] = 116;
            b[400 + i * 4] = 117;
        }
        let d = compare((10, 100, &a), (10, 100, &b));
        assert_eq!(d.differing, 5);
        assert!(d.ok());
        b[800] = 200;
        assert!(!compare((10, 100, &a), (10, 100, &b)).ok());
        assert!(!compare((10, 100, &a), (100, 10, &a)).same_size);
    }

    #[test]
    fn tiles_cover_image() {
        let p = tiny_skia::Pixmap::new(3, TILE_ROWS * 2 + 7).unwrap();
        let t = tile(&p).unwrap();
        assert_eq!(t.len(), 3);
        assert_eq!(t[2].height(), 7);
    }

    #[test]
    fn large_is_big_and_deterministic() {
        let t = large_text("# T\n\nx [a][ref1]\n\n[ref1]: u\n", 10_000);
        assert!(t.len() >= 10_000);
        assert_eq!(t, large_text("# T\n\nx [a][ref1]\n\n[ref1]: u\n", 10_000));
        assert!(t.contains("[p2-ref1]: u"));
    }
}
