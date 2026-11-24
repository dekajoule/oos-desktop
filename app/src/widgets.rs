// SPDX-License-Identifier: GPL-2.0-or-later
//! The window's own controls: icons drawn as shapes (no icon font, the same on every system), tool buttons, a switch,
//! segmented choices and status badges. Drawing only; what a control means is decided by its caller.

use crate::theme;
use eframe::egui::{self, pos2, vec2, Color32, CornerRadius, Painter, Pos2, Rect, Response, Sense, Shape, Stroke, Ui};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Icon {
    Play,
    Stop,
    Single,
    Force,
    Auto,
    Clear,
    Camera,
    Save,
    Search,
    Plug,
    CursorX,
    CursorY,
    Close,
}

/// Draws `icon` into the square `r` in colour `c`.
pub fn icon(p: &Painter, r: Rect, icon: Icon, c: Color32) {
    let s = Stroke::new(1.6, c);
    let (cx, cy, h) = (r.center().x, r.center().y, r.width() * 0.5);
    let at = |x: f32, y: f32| pos2(cx + x * h, cy + y * h);
    match icon {
        Icon::Play => {
            p.add(Shape::convex_polygon(vec![at(-0.5, -0.65), at(0.7, 0.0), at(-0.5, 0.65)], c, Stroke::NONE));
        }
        Icon::Stop => {
            p.rect_filled(Rect::from_center_size(r.center(), vec2(h * 1.15, h * 1.15)), 2.0, c);
        }
        Icon::Single => {
            p.line_segment([at(-0.75, 0.55), at(-0.25, 0.55)], s);
            p.line_segment([at(-0.25, 0.55), at(-0.25, -0.55)], s);
            p.line_segment([at(-0.25, -0.55), at(0.25, -0.55)], s);
            p.line_segment([at(0.25, -0.55), at(0.25, 0.55)], s);
            p.line_segment([at(0.25, 0.55), at(0.75, 0.55)], s);
        }
        Icon::Force => {
            let bolt =
                vec![at(0.15, -0.8), at(-0.45, 0.1), at(0.0, 0.1), at(-0.15, 0.8), at(0.45, -0.1), at(0.0, -0.1)];
            p.add(Shape::closed_line(bolt, s));
        }
        Icon::Auto => {
            let pts: Vec<Pos2> = (0..=24)
                .map(|i| {
                    let t = i as f32 / 24.0;
                    at(-0.8 + 1.6 * t, -0.45 * (t * std::f32::consts::TAU).sin())
                })
                .collect();
            p.add(Shape::line(pts, s));
            p.line_segment([at(-0.8, 0.75), at(0.8, 0.75)], Stroke::new(1.2, c));
        }
        Icon::Clear => {
            p.circle_stroke(r.center(), h * 0.65, s);
            p.line_segment([at(-0.45, 0.45), at(0.45, -0.45)], s);
        }
        Icon::Camera => {
            let body = Rect::from_min_max(at(-0.8, -0.4), at(0.8, 0.65));
            p.rect_stroke(body, 2.0, s, egui::StrokeKind::Middle);
            p.line_segment([at(-0.3, -0.4), at(-0.15, -0.65)], s);
            p.line_segment([at(-0.15, -0.65), at(0.15, -0.65)], s);
            p.line_segment([at(0.15, -0.65), at(0.3, -0.4)], s);
            p.circle_stroke(at(0.0, 0.12), h * 0.3, s);
        }
        Icon::Save => {
            p.line_segment([at(0.0, -0.8), at(0.0, 0.3)], s);
            p.line_segment([at(-0.4, -0.1), at(0.0, 0.3)], s);
            p.line_segment([at(0.4, -0.1), at(0.0, 0.3)], s);
            p.line_segment([at(-0.75, 0.75), at(0.75, 0.75)], s);
        }
        Icon::Search => {
            p.circle_stroke(at(-0.15, -0.15), h * 0.5, s);
            p.line_segment([at(0.22, 0.22), at(0.75, 0.75)], Stroke::new(2.0, c));
        }
        Icon::Plug => {
            p.circle_stroke(r.center(), h * 0.35, s);
            p.line_segment([at(-0.9, 0.0), at(-0.35, 0.0)], s);
            p.line_segment([at(0.35, 0.0), at(0.9, 0.0)], s);
        }
        Icon::CursorX => {
            for x in [-0.4, 0.4] {
                p.line_segment([at(x, -0.8), at(x, 0.8)], s);
            }
            p.line_segment([at(-0.4, 0.0), at(0.4, 0.0)], Stroke::new(1.0, c));
        }
        Icon::CursorY => {
            for y in [-0.4, 0.4] {
                p.line_segment([at(-0.8, y), at(0.8, y)], s);
            }
            p.line_segment([at(0.0, -0.4), at(0.0, 0.4)], Stroke::new(1.0, c));
        }
        Icon::Close => {
            p.line_segment([at(-0.5, -0.5), at(0.5, 0.5)], s);
            p.line_segment([at(-0.5, 0.5), at(0.5, -0.5)], s);
        }
    }
}

