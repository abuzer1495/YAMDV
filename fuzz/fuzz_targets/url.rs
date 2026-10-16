//! URL parser/serializer round trip, link classification and the linkify (inline pieces) pass.
#![no_main]
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    if data.len() > 16 * 1024 {
        return;
    }
    let s = String::from_utf8_lossy(data);
    if let Ok(u) = yamdv_core::link::parse_url(&s) {
        let out = u.serialize();
        if let Ok(u2) = yamdv_core::link::parse_url(&out) {
            assert_eq!(u2.serialize(), out);
        }
    }
    let _ = yamdv_core::link::classify(&s, false);
    let _ = yamdv_core::link::classify(&s, true);
    let _ = yamdv_core::link::percent_decode(&s);
    let _ = yamdv_core::doc::inline::pieces(&s, false);
    let _ = yamdv_core::doc::inline::autolink(&s);
    let _ = yamdv_core::image::resolve_policy(&s, "C:\\docs\\a.md", &mut yamdv_core::image::NoFs);
});
