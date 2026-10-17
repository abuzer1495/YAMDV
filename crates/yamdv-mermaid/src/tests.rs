#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::float_cmp
)]

use crate::draw::MockMeasure;
use crate::flow::{Dir, Line, Shape};
use crate::seq::{Event, Frame, NotePos, Tip};
use crate::{Diagram, Failure, parse};

fn flow(src: &str) -> crate::flow::Flow {
    match parse(src) {
        Ok(Diagram::Flow(f)) => f,
        o => panic!("{o:?}"),
    }
}

fn seq(src: &str) -> crate::seq::Seq {
    match parse(src) {
        Ok(Diagram::Seq(s)) => s,
        o => panic!("{o:?}"),
    }
}

fn syntax(src: &str) -> crate::Error {
    match parse(src) {
        Err(Failure::Syntax(e)) => e,
        o => panic!("{o:?}"),
    }
}

#[test]
fn flow_shapes_and_edges() {
    let f = flow(
        "flowchart LR\n  a[Rect] --> b(Round)\n  b --- c([Stadium])\n  c -.-> d[[Sub]]\n  d ==> e[(Db)]\n  e -.- f((Circle))\n  f === g{Rhombus}\n  g --> h>Asym]\n  %% comment\n  click a callback\n",
    );
    assert_eq!(f.dir, Dir::LR);
    let shapes: Vec<Shape> = f.nodes.iter().map(|n| n.shape).collect();
    assert_eq!(
        shapes,
        [
            Shape::Rect,
            Shape::Round,
            Shape::Stadium,
            Shape::Subroutine,
            Shape::Cylinder,
            Shape::Circle,
            Shape::Rhombus,
            Shape::Rect
        ]
    );
    assert_eq!(f.nodes[6].label, "Rhombus");
    let kinds: Vec<(Line, bool)> = f.edges.iter().map(|e| (e.line, e.arrow)).collect();
    assert_eq!(
        kinds,
        [
            (Line::Solid, true),
            (Line::Solid, false),
            (Line::Dotted, true),
            (Line::Thick, true),
            (Line::Dotted, false),
            (Line::Thick, false),
            (Line::Solid, true)
        ]
    );
}

#[test]
fn flow_labels_chains_groups() {
    let f = flow(
        "graph TD\nA -->|yes| B\nA -- no --> C\nB -. maybe .-> D\nC == sure ==> D\nA --> B & C --> D;E-->F\nX[\"quoted [x]\"]\nY[a<br>b]",
    );
    assert_eq!(f.edges[0].label.as_deref(), Some("yes"));
    assert_eq!(f.edges[1].label.as_deref(), Some("no"));
    assert!(f.edges[1].arrow);
    assert_eq!(f.edges[2].label.as_deref(), Some("maybe"));
    assert_eq!(f.edges[2].line, Line::Dotted);
    assert_eq!(f.edges[3].label.as_deref(), Some("sure"));
    assert_eq!(f.edges[3].line, Line::Thick);
    // A --> B & C --> D gives 4 edges, then E-->F.
    assert_eq!(f.edges.len(), 4 + 4 + 1);
    let x = f.nodes.iter().find(|n| n.id == "X").unwrap();
    assert_eq!(x.label, "quoted [x]");
    let y = f.nodes.iter().find(|n| n.id == "Y").unwrap();
    assert_eq!(y.label, "a\nb");
}

#[test]
fn flow_subgraphs_and_styles() {
    let f = flow(
        "flowchart TB\nclassDef hot fill:#f96,stroke:#333,color:#fff\nsubgraph one [First]\n a1 --> a2\n subgraph inner\n  i1\n end\nend\nsubgraph two\n b1:::hot\nend\na2 --> b1\nclass a1 hot\nstyle a2 fill:#0f0\n",
    );
    assert_eq!(f.subs.len(), 3);
    assert_eq!(f.subs[0].title, "First");
    assert_eq!(f.subs[1].parent, Some(0));
    let id = |s: &str| f.nodes.iter().position(|n| n.id == s).unwrap();
    assert_eq!(f.nodes[id("a1")].sub, Some(0));
    assert_eq!(f.nodes[id("i1")].sub, Some(1));
    assert_eq!(f.nodes[id("b1")].sub, Some(2));
    assert_eq!(f.colors(id("b1")).fill, Some(0xFF9966));
    assert_eq!(f.colors(id("b1")).color, Some(0xFFFFFF));
    assert_eq!(f.colors(id("a1")).stroke, Some(0x333333));
    assert_eq!(f.colors(id("a2")).fill, Some(0x00FF00));
}

