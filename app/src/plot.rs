// SPDX-License-Identifier: GPL-2.0-or-later
//! The waveform view: each record reduced to min/max per pixel column (pure, tested), drawn over a 10 x 8 grid.

use crate::theme;
use eframe::egui::{pos2, Painter, Rect, Shape, Stroke};
use oos_session::Waveform;
use std::collections::BTreeMap;

/// Divisions of the grid.
pub const HDIV: f32 = 10.0;
pub const VDIV: f32 = 8.0;

/// Min and max code of each of `columns` columns of `codes`; empty columns of a short record repeat the sample before.
pub fn column_minmax(codes: &[u16], columns: usize) -> Vec<(u16, u16)> {
    if codes.is_empty() || columns == 0 {
        return Vec::new();
    }
    (0..columns)
        .map(|c| {
            let a = c * codes.len() / columns;
            let b = ((c + 1) * codes.len() / columns).max(a + 1).min(codes.len());
            let s = &codes[a..b];
            (s.iter().copied().min().unwrap_or(0), s.iter().copied().max().unwrap_or(0))
        })
        .collect()
}

pub fn draw(p: &Painter, r: Rect, waves: &BTreeMap<u8, Waveform>) {
    p.rect_filled(r, 8.0, theme::PLOT_BG);
    let grid = Stroke::new(1.0, theme::GRID);
    for i in 1..HDIV as i32 {
        let x = r.left() + r.width() * i as f32 / HDIV;
        p.line_segment([pos2(x, r.top()), pos2(x, r.bottom())], grid);
    }
    for i in 1..VDIV as i32 {
        let y = r.top() + r.height() * i as f32 / VDIV;
        p.line_segment([pos2(r.left(), y), pos2(r.right(), y)], grid);
    }
    let cols = r.width().max(1.0) as usize;
    for w in waves.values() {
        // byte codes drawn over the whole 8-bit range; volts per division from the preamble come with the cursors
        let y = |code: u16| r.bottom() - r.height() * (code as f32 / 255.0);
        let stroke = Stroke::new(1.0, theme::channel(w.source));
        let shapes = column_minmax(&w.codes, cols)
            .into_iter()
            .enumerate()
            .map(|(i, (lo, hi))| {
                let x = r.left() + i as f32 + 0.5;
                Shape::line_segment([pos2(x, y(lo)), pos2(x, y(hi) - 0.5)], stroke)
            })
            .collect::<Vec<_>>();
        p.extend(shapes);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_column_keeps_its_extremes() {
        let codes = [5, 9, 1, 7, 3, 3, 8, 2];
        assert_eq!(column_minmax(&codes, 2), vec![(1, 9), (2, 8)]);
        assert_eq!(column_minmax(&codes, 8).len(), 8);
        assert_eq!(column_minmax(&[4, 6], 4), vec![(4, 4), (4, 4), (6, 6), (6, 6)]);
        assert!(column_minmax(&[], 4).is_empty());
    }
}
