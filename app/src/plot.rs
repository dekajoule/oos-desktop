// SPDX-License-Identifier: GPL-2.0-or-later
//! The waveform view: a 10 x 8 graticule, each record placed by its preamble and the channel's scale and offset as the
//! scope's screen places it, the markers of ground, trigger level and trigger time, the value under the pointer and
//! two pairs of cursors. Long records are reduced to min/max per pixel column (pure, tested). Only what the display
//! needs is computed here; measurements are the scope's (ADR 0002).

use crate::model::Model;
use crate::units::eng;
use crate::{theme, widgets};
use eframe::egui::{self, pos2, vec2, Align2, Color32, CornerRadius, Painter, Pos2, Rect, Sense, Shape, Stroke, Ui};
use oos_session::Waveform;

/// Divisions of the grid.
pub const HDIV: f32 = 10.0;
pub const VDIV: f32 = 8.0;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum CursorMode {
    #[default]
    Off,
    /// Two vertical lines: times.
    Time,
    /// Two horizontal lines: levels of one channel.
    Level,
}

/// The cursors, in divisions of the grid (x from 0 at the left, y from 0 at the centre, up positive), so that they
/// stay where they are on the screen when a scale changes, as on the scope.
#[derive(Clone, Debug)]
pub struct Cursors {
    pub mode: CursorMode,
    pub x: [f32; 2],
    pub y: [f32; 2],
    /// The channel whose volts the level cursors read.
    pub source: u8,
    dragging: Option<usize>,
}

impl Default for Cursors {
    fn default() -> Self {
        Cursors { mode: CursorMode::Off, x: [3.0, 7.0], y: [2.0, -2.0], source: 1, dragging: None }
    }
}

/// Where a record's samples land: pixel x of sample k is `x0 + dx * k`, pixel y of code c is `y0 + dy * c`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Place {
    pub x0: f32,
    pub dx: f32,
    pub y0: f32,
    pub dy: f32,
}

/// The time and level the grid shows: seconds per division and the time at the centre, volts per division and
/// offset of a channel. None where the scope has not answered yet.
#[derive(Clone, Copy, Debug, Default)]
pub struct Frame {
    pub time_scale: Option<f64>,
    pub time_offset: f64,
}

impl Frame {
    pub fn of(m: &Model) -> Frame {
        Frame { time_scale: m.float("time.scale", 1), time_offset: m.float("time.offset", 1).unwrap_or(0.0) }
    }

    /// The time at grid x (divisions from the left).
    pub fn time_at(&self, xdiv: f32) -> Option<f64> {
        self.time_scale.map(|s| self.time_offset + (xdiv as f64 - HDIV as f64 / 2.0) * s)
    }
}

/// Divisions above the centre of `volts` on a channel of `scale` V/div and `offset` V.
pub fn div_of(volts: f64, scale: f64, offset: f64) -> f64 {
    (volts + offset) / scale
}

/// Where the samples of `w` land in grid rect `g`. Without the time scale the record fills the width; without the
/// channel's scale its 8-bit codes fill the height.
pub fn place(w: &Waveform, g: Rect, f: &Frame, scale: Option<f64>, offset: f64) -> Place {
    let p = &w.preamble;
    let px_per_div = g.width() / HDIV;
    let (x0, dx) = match f.time_scale {
        Some(ts) if ts > 0.0 => {
            let t0 = (0.0 - p.xreference) * p.xincrement + p.xorigin;
            let x0 = g.center().x + ((t0 - f.time_offset) / ts) as f32 * px_per_div;
            (x0, (p.xincrement / ts) as f32 * px_per_div)
        }
        _ => (g.left(), g.width() / w.codes.len().max(1) as f32),
    };
    let py_per_div = g.height() / VDIV;
    let (y0, dy) = match scale {
        Some(s) if s > 0.0 => {
            // volts(c) = (c - yorigin - yreference) * yincrement, linear in c
            let v0 = (-p.yorigin - p.yreference) * p.yincrement;
            let y0 = g.center().y - div_of(v0, s, offset) as f32 * py_per_div;
            (y0, -(p.yincrement / s) as f32 * py_per_div)
        }
        _ => (g.bottom(), -g.height() / 255.0),
    };
    Place { x0, dx, y0, dy }
}

