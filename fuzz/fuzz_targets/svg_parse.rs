//! SVG bytes → bounded XML parser → intrinsic size → render at a small display size
//! (ADR-0021, S-RES-07: limits, no external references, no DTDs). Not run in CI; V2-I runs it long.
#![no_main]
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let Ok(s) = std::str::from_utf8(data) else { return };
    if let Ok((w, h)) = yamdv_svg::size(s) {
        assert!(w.is_finite() && h.is_finite() && w >= 1.0 && h >= 1.0);
    }
    if let Ok(img) = yamdv_svg::render(s, 64, 48, [0x20, 0x20, 0x20, 0xff]) {
        assert_eq!(img.rgba.len(), 64 * 48 * 4);
        assert!(img.texts.iter().all(|t| t.x.is_finite() || t.continues));
    }
});
