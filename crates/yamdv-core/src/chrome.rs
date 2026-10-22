//! Window chrome logic of Phase V2-A (prompt 011 §3): chrome tokens (B.3.5), the title-bar
//! layout and hit test (B.4), the word count (B.10.3, R-READ-07), middle ellipsis, popup-menu
//! navigation and placement (B.7.1) and the color-scheme card grid (B.7.3). Pure; the platform
//! layer draws and routes input.

/// Chrome tokens (B.3.5) for one mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Tokens {
    /// Sidebar hairline, splitter, cards.
    pub chrome_border: u32,
    /// Hover overlay color and alpha (percent).
    pub hover: (u32, u8),
    /// Pressed overlay color and alpha (percent).
    pub pressed: (u32, u8),
    /// Menu background.
    pub menu_bg: u32,
    /// Menu border.
    pub menu_border: u32,
    /// Hovered / keyboard-selected menu item.
    pub menu_hover: u32,
    /// Menu separator.
    pub separator: u32,
    /// Toast / zoom indicator background (alpha 92 %).
    pub toast_bg: u32,
    /// Toast text.
    pub toast_text: u32,
}

/// `caption_close_hover` (B.3.5), both modes.
pub const CLOSE_HOVER: u32 = 0xc42b1c;

/// Chrome tokens for light (`false`) or dark (`true`).
pub fn tokens(dark: bool) -> Tokens {
    if dark {
        Tokens {
            chrome_border: 0x333333,
            hover: (0xffffff, 6),
            pressed: (0xffffff, 10),
            menu_bg: 0x2b2b2b,
            menu_border: 0x3a3a3a,
            menu_hover: 0x383838,
            separator: 0x3a3a3a,
            toast_bg: 0x202020,
            toast_text: 0xffffff,
        }
    } else {
        Tokens {
            chrome_border: 0xe0e0e0,
            hover: (0x000000, 5),
            pressed: (0x000000, 9),
            menu_bg: 0xffffff,
            menu_border: 0xe0e0e0,
            menu_hover: 0xf0f0f0,
            separator: 0xeeeeee,
            toast_bg: 0x202020,
            toast_text: 0xffffff,
        }
    }
}

// ---- title bar (B.4) ------------------------------------------------------------------------

/// Title-bar height in DIPs.
pub const BAR_H: f32 = 32.0;
/// Caption button width (Minimize / Maximize / Close).
pub const CAPTION_W: f32 = 46.0;
/// ⚙ / ⋯ button width.
pub const TOOL_W: f32 = 32.0;
/// Gap between ⋯ and Minimize.
pub const TOOL_GAP: f32 = 4.0;
/// Resize border at the top edge (the other edges keep the system frame).
pub const RESIZE: f32 = 6.0;

/// A title-bar element.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Part {
    /// ⚙ Settings.
    Settings,
    /// ⋯ More.
    More,
    /// Minimize.
    Min,
    /// Maximize / Restore.
    Max,
    /// Close.
    Close,
    /// Drag area (`HTCAPTION`).
    Caption,
    /// Top resize border (`HTTOP`).
    Top,
}

/// The buttons from left to right with their x position (DIPs) for a bar of width `w`.
pub fn buttons(w: f32) -> [(Part, f32, f32); 5] {
    let close = w - CAPTION_W;
    let max = close - CAPTION_W;
    let min = max - CAPTION_W;
    let more = min - TOOL_GAP - TOOL_W;
    let set = more - TOOL_W;
    [
        (Part::Settings, set, TOOL_W),
        (Part::More, more, TOOL_W),
        (Part::Min, min, CAPTION_W),
        (Part::Max, max, CAPTION_W),
        (Part::Close, close, CAPTION_W),
    ]
}

/// Hit test of client point `(x, y)` (DIPs) in a bar of width `w`; `None` below the bar.
/// `maximized` windows have no top resize border.
pub fn hit(x: f32, y: f32, w: f32, maximized: bool) -> Option<Part> {
    if !(0.0..BAR_H).contains(&y) {
        return None;
    }
    for (p, bx, bw) in buttons(w) {
        if x >= bx && x < bx + bw {
            // The top resize border stays reachable over the caption buttons except Close.
            if !maximized && y < RESIZE / 2.0 && p != Part::Close {
                return Some(Part::Top);
            }
            return Some(p);
        }
    }
    if !maximized && y < RESIZE {
        return Some(Part::Top);
    }
    Some(Part::Caption)
}

/// Right edge of the title text area (left of ⚙, 8 px gap).
pub fn title_right(w: f32) -> f32 {
    buttons(w)[0].1 - 8.0
}