/// For each of `columns` pixel columns the lowest and highest code of the samples that land in it (sample k at
/// `x0 + dx * k`), extended to the last sample of the column before, so that the trace has no gaps. None for a column
/// without samples.
pub fn columns(codes: &[u16], x0: f32, dx: f32, columns: usize) -> Vec<Option<(u16, u16)>> {
    let mut out: Vec<Option<(u16, u16)>> = vec![None; columns];
    if columns == 0 || dx.is_nan() || dx <= 0.0 {
        return out;
    }
    // only the samples that can land on the grid
    let first = ((-x0 / dx).floor().max(0.0) as usize).saturating_sub(1).min(codes.len());
    let last = (((columns as f32 - x0) / dx).ceil().max(0.0) as usize).min(codes.len());
    let mut prev_col = usize::MAX;
    let mut prev_last: Option<u16> = None;
    for (k, &c) in codes.iter().enumerate().take(last).skip(first) {
        let x = x0 + dx * k as f32;
        if x < 0.0 {
            prev_last = Some(c);
            continue;
        }
        let col = x as usize;
        if col >= columns {
            break;
        }
        if col != prev_col {
            let start = prev_last.map_or((c, c), |l| (l.min(c), l.max(c)));
            out[col] = Some(start);
            prev_col = col;
        }
        if let Some((lo, hi)) = &mut out[col] {
            *lo = (*lo).min(c);
            *hi = (*hi).max(c);
        }
        prev_last = Some(c);
    }
    out
}

/// The sample of `w` nearest pixel x, and its time and volts.
fn sample_at(w: &Waveform, pl: &Place, x: f32) -> Option<(f64, f64)> {
    if w.codes.is_empty() || pl.dx <= 0.0 {
        return None;
    }
    let k = ((x - pl.x0) / pl.dx).round();
    if k < 0.0 || k as usize >= w.codes.len() {
        return None;
    }
    let k = k as usize;
    Some((w.time(k), w.volts(w.codes[k])))
}

/// Margins around the grid for the tags and the time labels.
const LEFT: f32 = 30.0;
const RIGHT: f32 = 30.0;
const TOP: f32 = 14.0;
const BOTTOM: f32 = 22.0;

/// Draws the view into `r` and handles the pointer over it (hover readout, dragging cursors).
pub fn draw(ui: &mut Ui, r: Rect, m: &Model, cursors: &mut Cursors) {
    let p = ui.painter_at(r);
    p.rect_filled(r, CornerRadius::same(theme::RADIUS + 4), theme::PLOT_BG);
    let g = Rect::from_min_max(pos2(r.left() + LEFT, r.top() + TOP), pos2(r.right() - RIGHT, r.bottom() - BOTTOM));
    if g.width() < 40.0 || g.height() < 40.0 {
        return;
    }
    let f = Frame::of(m);
    graticule(&p, g);
    time_labels(&p, g, &f);

    let resp = ui.interact(g, ui.scope_id().with("plot"), Sense::click_and_drag());
    let clip = p.with_clip_rect(g.expand(1.0));
    let mut placed = Vec::new();
    for w in m.waves.values() {
        if !m.is_on(w.source) && m.value("chan.on", w.source).is_some() {
            continue;
        }
        let scale = m.float("chan.scale", w.source);
        let offset = m.float("chan.offset", w.source).unwrap_or(0.0);
        let pl = place(w, g, &f, scale, offset);
        trace(&clip, g, w, &pl, theme::channel(w.source));
        placed.push((w, pl));
    }
    markers(&p, g, m, &f);
    if m.waves.is_empty() {
        let text = if m.identity.is_some() { "Waiting for records…" } else { "No scope connected" };
        p.text(g.center(), Align2::CENTER_CENTER, text, theme::body(15.0), theme::TEXT_FAINT);
    }
    cursor_lines(ui, &p, g, &resp, m, &f, cursors);
    if cursors.dragging.is_none() {
        if let Some(pos) = resp.hover_pos() {
            hover(&p, g, pos, &f, &placed, r);
        }
    }
}

