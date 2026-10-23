//! `gantt` (FUT-18 MUST): title, dateFormat, axisFormat, excludes, sections, tags, ids, `after`,
//! durations, today marker. Times are seconds since 1970-01-01 in the document's own (local)
//! clock: no time zone is applied anywhere.
// Task counts and pixel values are small and bounded.
#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    clippy::cast_sign_loss
)]

use crate::draw::{self, Drawing, Ink, Measure, Path, Prim, Role, TextPrim};
use crate::{Error, err, lines};

const DAY: i64 = 86_400;

/// One task bar.
#[derive(Debug, Clone, PartialEq)]
pub struct Task {
    /// Label.
    pub name: String,
    /// Section index.
    pub section: usize,
    /// Optional id.
    pub id: Option<String>,
    /// Start (s).
    pub start: i64,
    /// End (s).
    pub end: i64,
    /// `done` / `active` / `crit` / `milestone`.
    pub done: bool,
    /// Active.
    pub active: bool,
    /// Critical.
    pub crit: bool,
    /// Milestone.
    pub milestone: bool,
}

/// A parsed gantt chart.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Gantt {
    /// Title.
    pub title: Option<String>,
    /// `axisFormat` (strftime), default `%Y-%m-%d`.
    pub axis: String,
    /// Section names.
    pub sections: Vec<String>,
    /// Tasks in source order.
    pub tasks: Vec<Task>,
    /// Excluded days (days since epoch) inside the chart, for the shaded bands.
    pub excluded: Vec<i64>,
    /// `todayMarker off`.
    pub no_today: bool,
}

/// Days since 1970-01-01 of a proleptic Gregorian date.
pub fn days_from_civil(y: i64, m: u32, d: u32) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let mp = (i64::from(m) + 9) % 12;
    let doy = (153 * mp + 2) / 5 + i64::from(d) - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

/// The date of a day count: (year, month, day).
pub fn civil_from_days(z: i64) -> (i64, u32, u32) {
    let z = z + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    (
        if m <= 2 {
            yoe + era * 400 + 1
        } else {
            yoe + era * 400
        },
        m,
        d,
    )
}

/// 0 = Sunday.
pub fn weekday(day: i64) -> u32 {
    (day + 4).rem_euclid(7) as u32
}

/// Parses `v` with a dayjs-style `fmt` (YYYY YY MM M DD D HH H mm m ss s X x, literals).
pub fn parse_date(fmt: &str, v: &str) -> Option<i64> {
    let f: Vec<char> = fmt.chars().collect();
    let s: Vec<char> = v.trim().chars().collect();
    let (mut i, mut j) = (0, 0);
    let (mut y, mut mo, mut d, mut h, mut mi, mut se) = (1970i64, 1u32, 1u32, 0i64, 0i64, 0i64);
    let num = |j: &mut usize, max: usize| -> Option<i64> {
        let st = *j;
        while *j < s.len() && *j - st < max && s.get(*j).is_some_and(char::is_ascii_digit) {
            *j += 1;
        }
        (st < *j).then(|| s.get(st..*j)?.iter().collect::<String>().parse().ok())?
    };
    while let Some(&c) = f.get(i) {
        let run = f.get(i..)?.iter().take_while(|&&x| x == c).count();
        match c {
            'Y' => {
                let n = num(&mut j, if run >= 4 { 4 } else { 2 })?;
                y = if run >= 4 { n } else { 2000 + n };
            }
            'M' => mo = u32::try_from(num(&mut j, 2)?).ok()?,
            'D' => d = u32::try_from(num(&mut j, 2)?).ok()?,
            'H' => h = num(&mut j, 2)?,
            'm' => mi = num(&mut j, 2)?,
            's' => se = num(&mut j, 2)?,
            'X' => return num(&mut j, 12),
            'x' => return num(&mut j, 15).map(|ms| ms / 1000),
            _ => {
                for _ in 0..run {
                    if s.get(j) != Some(&c) {
                        return None;
                    }
                    j += 1;
                }
            }
        }
        i += run;
    }
    if j != s.len() || !(1..=12).contains(&mo) || !(1..=31).contains(&d) || h > 24 || mi > 59 {
        return None;
    }
    Some(days_from_civil(y, mo, d) * DAY + h * 3600 + mi * 60 + se)
}

