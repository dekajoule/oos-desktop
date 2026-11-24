// SPDX-License-Identifier: GPL-2.0-or-later
//! oos-desktop: the window (docs/architecture.md 3.4). Everything here draws from the [`model::Model`] and returns
//! actions; the talking is done by the session's worker (ADR 0005).

mod app;
mod icon;
mod model;
mod panels;
mod plot;
mod strip;
mod theme;
mod units;
mod widgets;

fn main() -> eframe::Result {
    let opts = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_inner_size([1440.0, 880.0])
            .with_min_inner_size([1024.0, 640.0])
            .with_title("OOS Desktop")
            .with_app_id("oos-desktop")
            .with_icon(icon::window_icon()),
        ..Default::default()
    };
    eframe::run_native("oos-desktop", opts, Box::new(|cc| Ok(Box::new(app::App::new(cc)))))
}
