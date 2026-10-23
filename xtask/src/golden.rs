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
pub const DOCS: [(&str, &str); 21] = [
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
    ("typical", "typical.md"),
    ("tables", "tables.md"),
    ("callouts", "callouts.md"),
    ("details", "details.md"),
    ("plaintext", "plain.txt"),
    ("math", "math.md"),
    ("mermaid", "mermaid.md"),
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
                if scale == "1.5" && !SCALE_15_DOCS.contains(&name) {
                    continue;
                }
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
                let tiles = tile(&crop(&load(&target)?, scale)?)?;
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
                if !gold.join(format!("{stem}.t0.png")).exists() {
                    // A golden added by a phase is committed from its first CI artifact.
                    println!("::warning::{png}: not committed yet");
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

/// Window-level goldens of prompt 005 (welcome, sidebar, find bar, scheme), produced by the
/// driver script `tests/golden/v1a/shots.yds` through the same draw path as `WM_PAINT`.
/// V1-B adds the Split Edit and Live Edit (active source block) shots (prompt 006 §2.7).
pub const SHELL_SHOTS: [&str; 21] = [
    // V1-D: the welcome page gained [Install YAMDV for this user] (prompt 008 §4).
    "welcome-install-light.png",
    "doc-sidebar-light.png",
    "doc-sidebar-dark.png",
    "find-light.png",
    "glacier-light.png",
    "split-light.png",
    "live-block-light.png",
    // V2-A (prompt 011 §4): custom title bar, layered menu, scheme cards.
    "top-light.png",
    "top-dark.png",
    "menu-light.png",
    "cards-light.png",
    // V2-C (prompt 013 §4): tooltip, banner + popover, image viewer, BUG-08 wide diagram.
    "top-v2c-footnote-light.png",
    "top-v2c-motw-dark.png",
    "top-v2c-image-light.png",
    "top-v2c-wide-light.png",
    "top-v2d-files-light.png",
    "top-v2d-floating-dark.png",
    "top-v2d-palette-light.png",
    "top-v2d-mindmap-light.png",
    // V2-F (prompt 016 §4): F2 inline reveal, F2 table with its tool strip and grips.
    "top-v2f-reveal-light.png",
    "top-v2f-table-dark.png",
];

/// `golden-shell --exe <yamdv.exe (dev-tools)> [--out dir] [--update]`: runs the shot script with
/// a fresh, empty profile (APPDATA / LOCALAPPDATA in the output folder) and compares the PNGs
/// with `tests/golden/v1a/` (same thresholds as `golden`). No committed shots yet: warns only.
pub fn shell(root: &Path, args: &[String]) -> Result<(), String> {
    let mut exe = None;
    let mut out = root.join("target").join("golden-shell");
    let mut update = false;
    let mut it = args.iter();
    while let Some(a) = it.next() {
        match a.as_str() {
            "--exe" => exe = it.next().map(PathBuf::from),
            "--out" => out = it.next().map(PathBuf::from).ok_or("--out needs a value")?,
            "--update" => update = true,
            s => return Err(format!("unknown option {s}")),
        }
    }
    let exe = exe.ok_or("usage: golden-shell --exe <yamdv.exe> [--out dir] [--update]")?;
    let profile = out.join("profile");
    let _ = std::fs::remove_dir_all(&profile);
    std::fs::create_dir_all(&profile).map_err(|e| e.to_string())?;
    let dir = root.join("tests").join("golden").join("v1a");
    let status = std::process::Command::new(&exe)
        .env("APPDATA", &profile)
        .env("LOCALAPPDATA", &profile)
        .env("YAMDV_SNAPSHOT_DIR", &out)
        .arg("--script")
        .arg(dir.join("shots.yds"))
        .status()
        .map_err(|e| format!("{}: {e}", exe.display()))?;
    if let Ok(log) = std::fs::read_to_string(dir.join("shots.yds.log")) {
        println!("{log}");
    }
    if !status.success() {
        return Err(format!("shot script failed ({status})"));
    }
    let committed = SHELL_SHOTS.iter().any(|n| dir.join(n).exists());
    let mut failures = Vec::new();
    for name in SHELL_SHOTS {
        let got = load(&out.join(name))?;
        if update {
            got.save_png(dir.join(name)).map_err(|e| e.to_string())?;
            println!("updated {name}");
            continue;
        }
        if !committed {
            continue;
        }
        let g = dir.join(name);
        if !g.exists() {
            // A shot added by a later phase is committed from the first CI artifact.
            println!("::warning::{name}: not committed yet");
            continue;
        }
        let b = load(&g)?;
        let d = compare(
            (got.width(), got.height(), got.data()),
            (b.width(), b.height(), b.data()),
        );
        println!(
            "{name}: {} / {} px differ — {}",
            d.differing,
            d.total,
            if d.ok() { "OK" } else { "FAIL" }
        );
        if !d.ok() {
            failures.push(name);
        }
    }
    if !committed && !update {
        println!("::warning::no committed shell goldens yet (tests/golden/v1a)");
    }
    if failures.is_empty() {
        Ok(())
    } else {
        Err(format!("shell golden failures: {}", failures.join(", ")))
    }
}

/// ACG matrix formats (prompt 004 §4.1 item 4, ADR-0022 rows): `tests/corpus/acg/<row>.md` each
/// shows one 64×48 (ICO 48×48) solid `#C03020` image.
pub const ACG_FORMATS: [&str; 10] = [
    "png",
    "jpg",
    "gif",
    "bmp",
    "ico-png",
    "ico-bmp",
    "tiff",
    "webp-lossy",
    "webp-lossless",
    "webp-alpha",
];

/// ADR-0022: WebP shows the "WebP codec not installed" placeholder, so it must not decode.
pub const ACG_PLACEHOLDER: [&str; 3] = ["webp-lossy", "webp-lossless", "webp-alpha"];

/// `acg-matrix --exe <yamdv.exe with dev-tools> [--out <dir>]`: renders every ACG document
/// (the process runs with ProhibitDynamicCode, S-PROC-04) and checks that the image pixels
/// arrived (centre pixel of the image ≈ `#C03020`). Prints a Markdown table; fails on a miss.
pub fn acg_matrix(root: &Path, args: &[String]) -> Result<(), String> {
    let mut exe = None;
    let mut out = root.join("target").join("acg");
    let mut it = args.iter();
    while let Some(a) = it.next() {
        match a.as_str() {
            "--exe" => exe = it.next().map(PathBuf::from),
            "--out" => out = it.next().map(PathBuf::from).unwrap_or(out),
            x => return Err(format!("acg-matrix: unknown argument {x}")),
        }
    }
    let exe = exe.ok_or("acg-matrix: --exe required")?;
    std::fs::create_dir_all(&out).map_err(|e| e.to_string())?;
    let mut bad = Vec::new();
    println!("| format | decoded under ACG | centre pixel |");
    println!("|---|---|---|");
    for f in ACG_FORMATS {
        let png = out.join(format!("acg-{f}.png"));
        let input = root
            .join("tests")
            .join("corpus")
            .join("acg")
            .join(format!("{f}.md"));
        let st = std::process::Command::new(&exe)
            .arg("--render-png")
            .arg(&png)
            .args(["--width", "1000", "--scale", "1.0", "--theme", "light"])
            .arg(&input)
            .status()
            .map_err(|e| e.to_string())?;
        let px = if st.success() { load(&png).ok() } else { None };
        // Column 650 px centred in 1000; the image is centred in the column, top at the first
        // block (y ≈ 24…40): scan the middle column for the colour.
        let hit = px.as_ref().and_then(|p| {
            let x = p.width() / 2;
            (0..p.height().min(200)).find_map(|y| {
                let i = ((y * p.width() + x) * 4) as usize;
                let [r, g, b]: [u8; 3] = p.data().get(i..i + 3)?.try_into().ok()?;
                let near = |a: u8, b: u8| a.abs_diff(b) <= 24;
                (near(r, 0xC0) && near(g, 0x30) && near(b, 0x20))
                    .then(|| format!("#{r:02X}{g:02X}{b:02X} @y={y}"))
            })
        });
        let placeholder = ACG_PLACEHOLDER.contains(&f);
        println!(
            "| {f} | {} | {} |",
            match (hit.is_some(), placeholder) {
                (true, _) => "yes",
                (false, true) => "placeholder (ADR-0022)",
                (false, false) => "NO",
            },
            hit.as_deref().unwrap_or("-")
        );
        if hit.is_some() == placeholder {
            bad.push(f);
        }
    }
    if bad.is_empty() {
        Ok(())
    } else {
        Err(format!("acg-matrix: unexpected result: {}", bad.join(", ")))
    }
}

/// Documents that also get scale-1.5 goldens (P-28).
pub const SCALE_15_DOCS: [&str; 2] = ["typical", "unicode"];
/// Goldens cover at most this many logical px of each document (P-28).
pub const MAX_LOGICAL_ROWS: u32 = 4000;

/// Crops an image to the first [`MAX_LOGICAL_ROWS`] logical px at `scale`.
pub fn crop(p: &tiny_skia::Pixmap, scale: &str) -> Result<tiny_skia::Pixmap, String> {
    let s: f32 = scale.parse().map_err(|_| format!("bad scale {scale}"))?;
    #[allow(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        clippy::cast_precision_loss
    )]
    let max = (MAX_LOGICAL_ROWS as f32 * s).round() as u32;
    if p.height() <= max {
        return Ok(p.clone());
    }
    p.clone_rect(tiny_skia::IntRect::from_xywh(0, 0, p.width(), max).ok_or("crop rect")?)
        .ok_or_else(|| "crop failed".to_string())
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
    fn crop_limits_rows() {
        let p = tiny_skia::Pixmap::new(2, 9000).unwrap();
        assert_eq!(crop(&p, "1.0").unwrap().height(), 4000);
        assert_eq!(crop(&p, "1.5").unwrap().height(), 6000);
        let q = tiny_skia::Pixmap::new(2, 10).unwrap();
        assert_eq!(crop(&q, "1.5").unwrap().height(), 10);
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
