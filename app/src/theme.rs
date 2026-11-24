// SPDX-License-Identifier: GPL-2.0-or-later
//! Colours and spacing, in one place: neutral, a white accent, round shapes, close to the scope's own look.

use eframe::egui::{Color32, Context, Visuals};

/// The trace colour of channel `ch` (1-based).
pub fn channel(ch: u8) -> Color32 {
    match ch {
        1 => Color32::from_rgb(0xf2, 0xd2, 0x3c),
        2 => Color32::from_rgb(0x3c, 0xd2, 0xf2),
        3 => Color32::from_rgb(0xe0, 0x5c, 0xe0),
        _ => Color32::from_rgb(0x4c, 0x8c, 0xf2),
    }
}

pub const PLOT_BG: Color32 = Color32::from_rgb(0x10, 0x10, 0x12);
pub const GRID: Color32 = Color32::from_rgb(0x3a, 0x3a, 0x40);
/// A value sent and not yet confirmed by the scope.
pub const PENDING: Color32 = Color32::from_rgb(0x9a, 0x9a, 0xa0);
pub const PROBLEM: Color32 = Color32::from_rgb(0xe0, 0x70, 0x60);

pub fn apply(ctx: &Context) {
    let mut v = Visuals::dark();
    v.selection.bg_fill = Color32::from_rgb(0x60, 0x60, 0x66);
    v.selection.stroke.color = Color32::WHITE;
    ctx.set_visuals(v);
}