fn graticule(p: &Painter, g: Rect) {
    let grid = Stroke::new(1.0, theme::GRID);
    let (dx, dy) = (g.width() / HDIV, g.height() / VDIV);
    for i in 1..HDIV as i32 {
        let x = (g.left() + dx * i as f32).round() + 0.5;
        p.line_segment([pos2(x, g.top()), pos2(x, g.bottom())], grid);
    }
    for i in 1..VDIV as i32 {
        let y = (g.top() + dy * i as f32).round() + 0.5;
        p.line_segment([pos2(g.left(), y), pos2(g.right(), y)], grid);
    }
    // the centre lines with ticks every fifth of a division, as on the scope
    let axis = Stroke::new(1.0, theme::GRID_AXIS);
    let (cx, cy) = (g.center().x.round() + 0.5, g.center().y.round() + 0.5);
    p.line_segment([pos2(cx, g.top()), pos2(cx, g.bottom())], axis);
    p.line_segment([pos2(g.left(), cy), pos2(g.right(), cy)], axis);
    for i in 1..(HDIV as i32 * 5) {
        let x = g.left() + dx * i as f32 / 5.0;
        p.line_segment([pos2(x, cy - 3.0), pos2(x, cy + 3.0)], axis);
        p.line_segment([pos2(x, g.top()), pos2(x, g.top() + 3.0)], axis);
        p.line_segment([pos2(x, g.bottom() - 3.0), pos2(x, g.bottom())], axis);
    }
    for i in 1..(VDIV as i32 * 5) {
        let y = g.top() + dy * i as f32 / 5.0;
        p.line_segment([pos2(cx - 3.0, y), pos2(cx + 3.0, y)], axis);
        p.line_segment([pos2(g.left(), y), pos2(g.left() + 3.0, y)], axis);
        p.line_segment([pos2(g.right() - 3.0, y), pos2(g.right(), y)], axis);
    }
    p.rect_stroke(g, CornerRadius::same(2), axis, egui::StrokeKind::Middle);
}

fn time_labels(p: &Painter, g: Rect, f: &Frame) {
    let dx = g.width() / HDIV;
    let every = if dx < 55.0 { 2 } else { 1 };
    for i in (0..=HDIV as i32).step_by(every) {
        let Some(t) = f.time_at(i as f32) else { return };
        let t = if t.abs() < f.time_scale.unwrap_or(1.0) * 1e-6 { 0.0 } else { t };
        let x = g.left() + dx * i as f32;
        let align = match i {
            0 => Align2::LEFT_TOP,
            10 => Align2::RIGHT_TOP,
            _ => Align2::CENTER_TOP,
        };
        p.text(pos2(x, g.bottom() + 5.0), align, eng(t, "s"), theme::mono(10.5), theme::TEXT_FAINT);
    }
}

fn trace(p: &Painter, g: Rect, w: &Waveform, pl: &Place, colour: Color32) {
    let y = |c: u16| pl.y0 + pl.dy * c as f32;
    let mut pts: Vec<Pos2> = Vec::new();
    if pl.dx >= 0.5 {
        // fewer samples than pixels: a line through the samples on the grid and one beyond each side
        let first = ((g.left() - pl.x0) / pl.dx).floor().max(0.0) as usize;
        let last = (((g.right() - pl.x0) / pl.dx).ceil() as usize + 1).min(w.codes.len());
        let first = first.min(last);
        for (k, &c) in w.codes[first..last].iter().enumerate() {
            pts.push(pos2(pl.x0 + pl.dx * (first + k) as f32, y(c)));
        }
    } else {
        let n = g.width().ceil() as usize;
        let mut prev: Option<f32> = None;
        for (i, col) in columns(&w.codes, pl.x0 - g.left(), pl.dx, n).into_iter().enumerate() {
            let Some((lo, hi)) = col else { continue };
            let x = g.left() + i as f32 + 0.5;
            let (a, b) = (y(lo), y(hi));
            // go from the end nearer the point before, so the path stays one line
            let (first, second) = match prev {
                Some(py) if (py - b).abs() < (py - a).abs() => (b, a),
                _ => (a, b),
            };
            pts.push(pos2(x, first));
            if (second - first).abs() > 0.5 {
                pts.push(pos2(x, second));
            }
            prev = Some(second);
        }
    }
    if pts.len() < 2 {
        return;
    }
    p.add(Shape::line(pts.clone(), Stroke::new(4.0, theme::alpha(colour, 38))));
    p.add(Shape::line(pts, Stroke::new(1.4, colour)));
}

