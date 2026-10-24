//! ICO directory parser (ADR-0022): picks the entry for a display size and turns it into a
//! payload WIC can decode under ACG (PNG as is; BMP with a file header and half height).

use super::MAX_FILE;

/// At most this many directory entries.
pub const MAX_ENTRIES: usize = 256;

/// One validated `ICONDIRENTRY`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Entry {
    /// Width (0 in the file means 256).
    pub w: u32,
    /// Height.
    pub h: u32,
    /// Bits per pixel (from the entry; 0 when unset).
    pub bpp: u16,
    /// Payload offset.
    pub off: usize,
    /// Payload size.
    pub len: usize,
}

/// The decodable form of an entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Payload {
    /// A PNG stream.
    Png(Vec<u8>),
    /// A 32-bit entry with its own alpha: top-down premultiplied BGRA (no codec needed).
    Bgra {
        /// `w * h * 4` bytes.
        px: Vec<u8>,
        /// Width.
        w: u32,
        /// Height.
        h: u32,
    },
    /// A complete `.bmp` file (header added, height halved); `mask_alpha` asks the caller to
    /// apply the AND mask (`mask`, 1 bpp rows padded to 4 bytes, bottom-up) as alpha.
    Bmp {
        /// The `.bmp` bytes.
        file: Vec<u8>,
        /// AND mask rows when the colors carry no alpha.
        mask: Option<Vec<u8>>,
        /// Pixel width.
        w: u32,
        /// Pixel height (halved).
        h: u32,
    },
}

fn le16(b: &[u8], o: usize) -> Option<u16> {
    Some(u16::from_le_bytes(b.get(o..o + 2)?.try_into().ok()?))
}

fn le32(b: &[u8], o: usize) -> Option<u32> {
    Some(u32::from_le_bytes(b.get(o..o + 4)?.try_into().ok()?))
}

/// Validated directory entries, or `None` when the directory is malformed.
#[must_use]
pub fn directory(b: &[u8]) -> Option<Vec<Entry>> {
    if b.len() as u64 > MAX_FILE || le16(b, 0)? != 0 || le16(b, 2)? != 1 {
        return None;
    }
    let n = usize::from(le16(b, 4)?);
    if n == 0 || n > MAX_ENTRIES {
        return None;
    }
    let mut out = Vec::with_capacity(n);
    for i in 0..n {
        let e = 6 + i * 16;
        let dim = |o: usize| {
            b.get(e + o)
                .map(|&v| if v == 0 { 256 } else { u32::from(v) })
        };
        let (w, h) = (dim(0)?, dim(1)?);
        let bpp = le16(b, e + 6)?;
        let len = le32(b, e + 8)? as usize;
        let off = le32(b, e + 12)? as usize;
        if len == 0 || off < 6 + n * 16 || off.checked_add(len)? > b.len() {
            return None;
        }
        out.push(Entry {
            w,
            h,
            bpp,
            off,
            len,
        });
    }
    Some(out)
}

/// The smallest entry covering `want` px (the larger side), else the largest; ties go to
/// the higher bit depth.
#[must_use]
pub fn pick(entries: &[Entry], want: u32) -> Option<Entry> {
    let key = |e: &Entry| (e.w.max(e.h), e.bpp);
    let fits = entries.iter().filter(|e| e.w.max(e.h) >= want);
    fits.min_by(|a, b| a.w.max(a.h).cmp(&b.w.max(b.h)).then(b.bpp.cmp(&a.bpp)))
        .or_else(|| entries.iter().max_by_key(|e| key(e)))
        .copied()
}

