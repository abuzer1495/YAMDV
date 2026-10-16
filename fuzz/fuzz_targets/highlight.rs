//! Every lexer on arbitrary text: first byte selects the language alias.
#![no_main]
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    if data.is_empty() || data.len() > 64 * 1024 {
        return;
    }
    let names = yamdv_highlight::ALL_NAMES;
    let name = names[usize::from(data[0]) % names.len()];
    let text = String::from_utf8_lossy(&data[1..]);
    if let Some(lang) = yamdv_highlight::lang_for(name) {
        let lines = yamdv_highlight::highlight_block(lang, &text);
        for (line, toks) in text.split('\n').zip(lines.iter()) {
            for (r, _) in toks {
                assert!(r.end <= line.len() + 1);
            }
        }
    }
});