/// A duration such as `3d`, `1.5h`, `2w`, `30m`, `10s`, `500ms`.
pub fn parse_duration(v: &str) -> Option<i64> {
    let v = v.trim();
    let split = v.find(|c: char| !(c.is_ascii_digit() || c == '.'))?;
    let n: f64 = v.get(..split)?.parse().ok()?;
    let unit = match v.get(split..)? {
        "ms" => 0.001,
        "s" => 1.0,
        "m" => 60.0,
        "h" => 3600.0,
        "d" => 86_400.0,
        "w" => 604_800.0,
        _ => return None,
    };
    let s = n * unit;
    (s.is_finite() && s < 1e13).then_some(s.round() as i64)
}

const MONTHS: [&str; 12] = [
    "January",
    "February",
    "March",
    "April",
    "May",
    "June",
    "July",
    "August",
    "September",
    "October",
    "November",
    "December",
];
const DAYS: [&str; 7] = [
    "Sunday",
    "Monday",
    "Tuesday",
    "Wednesday",
    "Thursday",
    "Friday",
    "Saturday",
];

/// Formats `t` with a strftime subset (`%Y %y %m %d %e %H %I %M %S %p %b %B %a %A %j %%`).
pub fn strftime(fmt: &str, t: i64) -> String {
    let day = t.div_euclid(DAY);
    let sod = t.rem_euclid(DAY);
    let (y, m, d) = civil_from_days(day);
    let (h, mi, se) = (sod / 3600, sod / 60 % 60, sod % 60);
    let mname = MONTHS.get(m as usize - 1).copied().unwrap_or("");
    let dname = DAYS.get(weekday(day) as usize).copied().unwrap_or("");
    let mut out = String::new();
    let mut it = fmt.chars();
    while let Some(c) = it.next() {
        if c != '%' {
            out.push(c);
            continue;
        }
        let s = match it.next() {
            Some('Y') => y.to_string(),
            Some('y') => format!("{:02}", y.rem_euclid(100)),
            Some('m') => format!("{m:02}"),
            Some('d') => format!("{d:02}"),
            Some('e') => d.to_string(),
            Some('H') => format!("{h:02}"),
            Some('I') => format!("{:02}", (h + 11) % 12 + 1),
            Some('p') => (if h < 12 { "AM" } else { "PM" }).to_string(),
            Some('M') => format!("{mi:02}"),
            Some('S') => format!("{se:02}"),
            Some('b') => mname.get(..3).unwrap_or("").to_string(),
            Some('B') => mname.to_string(),
            Some('a') => dname.get(..3).unwrap_or("").to_string(),
            Some('A') => dname.to_string(),
            Some('j') => format!("{:03}", day - days_from_civil(y, 1, 1) + 1),
            Some('%') => "%".to_string(),
            Some(o) => format!("%{o}"),
            None => "%".to_string(),
        };
        out.push_str(&s);
    }
    out
}

enum Start {
    At(i64),
    After(Vec<String>),
    Prev,
}
enum Finish {
    At(i64),
    For(i64),
    Until(String),
}
struct Raw {
    task: Task,
    start: Start,
    end: Finish,
    line: usize,
}

struct Excl {
    weekends: bool,
    days: Vec<u32>,
    dates: Vec<i64>,
}

impl Excl {
    fn hit(&self, day: i64) -> bool {
        let wd = weekday(day);
        (self.weekends && (wd == 0 || wd == 6))
            || self.days.contains(&wd)
            || self.dates.contains(&day)
    }
}