#[test]
fn flow_errors_have_positions() {
    let e = syntax("flowchart TD\n  A --> B\n  A -->\n");
    assert_eq!((e.line, e.col), (3, 8));
    let e = syntax("graph TD\nA[open --> B\n");
    assert_eq!(e.line, 2);
    let e = syntax("graph XY\n");
    assert_eq!(e.line, 1);
    let e = syntax("graph TD\nsubgraph s\nA\n");
    assert_eq!(e.line, 2);
    let e = syntax("graph TD\nend\n");
    assert_eq!(e.line, 2);
    let e = syntax("graph TD\nA --> B ??\n");
    assert_eq!((e.line, e.col), (2, 9));
}

#[test]
fn unsupported_types() {
    assert_eq!(
        parse("pie\n\"a\": 1"),
        Err(Failure::Unsupported("pie".into()))
    );
    assert_eq!(
        parse("%% c\ngantt\ntitle x"),
        Err(Failure::Unsupported("gantt".into()))
    );
    assert!(matches!(parse(""), Err(Failure::Syntax(_))));
}

#[test]
fn seq_constructs() {
    let s = seq(
        "sequenceDiagram\ntitle: Login\nautonumber\nparticipant A as Alice\nactor B\nA->>+B: solid arrow\nB-->>-A: dotted arrow\nA->B: open\nA-->B: dotted open\nA-xB: cross\nA--xB: dotted cross\nA-)B: async\nA--)B: dotted async\nactivate A\ndeactivate A\nNote left of A: l\nNote right of B: r\nNote over A,B: both\nloop Every minute\nA->>A: self\nend\nalt ok\nA->>B: y\nelse fail\nA->>B: n\nend\nopt maybe\nB->>A: o\nend\npar one\nA->>B: p\nand two\nB->>A: q\nend\n",
    );
    assert_eq!(s.title.as_deref(), Some("Login"));
    assert!(s.autonumber);
    assert_eq!(s.parts[0].label, "Alice");
    assert!(s.parts[1].actor);
    let tips: Vec<(bool, Tip)> = s
        .events
        .iter()
        .filter_map(|e| match e {
            Event::Msg(m) => Some((m.dotted, m.tip)),
            _ => None,
        })
        .take(8)
        .collect();
    assert_eq!(
        tips,
        [
            (false, Tip::Arrow),
            (true, Tip::Arrow),
            (false, Tip::None),
            (true, Tip::None),
            (false, Tip::Cross),
            (true, Tip::Cross),
            (false, Tip::Async),
            (true, Tip::Async)
        ]
    );
    let Event::Msg(m0) = &s.events[0] else {
        panic!()
    };
    assert_eq!(m0.act, Some(true));
    let Event::Msg(m1) = &s.events[1] else {
        panic!()
    };
    assert_eq!(m1.act, Some(false));
    assert!(s.events.iter().any(|e| matches!(
        e,
        Event::Note {
            pos: NotePos::Over,
            b: Some(1),
            ..
        }
    )));
    let frames: Vec<Frame> = s
        .events
        .iter()
        .filter_map(|e| match e {
            Event::Start(f, _) => Some(*f),
            _ => None,
        })
        .collect();
    assert_eq!(frames, [Frame::Loop, Frame::Alt, Frame::Opt, Frame::Par]);
    let d = Diagram::Seq(s).layout(&mut MockMeasure::default(), 1.0);
    assert!(d.w > 0.0 && d.h > 0.0);
    // Head boxes do not overlap.
    for w in d.boxes.windows(2) {
        assert!(w[0].0 + w[0].2 <= w[1].0);
    }
}

#[test]
fn seq_errors() {
    let e = syntax("sequenceDiagram\nA->>B: x\nelse\n");
    assert_eq!(e.line, 3);
    let e = syntax("sequenceDiagram\nloop x\nA->>B: x\n");
    assert_eq!(e.line, 2);
    let e = syntax("sequenceDiagram\nA=>B: x\n");
    assert_eq!(e.line, 2);
    let e = syntax("sequenceDiagram\nA-~B: x\n");
    assert_eq!((e.line, e.col), (2, 2));
    let e = syntax("sequenceDiagram\nNote above A: x\n");
    assert_eq!(e.line, 2);
}

