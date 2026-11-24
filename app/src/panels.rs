// SPDX-License-Identifier: GPL-2.0-or-later
//! The side panels, one line each in [`PANELS`]: what a panel shows ([`Body`]: rows of the table of settings in
//! oos-scpi, per channel or once, or the choice of measurements). A panel draws from the model and returns actions;
//! it never talks to the scope.

use crate::model::Model;
use crate::units::{eng, step125};
use crate::{theme, widgets};
use eframe::egui::{self, RichText};
use oos_session::{commands, measures, short, Action, Kind, Value, MEASURES};

pub struct PanelInfo {
    /// Stable: stored with the window's settings.
    pub id: &'static str,
    pub title: &'static str,
    pub body: Body,
}

pub enum Body {
    /// Rows of the table of settings, in the order shown; `per_channel`: one card per channel of the model, its
    /// `chan.on` as the switch in the card's head.
    Settings { keys: &'static [&'static str], per_channel: bool },
    /// The choice of measurements and their values.
    Measures,
}

/// Measurements shown before the user chose any.
pub const MEASURES_FIRST: &[&str] = &["vpp", "freq"];

/// Settings whose control has buttons for the 1-2-5 steps of the scope's knob.
const STEPPED: &[&str] = &["chan.scale", "time.scale"];

pub static PANELS: &[PanelInfo] = &[
    PanelInfo {
        id: "channels",
        title: "Channels",
        body: Body::Settings {
            keys: &[
                "chan.on",
                "chan.scale",
                "chan.offset",
                "chan.coupling",
                "chan.probe",
                "chan.bandwidth",
                "chan.invert",
            ],
            per_channel: true,
        },
    },
    PanelInfo {
        id: "timebase",
        title: "Horizontal",
        body: Body::Settings { keys: &["time.scale", "time.offset", "time.expand", "time.roll"], per_channel: false },
    },
    PanelInfo {
        id: "trigger",
        title: "Trigger",
        body: Body::Settings {
            keys: &[
                "trig.sweep",
                "trig.source",
                "trig.level",
                "trig.slope",
                "trig.coupling",
                "trig.holdoff",
                "trig.noise",
            ],
            per_channel: false,
        },
    },
    PanelInfo {
        id: "acquire",
        title: "Acquire",
        body: Body::Settings { keys: &["acq.type", "acq.averages"], per_channel: false },
    },
    PanelInfo { id: "measure", title: "Measure", body: Body::Measures },
];

/// The measurements read regularly: the chosen ones of the channels that are on, shown below the waveform.
pub fn measures_watched(m: &Model, shown: &[&'static str]) -> Vec<(&'static str, u8)> {
    m.channels_on().into_iter().flat_map(|c| shown.iter().map(move |&k| (k, c))).collect()
}

/// A value the user set: shown as pending in `m` and sent.
pub fn send(m: &mut Model, out: &mut Vec<Action>, key: &'static str, ch: u8, v: Value) {
    m.sent(key, ch, v.clone());
    out.push(Action::Set { key, ch, value: v });
}

/// The word a person reads for a SCPI word's short form.
pub fn word_label(w: &str) -> &str {
    match w {
        "POS" => "Rising",
        "NEG" => "Falling",
        "RFAL" => "Either",
        "NORM" => "Normal",
        "SING" => "Single",
        "AUTO" => "Auto",
        "LFR" => "LF reject",
        "HFR" => "HF reject",
        "AVER" => "Average",
        "HRES" => "High res",
        "PEAK" => "Peak",
        "CENT" => "Centre",
        "LB" => "Left",
        "RB" => "Right",
        "TRIG" => "Trigger",
        "OFF" => "Off",
        "CHAN1" => "CH1",
        "CHAN2" => "CH2",
        "CHAN3" => "CH3",
        "CHAN4" => "CH4",
        other => other,
    }
}

/// The tab row choosing the panel.
pub fn tabs(ui: &mut egui::Ui, panel: &mut &'static str) {
    let labels: Vec<&str> = PANELS.iter().map(|p| p.title).collect();
    let chosen = PANELS.iter().position(|p| p.id == *panel);
    if let Some(i) = widgets::segmented(ui, &labels, chosen, &[]) {
        *panel = PANELS[i].id;
    }
}

/// Draws panel `p`; what the user changes is appended to `out` and marked as sent in `m`. `shown`: the measurements
/// chosen, which the measurement panel changes.
pub fn draw(ui: &mut egui::Ui, p: &PanelInfo, m: &mut Model, shown: &mut Vec<&'static str>, out: &mut Vec<Action>) {
    match p.body {
        Body::Settings { keys, per_channel: true } => {
            for ch in 1..=m.channels() {
                channel_card(ui, p.id, keys, ch, m, out);
                ui.add_space(6.0);
            }
        }
        Body::Settings { keys, per_channel: false } => {
            theme::card().show(ui, |ui| {
                ui.set_width(ui.available_width());
                rows(ui, p.id, keys, 1, m, out);
            });
        }
        Body::Measures => measurements(ui, m, shown),
    }
}

fn channel_card(
    ui: &mut egui::Ui,
    id: &'static str,
    keys: &'static [&'static str],
    ch: u8,
    m: &mut Model,
    out: &mut Vec<Action>,
) {
    let colour = theme::channel(ch);
    theme::card().show(ui, |ui| {
        ui.set_width(ui.available_width());
        let on = m.is_on(ch);
        ui.horizontal(|ui| {
            let (r, _) = ui.allocate_exact_size(egui::vec2(4.0, 18.0), egui::Sense::hover());
            ui.painter().rect_filled(r, 2.0, colour);
            ui.label(RichText::new(format!("CH{ch}")).color(colour).strong().size(14.5));
            if let Some(s) = m.float("chan.scale", ch) {
                ui.label(RichText::new(eng(s, "V/div")).color(theme::TEXT_DIM).monospace());
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if keys.contains(&"chan.on") {
                    match m.value("chan.on", ch) {
                        Some((Value::Bool(b), pending)) => {
                            let mut v = *b;
                            if widgets::switch(ui, &mut v, colour).on_hover_text("show the channel").changed() {
                                send(m, out, "chan.on", ch, Value::Bool(v));
                            }
                            if pending {
                                pending_dot(ui);
                            }
                        }
                        _ => {
                            ui.label("…");
                        }
                    }
                }
            });
        });
        if on {
            ui.add_space(4.0);
            let rest: Vec<&'static str> = keys.iter().copied().filter(|k| *k != "chan.on").collect();
            rows(ui, id, &rest, ch, m, out);
        }
    });
}