/// Parses a `gantt` block.
#[allow(clippy::too_many_lines)]
pub fn parse(src: &str) -> Result<Gantt, Error> {
    let mut g = Gantt {
        axis: "%Y-%m-%d".into(),
        ..Gantt::default()
    };
    let mut fmt = "YYYY-MM-DD".to_string();
    let mut ex = Excl {
        weekends: false,
        days: Vec::new(),
        dates: Vec::new(),
    };
    let mut raw: Vec<Raw> = Vec::new();
    let mut ex_src: Vec<String> = Vec::new();
    for (ln, line) in lines(src).skip(1) {
        let l = line.trim();
        let kw = |k: &str| {
            l.strip_prefix(k)
                .filter(|r| r.is_empty() || r.starts_with(char::is_whitespace))
                .map(str::trim)
        };
        if let Some(v) = kw("title") {
            g.title = Some(v.to_string());
        } else if let Some(v) = kw("dateFormat") {
            fmt = v.to_string();
        } else if let Some(v) = kw("axisFormat") {
            g.axis = v.to_string();
        } else if let Some(v) = kw("excludes") {
            ex_src.extend(
                v.split([',', ' '])
                    .filter(|s| !s.is_empty())
                    .map(str::to_string),
            );
        } else if let Some(v) = kw("todayMarker") {
            g.no_today = v == "off";
        } else if let Some(v) = kw("section") {
            g.sections.push(v.to_string());
        } else if kw("tickInterval").is_some()
            || kw("weekday").is_some()
            || kw("inclusiveEndDates").is_some()
            || kw("topAxis").is_some()
            || kw("accTitle").is_some()
            || kw("accDescr").is_some()
            || kw("displayMode").is_some()
            || l.starts_with("click ")
        {
        } else if let Some((name, meta)) = l.split_once(':') {
            if raw.len() >= crate::MAX_NODES {
                return Err(err("Too many tasks", ln, 0));
            }
            let mut parts: Vec<&str> = meta.split(',').map(str::trim).collect();
            let mut task = Task {
                name: name.trim().to_string(),
                section: g.sections.len().saturating_sub(1),
                id: None,
                start: 0,
                end: 0,
                done: false,
                active: false,
                crit: false,
                milestone: false,
            };
            while let Some(&t) = parts.first() {
                match t {
                    "done" => task.done = true,
                    "active" => task.active = true,
                    "crit" => task.crit = true,
                    "milestone" => task.milestone = true,
                    _ => break,
                }
                parts.remove(0);
            }
            let (id, st, en) = match parts.as_slice() {
                [e] => (None, None, *e),
                [s, e] => (None, Some(*s), *e),
                [i, s, e] => (Some(*i), Some(*s), *e),
                _ => return Err(err("Expected [id,] [start,] end or duration", ln, 0)),
            };
            task.id = id.filter(|s| !s.is_empty()).map(str::to_string);
            let start = match st {
                None => Start::Prev,
                Some(s) => {
                    if let Some(r) = s.strip_prefix("after ") {
                        Start::After(r.split_whitespace().map(str::to_string).collect())
                    } else if let Some(t) = parse_date(&fmt, s) {
                        Start::At(t)
                    } else {
                        return Err(err(format!("Invalid start date \"{s}\""), ln, 0));
                    }
                }
            };
            let end = if let Some(r) = en.strip_prefix("until ") {
                Finish::Until(r.trim().to_string())
            } else if let Some(d) = parse_duration(en) {
                Finish::For(d)
            } else if let Some(t) = parse_date(&fmt, en) {
                Finish::At(t)
            } else {
                return Err(err(format!("Invalid end date or duration \"{en}\""), ln, 0));
            };
            raw.push(Raw {
                task,
                start,
                end,
                line: ln,
            });
        } else {
            return Err(err(format!("Unexpected gantt line \"{l}\""), ln, 0));
        }
    }
    for e in &ex_src {
        match e.to_ascii_lowercase().as_str() {
            "weekends" => ex.weekends = true,
            w => {
                if let Some(i) = DAYS.iter().position(|d| d.eq_ignore_ascii_case(w)) {
                    ex.days.push(i as u32);
                } else if let Some(t) = parse_date(&fmt, e) {
                    ex.dates.push(t.div_euclid(DAY));
                }
            }
        }
    }
    // Resolve `after` / `until` (forward references allowed): at most n passes.
    let n = raw.len();
    let mut done = vec![false; n];
    for _ in 0..=n {
        let mut progress = false;
        for i in 0..n {
            if done.get(i) != Some(&false) {
                continue;
            }
            let find = |id: &str| {
                raw.iter()
                    .position(|r| r.task.id.as_deref() == Some(id))
                    .filter(|&k| done.get(k) == Some(&true))
            };
            let Some(r) = raw.get(i) else { continue };
            let start = match &r.start {
                Start::At(t) => Some(*t),
                Start::Prev if i == 0 => {
                    return Err(err("The first task needs a start date", r.line, 0));
                }
                Start::Prev => done
                    .get(i - 1)
                    .copied()
                    .unwrap_or(false)
                    .then(|| raw.get(i - 1).map(|p| p.task.end))
                    .flatten(),
                Start::After(ids) => {
                    let ends: Option<Vec<i64>> = ids
                        .iter()
                        .map(|id| find(id).and_then(|k| raw.get(k)).map(|p| p.task.end))
                        .collect();
                    ends.and_then(|v| v.into_iter().max())
                }
            };
            let Some(start) = start else { continue };
            let end = match &r.end {
                Finish::At(t) => Some(*t),
                Finish::For(_) if r.task.milestone => Some(start),
                Finish::For(d) => Some(start + d),
                Finish::Until(id) => find(id).and_then(|k| raw.get(k)).map(|p| p.task.start),
            };
            let Some(mut end) = end else { continue };
            if !r.task.milestone && (ex.weekends || !ex.days.is_empty() || !ex.dates.is_empty()) {
                // Excluded days inside the task push its end out (Mermaid's fixTaskDates).
                let mut d = start.div_euclid(DAY);
                let mut guard = 0;
                while d * DAY < end && guard < 100_000 {
                    if ex.hit(d) {
                        end += DAY;
                    }
                    d += 1;
                    guard += 1;
                }
            }
            if let Some(r) = raw.get_mut(i) {
                r.task.start = start;
                r.task.end = end.max(start);
            }
            if let Some(x) = done.get_mut(i) {
                *x = true;
            }
            progress = true;
        }
        if !progress {
            break;
        }
    }
    if let Some(i) = done.iter().position(|d| !d) {
        let line = raw.get(i).map_or(0, |r| r.line);
        return Err(err("Unknown or circular task reference", line, 0));
    }
    g.tasks = raw.into_iter().map(|r| r.task).collect();
    if g.sections.is_empty() && !g.tasks.is_empty() {
        g.sections.push(String::new());
    }
    if let (Some(lo), Some(hi)) = (
        g.tasks.iter().map(|t| t.start).min(),
        g.tasks.iter().map(|t| t.end).max(),
    ) {
        let (a, b) = (lo.div_euclid(DAY), hi.div_euclid(DAY));
        g.excluded = (a..=b).filter(|&d| ex.hit(d)).take(400).collect();
    }
    Ok(g)
}

