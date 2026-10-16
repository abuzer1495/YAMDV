//! `cargo xtask gen-icons`: original app and document icons (UI_SPEC §B.14), drawn from code with
//! tiny-skia (xtask only) and written as multi-resolution `.ico` files. No external image assets.

use tiny_skia::{
    Color, FillRule, GradientStop, LineCap, LineJoin, LinearGradient, Paint, PathBuilder, Pixmap,
    Point, Shader, SpreadMode, Stroke, Transform,
};

/// Icon sizes (B.14).
pub const SIZES: &[u32] = &[16, 20, 24, 32, 40, 48, 64, 128, 256];

fn rgb(hex: u32) -> Color {
    let [_, r, g, b] = hex.to_be_bytes();
    Color::from_rgba8(r, g, b, 255)
}

fn rounded_rect(x: f32, y: f32, w: f32, h: f32, r: f32) -> Option<tiny_skia::Path> {
    let mut pb = PathBuilder::new();
    let k = 0.552_284_8 * r;
    pb.move_to(x + r, y);
    pb.line_to(x + w - r, y);
    pb.cubic_to(x + w - r + k, y, x + w, y + r - k, x + w, y + r);
    pb.line_to(x + w, y + h - r);
    pb.cubic_to(x + w, y + h - r + k, x + w - r + k, y + h, x + w - r, y + h);
    pb.line_to(x + r, y + h);
    pb.cubic_to(x + r - k, y + h, x, y + h - r + k, x, y + h - r);
    pb.line_to(x, y + r);
    pb.cubic_to(x, y + r - k, x + r - k, y, x + r, y);
    pb.close();
    pb.finish()
}

/// Draw the blue rounded square with the white "Y↓" mark into `pm` at (`x`,`y`) with size `s`.
fn draw_badge(pm: &mut Pixmap, x: f32, y: f32, s: f32, simplified: bool) -> Option<()> {
    let bg = rounded_rect(x, y, s, s, 0.22 * s)?;
    let mut paint = Paint {
        anti_alias: true,
        ..Paint::default()
    };
    if simplified {
        paint.set_color(rgb(0x2563eb));
    } else {
        paint.shader = LinearGradient::new(
            Point::from_xy(x, y),
            Point::from_xy(x, y + s),
            vec![
                GradientStop::new(0.0, rgb(0x2563eb)),
                GradientStop::new(1.0, rgb(0x1d4ed8)),
            ],
            SpreadMode::Pad,
            Transform::identity(),
        )
        .unwrap_or(Shader::SolidColor(rgb(0x2563eb)));
    }
    pm.fill_path(&bg, &paint, FillRule::Winding, Transform::identity(), None);

    let p = |fx: f32, fy: f32| (x + fx * s, y + fy * s);
    let mut pb = PathBuilder::new();
    // "Y": two arms meeting at the fork, plus the stem (left ~60 %).
    let (ax, ay) = p(0.17, 0.25);
    let (fx, fy) = p(0.37, 0.50);
    let (bx, by) = p(0.57, 0.25);
    let (sx, sy) = p(0.37, 0.78);
    pb.move_to(ax, ay);
    pb.line_to(fx, fy);
    pb.line_to(bx, by);
    pb.move_to(fx, fy);
    pb.line_to(sx, sy);
    // Down arrow (right ~40 %): stem + chevron.
    let (tx, ty) = p(0.76, 0.25);
    let (hx, hy) = p(0.76, 0.74);
    let (lx, ly) = p(0.65, 0.61);
    let (rx, ry) = p(0.87, 0.61);
    pb.move_to(tx, ty);
    pb.line_to(hx, hy);
    pb.move_to(lx, ly);
    pb.line_to(hx, hy);
    pb.line_to(rx, ry);
    let path = pb.finish()?;
    let stroke = Stroke {
        width: if simplified { 0.13 * s } else { 0.085 * s },
        line_cap: LineCap::Round,
        line_join: LineJoin::Round,
        ..Stroke::default()
    };
    let mut white = Paint {
        anti_alias: true,
        ..Paint::default()
    };
    white.set_color(Color::WHITE);
    pm.stroke_path(&path, &white, &stroke, Transform::identity(), None);
    Some(())
}

