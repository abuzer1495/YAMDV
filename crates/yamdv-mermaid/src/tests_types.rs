//! Gantt date math, pie percentages, state/class/ER/mindmap parsing + layout, init themes.

use crate::draw::MockMeasure;
use crate::flow::{End, Line, Shape};
use crate::gantt::{self, civil_from_days, days_from_civil, parse_date, parse_duration, strftime};
use crate::tests::check_layout;
use crate::{Diagram, Ink, Prim, Role, parse};

const D: i64 = 86_400;

#[test]
fn gantt_date_math() {
    for (y, m, d) in [
        (1970, 1, 1),
        (2000, 2, 29),
        (2024, 12, 31),
        (1900, 3, 1),
        (2026, 10, 23),
    ] {
        assert_eq!(civil_from_days(days_from_civil(y, m, d)), (y, m, d));
    }
    assert_eq!(
        days_from_civil(2026, 10, 23) - days_from_civil(2026, 1, 1),
        295
    );
    assert_eq!(gantt::weekday(days_from_civil(2026, 10, 23)), 5); // Friday
    let t = parse_date("YYYY-MM-DD HH:mm", "2026-03-01 09:30").unwrap();
    assert_eq!(t, days_from_civil(2026, 3, 1) * D + 9 * 3600 + 30 * 60);
    assert_eq!(
        parse_date("DD/MM/YY", "05/04/26"),
        Some(days_from_civil(2026, 4, 5) * D)
    );
    assert_eq!(parse_date("X", "86400"), Some(D));
    assert_eq!(parse_date("YYYY-MM-DD", "2026-13-01"), None);
    assert_eq!(parse_duration("2w"), Some(14 * D));
    assert_eq!(parse_duration("1.5h"), Some(5400));
    assert_eq!(parse_duration("30m"), Some(1800));
    assert_eq!(parse_duration("3x"), None);
    assert_eq!(
        strftime("%Y-%m-%d %a %b %e %H:%M %j", 0),
        "1970-01-01 Thu Jan 1 00:00 001"
    );

    let src = "gantt\n title Plan\n dateFormat YYYY-MM-DD\n axisFormat %d %b\n excludes weekends\n section A\n Spec :done, a1, 2026-10-23, 2d\n Build :active, a2, after a1, 1w\n section B\n Ship :crit, milestone, m1, after a2, 0d\n Docs :2026-10-20, 2026-10-22\n Later : 3d";
    let Ok(Diagram::Gantt(g)) = parse(src) else {
        panic!()
    };
    let start = days_from_civil(2026, 10, 23) * D; // a Friday
    let a1 = &g.tasks[0];
    assert!(a1.done && a1.start == start);
    // Fri + 2 days skipping Sat/Sun ends on Tuesday 00:00.
    assert_eq!(a1.end, start + 4 * D);
    assert_eq!(g.tasks[1].start, a1.end);
    let m1 = &g.tasks[2];
    assert!(m1.milestone && m1.crit && m1.start == m1.end && m1.start == g.tasks[1].end);
    assert_eq!(g.tasks[3].section, 1);
    assert_eq!(g.tasks[4].start, g.tasks[3].end);
    assert!(!g.excluded.is_empty());
    let d = gantt::layout(&g, &mut MockMeasure::default(), 1.0, Some(start + 3600));
    let today = d
        .prims
        .iter()
        .any(|p| matches!(p, Prim::Path(p) if p.stroke == Some(Ink::Role(Role::Today))));
    assert!(today && d.boxes.len() == 5);
    assert!(matches!(
        parse("gantt\n x : after nope, 1d"),
        Err(crate::Failure::Syntax(_))
    ));
    assert!(matches!(
        parse("gantt\n a : a, after b, 1d\n b : b, after a, 1d"),
        Err(crate::Failure::Syntax(_))
    ));
}

#[test]
fn pie_percentages() {
    let Ok(Diagram::Pie(p)) =
        parse("pie showData title Pets\n \"Dogs\" : 386\n \"Cats\" : 85.5\n \"Rats\" : 15")
    else {
        panic!()
    };
    assert!(p.show_data && p.title.as_deref() == Some("Pets"));
    assert_eq!(crate::pie::percentages(&p), [79, 18, 3]);
    let d = crate::pie::layout(&p, &mut MockMeasure::default(), 1.0);
    let series = d
        .prims
        .iter()
        .filter(
            |p| matches!(p, Prim::Path(p) if matches!(p.fill, Some(Ink::Role(Role::Series(_))))),
        )
        .count();
    assert_eq!(series, 6, "3 slices + 3 legend swatches");
    assert!(parse("pie\n Dogs : 3").is_err() && parse("pie\n \"a\" : -1").is_err());
}