/// The decodable payload of entry `e` of `b`.
#[must_use]
pub fn payload(b: &[u8], e: &Entry) -> Option<Payload> {
    let p = b.get(e.off..e.off + e.len)?;
    if p.starts_with(b"\x89PNG\r\n\x1a\n") {
        return Some(Payload::Png(p.to_vec()));
    }
    let hs = le32(p, 0)? as usize;
    if hs < 40 || hs > p.len() {
        return None;
    }
    let w = le32(p, 4)?;
    let h2 = le32(p, 8)? as i32;
    let bpp = u32::from(le16(p, 14)?);
    let colors = match le32(p, 32)? {
        0 if bpp <= 8 => 1usize << bpp,
        c => c as usize,
    };
    let h = h2.unsigned_abs() / 2;
    if w == 0 || h == 0 || w > 256 || h > 256 || !matches!(bpp, 1 | 4 | 8 | 24 | 32) {
        return None;
    }
    let pal = if bpp <= 8 { colors.min(256) * 4 } else { 0 };
    let row = |bits: u32| ((w * bits).div_ceil(32) * 4) as usize;
    let xor = row(bpp) * h as usize;
    let and = row(1) * h as usize;
    let pix = hs + pal;
    let mut info = p.get(..pix)?.to_vec();
    // Height field: the XOR bitmap only (positive = bottom-up).
    info.get_mut(8..12)?
        .copy_from_slice(&(i32::try_from(h).ok()?).to_le_bytes());
    // biSizeImage = XOR size (some writers leave the combined size).
    info.get_mut(20..24)?
        .copy_from_slice(&u32::try_from(xor).ok()?.to_le_bytes());
    let colors_px = p.get(pix..pix + xor)?;
    let mut file = Vec::with_capacity(14 + pix + xor);
    file.extend_from_slice(b"BM");
    file.extend_from_slice(&u32::try_from(14 + pix + xor).ok()?.to_le_bytes());
    file.extend_from_slice(&[0, 0, 0, 0]);
    file.extend_from_slice(&u32::try_from(14 + pix).ok()?.to_le_bytes());
    file.extend_from_slice(&info);
    file.extend_from_slice(colors_px);
    // 32 bpp with any alpha keeps it; otherwise the AND mask is the alpha.
    let has_alpha = bpp == 32 && colors_px.as_chunks::<4>().0.iter().any(|px| px[3] != 0);
    if has_alpha {
        let stride = (w * 4) as usize;
        let bottom_up = h2 > 0;
        let mut px = Vec::with_capacity(colors_px.len());
        for y in 0..h as usize {
            let r = if bottom_up { h as usize - 1 - y } else { y };
            let line = colors_px.get(r * stride..(r + 1) * stride)?;
            for &[b, g, rr, a] in line.as_chunks::<4>().0 {
                let m = |c: u8| u8::try_from(u16::from(c) * u16::from(a) / 255).unwrap_or(255);
                px.extend_from_slice(&[m(b), m(g), m(rr), a]);
            }
        }
        return Some(Payload::Bgra { px, w, h });
    }
    let mask = (!has_alpha)
        .then(|| p.get(pix + xor..pix + xor + and).map(<[u8]>::to_vec))
        .flatten();
    Some(Payload::Bmp { file, mask, w, h })
}

/// Applies an AND `mask` (bottom-up 1 bpp rows) as alpha to top-down premultiplied BGRA `px`
/// (`w × h`): set bits become transparent, clear bits opaque.
pub fn apply_mask(px: &mut [u8], mask: &[u8], w: u32, h: u32) {
    let stride = ((w.div_ceil(32)) * 4) as usize;
    for y in 0..h as usize {
        let src = (h as usize - 1 - y) * stride;
        for x in 0..w as usize {
            let bit = mask
                .get(src + x / 8)
                .is_some_and(|b| b & (0x80 >> (x % 8)) != 0);
            if let Some(p) = px.get_mut((y * w as usize + x) * 4..(y * w as usize + x) * 4 + 4) {
                if bit {
                    p.fill(0);
                } else if let Some(a) = p.get_mut(3) {
                    *a = 255;
                }
            }
        }
    }
}

#[cfg(test)]
#[allow(clippy::cast_possible_truncation)]
mod tests {
    use super::*;