/// How a tool button is filled.
#[derive(Clone, Copy, PartialEq)]
pub enum Tone {
    /// A plain button.
    Plain,
    /// A button in a colour: Run green, Stop red.
    Colour(Color32),
    /// A toggle that is on.
    On,
}

/// A button with an icon and an optional label; `hint` is shown on hover.
pub fn tool(ui: &mut Ui, enabled: bool, icon_: Icon, label: &str, tone: Tone, hint: &str) -> Response {
    let font = theme::body(theme::BODY);
    let galley = (!label.is_empty()).then(|| ui.painter().layout_no_wrap(label.to_string(), font, theme::TEXT));
    let isz = 14.0;
    let pad = 9.0;
    let w = pad * 2.0 + isz + galley.as_ref().map_or(0.0, |g| g.size().x + 7.0);
    let (r, resp) = ui.allocate_exact_size(vec2(w, 30.0), if enabled { Sense::click() } else { Sense::hover() });
    if ui.is_rect_visible(r) {
        let hot = enabled && resp.hovered();
        let down = enabled && resp.is_pointer_button_down_on();
        let (fill, stroke, fg) = match tone {
            Tone::Colour(c) => {
                let a = if down {
                    255
                } else if hot {
                    235
                } else {
                    210
                };
                (theme::alpha(c, a), Stroke::NONE, Color32::from_rgb(0x0b, 0x0d, 0x10))
            }
            Tone::On => (theme::SURFACE_ON, Stroke::new(1.0, theme::ACCENT), theme::ACCENT),
            Tone::Plain => {
                let fill = if down {
                    theme::SURFACE_ON
                } else if hot {
                    theme::SURFACE_HI
                } else {
                    theme::SURFACE
                };
                (fill, Stroke::new(1.0, theme::BORDER), theme::TEXT)
            }
        };
        let fg = if enabled { fg } else { theme::TEXT_FAINT };
        let fill = if enabled { fill } else { theme::alpha(theme::SURFACE, 160) };
        let p = ui.painter();
        p.rect(r, CornerRadius::same(theme::RADIUS), fill, stroke, egui::StrokeKind::Inside);
        let ir = Rect::from_center_size(pos2(r.left() + pad + isz / 2.0, r.center().y), vec2(isz, isz));
        icon(p, ir, icon_, fg);
        if let Some(g) = galley {
            p.galley_with_override_text_color(pos2(ir.right() + 7.0, r.center().y - g.size().y / 2.0), g, fg);
        }
    }
    if hint.is_empty() {
        resp
    } else {
        resp.on_hover_text(hint)
    }
}

/// A switch; true when it was flipped.
pub fn switch(ui: &mut Ui, on: &mut bool, colour: Color32) -> Response {
    let size = vec2(32.0, 18.0);
    let (r, mut resp) = ui.allocate_exact_size(size, Sense::click());
    if resp.clicked() {
        *on = !*on;
        resp.mark_changed();
    }
    if ui.is_rect_visible(r) {
        let t = ui.ctx().animate_bool_responsive(resp.id, *on);
        let off_fill = if resp.hovered() { theme::SURFACE_ON } else { theme::SURFACE_HI };
        let fill = lerp_colour(off_fill, theme::alpha(colour, 220), t);
        let p = ui.painter();
        p.rect(r, CornerRadius::same(9), fill, Stroke::new(1.0, theme::BORDER), egui::StrokeKind::Inside);
        let x = egui::lerp(r.left() + 9.0..=r.right() - 9.0, t);
        let knob = if *on { Color32::from_rgb(0x0b, 0x0d, 0x10) } else { theme::TEXT_DIM };
        p.circle_filled(pos2(x, r.center().y), 6.0, knob);
    }
    resp
}

fn lerp_colour(a: Color32, b: Color32, t: f32) -> Color32 {
    let l = |x: u8, y: u8| (x as f32 + (y as f32 - x as f32) * t).round() as u8;
    Color32::from_rgba_unmultiplied(l(a.r(), b.r()), l(a.g(), b.g()), l(a.b(), b.b()), l(a.a(), b.a()))
}

