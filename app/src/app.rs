// SPDX-License-Identifier: GPL-2.0-or-later
//! The window: the top bar with the connection and the run controls, the panels on the right, the waveform in the
//! middle with the measurements below it, the strip of channel, time base and trigger cards, and the status line.

use crate::model::Model;
use crate::panels::{self, MEASURES_FIRST, PANELS};
use crate::plot::{self, CursorMode, Cursors};
use crate::widgets::{self, Icon, Tone};
use crate::{icon, strip, theme};
use eframe::egui::{self, vec2, Margin, RichText};
use oos_session::{measures, Action, Endpoint, Event, Found, Handle, TriggerState, Watch};
use std::sync::mpsc::Receiver;
use std::time::{Duration, Instant};

pub struct App {
    address: String,
    session: Option<(Handle, Receiver<Event>)>,
    model: Model,
    /// The panel shown on the right (an id of `PANELS`).
    panel: &'static str,
    watched: Option<Watch>,
    /// The measurements chosen (keys of `MEASURES`).
    measures: Vec<&'static str>,
    /// A search for scopes under way, and the scopes the last one found.
    finding: Option<Receiver<Result<Vec<Found>, String>>>,
    found: Vec<Found>,
    cursors: Cursors,
    /// Wheel travel over the strip's cards, between frames.
    wheel: f32,
    /// Records per second, counted over the last second.
    rate: f32,
    counted_at: Instant,
}

impl App {
    pub fn new(cc: &eframe::CreationContext<'_>) -> App {
        theme::apply(&cc.egui_ctx);
        let get = |k: &str| cc.storage.and_then(|s| s.get_string(k));
        let address = get("address").unwrap_or_default();
        let measures = match get("measures") {
            Some(s) => s.split(',').filter_map(|k| measures::measure(k).map(|m| m.key)).collect(),
            None => MEASURES_FIRST.to_vec(),
        };
        let panel = get("panel").and_then(|id| PANELS.iter().find(|p| p.id == id)).unwrap_or(&PANELS[0]).id;
        // an address on the command line connects at the start
        let given = std::env::args().nth(1).filter(|a| !a.is_empty());
        let mut app = App {
            address: given.clone().unwrap_or(address),
            session: None,
            model: Model::default(),
            panel,
            watched: None,
            measures,
            finding: None,
            found: Vec::new(),
            cursors: Cursors::default(),
            wheel: 0.0,
            rate: 0.0,
            counted_at: Instant::now(),
        };
        if given.is_some() {
            app.connect(&cc.egui_ctx);
        }
        app
    }

    fn connect(&mut self, ctx: &egui::Context) {
        let Some(ep) = Endpoint::parse(&self.address) else {
            self.model = Model::default();
            self.model.problem = Some(format!("not an address: {}", self.address));
            return;
        };
        let c = ctx.clone();
        self.model = Model::default();
        self.watched = None;
        self.session = Some(oos_session::open(ep, move || c.request_repaint()));
    }

    fn disconnect(&mut self) {
        // dropping the handle ends the worker (Action::Close)
        self.session = None;
        self.watched = None;
        self.model = Model::default();
        self.model.notice = Some("disconnected".into());
    }

    fn find(&mut self, ctx: &egui::Context) {
        let c = ctx.clone();
        self.finding = Some(oos_session::find(move || c.request_repaint()));
    }

    fn send(&mut self, a: Action) {
        if let Some((h, _)) = &self.session {
            h.send(a);
        }
    }

    fn live(&self) -> bool {
        self.session.is_some() && self.model.identity.is_some()
    }

    /// Running unless the scope says it stopped (Run/Stop is one key, as on the scope).
    fn running(&self) -> bool {
        self.model.acquisition.is_none_or(|a| a.trigger != TriggerState::Stop)
    }

    fn take_events(&mut self) {
        if let Some((_, rx)) = &self.session {
            for e in rx.try_iter() {
                self.model.apply(e);
            }
        }
        if let Some(r) = self.finding.as_ref().and_then(|rx| rx.try_recv().ok()) {
            self.finding = None;
            match r {
                Ok(v) if v.is_empty() => self.model.notice = Some("no scope answered".into()),
                Ok(v) => {
                    self.model.notice = Some(format!("{} found", v.len()));
                    self.found = v;
                }
                Err(e) => self.model.problem = Some(format!("finding scopes: {e}")),
            }
        }
        if self.model.lost.is_some() {
            self.session = None;
        }
        let dt = self.counted_at.elapsed();
        if dt >= Duration::from_secs(1) {
            self.rate = self.model.records as f32 / dt.as_secs_f32();
            self.model.records = 0;
            self.counted_at = Instant::now();
        }
    }