    /// An ICO with a 16×16 8-bit... here: a 2×2 24-bit BMP entry and a 32×32 PNG-signature entry.
    fn sample() -> Vec<u8> {
        let mut bmp = Vec::new();
        bmp.extend_from_slice(&40u32.to_le_bytes());
        bmp.extend_from_slice(&2u32.to_le_bytes());
        bmp.extend_from_slice(&4i32.to_le_bytes()); // double height
        bmp.extend_from_slice(&1u16.to_le_bytes());
        bmp.extend_from_slice(&24u16.to_le_bytes());
        bmp.extend_from_slice(&[0; 24]);
        bmp.extend_from_slice(&[255, 0, 0, 0, 255, 0, 0, 0]); // row 0: 2 px + pad
        bmp.extend_from_slice(&[0, 0, 255, 9, 9, 9, 0, 0]); // row 1
        bmp.extend_from_slice(&[0x40, 0, 0, 0, 0x80, 0, 0, 0]); // AND rows
        let png = b"\x89PNG\r\n\x1a\nrest".to_vec();
        let mut f = vec![0, 0, 1, 0, 2, 0];
        let off0 = 6 + 32;
        for (w, bpp, len, off) in [
            (2u8, 24u16, bmp.len(), off0),
            (32, 32, png.len(), off0 + bmp.len()),
        ] {
            f.extend_from_slice(&[w, w, 0, 0, 1, 0]);
            f.extend_from_slice(&bpp.to_le_bytes());
            f.extend_from_slice(&(len as u32).to_le_bytes());
            f.extend_from_slice(&(off as u32).to_le_bytes());
        }
        f.extend_from_slice(&bmp);
        f.extend_from_slice(&png);
        f
    }

    #[test]
    fn directory_pick_payload() {
        let f = sample();
        let d = directory(&f).unwrap();
        assert_eq!(d.len(), 2);
        assert_eq!(pick(&d, 16).unwrap().w, 32);
        assert_eq!(pick(&d, 2).unwrap().w, 2);
        assert_eq!(pick(&d, 300).unwrap().w, 32, "largest when none covers");
        assert!(matches!(payload(&f, &d[1]), Some(Payload::Png(p)) if p.starts_with(b"\x89PNG")));
        let Some(Payload::Bmp { file, mask, w, h }) = payload(&f, &d[0]) else {
            panic!("bmp")
        };
        assert_eq!((w, h), (2, 2));
        assert!(file.starts_with(b"BM"));
        assert_eq!(
            i32::from_le_bytes(file[22..26].try_into().unwrap()),
            2,
            "height halved"
        );
        assert_eq!(u32::from_le_bytes(file[10..14].try_into().unwrap()), 54);
        assert_eq!(file.len(), 54 + 16);
        let mask = mask.unwrap();
        let mut px = vec![9u8; 2 * 2 * 4];
        apply_mask(&mut px, &mask, 2, 2);
        // Bottom-up: mask row 1 (0x80) is the top row → (0,0) transparent; row 0 (0x40) → (1,1).
        assert_eq!((px[3], px[7], px[11], px[15]), (0, 255, 255, 0));

        // A 1×1 32-bit entry with alpha 128 comes back premultiplied without a codec.
        let mut b32 = Vec::new();
        for v in [40u32, 1, 2] {
            b32.extend_from_slice(&v.to_le_bytes());
        }
        b32.extend_from_slice(&[1, 0, 32, 0]);
        b32.extend_from_slice(&[0; 24]);
        b32.extend_from_slice(&[200, 100, 50, 128, 0, 0, 0, 0]);
        let e = Entry {
            w: 1,
            h: 1,
            bpp: 32,
            off: 0,
            len: b32.len(),
        };
        assert_eq!(
            payload(&b32, &e),
            Some(Payload::Bgra {
                px: vec![100, 50, 25, 128],
                w: 1,
                h: 1
            })
        );

        // Malformed: bad type, zero count, payload outside the file, too many entries.
        let mut bad = f.clone();
        bad[2] = 2;
        assert!(directory(&bad).is_none());
        assert!(directory(&[0, 0, 1, 0, 0, 0]).is_none());
        let mut out = f.clone();
        out[6 + 12] = 0xff;
        assert!(directory(&out).is_none());
        let mut many = f;
        many[4..6].copy_from_slice(&257u16.to_le_bytes());
        assert!(directory(&many).is_none());
    }
}
