//! Clipboard bitmap (`CF_DIB` / `CF_DIBV5`) → top-down BGRA for the PNG encoder (prompt 015
//! §3.5 image paste). Bounded: at most 16 384 px per side and 64 Mpx; 24- and 32-bit BI_RGB /
//! BI_BITFIELDS only. Pure.

/// Largest side accepted.
pub const MAX_SIDE: u32 = 16_384;
/// Largest pixel count accepted.
pub const MAX_PIXELS: u64 = 64 * 1024 * 1024;

fn u16_at(b: &[u8], i: usize) -> Option<u16> {
    Some(u16::from_le_bytes(b.get(i..i + 2)?.try_into().ok()?))
}

fn u32_at(b: &[u8], i: usize) -> Option<u32> {
    Some(u32::from_le_bytes(b.get(i..i + 4)?.try_into().ok()?))
}

fn i32_at(b: &[u8], i: usize) -> Option<i32> {
    Some(i32::from_le_bytes(b.get(i..i + 4)?.try_into().ok()?))
}

/// `(width, height, top-down BGRA)` of a packed DIB, or `None` when unsupported / malformed.
pub fn to_bgra(b: &[u8]) -> Option<(u32, u32, Vec<u8>)> {
    let hs = u32_at(b, 0)? as usize;
    if !(40..=124).contains(&hs) {
        return None;
    }
    let w = i32_at(b, 4)?;
    let h = i32_at(b, 8)?;
    let bpp = u16_at(b, 14)?;
    let comp = u32_at(b, 16)?;
    let (wu, hu) = (w.unsigned_abs(), h.unsigned_abs());
    if w <= 0 || hu == 0 || wu > MAX_SIDE || hu > MAX_SIDE || u64::from(wu) * u64::from(hu) > MAX_PIXELS {
        return None;
    }
    let masks = match (comp, bpp) {
        (0, 24 | 32) => 0,
        // BI_BITFIELDS: three masks after a 40-byte header (inside larger headers).
        (3, 32) => {
            let at = 40;
            let (r, g, bl) = (u32_at(b, at)?, u32_at(b, at + 4)?, u32_at(b, at + 8)?);
            if (r, g, bl) != (0x00FF_0000, 0x0000_FF00, 0x0000_00FF) {
                return None;
            }
            if hs == 40 { 12 } else { 0 }
        }
        _ => return None,
    };
    let clr_used = u32_at(b, 32)? as usize;
    let off = hs + masks + clr_used.min(256) * 4;
    let bypp = usize::from(bpp / 8);
    let (wz, hz) = (wu as usize, hu as usize);
    let stride = (wz * usize::from(bpp)).div_ceil(32) * 4;
    let data = b.get(off..off.checked_add(stride.checked_mul(hz)?)?)?;
    let mut out = Vec::with_capacity(wz * hz * 4);
    let mut any_alpha = false;
    for y in 0..hz {
        let src_row = if h > 0 { hz - 1 - y } else { y };
        let row = data.get(src_row * stride..src_row * stride + wz * bypp)?;
        for p in row.chunks_exact(bypp) {
            let a = if bypp == 4 { p.get(3).copied()? } else { 255 };
            out.extend_from_slice(p.get(..3)?);
            out.push(a);
            any_alpha |= a != 0;
        }
    }
    // 32-bit BI_RGB often carries an all-zero "alpha": treat as opaque.
    if !any_alpha {
        for a in out.iter_mut().skip(3).step_by(4) {
            *a = 255;
        }
    }
    Some((wu, hu, out))
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    fn header(w: i32, h: i32, bpp: u16, comp: u32) -> Vec<u8> {
        let mut v = Vec::new();
        v.extend(40u32.to_le_bytes());
        v.extend(w.to_le_bytes());
        v.extend(h.to_le_bytes());
        v.extend(1u16.to_le_bytes());
        v.extend(bpp.to_le_bytes());
        v.extend(comp.to_le_bytes());
        v.extend([0u8; 20]);
        v
    }

    #[test]
    fn dib_24_bottom_up_and_32_zero_alpha() {
        // 2×2, 24-bit, bottom-up: stride 8 (6 bytes + 2 padding).
        let mut b = header(2, 2, 24, 0);
        b.extend([1, 2, 3, 4, 5, 6, 0, 0]); // bottom row
        b.extend([7, 8, 9, 10, 11, 12, 0, 0]); // top row
        let (w, h, px) = to_bgra(&b).unwrap();
        assert_eq!((w, h), (2, 2));
        assert_eq!(&px[..8], &[7, 8, 9, 255, 10, 11, 12, 255]);
        assert_eq!(&px[8..], &[1, 2, 3, 255, 4, 5, 6, 255]);
        // 1×1, 32-bit top-down, alpha 0 → opaque; BITFIELDS with the standard masks.
        let mut b = header(1, -1, 32, 3);
        b.extend(0x00FF_0000u32.to_le_bytes());
        b.extend(0x0000_FF00u32.to_le_bytes());
        b.extend(0x0000_00FFu32.to_le_bytes());
        b.extend([9, 8, 7, 0]);
        assert_eq!(to_bgra(&b).unwrap().2, vec![9, 8, 7, 255]);
    }

    #[test]
    fn dib_rejects_malformed() {
        let mut b = header(2, 2, 24, 0);
        b.extend([0u8; 9]);
        assert!(to_bgra(&b).is_none(), "truncated");
        assert!(to_bgra(&header(20_000, 1, 32, 0)).is_none(), "too wide");
        assert!(to_bgra(&header(1, 1, 8, 0)).is_none(), "palette");
        assert!(to_bgra(&header(1, 1, 32, 1)).is_none(), "RLE");
        assert!(to_bgra(&[1, 2, 3]).is_none());
    }
}