#[test]
fn state_class_er_mindmap() {
    let src = "stateDiagram-v2\n [*] --> Still\n Still --> Moving : push\n Moving --> [*]\n state \"Crash site\" as Crash\n Moving --> Crash\n state Fork <<fork>>\n note right of Crash : boom\n state Comp {\n  [*] --> Inner\n }";
    let d = check_layout(src);
    let Ok(Diagram::Graph(f)) = parse(src) else {
        panic!()
    };
    assert_eq!(f.nodes[0].shape, Shape::Start);
    assert_eq!(f.nodes.iter().filter(|n| n.shape == Shape::End).count(), 1);
    assert!(f.nodes.iter().any(|n| n.label == "Crash site"));
    assert_eq!(f.edges[1].label.as_deref(), Some("push"));
    assert!(
        f.edges
            .iter()
            .any(|e| e.line == Line::Dotted && e.head == End::None)
    );
    assert!(f.nodes.iter().filter(|n| n.shape == Shape::Start).count() == 3 && d.w > 0.0);

    let src = "classDiagram\n class Animal {\n  <<abstract>>\n  +String name\n  +eat() void\n }\n Animal <|-- Duck\n Duck *-- \"2\" Wing : has\n Duck : +swim()\n class List~T~\n Zoo o-- Animal\n Duck ..> Pond\n Duck ..|> Swimmer";
    check_layout(src);
    let Ok(Diagram::Graph(f)) = parse(src) else {
        panic!()
    };
    let animal = &f.nodes[0];
    assert!(
        animal.label.starts_with("«abstract»")
            && animal.sections == ["+String name", "+eat() void"]
    );
    assert_eq!(
        (f.edges[0].tail, f.edges[0].head),
        (End::Triangle, End::None)
    );
    assert_eq!(
        (f.edges[1].tail, f.edges[1].label.as_deref()),
        (End::DiamondFilled, Some("has  2"))
    );
    assert!(f.nodes.iter().any(|n| n.label == "List<T>"));
    assert_eq!(f.edges[2].tail, End::Diamond);
    assert_eq!(
        (f.edges[3].line, f.edges[3].head),
        (Line::Dotted, End::Arrow)
    );
    assert_eq!(f.edges[4].head, End::Triangle);
    assert_eq!(f.nodes[1].sections[1], "+swim()");

    let src = "erDiagram\n CUSTOMER ||--o{ ORDER : places\n ORDER ||--|{ LINE-ITEM : contains\n CUSTOMER }|..|| DELIVERY-ADDRESS : uses\n CUSTOMER {\n  string name PK\n  string email\n }";
    check_layout(src);
    let Ok(Diagram::Graph(f)) = parse(src) else {
        panic!()
    };
    assert_eq!(
        (f.edges[0].tail, f.edges[0].head),
        (End::One, End::ZeroMany)
    );
    assert_eq!(
        (f.edges[1].head, f.edges[2].tail, f.edges[2].line),
        (End::OneMany, End::OneMany, Line::Dotted)
    );
    assert_eq!(f.nodes[0].sections, ["string name PK\nstring email"]);
    assert!(parse("erDiagram\n A ||~~|| B : x").is_err());

    let Ok(d @ Diagram::Mind(_)) =
        parse("mindmap\n  root((Root))\n    A[Square]\n      A1\n    B{{Hex}}\n    C)Cloud(")
    else {
        panic!()
    };
    let Diagram::Mind(m) = &d else { panic!() };
    assert_eq!(m.root.0, "Root");
    assert_eq!(
        m.items.iter().map(|x| x.0).collect::<Vec<_>>(),
        [1, 2, 1, 1]
    );
    let out = d.layout(&mut MockMeasure::default(), 1.0);
    assert_eq!(out.boxes.len(), 5);
    assert!(parse("mindmap\n a\n b").is_err());
}

#[test]
fn init_themes() {
    let src = "%%{init: {'theme':'forest', 'themeVariables': {\"primaryColor\": \"#ff0000\", 'fontSize': '21px'}}}%%\nflowchart LR\n a --> b";
    let i = crate::theme::init(src).unwrap();
    assert!(i.roles.contains(&(Role::NodeFill, 0xFF0000)));
    assert!(i.roles.contains(&(Role::NodeStroke, 0x13540C)));
    assert_eq!(i.font_scale, Some(1.5));
    let Ok(d) = parse(src) else { panic!() };
    let mut out = d.layout(&mut MockMeasure::default(), 1.0);
    i.apply(&mut out);
    assert!(out.prims.iter().all(|p| match p {
        Prim::Path(p) => p.fill != Some(Ink::Role(Role::NodeFill)),
        Prim::Text(t) => t.color != Ink::Role(Role::Text),
    }));
    for t in ["default", "neutral", "dark", "base"] {
        assert!(
            crate::theme::init(&format!("%%{{init: {{\"theme\": \"{t}\"}}}}%%\npie")).is_some()
        );
    }
    assert!(crate::theme::init("pie").is_none());
}
