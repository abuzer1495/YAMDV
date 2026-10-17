//! `state.ini` model (ARCHITECTURE §E.12, R-SET-05): window geometry, history (R-NAV-08),
//! favorites (R-NAV-09), reading positions (R-READ-02), sidebar/zoom state. Pure; the platform
//! layer reads and atomically writes the file.

use crate::ini::Ini;

/// History cap (R-NAV-08).
pub const MAX_HISTORY: usize = 500;
/// Reading-position LRU cap (R-READ-02).
pub const MAX_POSITIONS: usize = 2000;
/// Favorites cap.
pub const MAX_FAVORITES: usize = 500;

/// Saved window rectangle in virtual-screen pixels (R-WIN-03).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct WinGeom {
    /// Left edge.
    pub x: i32,
    /// Top edge.
    pub y: i32,
    /// Width.
    pub w: i32,
    /// Height.
    pub h: i32,
    /// Maximized.
    pub max: bool,
}

/// A monitor work area `(left, top, right, bottom)`.
pub type WorkArea = (i32, i32, i32, i32);

/// Validates `g` against the current monitors (R-WIN-03): the title bar strip (top 32 px, 64 px
/// of width) must be on some monitor, the size is clamped to that monitor; otherwise the window
/// is centered on `primary`. `None` when there are no monitors.
pub fn validate_geom(g: WinGeom, monitors: &[WorkArea], primary: WorkArea) -> WinGeom {
    let visible = |m: &WorkArea| {
        let (l, t, r, b) = *m;
        let ix = (g.x + g.w).min(r) - g.x.max(l);
        g.y >= t && g.y + 32 <= b && ix >= 64
    };
    let fit = |m: WorkArea, g: WinGeom| {
        let (l, t, r, b) = m;
        let w = g.w.clamp(480, (r - l).max(480));
        let h = g.h.clamp(360, (b - t).max(360));
        let x = g.x.clamp(l, (r - w).max(l));
        let y = g.y.clamp(t, (b - h).max(t));
        WinGeom {
            x,
            y,
            w,
            h,
            max: g.max,
        }
    };
    if g.w > 0
        && g.h > 0
        && let Some(m) = monitors.iter().find(|m| visible(m))
    {
        return fit(*m, g);
    }
    let (l, t, r, b) = primary;
    let w = if g.w > 0 { g.w } else { 1100 };
    let h = if g.h > 0 { g.h } else { 760 };
    fit(
        primary,
        WinGeom {
            x: l + ((r - l) - w) / 2,
            y: t + ((b - t) - h) / 2,
            w,
            h,
            max: g.max,
        },
    )
}

/// A saved reading position (R-READ-02).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Position {
    /// Index of the top block.
    pub block: u32,
    /// Offset inside that block in parts per million of its height.
    pub ppm: u32,
}

/// The whole `state.ini`.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct State {
    /// Last window geometry.
    pub window: Option<WinGeom>,
    /// `(unix_ms, path)`, newest first.
    pub history: Vec<(i64, String)>,
    /// Favorite paths in insertion order.
    pub favorites: Vec<String>,
    /// `(unix_ms, path, position)`, newest first.
    pub positions: Vec<(i64, String, Position)>,
    /// Sidebar tab index (0 Contents, 1 History, 2 Favorites).
    pub sidebar_tab: u8,
    /// Collapsed TOC heading block indexes per document.
    pub toc: Vec<(String, Vec<u32>)>,
}

fn key_eq(a: &str, b: &str) -> bool {
    // Windows paths are case-insensitive; ASCII folding is enough for de-duplication.
    a.eq_ignore_ascii_case(b)
}

impl State {
    /// Parses `state.ini` (malformed entries are skipped).
    pub fn parse(ini: &Ini) -> Self {
        let mut s = Self::default();
        if let Some(v) = ini.get("window", "rect") {
            let n: Vec<i32> = v.split(',').filter_map(|x| x.trim().parse().ok()).collect();
            if let [x, y, w, h, m] = n.as_slice() {
                s.window = Some(WinGeom {
                    x: *x,
                    y: *y,
                    w: *w,
                    h: *h,
                    max: *m != 0,
                });
            }
        }
        for v in ini.get_all("history", "item") {
            if let Some((t, p)) = v.split_once('|')
                && let Ok(t) = t.parse()
                && !p.is_empty()
                && s.history.len() < MAX_HISTORY
            {
                s.history.push((t, p.to_string()));
            }
        }
        s.history.sort_by_key(|x| std::cmp::Reverse(x.0));
        for v in ini.get_all("favorites", "item") {
            if !v.is_empty() && s.favorites.len() < MAX_FAVORITES {
                s.favorites.push(v.to_string());
            }
        }
        for v in ini.get_all("positions", "item") {
            let mut it = v.splitn(4, '|');
            if let (Some(t), Some(b), Some(o), Some(p)) =
                (it.next(), it.next(), it.next(), it.next())
                && let (Ok(t), Ok(block), Ok(ppm)) = (t.parse(), b.parse(), o.parse::<u32>())
                && s.positions.len() < MAX_POSITIONS
            {
                let ppm = ppm.min(1_000_000);
                s.positions
                    .push((t, p.to_string(), Position { block, ppm }));
            }
        }
        s.positions.sort_by_key(|x| std::cmp::Reverse(x.0));
        for v in ini.get_all("toc", "item") {
            let mut parts = v.split('|');
            if let Some(p) = parts.next().filter(|p| !p.is_empty()) {
                let ids = parts.filter_map(|x| x.parse().ok()).collect();
                s.toc.push((p.to_string(), ids));
            }
        }
        s.sidebar_tab = ini
            .get("sidebar", "tab")
            .and_then(|v| v.parse().ok())
            .filter(|t| *t <= 2)
            .unwrap_or(0);
        s
    }

