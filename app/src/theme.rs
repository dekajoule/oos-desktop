// SPDX-License-Identifier: GPL-2.0-or-later
//! Colours, type and spacing, in one place: neutral greys, a white accent, round shapes, close to the scope's own
//! look. Drawing code takes its colours from the constants here, never from literals.

use eframe::egui::{
    self, style::WidgetVisuals, Color32, Context, CornerRadius, FontFamily, FontId, Frame, Margin, Shadow, Stroke,
    TextStyle, Theme, Visuals,
};

/// The trace colour of channel `ch` (1-based).
pub fn channel(ch: u8) -> Color32 {
    match ch {
        1 => Color32::from_rgb(0xf5, 0xd5, 0x3d),
        2 => Color32::from_rgb(0x3d, 0xd5, 0xf5),
        3 => Color32::from_rgb(0xe8, 0x6a, 0xe8),
        _ => Color32::from_rgb(0x5b, 0x8f, 0xf9),
    }
}

/// The window behind everything.
pub const BG: Color32 = Color32::from_rgb(0x0e, 0x10, 0x14);
/// Bars and the side panel.
pub const PANEL: Color32 = Color32::from_rgb(0x15, 0x18, 0x1e);
/// Cards on a panel.
pub const SURFACE: Color32 = Color32::from_rgb(0x1c, 0x20, 0x27);
/// A card or control under the pointer.
pub const SURFACE_HI: Color32 = Color32::from_rgb(0x26, 0x2b, 0x34);
/// A pressed or chosen control.
pub const SURFACE_ON: Color32 = Color32::from_rgb(0x33, 0x39, 0x44);
pub const BORDER: Color32 = Color32::from_rgb(0x2a, 0x2f, 0x38);
pub const TEXT: Color32 = Color32::from_rgb(0xe6, 0xe8, 0xec);
pub const TEXT_DIM: Color32 = Color32::from_rgb(0x8a, 0x92, 0x9f);
pub const TEXT_FAINT: Color32 = Color32::from_rgb(0x5c, 0x63, 0x6e);
/// The accent: white, as on the scope.
pub const ACCENT: Color32 = Color32::from_rgb(0xf4, 0xf5, 0xf7);

/// Running, triggered, connected.
pub const GOOD: Color32 = Color32::from_rgb(0x34, 0xc7, 0x6f);
/// Stopped.
pub const STOP: Color32 = Color32::from_rgb(0xe5, 0x48, 0x4d);
/// Waiting for a trigger, connecting.
pub const WAIT: Color32 = Color32::from_rgb(0xf2, 0xa5, 0x2b);
/// Drawing without a trigger.
pub const AUTO: Color32 = Color32::from_rgb(0x5b, 0x9c, 0xf6);
/// A value sent and not yet confirmed by the scope.
pub const PENDING: Color32 = Color32::from_rgb(0x9a, 0x9a, 0xa0);
pub const PROBLEM: Color32 = Color32::from_rgb(0xec, 0x6f, 0x5f);

pub const PLOT_BG: Color32 = Color32::from_rgb(0x07, 0x08, 0x0b);
pub const GRID: Color32 = Color32::from_rgb(0x20, 0x24, 0x2b);
/// The centre lines of the graticule and their ticks.
pub const GRID_AXIS: Color32 = Color32::from_rgb(0x3a, 0x40, 0x4a);
pub const CURSOR: Color32 = Color32::from_rgb(0xd8, 0xdb, 0xe0);

/// Corner radius of cards and controls.
pub const RADIUS: u8 = 6;

/// Text sizes.
pub const SMALL: f32 = 11.0;
pub const BODY: f32 = 13.5;

pub fn mono(size: f32) -> FontId {
    FontId::new(size, FontFamily::Monospace)
}

pub fn body(size: f32) -> FontId {
    FontId::new(size, FontFamily::Proportional)
}

/// `c` with alpha `a` (0..=255), for glows and tints.
pub fn alpha(c: Color32, a: u8) -> Color32 {
    Color32::from_rgba_unmultiplied(c.r(), c.g(), c.b(), a)
}

/// A card: a rounded surface with a thin border.
pub fn card() -> Frame {
    Frame::new()
        .fill(SURFACE)
        .stroke(Stroke::new(1.0, BORDER))
        .corner_radius(CornerRadius::same(RADIUS + 2))
        .inner_margin(Margin::same(10))
}

pub fn apply(ctx: &Context) {
    ctx.set_theme(Theme::Dark);
    ctx.style_mut_of(Theme::Dark, |s| {
        s.text_styles = [
            (TextStyle::Small, body(SMALL)),
            (TextStyle::Body, body(BODY)),
            (TextStyle::Button, body(BODY)),
            (TextStyle::Heading, body(17.0)),
            (TextStyle::Monospace, mono(12.5)),
        ]
        .into();
        let sp = &mut s.spacing;
        sp.item_spacing = egui::vec2(8.0, 6.0);
        sp.button_padding = egui::vec2(10.0, 4.0);
        sp.interact_size = egui::vec2(40.0, 26.0);
        sp.menu_margin = Margin::same(6);
        sp.window_margin = Margin::same(10);
        sp.combo_width = 120.0;
        s.visuals = visuals();
    });
}

fn visuals() -> Visuals {
    let mut v = Visuals::dark();
    let r = CornerRadius::same(RADIUS);
    let w = |bg: Color32, border: Color32, fg: Color32| WidgetVisuals {
        bg_fill: bg,
        weak_bg_fill: bg,
        bg_stroke: Stroke::new(1.0, border),
        corner_radius: r,
        fg_stroke: Stroke::new(1.0, fg),
        expansion: 0.0,
    };
    v.widgets.noninteractive = w(SURFACE, BORDER, TEXT_DIM);
    v.widgets.inactive = w(SURFACE_HI, BORDER, TEXT);
    v.widgets.hovered = w(SURFACE_ON, Color32::from_rgb(0x4a, 0x51, 0x5d), ACCENT);
    v.widgets.active = w(Color32::from_rgb(0x3d, 0x44, 0x50), ACCENT, ACCENT);
    v.widgets.open = w(SURFACE_ON, BORDER, ACCENT);
    v.selection.bg_fill = Color32::from_rgb(0x4a, 0x51, 0x5d);
    v.selection.stroke = Stroke::new(1.0, ACCENT);
    v.panel_fill = PANEL;
    v.window_fill = SURFACE;
    v.window_stroke = Stroke::new(1.0, BORDER);
    v.window_corner_radius = CornerRadius::same(RADIUS + 4);
    v.menu_corner_radius = CornerRadius::same(RADIUS + 2);
    let shadow = Shadow { offset: [0, 6], blur: 18, spread: 0, color: Color32::from_black_alpha(110) };
    v.window_shadow = shadow;
    v.popup_shadow = shadow;
    v.extreme_bg_color = Color32::from_rgb(0x0b, 0x0d, 0x11);
    v.faint_bg_color = Color32::from_rgb(0x19, 0x1d, 0x23);
    v.override_text_color = None;
    v.warn_fg_color = WAIT;
    v.error_fg_color = PROBLEM;
    v.hyperlink_color = ACCENT;
    v
}