fn rows(ui: &mut egui::Ui, id: &'static str, keys: &[&'static str], ch: u8, m: &mut Model, out: &mut Vec<Action>) {
    egui::Grid::new((id, ch)).num_columns(2).min_col_width(92.0).spacing([10.0, 8.0]).show(ui, |ui| {
        for key in keys {
            let Some(row) = commands::row(key) else { continue };
            ui.label(RichText::new(row.label).color(theme::TEXT_DIM));
            let now = m.value(row.key, ch);
            let pending = now.is_some_and(|(_, p)| p);
            let changed = ui.horizontal(|ui| {
                let v = setting(ui, row.key, &row.kind, now, m);
                if pending {
                    pending_dot(ui);
                }
                v
            });
            if let Some(v) = changed.inner {
                send(m, out, row.key, ch, v);
            }
            ui.end_row();
        }
    });
}

fn pending_dot(ui: &mut egui::Ui) {
    ui.label(RichText::new("●").color(theme::PENDING).size(9.0)).on_hover_text("sent, waiting for the scope");
}

/// The control of one setting; the new value when the user changed it.
fn setting(ui: &mut egui::Ui, key: &'static str, kind: &Kind, now: Option<(&Value, bool)>, m: &Model) -> Option<Value> {
    let Some((v, _)) = now else {
        ui.label(RichText::new("…").color(theme::TEXT_FAINT));
        return None;
    };
    match (kind, v) {
        (Kind::Float { unit, min, max }, Value::Float(x)) => {
            let mut y = *x;
            let stepped = STEPPED.contains(&key);
            let mut picked = None;
            if stepped && ui.small_button("‹").on_hover_text("one step down").clicked() {
                picked = Some(step125(*x, false).clamp(*min, *max));
            }
            let r = ui.add(
                egui::DragValue::new(&mut y)
                    .range(*min..=*max)
                    .speed((x.abs() * 0.01).max(1e-6))
                    .custom_formatter(|n, _| eng(n, unit)),
            );
            if stepped && ui.small_button("›").on_hover_text("one step up").clicked() {
                picked = Some(step125(*x, true).clamp(*min, *max));
            }
            if r.changed() && y != *x {
                picked = Some(y);
            }
            picked.map(Value::Float)
        }
        (Kind::Bool, Value::Bool(b)) => {
            let mut c = *b;
            widgets::switch(ui, &mut c, theme::ACCENT).changed().then_some(Value::Bool(c))
        }
        (Kind::Enum(words), Value::Word(w)) => {
            let shorts: Vec<&'static str> = words.iter().map(|w| short(w)).collect();
            let labels: Vec<&str> = shorts.iter().map(|s| word_label(s)).collect();
            let chosen = shorts.iter().position(|s| s == w);
            let fits = words.len() <= 4 && labels.iter().map(|l| l.len()).sum::<usize>() <= 26;
            if fits {
                // the channels of the trigger source in their colours, the others of the model left out
                let tint: Vec<_> = shorts.iter().map(|s| channel_of(s).map(theme::channel)).collect();
                let n = shorts.iter().take_while(|s| channel_of(s).is_none_or(|c| c <= m.channels())).count();
                widgets::segmented(ui, &labels[..n], chosen, &tint).map(|i| Value::Word(shorts[i]))
            } else {
                let mut pick = None;
                egui::ComboBox::from_id_salt((key, ui.scope_id())).selected_text(word_label(w)).show_ui(ui, |ui| {
                    for (i, s) in shorts.iter().enumerate() {
                        if ui.selectable_label(s == w, labels[i]).clicked() && s != w {
                            pick = Some(Value::Word(s));
                        }
                    }
                });
                pick
            }
        }
        _ => {
            ui.label(RichText::new("?").color(theme::PROBLEM));
            None
        }
    }
}

