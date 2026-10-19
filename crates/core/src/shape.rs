//! The line shape of a file: its indent and line lengths, sampled into at most 64 rows (`mockups/fakegen.py` `minimap`).
//! A renderer scales the rows to its own grid, such as the braille shape in the selection panel.

/// The most rows a shape keeps.
pub const MAX_ROWS: usize = 64;
/// Line lengths are capped at this many columns.
pub const MAX_LEN: u16 = 126;
/// A tab counts as this many columns.
pub const TAB: usize = 2;
/// A row that covers more lines than this keeps medians, not the minimum and maximum (unit `028`).
pub const MEDIAN_ABOVE: usize = 4;

/// One sampled row, in columns. A row of at most [`MEDIAN_ABOVE`] lines keeps the minimum indent and the maximum length
/// of its non-empty lines. A longer row keeps the median indent and the median length, so a long file shows its
/// typical lines, not a block. A row of only blank lines has length 0.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ShapeRow {
    pub indent: u16,
    pub len: u16,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct LineShape {
    /// The file's line count.
    pub lines: u64,
    /// At most [`MAX_ROWS`] rows. Row `r` of `R` covers lines `r * n / R` to `(r + 1) * n / R`.
    pub rows: Vec<ShapeRow>,
}

impl LineShape {
    /// Measures text. A line's indent is its leading whitespace, and its length ends at its last non-space character.
    pub fn from_text(text: &str) -> LineShape {
        LineShape::from_text_with(text, MEDIAN_ABOVE)
    }

    /// [`LineShape::from_text`] with another median threshold. `usize::MAX` keeps the minimum and maximum in every row.
    pub fn from_text_with(text: &str, median_above: usize) -> LineShape {
        LineShape::sample(&measure_lines(text), median_above)
    }

    /// Samples measured lines into at most [`MAX_ROWS`] rows.
    /// A row of more than `median_above` lines keeps the medians of its non-empty lines.
    pub fn sample(lines: &[ShapeRow], median_above: usize) -> LineShape {
        let n = lines.len();
        let rows_n = n.min(MAX_ROWS);
        let mut rows = Vec::with_capacity(rows_n);
        for r in 0..rows_n {
            let a = r * n / rows_n;
            let b = ((r + 1) * n / rows_n).max(a + 1);
            rows.push(summarize(&lines[a..b], median_above));
        }
        LineShape {
            lines: n as u64,
            rows,
        }
    }

    /// Decodes the `m` field of `mockups/fake.json`: two base64 characters per row, indent / 2 and (length + 1) / 2.
    pub fn decode(m: &str, lines: u64) -> Option<LineShape> {
        let digits: Option<Vec<u16>> = m.bytes().map(b64).collect();
        let digits = digits?;
        if digits.len() % 2 != 0 {
            return None;
        }
        let rows = digits
            .chunks(2)
            .map(|p| ShapeRow {
                indent: p[0] * 2,
                len: p[1] * 2,
            })
            .collect();
        Some(LineShape { lines, rows })
    }
}

/// One row for a run of measured lines, as [`LineShape::sample`] makes it: the minimum indent and the maximum length
/// of the non-empty lines when there are at most `median_above` lines, else their medians. Lengths cap at [`MAX_LEN`].
pub fn summarize(lines: &[ShapeRow], median_above: usize) -> ShapeRow {
    let text: Vec<&ShapeRow> = lines.iter().filter(|l| l.len > 0).collect();
    if text.is_empty() {
        ShapeRow::default()
    } else if lines.len() > median_above {
        let median = |mut v: Vec<u16>| {
            v.sort_unstable();
            v[(v.len() - 1) / 2]
        };
        ShapeRow {
            indent: median(text.iter().map(|l| l.indent).collect()),
            len: median(text.iter().map(|l| l.len.min(MAX_LEN)).collect()),
        }
    } else {
        ShapeRow {
            indent: text.iter().map(|l| l.indent).min().unwrap_or(0),
            len: text.iter().map(|l| l.len.min(MAX_LEN)).max().unwrap_or(0),
        }
    }
}

/// Every line of a text, measured: one row per line (unit `039`). A line's length is not capped.
pub fn measure_lines(text: &str) -> Vec<ShapeRow> {
    text.lines().map(measure).collect()
}

/// The indent and length of one line, with tabs expanded to [`TAB`] columns.
fn measure(line: &str) -> ShapeRow {
    let (mut col, mut indent, mut end) = (0usize, None, 0usize);
    for ch in line.chars() {
        let w = if ch == '\t' { TAB } else { 1 };
        if !ch.is_whitespace() {
            indent.get_or_insert(col);
            end = col + w;
        }
        col += w;
    }
    let clamp = |x: usize| x.min(usize::from(u16::MAX)) as u16;
    ShapeRow {
        indent: clamp(indent.unwrap_or(0)),
        len: clamp(end),
    }
}

fn b64(c: u8) -> Option<u16> {
    Some(u16::from(match c {
        b'A'..=b'Z' => c - b'A',
        b'a'..=b'z' => c - b'a' + 26,
        b'0'..=b'9' => c - b'0' + 52,
        b'+' => 62,
        b'/' => 63,
        _ => return None,
    }))
}