    /// What the visible parts need: the records of the channels, the settings, the chosen measurements, the state.
    fn watch(&mut self) {
        // the records of the channels that are on; all of them until the scope said which are
        let known = (1..=self.model.channels()).all(|c| self.model.value("chan.on", c).is_some());
        let channels = if known { self.model.channels_on() } else { (1..=self.model.channels()).collect() };
        let want = Watch {
            channels,
            settings: true,
            measures: panels::measures_watched(&self.model, &self.measures),
            acquisition: true,
        };
        if self.live() && self.watched.as_ref() != Some(&want) {
            self.send(Action::Watch(want.clone()));
            self.watched = Some(want);
        }
    }

    fn keys(&self, ctx: &egui::Context, out: &mut Vec<Action>) {
        if !self.live() || ctx.egui_wants_keyboard_input() {
            return;
        }
        ctx.input(|i| {
            if i.key_pressed(egui::Key::Space) {
                out.push(if self.running() { Action::Stop } else { Action::Run });
            }
            if i.key_pressed(egui::Key::S) {
                out.push(Action::Single);
            }
            if i.key_pressed(egui::Key::F) {
                out.push(Action::Force);
            }
            if i.key_pressed(egui::Key::C) {
                out.push(Action::Clear);
            }
        });
    }

    fn top_bar(&mut self, ui: &mut egui::Ui, ctx: &egui::Context, out: &mut Vec<Action>) {
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 6.0;
            let (r, _) = ui.allocate_exact_size(vec2(26.0, 26.0), egui::Sense::hover());
            icon::paint(ui.painter(), r);
            ui.label(RichText::new("OOS").strong().size(15.0).color(theme::TEXT));
            ui.label(RichText::new("Desktop").size(15.0).color(theme::TEXT_DIM));
            ui.add_space(14.0);

            let (colour, state) = match (&self.model.identity, &self.model.lost, self.session.is_some()) {
                (_, Some(_), _) => (theme::STOP, "lost"),
                (Some(_), None, true) => (theme::GOOD, "connected"),
                (None, None, true) => (theme::WAIT, "connecting"),
                _ => (theme::TEXT_FAINT, "not connected"),
            };
            let (r, _) = ui.allocate_exact_size(vec2(10.0, 26.0), egui::Sense::hover());
            ui.painter().circle_filled(r.center(), 4.0, colour);
            let field = egui::TextEdit::singleline(&mut self.address)
                .hint_text("scope.local, 192.0.2.7 or a resource")
                .desired_width(220.0)
                .margin(Margin::symmetric(8, 4));
            let r = ui.add(field).on_hover_text(state);
            let enter = r.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
            if self.session.is_some() {
                if widgets::tool(ui, true, Icon::Close, "Disconnect", Tone::Plain, "end the connection").clicked() {
                    self.disconnect();
                }
            } else if widgets::tool(ui, true, Icon::Plug, "Connect", Tone::Plain, "connect to this address").clicked()
                || enter
            {
                self.connect(ctx);
            }
            let label = if self.finding.is_some() { "Finding…" } else { "Find" };
            let hint = "look for scopes on the network and on USB";
            if widgets::tool(ui, self.finding.is_none(), Icon::Search, label, Tone::Plain, hint).clicked() {
                self.find(ctx);
            }
            let mut pick = None;
            if !self.found.is_empty() {
                ui.menu_button(format!("Found {} ▾", self.found.len()), |ui| {
                    for f in &self.found {
                        if ui.button(format!("{}  ·  {}", f.name, f.endpoint)).clicked() {
                            pick = Some(f.endpoint.to_string());
                            ui.close();
                        }
                    }
                });
            }
            if let Some(a) = pick {
                self.address = a;
                self.connect(ctx);
            }

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.spacing_mut().item_spacing.x = 6.0;
                let live = self.live();
                if self.live() {
                    strip::trigger_badge(ui, &self.model);
                    ui.add_space(4.0);
                }
                // the system's save dialog is modal; the file is written by the session's worker (ADR 0005)
                let chs = self.model.channels_on();
                let hint = "save the records of the channels that are on as CSV";
                if widgets::tool(ui, live && !chs.is_empty(), Icon::Save, "", Tone::Plain, hint).clicked() {
                    let f = rfd::FileDialog::new().set_file_name("oos-record.csv").add_filter("CSV", &["csv"]);
                    if let Some(path) = f.save_file() {
                        out.push(Action::SaveRecords { chs, path });
                    }
                }
                let hint = "save the scope's screen as PNG";
                if widgets::tool(ui, live, Icon::Camera, "", Tone::Plain, hint).clicked() {
                    let f = rfd::FileDialog::new().set_file_name("oos-screen.png").add_filter("PNG", &["png"]);
                    if let Some(path) = f.save_file() {
                        out.push(Action::SaveScreenshot(path));
                    }
                }
                ui.add_space(8.0);
                for (mode, ic, hint) in [
                    (CursorMode::Level, Icon::CursorY, "level cursors: two lines of one channel's volts"),
                    (CursorMode::Time, Icon::CursorX, "time cursors: two lines, their times and distance"),
                ] {
                    let on = self.cursors.mode == mode;
                    let tone = if on { Tone::On } else { Tone::Plain };
                    if widgets::tool(ui, true, ic, "", tone, hint).clicked() {
                        self.cursors.mode = if on { CursorMode::Off } else { mode };
                    }
                }
                ui.add_space(8.0);
                if widgets::tool(ui, live, Icon::Clear, "", Tone::Plain, "clear persistence and statistics (C)")
                    .clicked()
                {
                    out.push(Action::Clear);
                }
                let hint = "the scope sets channels, time base and trigger to the signals";
                if widgets::tool(ui, live, Icon::Auto, "Auto", Tone::Plain, hint).clicked() {
                    out.push(Action::Autoscale);
                }
                if widgets::tool(ui, live, Icon::Force, "", Tone::Plain, "one record without a trigger (F)").clicked() {
                    out.push(Action::Force);
                }
                if widgets::tool(ui, live, Icon::Single, "Single", Tone::Plain, "one record on the next trigger (S)")
                    .clicked()
                {
                    out.push(Action::Single);
                }
                let (ic, label, tone) = if !live {
                    (Icon::Play, "Run", Tone::Plain)
                } else if self.running() {
                    (Icon::Play, "Running", Tone::Colour(theme::GOOD))
                } else {
                    (Icon::Stop, "Stopped", Tone::Colour(theme::STOP))
                };
                if widgets::tool(ui, live, ic, label, tone, "Run/Stop (Space)").clicked() {
                    out.push(if self.running() { Action::Stop } else { Action::Run });
                }
            });
        });
    }

    fn status_line(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            let small = |t: String, c| RichText::new(t).size(theme::SMALL + 0.5).color(c);
            match (&self.model.identity, &self.model.lost) {
                (_, Some(why)) => {
                    ui.label(small(format!("Connection lost: {why}"), theme::PROBLEM));
                }
                (Some(id), None) => {
                    ui.label(small(format!("{} · OOS {}", id.model, id.version), theme::TEXT_DIM));
                    ui.label(small(format!("· {}", self.address), theme::TEXT_FAINT));
                }
                (None, None) if self.session.is_some() => {
                    ui.label(small("Connecting…".into(), theme::WAIT));
                }
                _ => {
                    ui.label(small("Not connected".into(), theme::TEXT_FAINT));
                }
            }
            ui.separator();
            let mut clear = false;
            if let Some(p) = &self.model.problem {
                ui.label(small(format!("⚠ {p}"), theme::PROBLEM));
                clear = ui.small_button("×").on_hover_text("dismiss").clicked();
            } else if let Some(n) = &self.model.notice {
                ui.label(small(n.clone(), theme::TEXT_DIM));
            }
            if clear {
                self.model.problem = None;
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label(small(format!("oos-desktop {}", env!("CARGO_PKG_VERSION")), theme::TEXT_FAINT));
                if self.live() {
                    ui.separator();
                    ui.label(small(format!("{:.1} records/s", self.rate), theme::TEXT_DIM));
                }
            });
        });
    }

    /// What the middle shows before a connection: how to reach a scope, and the ones found.
    fn welcome(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        let avail = ui.available_rect_before_wrap();
        let w = 460.0_f32.min(avail.width() - 40.0);
        ui.add_space((avail.height() * 0.18).max(20.0));
        ui.vertical_centered(|ui| {
            ui.set_max_width(w);
            theme::card().inner_margin(Margin::same(22)).show(ui, |ui| {
                ui.set_width(w - 44.0);
                ui.horizontal(|ui| {
                    let (r, _) = ui.allocate_exact_size(vec2(40.0, 40.0), egui::Sense::hover());
                    icon::paint(ui.painter(), r);
                    ui.vertical(|ui| {
                        ui.label(RichText::new("Connect to an oscilloscope").size(18.0).color(theme::TEXT));
                        ui.label(RichText::new("running OOS, over the network or USB").color(theme::TEXT_DIM));
                    });
                });
                ui.add_space(12.0);
                ui.label(
                    RichText::new(
                        "Type the scope's name or address into the field at the top, or let the program look for \
                         scopes. On the scope, switch SCPI on under System › Network › Remote services.",
                    )
                    .color(theme::TEXT_DIM),
                );
                ui.add_space(12.0);
                ui.horizontal(|ui| {
                    let label = if self.finding.is_some() { "Looking…" } else { "Find scopes" };
                    let hint = "an mDNS question on every network, and this computer's USB";
                    if widgets::tool(ui, self.finding.is_none(), Icon::Search, label, Tone::Plain, hint).clicked() {
                        self.find(ctx);
                    }
                    if !self.address.is_empty() {
                        let label = format!("Connect to {}", self.address);
                        if widgets::tool(ui, true, Icon::Plug, &label, Tone::Plain, "").clicked() {
                            self.connect(ctx);
                        }
                    }
                });
                if !self.found.is_empty() {
                    ui.add_space(10.0);
                    widgets::section(ui, "Found");
                    let mut pick = None;
                    for f in &self.found {
                        let text = RichText::new(format!("{}   {}", f.name, f.endpoint)).color(theme::TEXT);
                        if ui.add(egui::Button::new(text).min_size(vec2(w - 44.0, 30.0))).clicked() {
                            pick = Some(f.endpoint.to_string());
                        }
                    }
                    if let Some(a) = pick {
                        self.address = a;
                        self.connect(ctx);
                    }
                }
            });
        });
    }
}

