//! TSV / CSV clipboard parsing (R-TBL-07, RFC 4180 quoting) for pasting spreadsheets.

/// Delimiter-separated records: quoted fields may hold the delimiter, `""` and line breaks;
/// `\r\n` and `\n` end a record; a trailing line break adds no record.
pub fn parse(text: &str, delim: char) -> Vec<Vec<String>> {
    let mut rows = Vec::new();
    let mut row = Vec::new();
    let mut field = String::new();
    let mut quoted = false;
    let mut at_start = true;
    let mut it = text.chars().peekable();
    while let Some(c) = it.next() {
        if quoted {
            match c {
                '"' if it.peek() == Some(&'"') => {
                    field.push('"');
                    it.next();
                }
                '"' => quoted = false,
                '\r' if it.peek() == Some(&'\n') => {}
                _ => field.push(c),
            }
            continue;
        }
        match c {
            '"' if at_start => {
                quoted = true;
                at_start = false;
            }
            '\r' if it.peek() == Some(&'\n') => {}
            '\n' | '\r' => {
                row.push(std::mem::take(&mut field));
                rows.push(std::mem::take(&mut row));
                at_start = true;
            }
            _ if c == delim => {
                row.push(std::mem::take(&mut field));
                at_start = true;
            }
            _ => {
                field.push(c);
                at_start = false;
            }
        }
    }
    if !field.is_empty() || !row.is_empty() {
        row.push(field);
        rows.push(row);
    }
    rows
}

/// Clipboard text as spreadsheet cells: TSV when it has a tab, otherwise CSV when every
/// record has the same number (≥ 2) of comma-separated fields. `None` for ordinary text.
pub fn detect(text: &str) -> Option<(Vec<Vec<String>>, char)> {
    let delim = if text.contains('\t') {
        '\t'
    } else if text.contains(',') {
        ','
    } else {
        return None;
    };
    let rows = parse(text, delim);
    let n = rows.first()?.len();
    if n < 2 {
        return None;
    }
    if delim == ',' && (rows.len() < 2 || rows.iter().any(|r| r.len() != n)) {
        return None;
    }
    Some((rows, delim))
}