/// Middle ellipsis: the longest `head…tail` form of `s` for which `fits` is true (prefers the
/// tail, which holds the file name). Returns `s` when it fits, `…` when nothing does.
pub fn ellipsize_middle(s: &str, fits: impl Fn(&str) -> bool) -> String {
    if fits(s) {
        return s.to_string();
    }
    let chars: Vec<char> = s.chars().collect();
    let n = chars.len();
    let build = |keep: usize| -> String {
        let tail = keep.div_ceil(2) + keep / 6;
        let tail = tail.min(keep);
        let head = keep - tail;
        let mut out: String = chars.iter().take(head).collect();
        out.push('\u{2026}');
        out.extend(chars.iter().skip(n - tail));
        out
    };
    let (mut lo, mut hi) = (0usize, n.saturating_sub(1));
    let mut best = "\u{2026}".to_string();
    while lo <= hi {
        let mid = usize::midpoint(lo, hi);
        let t = build(mid);
        if fits(&t) {
            best = t;
            lo = mid + 1;
        } else if mid == 0 {
            break;
        } else {
            hi = mid - 1;
        }
    }
    best
}

// ---- word count (B.10.3, R-READ-07) ---------------------------------------------------------

/// CJK ideographs, kana and Hangul syllables count as one word each.
pub fn is_cjk(c: char) -> bool {
    matches!(c as u32,
        0x3040..=0x30FF       // Hiragana, Katakana
        | 0x3400..=0x4DBF     // CJK Ext A
        | 0x4E00..=0x9FFF     // CJK Unified
        | 0xAC00..=0xD7AF     // Hangul syllables
        | 0xF900..=0xFAFF     // CJK compatibility
        | 0x20000..=0x2FA1F) // CJK Ext B…
}

/// Counts `(words, characters)` of `text`. Characters exclude whitespace; a word is a run of
/// letters/digits (apostrophes and hyphens inside a word join it); every CJK character is a word.
pub fn count(text: &str) -> (usize, usize) {
    let (mut words, mut chars) = (0usize, 0usize);
    let mut in_word = false;
    let mut prev_joiner = false;
    for c in text.chars() {
        if !c.is_whitespace() {
            chars += 1;
        }
        if is_cjk(c) {
            words += 1;
            in_word = false;
        } else if c.is_alphanumeric() {
            if !in_word {
                words += 1;
                in_word = true;
            }
            prev_joiner = false;
            continue;
        } else if in_word && (c == '\'' || c == '\u{2019}' || c == '-') && !prev_joiner {
            prev_joiner = true;
            continue;
        } else {
            in_word = false;
        }
        prev_joiner = false;
    }
    (words, chars)
}

/// Markdown source → countable text: drops fenced-code fences, HTML comments' markers,
/// heading/list/quote markers and emphasis/link punctuation (counts what a reader sees, roughly).
pub fn reading_text(src: &str) -> String {
    let mut out = String::with_capacity(src.len());
    for line in src.lines() {
        let t = line.trim_start();
        if t.starts_with("```") || t.starts_with("~~~") {
            continue;
        }
        let t = t.trim_start_matches(['#', '>', ' ']);
        let t = t
            .strip_prefix("- [ ] ")
            .or_else(|| t.strip_prefix("- [x] "))
            .or_else(|| t.strip_prefix("- "))
            .or_else(|| t.strip_prefix("* "))
            .or_else(|| t.strip_prefix("+ "))
            .unwrap_or(t);
        for c in t.chars() {
            if !matches!(c, '*' | '_' | '`' | '[' | ']' | '|' | '~' | '=') {
                out.push(c);
            }
        }
        out.push('\n');
    }
    out
}

// ---- popup menus (B.7.1) --------------------------------------------------------------------

/// Next selectable index from `cur` in direction `dir` (±1), wrapping; `sel[i]` tells whether
/// entry `i` is selectable (separators are not). `None` when nothing is selectable.
pub fn step(sel: &[bool], cur: Option<usize>, dir: i32) -> Option<usize> {
    let n = sel.len();
    if n == 0 || !sel.iter().any(|s| *s) {
        return None;
    }
    let mut i = match cur {
        Some(c) => c.min(n - 1),
        None if dir > 0 => n - 1,
        None => 0,
    };
    for _ in 0..n {
        i = if dir > 0 {
            (i + 1) % n
        } else {
            (i + n - 1) % n
        };
        if sel.get(i).copied().unwrap_or(false) {
            return Some(i);
        }
    }
    None
}

/// First / last selectable index (`Home` / `End`).
pub fn edge(sel: &[bool], last: bool) -> Option<usize> {
    if last {
        sel.iter().rposition(|s| *s)
    } else {
        sel.iter().position(|s| *s)
    }
}