/// A row of choices of which one is chosen; the index clicked when it was not the chosen one. `tint` colours the
/// chosen one (a channel's colour), else the accent.
pub fn segmented(ui: &mut Ui, labels: &[&str], chosen: Option<usize>, tint: &[Option<Color32>]) -> Option<usize> {
    let font = theme::body(12.5);
    let galleys: Vec<_> =
        labels.iter().map(|l| ui.painter().layout_no_wrap((*l).to_string(), font.clone(), theme::TEXT)).collect();
    let pad = 9.0;
    let w: f32 = galleys.iter().map(|g| g.size().x + 2.0 * pad).sum();
    let (r, _) = ui.allocate_exact_size(vec2(w, 24.0), Sense::hover());
    let p = ui.painter().clone();
    p.rect(r, CornerRadius::same(theme::RADIUS), theme::BG, Stroke::new(1.0, theme::BORDER), egui::StrokeKind::Inside);
    let mut x = r.left();
    let mut picked = None;
    for (i, g) in galleys.into_iter().enumerate() {
        let cell = Rect::from_min_size(pos2(x, r.top()), vec2(g.size().x + 2.0 * pad, r.height()));
        x = cell.right();
        let resp = ui.interact(cell, ui.scope_id().with(("seg", labels[i], i)), Sense::click());
        let on = chosen == Some(i);
        let colour = tint.get(i).copied().flatten().unwrap_or(theme::ACCENT);
        if on {
            p.rect_filled(cell.shrink(2.0), CornerRadius::same(theme::RADIUS - 2), theme::SURFACE_ON);
        } else if resp.hovered() {
            p.rect_filled(cell.shrink(2.0), CornerRadius::same(theme::RADIUS - 2), theme::SURFACE_HI);
        }
        let fg = if on { colour } else { theme::TEXT_DIM };
        p.galley_with_override_text_color(cell.center() - g.size() / 2.0, g, fg);
        if resp.clicked() && !on {
            picked = Some(i);
        }
    }
    picked
}

/// A small rounded label with a coloured dot, for states (`RUN`, `WAIT`).
pub fn badge(ui: &mut Ui, text: &str, colour: Color32) -> Response {
    let font = theme::body(12.0);
    let g = ui.painter().layout_no_wrap(text.to_string(), font, colour);
    let (r, resp) = ui.allocate_exact_size(vec2(g.size().x + 30.0, 24.0), Sense::hover());
    let p = ui.painter();
    p.rect(
        r,
        CornerRadius::same(12),
        theme::alpha(colour, 28),
        Stroke::new(1.0, theme::alpha(colour, 90)),
        egui::StrokeKind::Inside,
    );
    p.circle_filled(pos2(r.left() + 12.0, r.center().y), 4.0, colour);
    p.galley(pos2(r.left() + 21.0, r.center().y - g.size().y / 2.0), g, colour);
    resp
}

/// A section title in small capitals of the dim colour.
pub fn section(ui: &mut Ui, text: &str) {
    ui.add_space(4.0);
    ui.label(egui::RichText::new(text.to_uppercase()).size(theme::SMALL).color(theme::TEXT_FAINT).strong());
}

/// A label tag with a pointed side, as the scope marks a channel's ground or the trigger level; `left`: the point
/// faces right (a tag on the left edge).
pub fn tag(p: &Painter, at: Pos2, text: &str, colour: Color32, left: bool) {
    let g = p.layout_no_wrap(text.to_string(), theme::body(10.5), Color32::from_rgb(0x0b, 0x0d, 0x10));
    let (w, h) = (g.size().x + 8.0, 14.0);
    let (x0, x1, tip) = if left { (at.x - w - 5.0, at.x - 5.0, at.x) } else { (at.x + 5.0, at.x + 5.0 + w, at.x) };
    let pts = if left {
        vec![pos2(x0, at.y - h / 2.0), pos2(x1, at.y - h / 2.0), pos2(tip, at.y), pos2(x1, at.y + h / 2.0)]
            .into_iter()
            .chain([pos2(x0, at.y + h / 2.0)])
            .collect()
    } else {
        vec![pos2(tip, at.y), pos2(x0, at.y - h / 2.0), pos2(x1, at.y - h / 2.0), pos2(x1, at.y + h / 2.0)]
            .into_iter()
            .chain([pos2(x0, at.y + h / 2.0)])
            .collect()
    };
    p.add(Shape::convex_polygon(pts, colour, Stroke::NONE));
    let cx = (x0 + x1) / 2.0;
    let size = g.size();
    p.galley(pos2(cx - size.x / 2.0, at.y - size.y / 2.0), g, Color32::from_rgb(0x0b, 0x0d, 0x10));
}
