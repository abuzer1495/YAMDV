//! R-OPEN-06/07, S-PARSE-08: every Appendix H "Encodings & bytes" fixture decodes to its expected
//! UTF-8 text.
#![allow(clippy::unwrap_used, clippy::expect_used, missing_docs)]

use yamdv_core::decode::{self, Guard, LineEndings};

fn case(name: &str) -> (decode::Decoded, String, Vec<u8>) {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/corpus/encodings");
    let bytes = std::fs::read(dir.join(name)).unwrap();
    let want =
        String::from_utf8(std::fs::read(dir.join(format!("{name}.expected"))).unwrap()).unwrap();
    (decode::decode(&bytes), want, bytes)
}

#[test]
fn r_open_06_all_fixtures() {
    let expect = [
        ("utf8-bom.md", "UTF-8", false),
        ("utf16le-bom.md", "UTF-16LE", false),
        ("utf16be-bom.md", "UTF-16BE", false),
        ("cp1254-turkish.md", "windows-1254", false),
        ("cp1252.md", "windows-1252", false),
        ("gb18030.md", "GBK", false),
        ("shift_jis.md", "Shift_JIS", false),
        ("invalid-utf8.md", "UTF-8", true),
        ("crlf.md", "UTF-8", false),
        ("mixed-endings.md", "UTF-8", false),
        ("no-final-newline.md", "UTF-8", false),
    ];
    for (name, enc, lossy) in expect {
        let (d, want, _) = case(name);
        assert_eq!(d.text, want, "{name}");
        assert_eq!(d.encoding, enc, "{name}");
        assert_eq!(d.lossy, lossy, "{name}");
    }
    assert_eq!(case("crlf.md").0.line_endings, LineEndings::CrLf);
    assert_eq!(
        case("mixed-endings.md").0.line_endings,
        LineEndings::Mixed {
            dominant_crlf: false
        }
    );
    assert!(!case("no-final-newline.md").0.final_newline);
}

#[test]
fn r_open_08_nul_fixture_is_binary() {
    let (_, _, bytes) = case("nul-bytes.bin.md");
    assert_eq!(decode::binary_guard(&bytes), Guard::AskBinary);
}
