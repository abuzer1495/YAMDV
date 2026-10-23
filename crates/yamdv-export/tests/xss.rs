//! S-TEST-02 / AC-7: XSS vector suite through sanitizer → model and through the CF_HTML writer.
#![allow(clippy::indexing_slicing, clippy::manual_strip)]
//! Asserts: no `<script`, no `on*=`, no `javascript:` / `vbscript:` / non-image `data:` anywhere.

use yamdv_core::doc::{Document, ReadOpts};

const VECTORS: &[&str] = &[
    "<script>alert(1)</script>",
    "<SCRIPT SRC=//x.example/x.js></SCRIPT>",
    "<scr<script>ipt>alert(1)</script>",
    "<img src=x onerror=alert(1)>",
    "<img src=\"x\" ONERROR=\"alert(1)\">",
    "<img src=javascript:alert(1)>",
    "<img src=\"jav&#x09;ascript:alert(1)\">",
    "<img src=\"&#106;&#97;&#118;&#97;&#115;&#99;&#114;&#105;&#112;&#116;&#58;alert(1)\">",
    "<img src=\"data:text/html;base64,PHNjcmlwdD5hbGVydCgxKTwvc2NyaXB0Pg==\">",
    "<img/src=x/onerror=alert(1)>",
    "<svg onload=alert(1)>",
    "<svg><script>alert(1)</script></svg>",
    "<body onload=alert(1)>",
    "<iframe src=javascript:alert(1)></iframe>",
    "<iframe srcdoc=\"<script>alert(1)</script>\"></iframe>",
    "<object data=javascript:alert(1)></object>",
    "<embed src=javascript:alert(1)>",
    "<a href=\"javascript:alert(1)\">x</a>",
    "<a href=\"JaVaScRiPt:alert(1)\">x</a>",
    "<a href=\" javascript:alert(1)\">x</a>",
    "<a href=\"java\nscript:alert(1)\">x</a>",
    "<a href=\"vbscript:msgbox(1)\">x</a>",
    "<a href=\"data:text/html,<script>alert(1)</script>\">x</a>",
    "<a href=\"x\" onclick=\"alert(1)\">x</a>",
    "<a href=\"x\" onmouseover=alert(1)>x</a>",
    "<div onmouseover=\"alert(1)\">hover</div>",
    "<div style=\"background:url(javascript:alert(1))\">s</div>",
    "<div style=\"width:expression(alert(1))\">s</div>",
    "<p style=\"behavior:url(x.htc)\">s</p>",
    "<style>body{background:url(javascript:alert(1))}</style>",
    "<link rel=stylesheet href=javascript:alert(1)>",
    "<meta http-equiv=refresh content=\"0;url=javascript:alert(1)\">",
    "<base href=javascript:alert(1)//>",
    "<form action=javascript:alert(1)><input type=submit></form>",
    "<button formaction=javascript:alert(1)>b</button>",
    "<input onfocus=alert(1) autofocus>",
    "<select onfocus=alert(1) autofocus></select>",
    "<textarea onfocus=alert(1) autofocus></textarea>",
    "<video><source onerror=alert(1)></video>",
    "<audio src=x onerror=alert(1)></audio>",
    "<details open ontoggle=alert(1)><summary>s</summary></details>",
    "<marquee onstart=alert(1)>m</marquee>",
    "<math><mtext><script>alert(1)</script></mtext></math>",
    "<table background=javascript:alert(1)><tr><td>t</td></tr></table>",
    "<td background=\"javascript:alert(1)\">t</td>",
    "<!--<script>alert(1)</script>-->",
    "<![CDATA[<script>alert(1)</script>]]>",
    "<noscript><p title=\"</noscript><img src=x onerror=alert(1)>\"></noscript>",
    "<template><script>alert(1)</script></template>",
    "<xmp><script>alert(1)</script></xmp>",
    "<plaintext><script>alert(1)</script>",
    "<img src=x:alert(alt) onerror=eval(src) alt=0>",
    "<a href=\"&#x6A;avascript:alert(1)\">x</a>",
    "<a href=\"javascript&colon;alert(1)\">x</a>",
    "<img src=`javascript:alert(1)`>",
    "<div id=\"x\" name=\"javascript:alert(1)\">x</div>",
    "<span title=\"&quot;&gt;&lt;script&gt;alert(1)&lt;/script&gt;\">t</span>",
    "[md link](javascript:alert(1))",
    "[md link](vbscript:msgbox(1))",
    "[md link](data:text/html,<script>alert(1)</script>)",
    "![md img](javascript:alert(1))",
    "<javascript:alert(1)>",
    "[ref]\n\n[ref]: javascript:alert(1)",
    "`<script>alert(1)</script>`",
    "```html\n<script>alert(1)</script>\n```",
];