fn overlap(a: (f32, f32, f32, f32), b: (f32, f32, f32, f32)) -> bool {
    a.0 < b.0 + b.2 - 0.01
        && b.0 < a.0 + a.2 - 0.01
        && a.1 < b.1 + b.3 - 0.01
        && b.1 < a.1 + a.3 - 0.01
}

fn check_layout(src: &str) -> crate::Drawing {
    let d = parse(src).unwrap();
    let Diagram::Flow(f) = &d else { panic!() };
    let out = d.layout(&mut MockMeasure::default(), 1.0);
    assert_eq!(out.boxes.len(), f.nodes.len());
    for i in 0..out.boxes.len() {
        for j in i + 1..out.boxes.len() {
            assert!(
                !overlap(out.boxes[i], out.boxes[j]),
                "{i} {j} {:?}",
                out.boxes
            );
        }
    }
    // Every edge connects its endpoints (starts/ends on the node boxes).
    let near = |p: (f32, f32), b: (f32, f32, f32, f32)| {
        p.0 >= b.0 - 0.5 && p.0 <= b.0 + b.2 + 0.5 && p.1 >= b.1 - 0.5 && p.1 <= b.1 + b.3 + 0.5
    };
    for (e, pts) in f.edges.iter().zip(&out.edges) {
        assert!(pts.len() >= 2);
        assert!(near(pts[0], out.boxes[e.from]), "{pts:?}");
        assert!(near(*pts.last().unwrap(), out.boxes[e.to]), "{pts:?}");
    }
    out
}

#[test]
fn layout_invariants() {
    check_layout("graph TD\nA-->B\nA-->C\nB-->D\nC-->D\nD-->A\nA-->D\nD-->D\nE");
    check_layout(
        "flowchart LR\nsubgraph s1\na-->b\nend\nsubgraph s2\nc-->d\nsubgraph s3\ne\nend\nend\nb-->c\na-->e\nx-->|label|b\n",
    );
    let mut big = String::from("graph TD\n");
    for i in 0..60 {
        big.push_str(&format!(
            "n{i} --> n{}\nn{i} --> n{}\n",
            (i * 7 + 3) % 60,
            (i * 13 + 5) % 60
        ));
    }
    check_layout(&big);
    check_layout("graph BT\nA{Decide}-->B((x))\nA-->C[(db)]\nB-->D([s])\nC-->D[[sub]]");
}

#[test]
fn lr_is_rotation_of_tb() {
    // Square nodes (circles) and no labels: LR = TB with x/y swapped.
    let body = "A((A))-->B((B))\nA-->C((C))\nB-->D((D))\nC-->D\nC-->E((E))\n";
    let tb = check_layout(&format!("graph TB\n{body}"));
    let lr = check_layout(&format!("graph LR\n{body}"));
    for (a, b) in tb.boxes.iter().zip(&lr.boxes) {
        let ca = (a.0 + a.2 / 2.0, a.1 + a.3 / 2.0);
        let cb = (b.0 + b.2 / 2.0, b.1 + b.3 / 2.0);
        assert!(
            (ca.0 - cb.1).abs() < 0.5 && (ca.1 - cb.0).abs() < 0.5,
            "{ca:?} {cb:?}"
        );
    }
    assert!((tb.w - lr.h).abs() < 0.5 && (tb.h - lr.w).abs() < 0.5);
}

#[test]
fn scale_shrinks() {
    let d = parse("graph LR\nA[a long label here]-->B[another long label]-->C").unwrap();
    let a = d.layout(&mut MockMeasure::default(), 1.0);
    let b = d.layout(&mut MockMeasure::default(), 0.5);
    assert!((b.w - a.w * 0.5).abs() < 4.0, "{} {}", a.w, b.w);
}

#[test]
fn cycle_heavy_graph_is_bounded() {
    let mut s = String::from("graph TD\n");
    for i in 0..300 {
        s.push_str(&format!("a{i}-->a{}\n", i + 1));
    }
    for i in 0..300 {
        s.push_str(&format!("a300-->a{i}\n"));
    }
    let d = parse(&s).unwrap();
    let out = d.layout(&mut MockMeasure::default(), 1.0);
    assert!(out.complex || out.w > 0.0);
}
