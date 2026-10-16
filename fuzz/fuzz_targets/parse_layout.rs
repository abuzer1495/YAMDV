//! bytes → decode → block index → all detail trees → mock layout at width 600 → display list.
#![no_main]
use libfuzzer_sys::fuzz_target;
use yamdv_core::doc::{Document, ReadOpts};
use yamdv_layout::block::{EnStrings, Metrics};
use yamdv_layout::shaper::MockShaper;
use yamdv_layout::viewer::{Content, Env, Viewer};

fuzz_target!(|data: &[u8]| {
    if data.len() > 64 * 1024 {
        return;
    }
    let d = yamdv_core::decode::decode(data);
    let plain = data.first() == Some(&0x01);
    let mut content = if plain {
        Content::plain(d.text)
    } else {
        let doc = Document::new(d.text, ReadOpts::default());
        for i in 0..doc.index.blocks.len() {
            let _ = doc.detail_uncached(i);
        }
        Content::Markdown(doc)
    };
    let m = Metrics::default();
    let pal = yamdv_core::theme::palette("default", false);
    let mut shaper = MockShaper;
    let mut v = Viewer::new(&content, 600.0, m);
    let h = v.total_height() + 1.0;
    let mut env = Env { shaper: &mut shaper, m, pal: &pal, strings: &EnStrings, single_newline_break: false };
    v.prepare(&mut content, &mut env, h);
    let _ = v.visible(&content, h).len();
});
