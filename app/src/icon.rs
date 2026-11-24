// SPDX-License-Identifier: GPL-2.0-or-later
//! The program's mark, drawn here and not loaded from a file: a dark rounded square with a trace of CH1's colour. The
//! same shape is the window icon (pixels) and the mark in the top bar (shapes).

use crate::theme;
use eframe::egui::{self, pos2, CornerRadius, Painter, Pos2, Rect, Shape, Stroke};

/// The trace of the mark, in 0..1 of the square: one period of a sine with a sharper edge, as a scope trace looks.
fn trace(t: f32) -> f32 {
    let s = (t * std::f32::consts::TAU).sin();
    0.5 - 0.26 * s.signum() * s.abs().powf(0.6)
}

/// The window icon: `n` x `n` RGBA pixels, unmultiplied alpha.
pub fn pixels(n: u32) -> Vec<u8> {
    let nf = n as f32;
    let radius = nf * 0.22;
    let mut out = Vec::with_capacity((n * n * 4) as usize);
    let bg = [0x15u8, 0x18, 0x1e];
    let fg = theme::channel(1);
    let line = nf * 0.055;
    for y in 0..n {
        for x in 0..n {
            let (px, py) = (x as f32 + 0.5, y as f32 + 0.5);
            // coverage of the rounded square, a pixel's worth of smoothing at the edge
            let cx = px.clamp(radius, nf - radius);
            let cy = py.clamp(radius, nf - radius);
            let d = ((px - cx).powi(2) + (py - cy).powi(2)).sqrt();
            let inside = (radius - d + 0.5).clamp(0.0, 1.0);
            // distance to the trace, sampled along it
            let margin = nf * 0.16;
            let mut near = f32::MAX;
            for i in 0..=96 {
                let t = i as f32 / 96.0;
                let tx = margin + t * (nf - 2.0 * margin);
                let ty = trace(t) * nf;
                near = near.min(((px - tx).powi(2) + (py - ty).powi(2)).sqrt());
            }
            let ink = (line - near + 0.5).clamp(0.0, 1.0);
            let mix = |b: u8, f: u8| (b as f32 + (f as f32 - b as f32) * ink).round() as u8;
            out.extend_from_slice(&[
                mix(bg[0], fg.r()),
                mix(bg[1], fg.g()),
                mix(bg[2], fg.b()),
                (inside * 255.0).round() as u8,
            ]);
        }
    }
    out
}

pub fn window_icon() -> egui::IconData {
    let n = 128;
    egui::IconData { rgba: pixels(n), width: n, height: n }
}

/// The mark drawn into `r` (a square).
pub fn paint(p: &Painter, r: Rect) {
    p.rect(
        r,
        CornerRadius::same((r.width() * 0.22) as u8),
        theme::SURFACE,
        Stroke::new(1.0, theme::BORDER),
        egui::StrokeKind::Inside,
    );
    let m = r.width() * 0.16;
    let pts: Vec<Pos2> = (0..=40)
        .map(|i| {
            let t = i as f32 / 40.0;
            pos2(r.left() + m + t * (r.width() - 2.0 * m), r.top() + trace(t) * r.height())
        })
        .collect();
    p.add(Shape::line(pts, Stroke::new(r.width() * 0.08, theme::channel(1))));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_icon_is_opaque_inside_clear_in_the_corners_and_has_the_trace() {
        let n = 64;
        let px = pixels(n);
        assert_eq!(px.len(), (n * n * 4) as usize);
        let at = |x: u32, y: u32| &px[((y * n + x) * 4) as usize..((y * n + x) * 4 + 4) as usize];
        assert_eq!(at(0, 0)[3], 0, "corner");
        assert_eq!(at(n / 2, n - 3)[3], 255, "inside");
        // the trace crosses the middle of the square
        assert!(at(n / 2, n / 2)[0] > 0xc0, "{:?}", at(n / 2, n / 2));
    }
}