/// Markup only (tags and attributes): escaped document text such as a code span that reads
/// "javascript:alert(1)" is inert and stays allowed; raw `<script` is checked on the whole output.
fn tags_only(out: &str) -> String {
    let mut t = String::new();
    let mut in_tag = false;
    for c in out.chars() {
        if c == '<' {
            in_tag = true;
        }
        if in_tag {
            t.push(c);
        }
        if c == '>' {
            in_tag = false;
            t.push(' ');
        }
    }
    t
}

fn forbidden(out: &str) -> Option<&'static str> {
    if out.to_ascii_lowercase().contains("<script") {
        return Some("<script");
    }
    let l = out.to_ascii_lowercase();
    if l.contains("javascript:") {
        return Some("javascript:");
    }
    if l.contains("vbscript:") {
        return Some("vbscript:");
    }
    let b = l.as_bytes();
    // on*= inside a tag.
    let mut in_tag = false;
    for (i, &c) in b.iter().enumerate() {
        match c {
            b'<' => in_tag = true,
            b'>' => in_tag = false,
            b'o' if in_tag && i > 0 && matches!(b[i - 1], b' ' | b'/' | b'\t' | b'\n') => {
                let rest = &l[i..];
                if rest.starts_with("on") {
                    let name: String = rest[2..]
                        .chars()
                        .take_while(|c| c.is_ascii_alphabetic())
                        .collect();
                    let after = &rest[2 + name.len()..];
                    if !name.is_empty() && after.trim_start().starts_with('=') {
                        return Some("on*=");
                    }
                }
            }
            _ => {}
        }
    }
    for (i, _) in l.match_indices("data:") {
        if !l[i..].starts_with("data:image/") {
            return Some("data:");
        }
    }
    None
}

#[test]
fn s_test_02_xss_vector_count() {
    assert!(VECTORS.len() >= 50, "{}", VECTORS.len());
}

/// The CF_HTML writer (Ctrl+C) never emits active content for any vector, alone or inside text.
#[test]
fn s_test_02_xss_cf_html_output() {
    for v in VECTORS {
        for src in [
            v.to_string(),
            format!("text {v} more\n"),
            format!("<div>\n\n{v}\n\n</div>\n"),
        ] {
            let doc = Document::new(src.clone(), ReadOpts::default());
            let len = u32::try_from(src.len()).unwrap();
            let frag = yamdv_export::html::fragment(&doc, 0..len);
            let wrapped = yamdv_export::html::cf_html(&frag);
            // Code spans legitimately contain escaped text ("&lt;script", "javascript:"); the raw
            // string `<script` must never appear, and no scheme may appear inside markup.
            assert!(!wrapped.to_ascii_lowercase().contains("<script"), "{v:?}");
            assert_eq!(
                forbidden(&tags_only(&wrapped)),
                None,
                "vector {v:?}\n{wrapped}"
            );
        }
    }
}

/// Sanitizer → model: no node, run text or link destination carries active content.
#[test]
fn s_test_02_xss_sanitized_model() {
    for v in VECTORS {
        let mut budget = yamdv_core::html::MAX_TAGS;
        for t in yamdv_core::html::sanitize(v, &mut budget) {
            if let yamdv_core::html::Tok::Open { tag, attrs, .. } = &t {
                assert!(
                    ![
                        "script", "style", "iframe", "object", "embed", "svg", "base", "meta",
                        "link", "form"
                    ]
                    .contains(&tag.as_str()),
                    "{v}: {tag}"
                );
                for (k, val) in attrs {
                    assert!(!k.starts_with("on"), "{v}: {k}");
                    assert_eq!(forbidden(val), None, "{v}: {k}={val}");
                    assert!(!val.to_ascii_lowercase().contains("expression("), "{v}");
                }
            }
        }
        let doc = Document::new((*v).to_string(), ReadOpts::default());
        for i in 0..doc.index.blocks.len() {
            let d = doc.detail_uncached(i);
            for l in &d.links {
                let t = yamdv_core::link::classify(&l.dest, l.wiki);
                // Active schemes may be *recorded* but must classify as blocked, never external.
                if l.dest
                    .to_ascii_lowercase()
                    .replace(['\t', '\n', ' '], "")
                    .contains("script:")
                {
                    assert!(matches!(t, yamdv_core::link::Target::Blocked), "{v}: {t:?}");
                }
            }
        }
    }
}

