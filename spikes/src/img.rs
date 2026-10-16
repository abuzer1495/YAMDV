//! Test image: generated in code as a PNG (stored deflate), decoded through WIC (§4.7).

use std::time::Instant;

use windows::Win32::Graphics::Imaging::{
    CLSID_WICImagingFactory, GUID_WICPixelFormat32bppPBGRA, IWICImagingFactory,
    WICBitmapDitherTypeNone, WICBitmapInterpolationModeHighQualityCubic,
    WICBitmapPaletteTypeCustom, WICDecodeMetadataCacheOnDemand,
};
use windows::Win32::System::Com::{CLSCTX_INPROC_SERVER, CoCreateInstance};

use crate::app::Log;

fn crc32(data: &[u8]) -> u32 {
    let mut c = 0xFFFF_FFFFu32;
    for &b in data {
        c ^= u32::from(b);
        for _ in 0..8 {
            c = if c & 1 != 0 {
                0xEDB8_8320 ^ (c >> 1)
            } else {
                c >> 1
            };
        }
    }
    !c
}

fn chunk(out: &mut Vec<u8>, kind: &[u8; 4], data: &[u8]) {
    out.extend_from_slice(&(data.len() as u32).to_be_bytes());
    let mut c = Vec::with_capacity(data.len() + 4);
    c.extend_from_slice(kind);
    c.extend_from_slice(data);
    out.extend_from_slice(&c);
    out.extend_from_slice(&crc32(&c).to_be_bytes());
}

/// Generates a `w`×`h` RGBA PNG (gradient, disc and stripes).
pub fn generate_png(w: u32, h: u32) -> Vec<u8> {
    let mut raw = Vec::with_capacity(((w * 4 + 1) * h) as usize);
    for y in 0..h {
        raw.push(0);
        for x in 0..w {
            let (fx, fy) = (x as f32 / w as f32, y as f32 / h as f32);
            let dx = fx - 0.65;
            let dy = fy - 0.45;
            let disc = dx * dx + dy * dy < 0.04;
            let stripe = (x / 20 + y / 20) % 2 == 0 && fy > 0.75;
            let (r, g, b) = if disc {
                (0xf5, 0x9e, 0x0b)
            } else if stripe {
                (0x1e, 0x29, 0x3b)
            } else {
                ((40.0 + 180.0 * fx) as u8, (90.0 + 120.0 * fy) as u8, 220)
            };
            raw.extend_from_slice(&[r, g, b, 255]);
        }
    }
    // zlib stream with stored blocks.
    let mut z = vec![0x78, 0x01];
    let blocks: Vec<&[u8]> = raw.chunks(65_535).collect();
    for (i, b) in blocks.iter().enumerate() {
        z.push(u8::from(i + 1 == blocks.len()));
        let len = b.len() as u16;
        z.extend_from_slice(&len.to_le_bytes());
        z.extend_from_slice(&(!len).to_le_bytes());
        z.extend_from_slice(b);
    }
    let (mut a, mut s2) = (1u32, 0u32);
    for &v in &raw {
        a = (a + u32::from(v)) % 65_521;
        s2 = (s2 + a) % 65_521;
    }
    z.extend_from_slice(&((s2 << 16) | a).to_be_bytes());

    let mut out = vec![0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A];
    let mut ihdr = Vec::new();
    ihdr.extend_from_slice(&w.to_be_bytes());
    ihdr.extend_from_slice(&h.to_be_bytes());
    ihdr.extend_from_slice(&[8, 6, 0, 0, 0]);
    chunk(&mut out, b"IHDR", &ihdr);
    chunk(&mut out, b"IDAT", &z);
    chunk(&mut out, b"IEND", &[]);
    out
}

/// Decodes `png` with WIC and scales it to `w`×`h` PBGRA. Logs WIC init cost separately.
pub fn decode_png(png: &[u8], w: u32, h: u32, log: &mut Log) -> windows::core::Result<Vec<u8>> {
    unsafe {
        let t = Instant::now();
        let before = crate::app::private_ws();
        let wic: IWICImagingFactory =
            CoCreateInstance(&CLSID_WICImagingFactory, None, CLSCTX_INPROC_SERVER)?;
        log.kv(
            "wic.init_ms",
            format!("{:.2}", t.elapsed().as_secs_f64() * 1000.0),
        );
        log.kv(
            "wic.init_pws_delta",
            crate::app::private_ws() as i64 - before as i64,
        );
        let t = Instant::now();
        let stream = wic.CreateStream()?;
        stream.InitializeFromMemory(png)?;
        let dec =
            wic.CreateDecoderFromStream(&stream, std::ptr::null(), WICDecodeMetadataCacheOnDemand)?;
        let frame = dec.GetFrame(0)?;
        let conv = wic.CreateFormatConverter()?;
        conv.Initialize(
            &frame,
            &GUID_WICPixelFormat32bppPBGRA,
            WICBitmapDitherTypeNone,
            None,
            0.0,
            WICBitmapPaletteTypeCustom,
        )?;
        let scaler = wic.CreateBitmapScaler()?;
        scaler.Initialize(&conv, w, h, WICBitmapInterpolationModeHighQualityCubic)?;
        let mut buf = vec![0u8; (w * h * 4) as usize];
        scaler.CopyPixels(std::ptr::null(), w * 4, &mut buf)?;
        log.kv(
            "wic.decode_ms",
            format!("{:.2}", t.elapsed().as_secs_f64() * 1000.0),
        );
        log.pws("wic_decoded");
        Ok(buf)
    }
}
