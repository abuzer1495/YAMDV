//! Font warm-up plan (F-10): the styles a document uses, each with a tiny sample, so the
//! platform can create text formats and lay out samples in idle time after the first paint.

use crate::block::Metrics;
use crate::shaper::{Family, Style};

/// One warm-up task: a style and a short sample string.
pub type WarmTask = (Style, &'static str);

fn st(family: Family, size: f32, weight: u16, italic: bool) -> Style {
    Style {
        family,
        size,
        weight,
        italic,
        color: 0,
        strike: false,
        underline: false,
    }
}

/// True for CJK ideographs, kana, hangul and full-width forms.
fn is_cjk(c: char) -> bool {
    matches!(u32::from(c), 0x2E80..=0x9FFF | 0xAC00..=0xD7AF | 0xF900..=0xFAFF | 0xFF00..=0xFFEF | 0x20000..=0x2FFFF)
}

/// True for emoji / pictographs.
fn is_emoji(c: char) -> bool {
    matches!(u32::from(c), 0x1F000..=0x1FAFF | 0x2600..=0x27BF)
}

/// Warm-up tasks for a document: regular, bold, italic, bold-italic, code, heading weights; CJK
/// and emoji fallback only if the text contains such characters. Deduplicated by
/// (family, size, weight, italic).
pub fn warm_plan(m: &Metrics, text: &str) -> Vec<WarmTask> {
    const S: &str = "Ag fi 0";
    let b = m.font_size;
    let mut v: Vec<WarmTask> = vec![
        (st(Family::Body, b, 400, false), S),
        (st(Family::Body, b, 600, false), S),
        (st(Family::Body, b, 400, true), S),
        (st(Family::Body, b, 600, true), S),
        (st(Family::Code, m.code_size(), 400, false), S),
    ];
    for l in 1..=6 {
        v.push((st(Family::Body, m.heading_size(l), 600, false), S));
    }
    let (mut cjk, mut emoji) = (false, false);
    for c in text.chars() {
        cjk |= is_cjk(c);
        emoji |= is_emoji(c);
        if cjk && emoji {
            break;
        }
    }
    if cjk {
        v.push((st(Family::Body, b, 400, false), "漢字かな한글"));
    }
    if emoji {
        v.push((st(Family::Body, b, 400, false), "😀👍"));
    }
    let mut seen = Vec::new();
    v.retain(|(s, t)| {
        let k = (
            s.family == Family::Code,
            s.size.to_bits(),
            s.weight,
            s.italic,
            *t,
        );
        if seen.contains(&k) {
            false
        } else {
            seen.push(k);
            true
        }
    });
    v
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plain_ascii_has_no_fallback_tasks() {
        let p = warm_plan(&Metrics::default(), "hello");
        // h5 = 16 px/600 duplicates bold body → deduplicated.
        assert_eq!(p.len(), 10);
        assert!(p.iter().all(|(_, t)| t.is_ascii()));
    }

    #[test]
    fn cjk_and_emoji_add_tasks() {
        let p = warm_plan(&Metrics::default(), "漢 😀");
        assert_eq!(p.len(), 12);
    }
}