fn markers(p: &Painter, g: Rect, m: &Model, f: &Frame) {
    let py_per_div = g.height() / VDIV;
    let y_of = |volts: f64, ch: u8| -> Option<f32> {
        let s = m.float("chan.scale", ch)?;
        let o = m.float("chan.offset", ch).unwrap_or(0.0);
        (s > 0.0).then(|| g.center().y - div_of(volts, s, o) as f32 * py_per_div)
    };
    let clamp = |y: f32| y.clamp(g.top() + 7.0, g.bottom() - 7.0);
    for ch in m.channels_on() {
        if let Some(y) = y_of(0.0, ch) {
            let c = theme::channel(ch);
            let c = if clamp(y) != y { theme::alpha(c, 140) } else { c };
            widgets::tag(p, pos2(g.left() - 1.0, clamp(y)), &ch.to_string(), c, true);
        }
    }
    if let (Some(src), Some(level)) = (m.trigger_source(), m.float("trig.level", 1)) {
        if let Some(y) = y_of(level, src) {
            let c = theme::channel(src);
            widgets::tag(p, pos2(g.right() + 1.0, clamp(y)), "T", c, false);
            let dash = Shape::dashed_line(
                &[pos2(g.left(), y), pos2(g.right(), y)],
                Stroke::new(1.0, theme::alpha(c, 60)),
                4.0,
                6.0,
            );
            if (g.top()..=g.bottom()).contains(&y) {
                p.extend(dash);
            }
        }
    }
    if let Some(ts) = f.time_scale.filter(|s| *s > 0.0) {
        // the trigger is at time 0
        let x = g.center().x - (f.time_offset / ts) as f32 * g.width() / HDIV;
        let off = !(g.left()..=g.right()).contains(&x);
        let x = x.clamp(g.left() + 6.0, g.right() - 6.0);
        let c = if off { theme::alpha(theme::WAIT, 140) } else { theme::WAIT };
        let tri = vec![pos2(x - 6.0, g.top() - 11.0), pos2(x + 6.0, g.top() - 11.0), pos2(x, g.top() - 2.0)];
        p.add(Shape::convex_polygon(tri, c, Stroke::NONE));
    }
}

fn hover(p: &Painter, g: Rect, pos: Pos2, f: &Frame, placed: &[(&Waveform, Place)], r: Rect) {
    let faint = Stroke::new(1.0, theme::alpha(theme::TEXT, 50));
    p.extend(Shape::dashed_line(&[pos2(pos.x, g.top()), pos2(pos.x, g.bottom())], faint, 3.0, 4.0));
    p.extend(Shape::dashed_line(&[pos2(g.left(), pos.y), pos2(g.right(), pos.y)], faint, 3.0, 4.0));
    let mut lines: Vec<(String, Color32)> = Vec::new();
    let xdiv = (pos.x - g.left()) / g.width() * HDIV;
    if let Some(t) = f.time_at(xdiv) {
        lines.push((format!("t  {}", eng(t, "s")), theme::TEXT));
    }
    for (w, pl) in placed {
        if let Some((_, v)) = sample_at(w, pl, pos.x) {
            lines.push((format!("CH{}  {}", w.source, eng(v, "V")), theme::channel(w.source)));
        }
    }
    if lines.is_empty() {
        return;
    }
    let font = theme::mono(11.5);
    let gals: Vec<_> = lines.into_iter().map(|(t, c)| p.layout_no_wrap(t, font.clone(), c)).collect();
    let w = gals.iter().map(|g| g.size().x).fold(0.0, f32::max) + 16.0;
    let h = gals.iter().map(|g| g.size().y + 2.0).sum::<f32>() + 10.0;
    let mut at = pos + vec2(14.0, 14.0);
    if at.x + w > r.right() - 4.0 {
        at.x = pos.x - 14.0 - w;
    }
    if at.y + h > r.bottom() - 4.0 {
        at.y = pos.y - 14.0 - h;
    }
    let bx = Rect::from_min_size(at, vec2(w, h));
    p.rect(
        bx,
        CornerRadius::same(theme::RADIUS),
        theme::alpha(theme::SURFACE, 235),
        Stroke::new(1.0, theme::BORDER),
        egui::StrokeKind::Inside,
    );
    let mut y = bx.top() + 5.0;
    for gal in gals {
        let hgt = gal.size().y;
        p.galley(pos2(bx.left() + 8.0, y), gal, theme::TEXT);
        y += hgt + 2.0;
    }
}

