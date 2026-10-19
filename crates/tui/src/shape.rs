//! The braille shape of a whole file for the selection panel (`fileShape` in `mockups/v5-template.html`).

use minimap_core::shape::{LineShape, MAX_LEN};

/// Braille dot bits by dot row and column (`mockups/v3-template.html:216`).
pub const BIT: [[u8; 2]; 4] = [[0x01, 0x08], [0x02, 0x10], [0x04, 0x20], [0x40, 0x80]];

/// A file's shape as braille cells. Each dot row stands for `scale` lines, and each dot column for `126 / (2 * cols)`
/// columns of text.
#[derive(Debug, Clone, PartialEq)]
pub struct Braille {
    pub cols: usize,
    pub rows: usize,
    /// Dot bits per cell, row by row.
    pub cells: Vec<u8>,
    /// Lines per dot row, at least 1.
    pub scale: f64,
    /// The cell rows that hold lines. A short file uses fewer than `rows`.
    pub rows_used: usize,
}

impl Braille {
    /// Scales a line shape to `cols` x `rows` cells.
    pub fn new(shape: &LineShape, cols: usize, rows: usize) -> Braille {
        let n = shape.lines as f64;
        let (dots_w, dots_h) = (cols * 2, rows * 4);
        let r_n = shape.rows.len();
        let scale = (n / dots_h as f64).max(1.0);
        let cpd = f64::from(MAX_LEN) / dots_w as f64;
        let mut cells = vec![0u8; cols * rows];
        let used = if r_n == 0 {
            0
        } else {
            dots_h.min((n / scale).ceil() as usize)
        };
        for dy in 0..used {
            let l0 = dy as f64 * scale;
            let l1 = n.min((dy + 1) as f64 * scale);
            let d0 = (l0 / n * r_n as f64).floor() as usize;
            let d1 = (d0 + 1).max((l1 / n * r_n as f64).ceil() as usize);
            let (mut indent, mut len) = (u16::MAX, 0);
            for row in &shape.rows[d0.min(r_n)..d1.min(r_n)] {
                if row.len == 0 {
                    continue;
                }
                len = len.max(row.len);
                indent = indent.min(row.indent);
            }
            if len == 0 {
                continue;
            }
            for dx in 0..dots_w {
                let cx = dx as f64 * cpd + cpd / 2.0;
                if cx >= f64::from(indent) && cx < f64::from(len) {
                    cells[(dy >> 2) * cols + (dx >> 1)] |= BIT[dy & 3][dx & 1];
                }
            }
        }
        Braille {
            cols,
            rows,
            cells,
            scale,
            rows_used: used.div_ceil(4).max(1),
        }
    }

    /// The cell row that shows a 1-based line.
    pub fn row_of(&self, line: u32) -> usize {
        let r = (f64::from(line.max(1) - 1) / self.scale / 4.0).floor() as usize;
        r.min(self.rows - 1)
    }

    /// The first line that a cell row shows.
    pub fn first_line(&self, row: usize) -> u64 {
        (row as f64 * 4.0 * self.scale).floor() as u64 + 1
    }

    pub fn glyph(&self, col: usize, row: usize) -> char {
        match self.cells[row * self.cols + col] {
            0 => ' ',
            b => char::from_u32(0x2800 + u32::from(b)).unwrap_or(' '),
        }
    }
}
