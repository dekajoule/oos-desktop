// SPDX-License-Identifier: GPL-2.0-or-later
//! The window: the connect bar and run control at the top, the panels on the right, the waveform in the middle.

use crate::model::Model;
use crate::panels::{self, MEASURES_FIRST, PANELS};
use crate::{plot, theme};
use eframe::egui::{self, RichText};
use oos_session::{measures, Action, Endpoint, Event, Found, Handle, Watch};
use std::sync::mpsc::Receiver;

pub struct App {
    address: String,
    session: Option<(Handle, Receiver<Event>)>,
    model: Model,
    /// The panel shown on the right (an id of `PANELS`).
    panel: &'static str,
    watched: Option<Watch>,
    /// The measurements chosen in the measurement panel (keys of `MEASURES`).
    measures: Vec<&'static str>,
    /// A search for scopes under way, and the scopes the last one found.
    finding: Option<Receiver<Result<Vec<Found>, String>>>,
    found: Vec<Found>,
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
        App {
            address,
            session: None,
            model: Model::default(),
            panel: PANELS[0].id,
            watched: None,
            measures,
            finding: None,
            found: Vec::new(),
        }
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

    fn send(&mut self, a: Action) {
        if let Some((h, _)) = &self.session {
            h.send(a);
        }
    }
}

impl eframe::App for App {
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        storage.set_string("address", self.address.clone());
        storage.set_string("measures", self.measures.join(","));
    }

    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        if let Some((_, rx)) = &self.session {
            for e in rx.try_iter() {
                self.model.apply(e);
            }
        }
        if let Some(r) = self.finding.as_ref().and_then(|rx| rx.try_recv().ok()) {
            self.finding = None;
            match r {
                Ok(v) if v.is_empty() => self.model.notice = Some("no scope answered on this network".into()),
                Ok(v) => {
                    self.model.notice = Some(format!("{} found: choose under Found", v.len()));
                    self.found = v;
                }
                Err(e) => self.model.problem = Some(format!("finding scopes: {e}")),
            }
        }
        if self.model.lost.is_some() {
            self.session = None;
        }
        // what the visible parts need: the shown channels' records, the settings while a panel is open
        let panel = PANELS.iter().find(|p| p.id == self.panel).unwrap_or(&PANELS[0]);
        let want = Watch {
            channels: (1..=self.model.channels()).collect(),
            settings: true,
            measures: panels::measures_watched(panel, &self.model, &self.measures),
        };
        if self.session.is_some() && self.model.identity.is_some() && self.watched.as_ref() != Some(&want) {
            self.send(Action::Watch(want.clone()));
            self.watched = Some(want);
        }

        let mut out = Vec::new();
        egui::Panel::top("bar").show(ui, |ui| {
            ui.horizontal(|ui| {
                let r = ui.add(egui::TextEdit::singleline(&mut self.address).hint_text("scope.local or 192.0.2.7"));
                let enter = r.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
                if ui.button("Connect").clicked() || enter {
                    self.connect(&ctx);
                }
                let label = if self.finding.is_some() { "Finding…" } else { "Find" };
                let find = ui.add_enabled(self.finding.is_none(), egui::Button::new(label));
                if find.clicked() {
                    let c = ctx.clone();
                    self.finding = Some(oos_session::find(move || c.request_repaint()));
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
                    self.connect(&ctx);
                }
                ui.separator();
                let live = self.session.is_some();
                for (label, a) in [("Run", Action::Run), ("Stop", Action::Stop), ("Single", Action::Single)] {
                    if ui.add_enabled(live, egui::Button::new(label)).clicked() {
                        out.push(a);
                    }
                }
                ui.separator();
                // the system's save dialog is modal; the file is written by the session's worker (ADR 0005)
                if ui.add_enabled(live, egui::Button::new("Screenshot…")).clicked() {
                    let f = rfd::FileDialog::new().set_file_name("oos-screen.png").add_filter("PNG", &["png"]);
                    if let Some(path) = f.save_file() {
                        out.push(Action::SaveScreenshot(path));
                    }
                }
                let chs = self.model.channels_on();
                if ui.add_enabled(live && !chs.is_empty(), egui::Button::new("Save record…")).clicked() {
                    let f = rfd::FileDialog::new().set_file_name("oos-record.csv").add_filter("CSV", &["csv"]);
                    if let Some(path) = f.save_file() {
                        out.push(Action::SaveRecords { chs, path });
                    }
                }
                ui.separator();
                match (&self.model.identity, &self.model.lost) {
                    (_, Some(why)) => ui.label(RichText::new(format!("lost: {why}")).color(theme::PROBLEM)),
                    (Some(id), None) => ui.label(format!("{} · OOS {}", id.model, id.version)),
                    (None, None) if self.session.is_some() => ui.label("connecting…"),
                    _ => ui.label("not connected"),
                };
            });
        });
        egui::Panel::bottom("status").show(ui, |ui| {
            ui.horizontal(|ui| {
                if let Some(n) = &self.model.notice {
                    ui.label(n);
                }
                if let Some(p) = &self.model.problem {
                    ui.label(RichText::new(p).color(theme::PROBLEM));
                }
            });
        });
        egui::Panel::right("panels").min_size(260.0).show(ui, |ui| {
            ui.horizontal(|ui| {
                for p in PANELS {
                    ui.selectable_value(&mut self.panel, p.id, p.title);
                }
            });
            ui.separator();
            if let Some(p) = PANELS.iter().find(|p| p.id == self.panel) {
                egui::ScrollArea::vertical()
                    .show(ui, |ui| panels::draw(ui, p, &mut self.model, &mut self.measures, &mut out));
            }
        });
        egui::CentralPanel::default().show(ui, |ui| {
            let r = ui.available_rect_before_wrap();
            plot::draw(ui.painter(), r, &self.model.waves);
        });
        for a in out {
            self.send(a);
        }
    }
}