fn cursor_lines(ui: &Ui, p: &Painter, g: Rect, resp: &egui::Response, m: &Model, f: &Frame, c: &mut Cursors) {
    if c.mode == CursorMode::Off {
        c.dragging = None;
        return;
    }
    let (dx, dy) = (g.width() / HDIV, g.height() / VDIV);
    let px = |i: usize, c: &Cursors| match c.mode {
        CursorMode::Time => g.left() + c.x[i] * dx,
        _ => g.center().y - c.y[i] * dy,
    };
    // dragging: the line nearest the pointer at the start of the drag, within reach
    if resp.drag_started() {
        if let Some(pos) = resp.interact_pointer_pos() {
            let at = if c.mode == CursorMode::Time { pos.x } else { pos.y };
            let near = (0..2).min_by(|&a, &b| (px(a, c) - at).abs().total_cmp(&(px(b, c) - at).abs()));
            c.dragging = near.filter(|&i| (px(i, c) - at).abs() < 12.0);
        }
    }
    if resp.dragged() {
        if let (Some(i), Some(pos)) = (c.dragging, resp.interact_pointer_pos()) {
            match c.mode {
                CursorMode::Time => c.x[i] = ((pos.x - g.left()) / dx).clamp(0.0, HDIV),
                _ => c.y[i] = ((g.center().y - pos.y) / dy).clamp(-VDIV / 2.0, VDIV / 2.0),
            }
        }
    }
    if resp.drag_stopped() {
        c.dragging = None;
    }
    if let Some(pos) = resp.hover_pos() {
        let at = if c.mode == CursorMode::Time { pos.x } else { pos.y };
        if c.dragging.is_some() || (0..2).any(|i| (px(i, c) - at).abs() < 12.0) {
            let icon = if c.mode == CursorMode::Time {
                egui::CursorIcon::ResizeHorizontal
            } else {
                egui::CursorIcon::ResizeVertical
            };
            ui.ctx().set_cursor_icon(icon);
        }
    }
    let stroke = Stroke::new(1.0, theme::CURSOR);
    for (i, name) in ["A", "B"].iter().enumerate() {
        let v = px(i, c);
        let (a, b, label_at, align) = match c.mode {
            CursorMode::Time => (pos2(v, g.top()), pos2(v, g.bottom()), pos2(v + 4.0, g.top() + 3.0), Align2::LEFT_TOP),
            _ => (pos2(g.left(), v), pos2(g.right(), v), pos2(g.right() - 4.0, v - 3.0), Align2::RIGHT_BOTTOM),
        };
        p.extend(Shape::dashed_line(&[a, b], stroke, 6.0, 4.0));
        p.text(label_at, align, *name, theme::body(11.0), theme::CURSOR);
    }
    // the readout in the upper left corner of the grid
    let mut rows: Vec<String> = Vec::new();
    match c.mode {
        CursorMode::Time => {
            if let (Some(a), Some(b)) = (f.time_at(c.x[0]), f.time_at(c.x[1])) {
                let d = b - a;
                rows.push(format!("A   {}", eng(a, "s")));
                rows.push(format!("B   {}", eng(b, "s")));
                rows.push(format!("ΔT  {}", eng(d, "s")));
                if d.abs() > 0.0 {
                    rows.push(format!("1/ΔT {}", eng(1.0 / d.abs(), "Hz")));
                }
            }
        }
        _ => {
            let ch = c.source;
            if let Some(s) = m.float("chan.scale", ch) {
                let o = m.float("chan.offset", ch).unwrap_or(0.0);
                let volts = |y: f32| y as f64 * s - o;
                let (a, b) = (volts(c.y[0]), volts(c.y[1]));
                rows.push(format!("CH{ch}"));
                rows.push(format!("A   {}", eng(a, "V")));
                rows.push(format!("B   {}", eng(b, "V")));
                rows.push(format!("ΔV  {}", eng(b - a, "V")));
            }
        }
    }
    if rows.is_empty() {
        return;
    }
    let font = theme::mono(11.5);
    let gals: Vec<_> = rows.into_iter().map(|t| p.layout_no_wrap(t, font.clone(), theme::CURSOR)).collect();
    let w = gals.iter().map(|g| g.size().x).fold(0.0, f32::max) + 16.0;
    let h = gals.iter().map(|g| g.size().y + 2.0).sum::<f32>() + 10.0;
    let bx = Rect::from_min_size(g.left_top() + vec2(8.0, 8.0), vec2(w, h));
    p.rect(
        bx,
        CornerRadius::same(theme::RADIUS),
        theme::alpha(theme::SURFACE, 225),
        Stroke::new(1.0, theme::BORDER),
        egui::StrokeKind::Inside,
    );
    let mut y = bx.top() + 5.0;
    for gal in gals {
        let hgt = gal.size().y;
        p.galley(pos2(bx.left() + 8.0, y), gal, theme::CURSOR);
        y += hgt + 2.0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use oos_session::Preamble;

    #[test]
    fn each_column_keeps_its_extremes_and_joins_the_one_before() {
        let codes = [5, 9, 1, 7, 3, 3, 8, 2];
        // four samples per column
        let c = columns(&codes, 0.0, 0.25, 2);
        assert_eq!(c, vec![Some((1, 9)), Some((2, 8))]);
        // samples left and right of the grid are left out, but the one before the grid joins the first column
        let c = columns(&codes, -0.5, 0.25, 1);
        assert_eq!(c, vec![Some((1, 9))]);
        // two pixels per sample leave every other column empty
        assert_eq!(columns(&[4, 6], 0.0, 2.0, 4), vec![Some((4, 4)), None, Some((4, 6)), None]);
        assert!(columns(&[], 0.0, 1.0, 4).iter().all(Option::is_none));
        assert!(columns(&codes, 0.0, 0.0, 4).iter().all(Option::is_none));
    }

    fn wave(points: usize) -> Waveform {
        // 3600 points over 10 divisions of 0.5 ms around time 0; 16 mV per code, code 128 at -yorigin's volts
        let p = Preamble::parse(&format!("0,0,{points},1,1.388889E-06,-2.500000E-03,0,1.600000E-02,-78,128"))
            .expect("preamble");
        Waveform { source: 1, preamble: p, codes: vec![128; points] }
    }

    #[test]
    fn a_screen_record_fills_the_grid_as_on_the_scope() {
        let g = Rect::from_min_size(pos2(0.0, 0.0), vec2(1000.0, 800.0));
        let w = wave(3600);
        let f = Frame { time_scale: Some(5e-4), time_offset: 0.0 };
        // 0.5 V/div, offset -1.25 V: code 128 is 1.248 V, the centre of the screen
        let pl = place(&w, g, &f, Some(0.5), -1.25);
        assert!(pl.x0.abs() < 0.01, "{pl:?}");
        assert!((pl.x0 + pl.dx * 3600.0 - 1000.0).abs() < 0.1, "{pl:?}");
        let y128 = pl.y0 + pl.dy * 128.0;
        assert!((y128 - 400.0).abs() < 0.5, "{y128}");
        // one division (31.25 codes) up is 100 px up
        assert!((pl.dy * 31.25 + 100.0).abs() < 0.01, "{pl:?}");
        // without the scales the record fills the rect
        let pl = place(&w, g, &Frame::default(), None, 0.0);
        assert_eq!((pl.x0, pl.y0), (0.0, 800.0));
    }

    #[test]
    fn the_time_at_a_division() {
        let f = Frame { time_scale: Some(1e-3), time_offset: 2e-3 };
        assert_eq!(f.time_at(5.0), Some(2e-3));
        assert!((f.time_at(0.0).expect("t") + 3e-3).abs() < 1e-12);
        assert_eq!(Frame::default().time_at(5.0), None);
    }
}