/// Type-to-jump: the next entry after `cur` whose label starts with `ch` (case-insensitive,
/// first letter or digit of the label), wrapping.
pub fn jump(labels: &[Option<&str>], cur: Option<usize>, ch: char) -> Option<usize> {
    let n = labels.len();
    let want: String = ch.to_lowercase().collect();
    let start = cur.map_or(0, |c| c + 1);
    (0..n).map(|k| (start + k) % n).find(|&i| {
        labels.get(i).copied().flatten().is_some_and(|l| {
            l.chars()
                .find(|c| c.is_alphanumeric())
                .is_some_and(|c| c.to_lowercase().collect::<String>() == want)
        })
    })
}

/// A screen rectangle `(left, top, right, bottom)` in pixels.
pub type Px = (i32, i32, i32, i32);

/// Places a popup of size `(w, h)` (pixels) at `anchor` inside the monitor work area `work`,
/// flipping to stay on it. `parent` is the item rectangle of a submenu's parent row: the
/// submenu opens to its right (or left when that does not fit), aligned with the row.
pub fn place(anchor: (i32, i32), (w, h): (i32, i32), work: Px, parent: Option<Px>) -> (i32, i32) {
    let (wl, wt, wr, wb) = work;
    let (mut x, mut y) = match parent {
        Some((l, t, r, _)) => {
            if r + w <= wr {
                (r, t)
            } else {
                (l - w, t)
            }
        }
        None => anchor,
    };
    if parent.is_none() && x + w > wr {
        x = anchor.0 - w;
    }
    if y + h > wb {
        y = match parent {
            Some((_, _, _, b)) => b - h,
            None => anchor.1 - h,
        };
    }
    x = x.clamp(wl, (wr - w).max(wl));
    y = y.clamp(wt, (wb - h).max(wt));
    (x, y)
}

// ---- color-scheme cards (B.7.3) -------------------------------------------------------------

/// Card size, columns and gaps (DIPs).
pub const CARD_W: f32 = 112.0;
/// Card height.
pub const CARD_H: f32 = 84.0;
/// Columns.
pub const CARD_COLS: usize = 3;
/// Gap between cards and around the grid.
pub const CARD_GAP: f32 = 12.0;

/// Flyout size `(w, h)` for `n` cards.
pub fn cards_size(n: usize) -> (f32, f32) {
    let rows = n.div_ceil(CARD_COLS);
    let cols = CARD_COLS.min(n.max(1));
    (
        CARD_GAP + cols as f32 * (CARD_W + CARD_GAP),
        CARD_GAP + rows as f32 * (CARD_H + CARD_GAP),
    )
}

/// Rectangle `(x, y, w, h)` of card `i`.
pub fn card_rect(i: usize) -> (f32, f32, f32, f32) {
    let (c, r) = (i % CARD_COLS, i / CARD_COLS);
    (
        CARD_GAP + c as f32 * (CARD_W + CARD_GAP),
        CARD_GAP + r as f32 * (CARD_H + CARD_GAP),
        CARD_W,
        CARD_H,
    )
}

/// Card under flyout point `(x, y)` of `n` cards.
pub fn card_at(x: f32, y: f32, n: usize) -> Option<usize> {
    (0..n).find(|&i| {
        let (cx, cy, cw, chh) = card_rect(i);
        x >= cx && x < cx + cw && y >= cy && y < cy + chh
    })
}

