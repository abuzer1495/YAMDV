//! TeX source → parser → math layout with the mock font (no MATH table: the defaults path) at
//! text and display style (prompt 007 §2).
#![no_main]
use libfuzzer_sys::fuzz_target;
use yamdv_math::font::{MathFont, MockFont};
use yamdv_math::{MathTable, render};

fuzz_target!(|data: &[u8]| {
    let Ok(s) = std::str::from_utf8(data) else { return };
    let font = MockFont::default();
    let table = MathTable::new(font.math_table(), font.units_per_em());
    for display in [false, true] {
        if let Ok(b) = render(s, &font, &table, 20.0, display) {
            assert!(b.width.is_finite() && b.ascent.is_finite() && b.descent.is_finite());
        }
    }
});
