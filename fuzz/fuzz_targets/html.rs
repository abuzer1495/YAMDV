//! HTML tokenizer + sanitizer, then the full model (structure pass) on the same bytes as an HTML block.
#![no_main]
use libfuzzer_sys::fuzz_target;
use yamdv_core::doc::{Document, ReadOpts};

fuzz_target!(|data: &[u8]| {
    if data.len() > 64 * 1024 {
        return;
    }
    let s = String::from_utf8_lossy(data);
    let mut budget = 100_000;
    // Text tokens are shown as text (never interpreted); only elements and attributes matter.
    for t in yamdv_core::html::sanitize(&s, &mut budget) {
        if let yamdv_core::html::Tok::Open { tag, attrs, .. } = &t {
            assert!(!yamdv_core::html::DROP.contains(&tag.as_str()), "{tag}");
            for (k, v) in attrs {
                assert!(!k.starts_with("on"), "{k}");
                let l = v.to_ascii_lowercase();
                assert!(!l.contains("javascript:") && !l.contains("vbscript:"), "{v}");
            }
        }
    }
    let _ = yamdv_core::html::decode_entities(&s);
    let _ = yamdv_core::html::sanitize_style(&s);
    let doc = Document::new(format!("<div>\n{s}\n</div>\n"), ReadOpts::default());
    for i in 0..doc.index.blocks.len() {
        let _ = doc.detail_uncached(i);
    }
});
