//! HTML converter, asset and sandbox unit tests (prompt 015 §3.5, S-TEST-02 assets sandbox).
#![allow(clippy::unwrap_used, clippy::indexing_slicing)]

use super::assets::{self, Stamp};
use super::{cf_html_fragment, html2md};

fn md(h: &str) -> String {
    html2md::convert(h, &mut |_, ext| Some(format!("assets/doc/image-1.{ext}"))).unwrap()
}

const PNG_1X1: &str = "data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mP8z8BQDwAEhQGAhKmMIQAAAABJRU5ErkJggg==";

#[test]
fn html_to_markdown_cases() {
    let cases: &[(&str, &str)] = &[
        ("<h1>T</h1>", "# T"),
        ("<h2>Sub <b>bold</b></h2>", "## Sub **bold**"),
        ("<h6>x</h6>", "###### x"),
        ("<p>a</p><p>b</p>", "a\n\nb"),
        ("<b>x</b>", "**x**"),
        ("<strong> x </strong>y", " **x** y"),
        ("<i>x</i> <em>y</em>", "*x* *y*"),
        ("<s>x</s><del>y</del>", "~~x~~~~y~~"),
        ("<mark>hi</mark>", "==hi=="),
        ("<code>a*b</code>", "`a*b`"),
        ("<code>a`b</code>", "`` a`b ``"),
        ("a<br>b", "a\\\nb"),
        (
            "<a href=\"https://x.org/(a)\">X</a>",
            "[X](https://x.org/%28a%29)",
        ),
        ("<a href=\"mailto:a@b.c\">m</a>", "[m](mailto:a@b.c)"),
        ("<a href=\"../doc.md#h\">r</a>", "[r](../doc.md#h)"),
        ("<a href=\"file:///C:/x\">f</a>", "f"),
        ("<a href=\"ms-settings:\">s</a>", "s"),
        ("<a href=\"//evil.example/x\">p</a>", "p"),
        (
            "<a href=\"https://x.org/?a=1&b=2\">q</a>",
            "[q](https://x.org/?a=1&b=2)",
        ),
        ("<a href=\"https://x\"></a>", ""),
        ("<ul><li>a</li><li>b</li></ul>", "- a\n- b"),
        ("<ol><li>a</li><li>b</li></ol>", "1. a\n2. b"),
        ("<ul><li>a<ul><li>b</li></ul></li></ul>", "- a\n\n  - b"),
        ("<blockquote><p>q</p></blockquote>", "> q"),
        (
            "<pre><code>let x = 1;\nfn *a</code></pre>",
            "```\nlet x = 1;\nfn *a\n```",
        ),
        ("<hr>", "---"),
        (
            "<table><tr><th>A</th><th>B</th></tr><tr><td>1</td><td>2|3</td></tr></table>",
            "| A | B |\n|---|---|\n| 1 | 2\\|3 |",
        ),
        (
            "<table><tr><td>x</td></tr><tr><td>y</td><td>z</td></tr></table>",
            "| x |  |\n|---|---|\n| y | z |",
        ),
        (
            "<p>1 * 2 _ [x] &lt;y&gt;</p>",
            "1 \\* 2 \\_ \\[x\\] \\<y\\>",
        ),
        ("<p>a   \n  b</p>", "a b"),
        ("<script>alert(1)</script>ok", "ok"),
        ("<style>p{}</style><p>s</p>", "s"),
        (
            "<img src=\"https://x.org/i.png\" alt=\"r\">",
            "![r](https://x.org/i.png)",
        ),
        ("<img src=\"C:\\\\x.png\" alt=\"local\">", "local"),
        (
            &format!("<img src=\"{PNG_1X1}\" alt=\"d\">")[..],
            "![d](assets/doc/image-1.png)",
        ),
        (
            "<img src=\"data:image/svg+xml;base64,PHN2Zz4=\" alt=\"v\">",
            "v",
        ),
        ("<div>x<span>y</span></div>", "xy"),
        ("<p>unclosed <b>bold", "unclosed **bold**"),
    ];
    assert!(cases.len() >= 30);
    let bad: Vec<String> = cases
        .iter()
        .filter(|(h, w)| md(h) != *w)
        .map(|(h, w)| format!("{h:?}\n  got  {:?}\n  want {w:?}", md(h)))
        .collect();
    assert!(bad.is_empty(), "{}", bad.join("\n"));
}