/// Render the app icon at `size` px.
pub fn app_icon(size: u32) -> Option<Pixmap> {
    let mut pm = Pixmap::new(size, size)?;
    #[allow(clippy::cast_precision_loss)]
    let s = size as f32;
    draw_badge(&mut pm, 0.0, 0.0, s, size <= 16)?;
    Some(pm)
}

/// Render the document icon at `size` px.
pub fn doc_icon(size: u32) -> Option<Pixmap> {
    let mut pm = Pixmap::new(size, size)?;
    #[allow(clippy::cast_precision_loss)]
    let s = size as f32;
    let (l, t, r, b) = (0.16 * s, 0.04 * s, 0.84 * s, 0.96 * s);
    let fold = 0.22 * s;
    let mut pb = PathBuilder::new();
    pb.move_to(l, t);
    pb.line_to(r - fold, t);
    pb.line_to(r, t + fold);
    pb.line_to(r, b);
    pb.line_to(l, b);
    pb.close();
    let page = pb.finish()?;
    let mut paint = Paint {
        anti_alias: true,
        ..Paint::default()
    };
    paint.set_color(Color::WHITE);
    pm.fill_path(
        &page,
        &paint,
        FillRule::Winding,
        Transform::identity(),
        None,
    );
    let mut fb = PathBuilder::new();
    fb.move_to(r - fold, t);
    fb.line_to(r - fold, t + fold);
    fb.line_to(r, t + fold);
    let fold_path = fb.finish()?;
    let line_w = (s / 32.0).max(1.0);
    let stroke = Stroke {
        width: line_w,
        line_join: LineJoin::Round,
        ..Stroke::default()
    };
    paint.set_color(rgb(0xd0d0d0));
    pm.stroke_path(&page, &paint, &stroke, Transform::identity(), None);
    pm.stroke_path(&fold_path, &paint, &stroke, Transform::identity(), None);
    let badge = 0.46 * s;
    draw_badge(&mut pm, (s - badge) / 2.0, 0.44 * s, badge, size <= 24)?;
    Some(pm)
}

/// Encode RGBA (straight alpha) pixels as PNG.
#[cfg_attr(not(windows), allow(dead_code))]
pub fn encode_png(w: u32, h: u32, rgba: Vec<u8>) -> Result<Vec<u8>, String> {
    let size = tiny_skia::IntSize::from_wh(w, h).ok_or("bad size")?;
    let pm = Pixmap::from_vec(premultiply(rgba), size).ok_or("bad pixel buffer")?;
    pm.encode_png().map_err(|e| e.to_string())
}

/// Write RGBA (straight alpha) pixels to a PNG file.
#[cfg_attr(not(windows), allow(dead_code))]
pub fn write_png(path: &str, w: u32, h: u32, rgba: Vec<u8>) -> Result<(), String> {
    std::fs::write(path, encode_png(w, h, rgba)?).map_err(|e| format!("{path}: {e}"))
}

#[cfg_attr(not(windows), allow(dead_code))]
fn premultiply(mut rgba: Vec<u8>) -> Vec<u8> {
    for px in rgba.as_chunks_mut::<4>().0 {
        let a = u16::from(px.get(3).copied().unwrap_or(255));
        for c in px.iter_mut().take(3) {
            *c = u8::try_from((u16::from(*c) * a + 127) / 255).unwrap_or(255);
        }
    }
    rgba
}

/// Straight-alpha BGRA rows (top-down) from a premultiplied tiny-skia pixmap.
fn bgra_straight(pm: &Pixmap) -> Vec<u8> {
    let mut out = Vec::with_capacity(pm.data().len());
    for p in pm.pixels() {
        let c = p.demultiply();
        out.extend_from_slice(&[c.blue(), c.green(), c.red(), c.alpha()]);
    }
    out
}

