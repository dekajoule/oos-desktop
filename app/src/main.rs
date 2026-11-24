// SPDX-License-Identifier: GPL-2.0-or-later
//! oos-desktop: the window (docs/architecture.md 3.4). Everything here draws from the [`model::Model`] and returns
//! actions; the talking is done by the session's worker (ADR 0005).

mod app;
mod model;
mod panels;
mod plot;
mod theme;

fn main() -> eframe::Result {
    let opts = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default().with_inner_size([1280.0, 800.0]).with_title("oos-desktop"),
        ..Default::default()
    };
    eframe::run_native("oos-desktop", opts, Box::new(|cc| Ok(Box::new(app::App::new(cc)))))
}