/// The channel a word such as `CHAN2` names.
fn channel_of(s: &str) -> Option<u8> {
    s.strip_prefix("CHAN")?.parse().ok()
}

/// The chosen measurements, one column per channel that is on, and the choice below.
fn measurements(ui: &mut egui::Ui, m: &Model, shown: &mut Vec<&'static str>) {
    let chs = m.channels_on();
    theme::card().show(ui, |ui| {
        ui.set_width(ui.available_width());
        widgets::section(ui, "Values");
        if chs.is_empty() {
            ui.label(RichText::new("No channel is on.").color(theme::TEXT_DIM));
        }
        egui::Grid::new("measures").num_columns(chs.len() + 1).striped(true).spacing([14.0, 6.0]).show(ui, |ui| {
            if chs.is_empty() {
                return;
            }
            ui.label("");
            for &c in &chs {
                ui.label(RichText::new(format!("CH{c}")).color(theme::channel(c)).strong());
            }
            ui.end_row();
            for &k in shown.iter() {
                let Some(row) = measures::measure(k) else { continue };
                ui.label(RichText::new(row.label).color(theme::TEXT_DIM));
                for &c in &chs {
                    match m.measured(k, c) {
                        Some(Some(x)) => ui.label(RichText::new(eng(x, row.unit)).monospace()),
                        Some(None) => ui.label("—").on_hover_text("the scope has no value for it on this record"),
                        None => ui.label(RichText::new("…").color(theme::TEXT_FAINT)),
                    };
                }
                ui.end_row();
            }
        });
    });
    ui.add_space(6.0);
    theme::card().show(ui, |ui| {
        ui.set_width(ui.available_width());
        widgets::section(ui, "Shown below the waveform");
        egui::Grid::new("choose").num_columns(2).spacing([16.0, 6.0]).show(ui, |ui| {
            for (i, row) in MEASURES.iter().enumerate() {
                let mut on = shown.contains(&row.key);
                if ui.checkbox(&mut on, row.label).changed() {
                    if on {
                        shown.push(row.key);
                    } else {
                        shown.retain(|k| *k != row.key);
                    }
                }
                if i % 2 == 1 {
                    ui.end_row();
                }
            }
        });
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    // Break it catches: a panel naming a key the table lacks, two panels with one id, a per-channel panel with a row
    // that has no channel, a row of the table that no panel shows.
    #[test]
    fn every_panel_is_complete() {
        let mut ids = std::collections::HashSet::new();
        let mut shown = std::collections::HashSet::new();
        for p in PANELS {
            assert!(ids.insert(p.id), "{}: id used twice", p.id);
            if let Body::Settings { keys, per_channel } = p.body {
                assert!(!keys.is_empty(), "{}: no keys", p.id);
                for k in keys {
                    let row = commands::row(k).unwrap_or_else(|| panic!("{}: no row {k} in the table", p.id));
                    assert_eq!(row.per_channel(), per_channel, "{}: {k} and the panel differ on channels", p.id);
                    shown.insert(*k);
                }
            }
        }
        for r in oos_session::ROWS {
            assert!(shown.contains(r.key), "{}: no panel shows this row", r.key);
        }
        for k in MEASURES_FIRST {
            assert!(measures::measure(k).is_some(), "MEASURES_FIRST: no measurement {k}");
        }
    }

    #[test]
    fn the_chosen_measurements_of_the_channels_that_are_on_are_watched() {
        use oos_session::Event;
        let mut m = Model::default();
        for (c, on) in [(1, true), (2, false), (3, true), (4, false)] {
            m.apply(Event::Setting { key: "chan.on", ch: c, value: Value::Bool(on) });
        }
        let got = measures_watched(&m, &["vpp", "freq"]);
        assert_eq!(got, vec![("vpp", 1), ("freq", 1), ("vpp", 3), ("freq", 3)]);
        assert!(measures_watched(&m, &[]).is_empty());
    }

    #[test]
    fn words_read_as_a_person_says_them() {
        assert_eq!(word_label("POS"), "Rising");
        assert_eq!(word_label("20M"), "20M");
        assert_eq!(channel_of("CHAN2"), Some(2));
        assert_eq!(channel_of("AUTO"), None);
    }
}
