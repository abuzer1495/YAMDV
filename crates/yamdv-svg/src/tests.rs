use crate::{Anchor, Error, render, size, xml};

fn px(img: &crate::Image, x: u32, y: u32) -> [u8; 4] {
    let i = ((y * img.w + x) * 4) as usize;
    let p = img.rgba.get(i..i + 4).unwrap_or(&[0; 4]);
    [p[0], p[1], p[2], p[3]]
}

#[test]
fn limits_and_security() {
    // DTDs / entity declarations are rejected outright (no expansion).
    let bomb = "<!DOCTYPE svg [<!ENTITY a \"aaaa\">]><svg>&a;</svg>";
    assert_eq!(xml::parse(bomb).err(), Some(Error::Dtd));
    assert_eq!(
        xml::parse("<svg><!ENTITY x 'y'></svg>").err(),
        Some(Error::Dtd)
    );
    assert_eq!(xml::parse("<html/>").err(), Some(Error::NoSvg));
    assert_eq!(xml::parse("<svg><g></svg>").err(), Some(Error::Malformed));
    let deep = format!("<svg>{}{}</svg>", "<g>".repeat(64), "</g>".repeat(64));
    assert_eq!(xml::parse(&deep).err(), Some(Error::TooDeep));
    let many = format!("<svg>{}</svg>", "<g/>".repeat(xml::MAX_ELEMENTS));
    assert_eq!(xml::parse(&many).err(), Some(Error::TooMany));
    let big = format!("<svg>{}</svg>", " ".repeat(xml::MAX_INPUT));
    assert_eq!(xml::parse(&big).err(), Some(Error::TooLarge));
    assert_eq!(xml::unescape("&lt;&#65;&#x42;&foo;"), "<AB&foo;");
    assert_eq!(
        render("<svg/>", 20_000, 20_000, [0; 4]).err(),
        Some(Error::TooLarge)
    );

    // Scripts, foreignObject, images, event attributes, external references: nothing drawn.
    let hostile = r#"<svg width="10" height="10" onload="alert(1)">
        <script>alert(1)</script>
        <foreignObject width="10" height="10"><div>x</div></foreignObject>
        <image href="file:///c:/x.png" width="10" height="10"/>
        <use href="other.svg#r"/>
        <use xlink:href="https://example.com/a.svg#r"/>
        <rect width="10" height="10" fill="url(other.svg#g)"/>
        <style>@import url(https://example.com/x.css); rect { fill: red }</style>
    </svg>"#;
    let img = render(hostile, 10, 10, [0; 4]).unwrap();
    assert!(img.rgba.iter().all(|&b| b == 0) && img.texts.is_empty());

    // `use` recursion and fan-out stay bounded.
    let rec = r##"<svg width="4" height="4"><g id="a"><use href="#a"/><rect width="1" height="1"/></g></svg>"##;
    assert!(render(rec, 4, 4, [0; 4]).is_ok());
    let mut fan =
        String::from(r#"<svg width="4" height="4"><defs><rect id="l0" width="1" height="1"/>"#);
    for i in 1..9 {
        fan.push_str(&format!(
            r#"<g id="l{i}">{}</g>"#,
            format!(r##"<use href="#l{}"/>"##, i - 1).repeat(10)
        ));
    }
    fan.push_str(r##"</defs><use href="#l8"/></svg>"##);
    let t = std::time::Instant::now();
    assert!(render(&fan, 4, 4, [0; 4]).is_ok());
    assert!(t.elapsed().as_secs() < 5);

    // Path command budget.
    let mut budget = 3;
    let p = crate::path::parse("M0 0 L1 1 L2 2 L3 3 L4 4", &mut budget).unwrap();
    assert_eq!((budget, p.len()), (0, 3));
}

#[test]
fn shapes_gradients_use_text() {
    assert_eq!(size("<svg viewBox='0 0 40 20'/>"), Ok((40.0, 20.0)));
    assert_eq!(
        size("<svg width='2in' viewBox='0 0 40 20'/>"),
        Ok((192.0, 96.0))
    );
    assert_eq!(size("<svg/>"), Ok((300.0, 150.0)));

    let svg = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 100 50" color="#00f">
      <defs>
        <linearGradient id="g"><stop offset="0" stop-color="#f00"/><stop offset="100%" stop-color="#00f"/></linearGradient>
        <linearGradient id="g2" href="#g" x1="1" x2="0"/>
        <circle id="dot" r="5" style="fill: currentColor"/>
      </defs>
      <rect width="50" height="10" fill="url(#g)"/>
      <rect y="10" width="50" height="10" fill="url(#g2)"/>
      <use href="#dot" x="75" y="25"/>
      <path d="M0 30 h20 v20 h-20 z" fill="rgb(0,128,0)"/>
      <path d="M30 40 a10 10 0 1 0 20 0 a10 10 0 1 0 -20 0" fill="none" stroke="black" stroke-width="2"/>
      <g transform="translate(10 0)" fill="#123456" font-size="10">
        <text x="40" y="48" text-anchor="middle" font-family="'Fira Sans', sans-serif" font-weight="bold">Hi <tspan fill="red">there</tspan></text>
      </g>
    </svg>"##;
    let img = render(svg, 200, 100, [9, 9, 9, 255]).unwrap();
    let left = px(&img, 2, 10);
    let right = px(&img, 97, 10);
    assert!(left[0] > 200 && left[2] < 40, "{left:?}");
    assert!(right[2] > 200 && right[0] < 40, "{right:?}");
    // href-inherited stops, reversed direction.
    assert!(px(&img, 2, 30)[2] > 200);
    // currentColor from the root `color` attribute (not the passed text color).
    assert_eq!(px(&img, 150, 50), [0, 0, 255, 255]);
    assert_eq!(px(&img, 20, 80), [0, 128, 0, 255]);
    // Arc circle outline (stroke) at its left edge, hollow inside.
    assert!(px(&img, 60, 80)[3] > 100 && px(&img, 80, 80)[3] == 0);

    let runs = &img.texts;
    assert_eq!(runs.len(), 2, "{runs:?}");
    let (a, b) = (&runs[0], &runs[1]);
    assert_eq!((a.text.as_str(), b.text.as_str()), ("Hi ", "there"));
    assert_eq!((a.x, a.y, a.size), (100.0, 96.0, 20.0));
    assert!(a.bold && a.anchor == Anchor::Middle && a.family == "Fira Sans" && !a.continues);
    assert_eq!(
        (a.color, b.color),
        ([0x12, 0x34, 0x56, 255], [255, 0, 0, 255])
    );
    assert!(b.continues);

    // The passed text color is the default currentColor.
    let cc = render(
        r#"<svg width="2" height="2"><rect width="2" height="2" fill="currentColor"/></svg>"#,
        2,
        2,
        [1, 2, 3, 255],
    )
    .unwrap();
    assert_eq!(px(&cc, 1, 1), [1, 2, 3, 255]);
}