/// Axis tick step (s) giving at most ~10 ticks over `span`.
pub fn tick_step(span: i64) -> i64 {
    const STEPS: [i64; 15] = [
        60,
        300,
        900,
        1800,
        3600,
        3 * 3600,
        6 * 3600,
        12 * 3600,
        DAY,
        2 * DAY,
        7 * DAY,
        14 * DAY,
        30 * DAY,
        91 * DAY,
        365 * DAY,
    ];
    STEPS
        .iter()
        .copied()
        .find(|s| span / s <= 10)
        .unwrap_or(365 * DAY * ((span / (3650 * DAY)).max(1)))
}

fn rect(x: f32, y: f32, w: f32, h: f32, r: f32, fill: Role) -> Prim {
    Prim::Path(Path {
        segs: draw::rrect(x, y, w, h, r),
        fill: Some(Ink::Role(fill)),
        stroke: None,
        width: 0.0,
        dash: false,
    })
}

fn vline(x: f32, y0: f32, y1: f32, role: Role, w: f32) -> Prim {
    Prim::Path(Path {
        segs: draw::poly(&[(x, y0), (x, y1)], false),
        fill: None,
        stroke: Some(Ink::Role(role)),
        width: w,
        dash: false,
    })
}

/// Lays the chart out; `now` is the local wall clock (s since epoch) for the today marker.
#[allow(clippy::too_many_lines)]
pub fn layout(g: &Gantt, m: &mut dyn Measure, s: f32, now: Option<i64>) -> Drawing {
    let mut prims = Vec::new();
    let text = Ink::Role(Role::Text);
    let (row, bar) = (24.0 * s, 20.0 * s);
    let mut top = 8.0 * s;
    let total_w = 760.0 * s;
    if let Some(t) = &g.title {
        let tt = m.text(t, 16.0 * s, total_w, true);
        prims.push(Prim::Text(TextPrim {
            id: tt.0,
            x: (total_w - tt.1) / 2.0,
            y: top,
            color: text,
        }));
        top += tt.2 + 12.0 * s;
    }
    let secs: Vec<(u32, f32, f32)> = g
        .sections
        .iter()
        .map(|n| m.text(n, 13.0 * s, 160.0 * s, true))
        .collect();
    let left = secs.iter().map(|t| t.1).fold(0.0, f32::max) + 16.0 * s;
    let chart_w = total_w - left - 8.0 * s;
    let lo = g.tasks.iter().map(|t| t.start).min().unwrap_or(0);
    let hi = g
        .tasks
        .iter()
        .map(|t| t.end)
        .max()
        .unwrap_or(lo)
        .max(lo + 1);
    let span = hi - lo;
    let px = |t: i64| left + (t - lo) as f32 / span as f32 * chart_w;
    let rows = g.tasks.len() as f32;
    let (y0, y1) = (top, top + rows * row);
    // Section stripes + names.
    for (si, st) in secs.iter().enumerate() {
        let idx: Vec<usize> = (0..g.tasks.len())
            .filter(|&i| g.tasks.get(i).is_some_and(|t| t.section == si))
            .collect();
        let (Some(&a), Some(&b)) = (idx.first(), idx.last()) else {
            continue;
        };
        let (ya, yb) = (y0 + a as f32 * row, y0 + (b + 1) as f32 * row);
        if si % 2 == 0 {
            prims.push(rect(0.0, ya, total_w, yb - ya, 0.0, Role::ClusterFill));
        }
        prims.push(Prim::Text(TextPrim {
            id: st.0,
            x: 6.0 * s,
            y: (ya + yb - st.2) / 2.0,
            color: text,
        }));
    }
    for &d in &g.excluded {
        let (a, b) = (px((d * DAY).max(lo)), px(((d + 1) * DAY).min(hi)));
        if b > a {
            prims.push(rect(a, y0, b - a, y1 - y0, 0.0, Role::Grid));
        }
    }
    // Grid + axis labels.
    let step = tick_step(span);
    let mut t = lo.div_euclid(step) * step;
    if (7 * DAY..30 * DAY).contains(&step) {
        // Weeks start on Monday.
        t = (lo.div_euclid(DAY) - i64::from((weekday(lo.div_euclid(DAY)) + 6) % 7)) * DAY;
    }
    let mut label_h: f32 = 0.0;
    let mut guard = 0;
    while t <= hi && guard < 64 {
        if t >= lo {
            let x = px(t);
            prims.push(vline(x, y0, y1, Role::Grid, s));
            let l = m.text(&strftime(&g.axis, t), 11.0 * s, 120.0 * s, false);
            prims.push(Prim::Text(TextPrim {
                id: l.0,
                x: x - l.1 / 2.0,
                y: y1 + 4.0 * s,
                color: text,
            }));
            label_h = label_h.max(l.2);
        }
        t += step;
        guard += 1;
    }
    let mut boxes = Vec::new();
    for (i, k) in g.tasks.iter().enumerate() {
        let y = y0 + i as f32 * row + (row - bar) / 2.0;
        let (xa, xb) = (px(k.start), px(k.end));
        let role = if k.crit {
            Role::GanttCrit
        } else if k.milestone {
            Role::GanttMilestone
        } else if k.done {
            Role::GanttDone
        } else if k.active {
            Role::GanttActive
        } else {
            Role::GanttBar
        };
        let lt = m.text(&k.name, 12.0 * s, chart_w, false);
        if k.milestone {
            let (cx, cy, r) = (xa, y + bar / 2.0, bar / 2.0);
            prims.push(Prim::Path(Path {
                segs: draw::poly(
                    &[(cx, cy - r), (cx + r, cy), (cx, cy + r), (cx - r, cy)],
                    true,
                ),
                fill: Some(Ink::Role(role)),
                stroke: None,
                width: 0.0,
                dash: false,
            }));
            boxes.push((cx - r, cy - r, 2.0 * r, 2.0 * r));
            prims.push(Prim::Text(TextPrim {
                id: lt.0,
                x: cx + r + 4.0 * s,
                y: cy - lt.2 / 2.0,
                color: text,
            }));
            continue;
        }
        let w = (xb - xa).max(2.0 * s);
        prims.push(rect(xa, y, w, bar, 3.0 * s, role));
        if k.crit && k.done {
            prims.push(Prim::Path(Path {
                segs: draw::rrect(xa, y, w, bar, 3.0 * s),
                fill: None,
                stroke: Some(Ink::Role(Role::GanttDone)),
                width: 2.0 * s,
                dash: false,
            }));
        }
        boxes.push((xa, y, w, bar));
        let inside = lt.1 + 8.0 * s <= w;
        let tx = if inside {
            xa + (w - lt.1) / 2.0
        } else if xb + 4.0 * s + lt.1 <= total_w {
            xb + 4.0 * s
        } else {
            (xa - 4.0 * s - lt.1).max(0.0)
        };
        prims.push(Prim::Text(TextPrim {
            id: lt.0,
            x: tx,
            y: y + (bar - lt.2) / 2.0,
            color: text,
        }));
    }
    if let Some(n) = now.filter(|&n| !g.no_today && n >= lo && n <= hi) {
        prims.push(vline(px(n), y0 - 4.0 * s, y1, Role::Today, 2.0 * s));
    }
    Drawing {
        w: total_w,
        h: y1 + label_h + 12.0 * s,
        prims,
        boxes,
        edges: Vec::new(),
        complex: false,
    }
}
