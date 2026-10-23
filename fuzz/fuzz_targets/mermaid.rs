//! Mermaid source → every supported diagram parser → layout + init theme with the mock measurer (prompt 007 §3.5).
#![no_main]
use libfuzzer_sys::fuzz_target;
use yamdv_mermaid::draw::MockMeasure;

fuzz_target!(|data: &[u8]| {
    let Ok(s) = std::str::from_utf8(data) else { return };
    if let Ok(d) = yamdv_mermaid::parse(s) {
        let (n, e) = d.counts();
        if n > 2000 || e > 6000 {
            return;
        }
        let mut out = d.layout_at(&mut MockMeasure::default(), 1.0, Some(1_790_000_000));
        if let Some(i) = yamdv_mermaid::theme::init(s) {
            i.apply(&mut out);
        }
        assert!(out.w.is_finite() && out.h.is_finite());
    }
});