impl eframe::App for App {
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        storage.set_string("address", self.address.clone());
        storage.set_string("measures", self.measures.join(","));
        storage.set_string("panel", self.panel.to_string());
    }

    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        self.take_events();
        self.watch();
        let mut out = Vec::new();
        self.keys(&ctx, &mut out);

        let bar = egui::Frame::new().fill(theme::PANEL).inner_margin(Margin::symmetric(12, 8));
        egui::Panel::top("bar").frame(bar).show(ui, |ui| self.top_bar(ui, &ctx, &mut out));
        let status = egui::Frame::new().fill(theme::PANEL).inner_margin(Margin::symmetric(12, 4));
        egui::Panel::bottom("status").frame(status).show(ui, |ui| self.status_line(ui));
        if self.live() {
            let frame = egui::Frame::new().fill(theme::BG).inner_margin(Margin::symmetric(12, 8));
            egui::Panel::bottom("strip")
                .frame(frame)
                .show_separator_line(false)
                .show(ui, |ui| strip::cards(ui, &mut self.model, &mut out, &mut self.wheel));
        }
        let side = egui::Frame::new().fill(theme::PANEL).inner_margin(Margin::same(10));
        egui::Panel::right("panels").frame(side).min_size(320.0).default_size(340.0).show(ui, |ui| {
            panels::tabs(ui, &mut self.panel);
            ui.add_space(8.0);
            if let Some(p) = PANELS.iter().find(|p| p.id == self.panel) {
                egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
                    if self.model.identity.is_none() && !matches!(p.body, panels::Body::Measures) {
                        ui.label(RichText::new("Connect to a scope to see its settings.").color(theme::TEXT_FAINT));
                    } else {
                        panels::draw(ui, p, &mut self.model, &mut self.measures, &mut out);
                    }
                    if self.cursors.mode == CursorMode::Level {
                        ui.add_space(8.0);
                        theme::card().show(ui, |ui| {
                            ui.set_width(ui.available_width());
                            widgets::section(ui, "Level cursors read");
                            let n = self.model.channels();
                            let labels: Vec<String> = (1..=n).map(|c| format!("CH{c}")).collect();
                            let labels: Vec<&str> = labels.iter().map(String::as_str).collect();
                            let tint: Vec<_> = (1..=n).map(|c| Some(theme::channel(c))).collect();
                            let chosen = Some(self.cursors.source.saturating_sub(1) as usize);
                            if let Some(i) = widgets::segmented(ui, &labels, chosen, &tint) {
                                self.cursors.source = i as u8 + 1;
                            }
                        });
                    }
                });
            }
        });
        let centre = egui::Frame::new().fill(theme::BG).inner_margin(Margin { left: 12, right: 8, top: 10, bottom: 4 });
        egui::CentralPanel::default().frame(centre).show(ui, |ui| {
            if self.session.is_none() && self.model.identity.is_none() {
                self.welcome(ui, &ctx);
                return;
            }
            let chips = !self.model.channels_on().is_empty() && !self.measures.is_empty();
            let below = if chips { 48.0 } else { 0.0 };
            let mut r = ui.available_rect_before_wrap();
            r.max.y -= below;
            plot::draw(ui, r, &self.model, &mut self.cursors);
            ui.advance_cursor_after_rect(r);
            if chips {
                ui.add_space(6.0);
                strip::measurements(ui, &self.model, &self.measures);
            }
        });
        for a in out {
            self.send(a);
        }
    }
}