    /// Serializes to INI text.
    pub fn write(&self) -> String {
        let mut ini = Ini::default();
        if let Some(g) = self.window {
            ini.set(
                "window",
                "rect",
                &format!("{},{},{},{},{}", g.x, g.y, g.w, g.h, u8::from(g.max)),
            );
        }
        ini.set("sidebar", "tab", &self.sidebar_tab.to_string());
        let mut out = ini.write();
        let mut list = |sec: &str, items: Vec<String>| {
            out.push_str(&format!("[{sec}]\n"));
            for i in items {
                out.push_str("item = ");
                out.push_str(&crate::ini::quote(&i));
                out.push('\n');
            }
        };
        list(
            "history",
            self.history
                .iter()
                .map(|(t, p)| format!("{t}|{p}"))
                .collect(),
        );
        list("favorites", self.favorites.clone());
        list(
            "positions",
            self.positions
                .iter()
                .map(|(t, p, q)| format!("{t}|{}|{}|{p}", q.block, q.ppm))
                .collect(),
        );
        list(
            "toc",
            self.toc
                .iter()
                .filter(|(_, v)| !v.is_empty())
                .map(|(p, v)| {
                    let ids: Vec<String> = v.iter().map(u32::to_string).collect();
                    format!("{p}|{}", ids.join("|"))
                })
                .collect(),
        );
        out
    }

    /// Records an open (R-OPEN-13): moves `path` to the top.
    pub fn add_history(&mut self, path: &str, now_ms: i64) {
        self.history.retain(|(_, p)| !key_eq(p, path));
        self.history.insert(0, (now_ms, path.to_string()));
        self.history.truncate(MAX_HISTORY);
    }

    /// Removes one history entry.
    pub fn remove_history(&mut self, path: &str) {
        self.history.retain(|(_, p)| !key_eq(p, path));
    }

    /// `true` when `path` is a favorite.
    pub fn is_favorite(&self, path: &str) -> bool {
        self.favorites.iter().any(|p| key_eq(p, path))
    }

    /// Adds or removes a favorite; returns the new state (`true` = favorite).
    pub fn toggle_favorite(&mut self, path: &str) -> bool {
        if self.is_favorite(path) {
            self.favorites.retain(|p| !key_eq(p, path));
            false
        } else {
            if self.favorites.len() < MAX_FAVORITES {
                self.favorites.push(path.to_string());
            }
            true
        }
    }

    /// Saved position of `path`.
    pub fn position(&self, path: &str) -> Option<Position> {
        self.positions
            .iter()
            .find(|(_, p, _)| key_eq(p, path))
            .map(|x| x.2)
    }

    /// Stores the position of `path` (LRU, R-READ-02).
    pub fn set_position(&mut self, path: &str, pos: Position, now_ms: i64) {
        self.positions.retain(|(_, p, _)| !key_eq(p, path));
        self.positions.insert(0, (now_ms, path.to_string(), pos));
        self.positions.truncate(MAX_POSITIONS);
    }

    /// Collapsed TOC nodes of `path`.
    pub fn toc_collapsed(&self, path: &str) -> &[u32] {
        self.toc
            .iter()
            .find(|(p, _)| key_eq(p, path))
            .map_or(&[], |x| x.1.as_slice())
    }

    /// Replaces the collapsed TOC nodes of `path`.
    pub fn set_toc_collapsed(&mut self, path: &str, ids: Vec<u32>) {
        self.toc.retain(|(p, _)| !key_eq(p, path));
        if !ids.is_empty() {
            self.toc.insert(0, (path.to_string(), ids));
            self.toc.truncate(MAX_POSITIONS);
        }
    }
}

/// History group (R-NAV-08).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Group {
    /// Opened since this process started.
    Session,
    /// Today (local).
    Today,
    /// Yesterday (local).
    Yesterday,
    /// An older local date `(year, month, day)`.
    Date(i32, u32, u32),
}

