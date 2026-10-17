//! Pagination for printing and PDF through the print path (R-EXP-03, FUT-39): page breaks fall
//! between blocks or at allowed positions inside a block (line boundaries, table row boundaries);
//! an element taller than a whole page is scaled down to fit one page.

/// One top-level block in document coordinates (any unit, e.g. DIPs).
#[derive(Debug, Clone, PartialEq)]
pub struct Unit {
    /// Top (the gap before the block belongs to the previous unit).
    pub top: f64,
    /// Bottom.
    pub bottom: f64,
    /// Allowed break positions strictly inside `top..bottom` (sorted ascending). Empty for
    /// elements that never split (diagrams, formulas, single table rows, single code lines).
    pub breaks: Vec<f64>,
}

/// One page: the document slice `y0..y1` drawn at `scale` (1 = natural size).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Page {
    /// Slice top.
    pub y0: f64,
    /// Slice bottom.
    pub y1: f64,
    /// Scale (< 1 only for an element taller than a page).
    pub scale: f64,
}

/// Splits the document into pages of height `page_h`. Never emits a blank trailing page.
pub fn paginate(units: &[Unit], page_h: f64) -> Vec<Page> {
    let mut pages = Vec::new();
    if page_h <= 0.0 {
        return pages;
    }
    let eps = 1e-6;
    let end = units.iter().map(|u| u.bottom).fold(0.0, f64::max);
    let mut start = 0.0;
    let mut k = 0usize;
    while let Some(u) = units.get(k) {
        if u.bottom <= start + eps {
            k += 1;
            continue;
        }
        if u.bottom - start <= page_h + eps {
            k += 1;
            continue;
        }
        // The unit crosses the page end: the last allowed break that fits.
        let cands = std::iter::once(u.top).chain(u.breaks.iter().copied());
        let best = cands
            .filter(|&b| b > start + eps && b - start <= page_h + eps)
            .fold(None, |m: Option<f64>, b| Some(m.map_or(b, |m| m.max(b))));
        if let Some(b) = best {
            pages.push(Page {
                y0: start,
                y1: b,
                scale: 1.0,
            });
            start = b;
            continue;
        }
        // Nothing fits: the piece up to the next break (or the unit end) is taller than a page.
        let seg_end = u
            .breaks
            .iter()
            .copied()
            .find(|&b| b > start + eps)
            .unwrap_or(u.bottom);
        pages.push(Page {
            y0: start,
            y1: seg_end,
            scale: (page_h / (seg_end - start)).min(1.0),
        });
        start = seg_end;
    }
    if end > start + eps {
        pages.push(Page {
            y0: start,
            y1: end,
            scale: 1.0,
        });
    }
    pages
}

/// Footer text "n / N" (`export.pdf_page_numbers`).
pub fn footer(n: usize, total: usize) -> String {
    format!("{n} / {total}")
}

/// Paper size in millimetres for `export.pdf_page_size` (`auto` = Letter for en-US / en-CA,
/// A4 otherwise) and the user's locale tag.
pub fn paper_mm(setting: &str, locale: &str) -> (f64, f64) {
    const A4: (f64, f64) = (210.0, 297.0);
    const LETTER: (f64, f64) = (215.9, 279.4);
    match setting {
        "a4" => A4,
        "letter" => LETTER,
        _ => {
            let l = locale.replace('_', "-").to_ascii_lowercase();
            if l == "en-us" || l == "en-ca" || l.starts_with("en-us-") || l.starts_with("en-ca-") {
                LETTER
            } else {
                A4
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn u(top: f64, bottom: f64, breaks: &[f64]) -> Unit {
        Unit {
            top,
            bottom,
            breaks: breaks.to_vec(),
        }
    }

    #[test]
    fn r_exp_03_breaks_between_blocks() {
        let p = paginate(
            &[u(0.0, 40.0, &[]), u(40.0, 80.0, &[]), u(80.0, 130.0, &[])],
            100.0,
        );
        assert_eq!(p.len(), 2);
        assert_eq!((p[0].y0, p[0].y1), (0.0, 80.0));
        assert_eq!((p[1].y0, p[1].y1, p[1].scale), (80.0, 130.0, 1.0));
    }

    #[test]
    fn r_exp_03_tall_block_breaks_at_line_boundary() {
        let p = paginate(
            &[
                u(0.0, 50.0, &[]),
                u(50.0, 170.0, &[70.0, 90.0, 110.0, 130.0, 150.0]),
            ],
            100.0,
        );
        assert_eq!((p[0].y0, p[0].y1), (0.0, 90.0));
        assert_eq!((p[1].y0, p[1].y1), (90.0, 170.0));
        assert!(p.iter().all(|x| x.y1 - x.y0 <= 100.0 && x.scale == 1.0));
    }

    #[test]
    fn r_exp_03_atomic_element_moves_to_next_page() {
        // A diagram (no breaks) that does not fit the rest of the page starts a new page.
        let p = paginate(&[u(0.0, 60.0, &[]), u(60.0, 140.0, &[])], 100.0);
        assert_eq!((p[0].y0, p[0].y1), (0.0, 60.0));
        assert_eq!((p[1].y0, p[1].y1, p[1].scale), (60.0, 140.0, 1.0));
    }

    #[test]
    fn r_exp_03_oversized_element_scaled_to_fit() {
        let p = paginate(&[u(0.0, 20.0, &[]), u(20.0, 270.0, &[])], 100.0);
        assert_eq!(p.len(), 2);
        assert_eq!((p[1].y0, p[1].y1), (20.0, 270.0));
        assert!((p[1].scale - 0.4).abs() < 1e-9);
    }

    #[test]
    fn r_exp_03_no_blank_trailing_page_and_footer() {
        let p = paginate(&[u(0.0, 100.0, &[]), u(100.0, 200.0, &[])], 100.0);
        assert_eq!(p.len(), 2);
        assert_eq!(footer(1, p.len()), "1 / 2");
        assert!(paginate(&[], 100.0).is_empty());
    }

    #[test]
    fn r_exp_03_paper_size_auto() {
        assert_eq!(paper_mm("auto", "en-US"), (215.9, 279.4));
        assert_eq!(paper_mm("auto", "en-CA"), (215.9, 279.4));
        assert_eq!(paper_mm("auto", "tr-TR"), (210.0, 297.0));
        assert_eq!(paper_mm("auto", "en-GB"), (210.0, 297.0));
        assert_eq!(paper_mm("letter", "tr-TR"), (215.9, 279.4));
        assert_eq!(paper_mm("a4", "en-US"), (210.0, 297.0));
    }
}