/// Keyboard move in the card grid: `dx`/`dy` ∈ {−1, 0, 1}; stays inside `0..n`.
pub fn card_move(cur: usize, dx: i32, dy: i32, n: usize) -> usize {
    let delta = i64::from(dx) + i64::from(dy) * 3;
    let i = i64::try_from(cur).unwrap_or(0) + delta;
    usize::try_from(i).ok().filter(|&i| i < n).unwrap_or(cur)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn word_count_latin_and_cjk() {
        assert_eq!(count("Hello, world!"), (2, 12));
        assert_eq!(count("don't stop — e-mail"), (3, 16));
        assert_eq!(count("日本語のテキスト"), (8, 8));
        assert_eq!(count("YAMDV 是 好的"), (4, 8));
        assert_eq!(count("  \n\t "), (0, 0));
        assert_eq!(count("a--b"), (2, 4));
    }

    #[test]
    fn reading_text_strips_markup() {
        let t = reading_text("# Title\n\n- [x] **done** item\n```rust\nlet x = 1;\n```\n> quote");
        assert_eq!(count(&t).0, 7);
        assert!(!t.contains('#') && !t.contains('*') && !t.contains("```"));
    }

    #[test]
    fn caption_hit_test() {
        let w = 1000.0;
        assert_eq!(hit(999.0, 16.0, w, false), Some(Part::Close));
        assert_eq!(hit(1000.0 - 46.0 - 1.0, 16.0, w, false), Some(Part::Max));
        assert_eq!(hit(1000.0 - 92.0 - 1.0, 16.0, w, false), Some(Part::Min));
        assert_eq!(
            hit(1000.0 - 138.0 - 4.0 - 1.0, 16.0, w, false),
            Some(Part::More)
        );
        assert_eq!(
            hit(1000.0 - 138.0 - 36.0 - 1.0, 16.0, w, false),
            Some(Part::Settings)
        );
        // The 4 px gap and the title area are caption.
        assert_eq!(
            hit(1000.0 - 138.0 - 2.0, 16.0, w, false),
            Some(Part::Caption)
        );
        assert_eq!(hit(300.0, 16.0, w, false), Some(Part::Caption));
        assert_eq!(hit(300.0, 2.0, w, false), Some(Part::Top));
        assert_eq!(hit(300.0, 2.0, w, true), Some(Part::Caption));
        assert_eq!(hit(999.0, 1.0, w, false), Some(Part::Close));
        assert_eq!(hit(300.0, 40.0, w, false), None);
        assert!(title_right(w) < buttons(w)[0].1);
    }

    #[test]
    fn middle_ellipsis_keeps_both_ends() {
        let fits = |s: &str| s.chars().count() <= 12;
        assert_eq!(ellipsize_middle("short.md", fits), "short.md");
        let e = ellipsize_middle("C:\\Users\\me\\Documents\\notes.md", fits);
        assert_eq!(e.chars().count(), 12);
        assert!(e.contains('\u{2026}') && e.starts_with('C') && e.ends_with(".md"));
        assert_eq!(ellipsize_middle("abc", |_| false), "\u{2026}");
    }

    #[test]
    fn menu_navigation() {
        let sel = [true, false, true, true];
        assert_eq!(step(&sel, None, 1), Some(0));
        assert_eq!(step(&sel, Some(0), 1), Some(2));
        assert_eq!(step(&sel, Some(3), 1), Some(0));
        assert_eq!(step(&sel, Some(0), -1), Some(3));
        assert_eq!(step(&sel, None, -1), Some(3));
        assert_eq!(step(&[false], None, 1), None);
        assert_eq!(edge(&sel, false), Some(0));
        assert_eq!(edge(&sel, true), Some(3));
        let labels = [
            Some("Copy"),
            None,
            Some("Close"),
            Some("&About"),
            Some("copy 2"),
        ];
        assert_eq!(jump(&labels, None, 'c'), Some(0));
        assert_eq!(jump(&labels, Some(0), 'C'), Some(2));
        assert_eq!(jump(&labels, Some(2), 'c'), Some(4));
        assert_eq!(jump(&labels, Some(4), 'c'), Some(0));
        assert_eq!(jump(&labels, None, 'a'), Some(3));
        assert_eq!(jump(&labels, None, 'z'), None);
    }

    #[test]
    fn popup_placement_flips() {
        let work = (0, 0, 1000, 800);
        assert_eq!(place((100, 100), (200, 300), work, None), (100, 100));
        assert_eq!(place((900, 700), (200, 300), work, None), (700, 400));
        // Submenu: right of the parent row, or left when it does not fit.
        assert_eq!(
            place((0, 0), (200, 100), work, Some((100, 50, 300, 82))),
            (300, 50)
        );
        assert_eq!(
            place((0, 0), (200, 100), work, Some((700, 50, 900, 82))),
            (500, 50)
        );
        // Bottom flip aligns the submenu with the row bottom.
        assert_eq!(
            place((0, 0), (200, 300), work, Some((100, 700, 300, 732))),
            (300, 432)
        );
        // Never leaves the work area.
        assert_eq!(place((-50, -50), (200, 300), work, None), (0, 0));
    }

    #[test]
    fn card_grid() {
        let (w, h) = cards_size(11);
        assert!((w - (12.0 + 3.0 * 124.0)).abs() < 0.01);
        assert!((h - (12.0 + 4.0 * 96.0)).abs() < 0.01);
        assert_eq!(card_at(13.0, 13.0, 11), Some(0));
        assert_eq!(card_at(12.0 + 124.0 + 1.0, 12.0 + 96.0 + 1.0, 11), Some(4));
        assert_eq!(card_at(5.0, 5.0, 11), None);
        assert_eq!(
            card_at(12.0 + 2.0 * 124.0 + 1.0, 12.0 + 3.0 * 96.0 + 1.0, 11),
            None
        );
        assert_eq!(card_move(0, 1, 0, 11), 1);
        assert_eq!(card_move(0, -1, 0, 11), 0);
        assert_eq!(card_move(7, 0, 1, 11), 10);
        assert_eq!(card_move(9, 0, 1, 11), 9);
    }

    #[test]
    fn tokens_per_mode() {
        assert_eq!(tokens(false).menu_bg, 0xffffff);
        assert_eq!(tokens(true).menu_bg, 0x2b2b2b);
        assert_eq!(tokens(true).menu_hover, 0x383838);
    }
}