/// Local civil date of `unix_ms` shifted by `tz_offset_min`.
pub fn local_date(unix_ms: i64, tz_offset_min: i32) -> (i32, u32, u32) {
    let days = (unix_ms + i64::from(tz_offset_min) * 60_000).div_euclid(86_400_000);
    // Howard Hinnant's civil_from_days.
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = u32::try_from(doy - (153 * mp + 2) / 5 + 1).unwrap_or(1);
    let m = u32::try_from(if mp < 10 { mp + 3 } else { mp - 9 }).unwrap_or(1);
    let y = i32::try_from(yoe + era * 400 + i64::from(m <= 2)).unwrap_or(1970);
    (y, m, d)
}

/// Groups history entries (newest first) for the sidebar.
pub fn group_history(
    history: &[(i64, String)],
    session_start_ms: i64,
    now_ms: i64,
    tz_offset_min: i32,
) -> Vec<(Group, Vec<usize>)> {
    let today = local_date(now_ms, tz_offset_min);
    let yesterday = local_date(now_ms - 86_400_000, tz_offset_min);
    let mut out: Vec<(Group, Vec<usize>)> = Vec::new();
    for (i, (t, _)) in history.iter().enumerate() {
        let g = if *t >= session_start_ms {
            Group::Session
        } else {
            let d = local_date(*t, tz_offset_min);
            if d == today {
                Group::Today
            } else if d == yesterday {
                Group::Yesterday
            } else {
                Group::Date(d.0, d.1, d.2)
            }
        };
        match out.iter_mut().find(|(x, _)| *x == g) {
            Some((_, v)) => v.push(i),
            None => out.push((g, vec![i])),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn r_set_05_state_round_trip() {
        let mut s = State {
            window: Some(WinGeom {
                x: 10,
                y: 20,
                w: 900,
                h: 700,
                max: true,
            }),
            sidebar_tab: 2,
            ..State::default()
        };
        s.add_history("C:\\a b|c.md", 5);
        s.add_history("C:\\x.md", 9);
        s.toggle_favorite("C:\\x.md");
        s.set_position("C:\\x.md", Position { block: 4, ppm: 500 }, 9);
        s.set_toc_collapsed("C:\\x.md", vec![3, 7]);
        let back = State::parse(&Ini::parse(s.write().as_bytes()));
        assert_eq!(back, s);
    }

    #[test]
    fn r_nav_08_history_dedup_cap_and_groups() {
        let mut s = State::default();
        for i in 0..600 {
            s.add_history(&format!("C:\\{i}.md"), i);
        }
        s.add_history("c:\\5.MD", 1000);
        assert_eq!(s.history.len(), MAX_HISTORY);
        assert_eq!(s.history[0].1, "c:\\5.MD");
        let day = 86_400_000;
        let now = 20_000 * day + 3_600_000;
        let h = vec![
            (now - 10, "s".to_string()),
            (now - 2_000_000, "t".to_string()),
            (now - day, "y".to_string()),
            (now - 3 * day, "d".to_string()),
        ];
        let g = group_history(&h, now - 1000, now, 0);
        let kinds: Vec<_> = g.iter().map(|x| x.0.clone()).collect();
        assert_eq!(kinds[0], Group::Session);
        assert_eq!(kinds[1], Group::Today);
        assert_eq!(kinds[2], Group::Yesterday);
        assert!(matches!(kinds[3], Group::Date(..)));
        assert_eq!(local_date(0, 0), (1970, 1, 1));
        assert_eq!(local_date(951_782_400_000, 0), (2000, 2, 29));
    }

    #[test]
    fn r_nav_09_favorites_toggle() {
        let mut s = State::default();
        assert!(s.toggle_favorite("C:\\a.md"));
        assert!(s.is_favorite("c:\\A.md"));
        assert!(!s.toggle_favorite("C:\\A.md"));
        assert!(s.favorites.is_empty());
    }

    #[test]
    fn r_win_03_geometry_validation() {
        let mons = [(0, 0, 1920, 1040), (1920, 0, 3840, 1040)];
        let ok = WinGeom {
            x: 2000,
            y: 100,
            w: 900,
            h: 600,
            max: false,
        };
        assert_eq!(validate_geom(ok, &mons, mons[0]), ok);
        let off = WinGeom {
            x: 5000,
            y: 100,
            w: 900,
            h: 600,
            max: false,
        };
        let g = validate_geom(off, &mons, mons[0]);
        assert!(g.x >= 0 && g.x + g.w <= 1920);
        let huge = WinGeom {
            x: 0,
            y: 0,
            w: 5000,
            h: 5000,
            max: false,
        };
        let g = validate_geom(huge, &mons, mons[0]);
        assert_eq!((g.w, g.h), (1920, 1040));
    }
}