/// Build an `.ico` file: sizes < 256 as 32-bit DIB with AND mask, 256 as PNG (B.14).
pub fn build_ico(images: &[Pixmap]) -> Result<Vec<u8>, String> {
    let mut entries: Vec<(u32, Vec<u8>)> = Vec::new();
    for pm in images {
        let s = pm.width();
        let data = if s >= 256 {
            pm.encode_png().map_err(|e| e.to_string())?
        } else {
            let mut d = Vec::new();
            d.extend_from_slice(&40u32.to_le_bytes());
            d.extend_from_slice(&s.to_le_bytes());
            d.extend_from_slice(&(s * 2).to_le_bytes());
            d.extend_from_slice(&1u16.to_le_bytes());
            d.extend_from_slice(&32u16.to_le_bytes());
            d.extend_from_slice(&[0u8; 24]);
            let bgra = bgra_straight(pm);
            let row = s as usize * 4;
            for y in (0..s as usize).rev() {
                d.extend_from_slice(bgra.get(y * row..(y + 1) * row).ok_or("row")?);
            }
            let mask_row = (s as usize).div_ceil(32) * 4;
            d.extend(std::iter::repeat_n(0u8, mask_row * s as usize));
            d
        };
        entries.push((s, data));
    }
    let count = u16::try_from(entries.len()).map_err(|e| e.to_string())?;
    let mut out = Vec::new();
    out.extend_from_slice(&[0, 0, 1, 0]);
    out.extend_from_slice(&count.to_le_bytes());
    let mut offset = 6 + 16 * entries.len();
    for (s, data) in &entries {
        let dim = if *s >= 256 {
            0
        } else {
            u8::try_from(*s).map_err(|e| e.to_string())?
        };
        out.extend_from_slice(&[dim, dim, 0, 0]);
        out.extend_from_slice(&1u16.to_le_bytes());
        out.extend_from_slice(&32u16.to_le_bytes());
        out.extend_from_slice(
            &u32::try_from(data.len())
                .map_err(|e| e.to_string())?
                .to_le_bytes(),
        );
        out.extend_from_slice(
            &u32::try_from(offset)
                .map_err(|e| e.to_string())?
                .to_le_bytes(),
        );
        offset += data.len();
    }
    for (_, data) in entries {
        out.extend_from_slice(&data);
    }
    Ok(out)
}

/// Generate `assets/icons/yamdv.ico` and `assets/icons/document.ico` (+ a 256 px preview PNG each).
pub fn generate(root: &std::path::Path) -> Result<Vec<String>, String> {
    let dir = root.join("assets").join("icons");
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let mut written = Vec::new();
    for (name, f) in [
        ("yamdv", app_icon as fn(u32) -> Option<Pixmap>),
        ("document", doc_icon),
    ] {
        let images: Vec<Pixmap> = SIZES
            .iter()
            .map(|&s| f(s).ok_or("render failed"))
            .collect::<Result<_, _>>()?;
        let ico = build_ico(&images)?;
        let path = dir.join(format!("{name}.ico"));
        std::fs::write(&path, ico).map_err(|e| e.to_string())?;
        written.push(path.display().to_string());
        let preview = dir.join(format!("{name}-256.png"));
        if let Some(big) = images.last() {
            std::fs::write(&preview, big.encode_png().map_err(|e| e.to_string())?)
                .map_err(|e| e.to_string())?;
            written.push(preview.display().to_string());
        }
    }
    Ok(written)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn r_sys_07_icons_render_all_sizes() {
        for &s in SIZES {
            let a = app_icon(s).unwrap();
            assert_eq!(a.width(), s);
            // Corner is transparent (rounded square), center is opaque.
            assert_eq!(a.pixel(0, 0).unwrap().alpha(), 0);
            assert_eq!(a.pixel(s / 2, s - 2).unwrap().alpha(), 255);
            assert!(doc_icon(s).is_some());
        }
    }

    #[test]
    fn ico_container_layout() {
        let imgs: Vec<Pixmap> = [16, 256].iter().map(|&s| app_icon(s).unwrap()).collect();
        let ico = build_ico(&imgs).unwrap();
        assert_eq!(&ico[0..6], &[0, 0, 1, 0, 2, 0]);
        assert_eq!(ico[6], 16); // first entry width
        assert_eq!(ico[6 + 16], 0); // 256 encoded as 0
        let off = u32::from_le_bytes(ico[6 + 16 + 12..6 + 16 + 16].try_into().unwrap()) as usize;
        assert_eq!(&ico[off..off + 4], b"\x89PNG");
    }
}