#[test]
fn data_images_become_assets_and_size_cap_falls_back() {
    let mut got = Vec::new();
    let out = html2md::convert(&format!("<img src=\"{PNG_1X1}\">"), &mut |b, e| {
        got.push((b.len(), e));
        Some("assets/n/image.png".into())
    });
    assert_eq!(out.as_deref(), Some("![](assets/n/image.png)"));
    assert_eq!(got.len(), 1);
    assert!(got[0].0 > 60 && got[0].1 == "png");
    let big = "x".repeat(super::MAX_HTML + 1);
    assert!(html2md::convert(&big, &mut |_, _| None).is_none());
}

#[test]
fn cf_html_fragment_offsets_and_markers() {
    let body = "<html><body><!--StartFragment--><b>x</b><!--EndFragment--></body></html>";
    let head = "Version:0.9\r\nStartHTML:0000000105\r\nEndHTML:0000000177\r\nStartFragment:0000000141\r\nEndFragment:0000000149\r\n";
    let raw = format!("{head}{body}");
    let a = raw.find("<b>").unwrap();
    let fixed = raw
        .replace("0000000141", &format!("{a:010}"))
        .replace("0000000149", &format!("{:010}", a + 8));
    assert_eq!(cf_html_fragment(&fixed), "<b>x</b>");
    assert_eq!(cf_html_fragment(body), "<b>x</b>");
}

#[test]
fn asset_names_dirs_and_links() {
    let t = Stamp {
        y: 2026,
        mo: 10,
        d: 3,
        h: 9,
        mi: 5,
        s: 7,
    };
    assert_eq!(assets::image_name(t, 0, "png"), "image-20261003-090507.png");
    assert_eq!(
        assets::image_name(t, 2, "png"),
        "image-20261003-090507-2.png"
    );
    assert_eq!(
        assets::asset_dir("assets", "My Notes").as_deref(),
        Some("assets/My Notes")
    );
    assert_eq!(
        assets::asset_dir("media\\img", "a").as_deref(),
        Some("media/img/a")
    );
    for bad in [
        "..",
        "../x",
        "a/../../b",
        "C:\\x",
        "\\\\srv\\s",
        "/abs",
        "a:b",
        "",
    ] {
        assert_eq!(assets::asset_dir(bad, "doc"), None, "{bad}");
    }
    assert_eq!(
        assets::asset_dir("assets", "con").as_deref(),
        Some("assets/_con")
    );
    assert_eq!(
        assets::image_markdown("assets/My Notes", "i.png"),
        "![](assets/My%20Notes/i.png)"
    );
    assert_eq!(
        assets::file_markdown("assets/d", "my [file].pdf"),
        "[my \\[file\\].pdf](assets/d/my%20[file].pdf)"
    );
    assert_eq!(
        assets::sanitize_name("a<b>:c?.txt. ").as_deref(),
        Some("abc.txt")
    );
    assert_eq!(assets::sanitize_name("...").as_deref(), None);
    let long = format!("{}.png", "x".repeat(300));
    let s = assets::sanitize_name(&long).unwrap();
    assert!(s.encode_utf16().count() <= assets::MAX_NAME && s.ends_with(".png"));
}

#[test]
fn sandbox_within_rejects_escapes() {
    let root = r"C:\Docs";
    assert!(assets::within(root, r"\\?\C:\Docs\assets\n"));
    assert!(assets::within(root, r"c:\docs\Assets\N"));
    assert!(!assets::within(root, r"C:\DocsEvil\assets"));
    assert!(!assets::within(root, r"C:\Docs\..\Windows"));
    // A junction `assets` → C:\Windows resolves (GetFinalPathNameByHandleW) outside the root.
    assert!(!assets::within(root, r"\\?\C:\Windows\Temp"));
    assert!(assets::within(
        r"\\srv\share\d",
        r"\\?\UNC\srv\share\d\assets"
    ));
    assert!(!assets::within(
        r"\\srv\share\d",
        r"\\?\UNC\srv\other\d\assets"
    ));
    assert!(!assets::within("", r"C:\x"));
}