/// V2-E context (prompt 015 §3.5): pasted clipboard HTML → Markdown never yields active
/// content, neither in the Markdown links nor after re-rendering it through the CF_HTML writer.
#[test]
fn s_test_02_xss_paste_html_to_markdown() {
    for v in VECTORS {
        let mut saved = 0;
        let md = yamdv_core::paste::html2md::convert(v, &mut |_, _| {
            saved += 1;
            Some("assets/doc/image-1.png".into())
        })
        .unwrap();
        // Escaped text (`\<script\>`, `\[x\](javascript:…)`) is inert; raw markup is not.
        assert!(
            !md.to_ascii_lowercase()
                .replace("\\<", "")
                .contains("<script"),
            "{v:?} -> {md:?}"
        );
        let doc = Document::new(md.clone(), ReadOpts::default());
        let len = u32::try_from(md.len()).unwrap();
        let out = yamdv_export::html::cf_html(&yamdv_export::html::fragment(&doc, 0..len));
        assert!(!out.to_ascii_lowercase().contains("<script"), "{v:?}");
        assert_eq!(forbidden(&tags_only(&out)), None, "vector {v:?}\n{out}");
        for i in 0..doc.index.blocks.len() {
            for l in &doc.detail_uncached(i).links {
                let d = l.dest.to_ascii_lowercase().replace(['\t', '\n', ' '], "");
                for bad in ["javascript:", "vbscript:", "livescript:", "data:", "file:"] {
                    assert!(!d.contains(bad), "vector {v:?} -> link {:?}", l.dest);
                }
            }
        }
    }
}

/// Fuzz regressions (`tests/regressions/fuzz-*.md`) go through model + CF_HTML.
#[test]
fn fuzz_regressions_cf_html_safe() {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/regressions");
    for e in std::fs::read_dir(dir).unwrap() {
        let p = e.unwrap().path();
        let src = String::from_utf8_lossy(&std::fs::read(&p).unwrap()).into_owned();
        let doc = Document::new(src.clone(), ReadOpts::default());
        let len = u32::try_from(src.len()).unwrap();
        let out = yamdv_export::html::cf_html(&yamdv_export::html::fragment(&doc, 0..len));
        assert!(
            !out.to_ascii_lowercase().contains("<script"),
            "{}",
            p.display()
        );
        assert_eq!(forbidden(&tags_only(&out)), None, "{}", p.display());
    }
}

/// Host that embeds every image, formula and diagram (worst case for the exported document).
struct EmbedAll;
impl yamdv_export::html::ExportHost for EmbedAll {
    fn image(
        &self,
        _img: &yamdv_core::doc::detail::ImageRef,
    ) -> Option<(&'static str, Vec<u8>, u32, u32)> {
        Some(("image/png", b"<script>alert(1)</script>".to_vec(), 1, 1))
    }
    fn math(&self, _tex: &str, _display: bool) -> Option<yamdv_export::html::Png> {
        Some(yamdv_export::html::Png {
            w: 2,
            h: 2,
            descent: 0,
            bytes: b"javascript:alert(1)".to_vec(),
        })
    }
    fn diagram(&self, _source: &str) -> Option<yamdv_export::html::Png> {
        self.math("", true)
    }
}

/// Fourth context (prompt 008 §6): the exported static HTML document. No script at all, the CSP
/// is present, no active content in markup, every `src` is a `data:image/` URI.
#[test]
fn s_test_02_xss_exported_html() {
    let opts = yamdv_export::html::DocOpts {
        title: "<script>alert(1)</script>".into(),
        pal: yamdv_core::theme::palette("default", false),
        dark: false,
        toc: true,
    };
    for v in VECTORS {
        for src in [
            v.to_string(),
            format!("# {v}\n\ntext {v} more $x$ ![i]({v})\n\n```mermaid\n{v}\n```\n"),
            format!(
                "> [!NOTE] {v}\n> {v}\n\n| {v} | b |\n|---|---|\n| c | d |\n\n- [ ] {v}[^n]\n\n[^n]: {v}\n"
            ),
        ] {
            let doc = Document::new(src.clone(), ReadOpts::default());
            let out = yamdv_export::html::document(&doc, &EmbedAll, &opts);
            assert!(
                !out.to_ascii_lowercase().contains("<script"),
                "{v:?}\n{out}"
            );
            assert!(out.contains(yamdv_export::html::EXPORT_CSP), "{v:?}");
            // The CSP itself names `img-src data:`; everything else must be clean.
            let body = out.replace(yamdv_export::html::EXPORT_CSP, "");
            assert_eq!(forbidden(&tags_only(&body)), None, "vector {v:?}\n{out}");
            for (i, _) in out.match_indices(" src=\"") {
                assert!(out[i + 6..].starts_with("data:image/"), "{v:?}");
            }
        }
    }
}
