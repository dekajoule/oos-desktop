// SPDX-License-Identifier: GPL-2.0-or-later
//! The side panels, one line each in [`PANELS`]: what a panel shows ([`Body`]: rows of the table of settings in
//! oos-scpi, per channel or once, or the measurements). A panel draws from the model and returns actions; it never
//! talks to the scope.

use crate::model::Model;
use crate::theme;
use eframe::egui::{self, RichText};
use oos_session::{commands, measures, short, Action, Kind, Value, MEASURES};

pub struct PanelInfo {
    /// Stable: stored with the window layout.
    pub id: &'static str,
    pub title: &'static str,
    pub body: Body,
}

pub enum Body {
    /// Rows of the table of settings, in the order shown; `per_channel`: one group per channel of the model.
    Settings { keys: &'static [&'static str], per_channel: bool },
    /// The chosen measurements of the channels that are on.
    Measures,
}

/// Measurements shown before the user chose any.
pub const MEASURES_FIRST: &[&str] = &["vpp", "freq"];

pub static PANELS: &[PanelInfo] = &[
    PanelInfo {
        id: "channels",
        title: "Channels",
        body: Body::Settings { keys: &["chan.on", "chan.scale", "chan.offset", "chan.coupling"], per_channel: true },
    },
    PanelInfo {
        id: "timebase",
        title: "Time base",
        body: Body::Settings { keys: &["time.scale", "time.offset"], per_channel: false },
    },
    PanelInfo {
        id: "trigger",
        title: "Trigger",
        body: Body::Settings { keys: &["trig.sweep", "trig.source", "trig.level", "trig.slope"], per_channel: false },
    },
    PanelInfo { id: "measure", title: "Measure", body: Body::Measures },
];

/// What panel `p` wants read regularly: the measurements of the channels that are on, while it is shown.
pub fn measures_watched(p: &PanelInfo, m: &Model, shown: &[&'static str]) -> Vec<(&'static str, u8)> {
    match p.body {
        Body::Measures => m.channels_on().into_iter().flat_map(|c| shown.iter().map(move |&k| (k, c))).collect(),
        Body::Settings { .. } => Vec::new(),
    }
}

/// Draws panel `p`; what the user changes is appended to `out` and marked as sent in `m`. `shown`: the measurements
/// chosen, which the measurement panel changes.
pub fn draw(ui: &mut egui::Ui, p: &PanelInfo, m: &mut Model, shown: &mut Vec<&'static str>, out: &mut Vec<Action>) {
    ui.heading(p.title);
    match p.body {
        Body::Settings { keys, per_channel } => settings(ui, p.id, keys, per_channel, m, out),
        Body::Measures => measurements(ui, m, shown),
    }
}

/// The chosen measurements, one column per channel that is on, and the choice below.
fn measurements(ui: &mut egui::Ui, m: &Model, shown: &mut Vec<&'static str>) {
    let chs = m.channels_on();
    egui::Grid::new("measures").num_columns(chs.len() + 1).striped(true).show(ui, |ui| {
        ui.label("");
        for &c in &chs {
            ui.label(RichText::new(format!("CH{c}")).color(theme::channel(c)).strong());
        }
        ui.end_row();
        for &k in shown.iter() {
            let Some(row) = measures::measure(k) else { continue };
            ui.label(row.label);
            for &c in &chs {
                match m.measured(k, c) {
                    Some(Some(x)) => ui.label(eng(x, row.unit)),
                    Some(None) => ui.label("—").on_hover_text("the scope has no value for it on this record"),
                    None => ui.label("…"),
                };
            }
            ui.end_row();
        }
    });
    ui.add_space(8.0);
    ui.collapsing("Choose", |ui| {
        for row in MEASURES {
            let mut on = shown.contains(&row.key);
            if ui.checkbox(&mut on, row.label).changed() {
                if on {
                    shown.push(row.key);
                } else {
                    shown.retain(|k| *k != row.key);
                }
            }
        }
    });
}

fn settings(
    ui: &mut egui::Ui,
    id: &'static str,
    keys: &'static [&'static str],
    per_channel: bool,
    m: &mut Model,
    out: &mut Vec<Action>,
) {
    let chs: Vec<u8> = if per_channel { (1..=m.channels()).collect() } else { vec![1] };
    for ch in chs {
        if per_channel {
            ui.label(RichText::new(format!("CH{ch}")).color(theme::channel(ch)).strong());
        }
        egui::Grid::new((id, ch)).num_columns(2).show(ui, |ui| {
            for key in keys {
                let Some(row) = commands::row(key) else { continue };
                ui.label(row.label);
                let now = m.value(row.key, ch);
                let pending = now.is_some_and(|(_, p)| p);
                let changed = ui.horizontal(|ui| {
                    let v = setting(ui, row.key, &row.kind, now);
                    if pending {
                        ui.label(RichText::new("●").color(theme::PENDING)).on_hover_text("sent, waiting for the scope");
                    }
                    v
                });
                if let Some(v) = changed.inner {
                    m.sent(row.key, ch, v.clone());
                    out.push(Action::Set { key: row.key, ch, value: v });
                }
                ui.end_row();
            }
        });
        ui.add_space(6.0);
    }
}

/// The control of one setting; the new value when the user changed it.
fn setting(ui: &mut egui::Ui, key: &'static str, kind: &Kind, now: Option<(&Value, bool)>) -> Option<Value> {
    let Some((v, _)) = now else {
        ui.label("…");
        return None;
    };
    match (kind, v) {
        (Kind::Float { unit, min, max }, Value::Float(x)) => {
            let mut y = *x;
            let r = ui.add(
                egui::DragValue::new(&mut y)
                    .range(*min..=*max)
                    .speed((x.abs() * 0.01).max(1e-6))
                    .custom_formatter(|n, _| eng(n, unit)),
            );
            (r.changed() && y != *x).then_some(Value::Float(y))
        }
        (Kind::Bool, Value::Bool(b)) => {
            let mut c = *b;
            let r = ui.checkbox(&mut c, "");
            r.changed().then_some(Value::Bool(c))
        }
        (Kind::Enum(words), Value::Word(w)) => {
            let mut pick = None;
            egui::ComboBox::from_id_salt((key, ui.scope_id())).selected_text(*w).show_ui(ui, |ui| {
                for word in *words {
                    let s = short(word);
                    if ui.selectable_label(s == *w, *word).clicked() && s != *w {
                        pick = Some(Value::Word(s));
                    }
                }
            });
            pick
        }
        _ => {
            ui.label(RichText::new("?").color(theme::PROBLEM));
            None
        }
    }
}

/// `0.005` with unit `s/div` -> `5 ms/div`.
pub fn eng(x: f64, unit: &str) -> String {
    let a = x.abs();
    let (k, p) = [(1e-9, "n"), (1e-6, "µ"), (1e-3, "m"), (1.0, ""), (1e3, "k")]
        .into_iter()
        .rev()
        .find(|(k, _)| a >= *k)
        .unwrap_or((1.0, ""));
    let n = x / k;
    let t = format!("{n:.3}");
    let t = t.trim_end_matches('0').trim_end_matches('.');
    format!("{t} {p}{unit}")
}

#[cfg(test)]
mod tests {
    use super::*;

    // Break it catches: a panel naming a key the table lacks, two panels with one id, a per-channel panel with a row
    // that has no channel.
    #[test]
    fn every_panel_is_complete() {
        let mut ids = std::collections::HashSet::new();
        for p in PANELS {
            assert!(ids.insert(p.id), "{}: id used twice", p.id);
            if let Body::Settings { keys, per_channel } = p.body {
                assert!(!keys.is_empty(), "{}: no keys", p.id);
                for k in keys {
                    let row = commands::row(k).unwrap_or_else(|| panic!("{}: no row {k} in the table", p.id));
                    assert_eq!(row.per_channel(), per_channel, "{}: {k} and the panel differ on channels", p.id);
                }
            }
        }
        for k in MEASURES_FIRST {
            assert!(measures::measure(k).is_some(), "MEASURES_FIRST: no measurement {k}");
        }
    }

    #[test]
    fn the_measure_panel_watches_its_measurements_of_the_channels_that_are_on() {
        use oos_session::Event;
        let mut m = Model::default();
        for (c, on) in [(1, true), (2, false), (3, true), (4, false)] {
            m.apply(Event::Setting { key: "chan.on", ch: c, value: Value::Bool(on) });
        }
        let measure = PANELS.iter().find(|p| p.id == "measure").expect("measure panel");
        let got = measures_watched(measure, &m, &["vpp", "freq"]);
        assert_eq!(got, vec![("vpp", 1), ("freq", 1), ("vpp", 3), ("freq", 3)]);
        assert!(measures_watched(&PANELS[0], &m, &["vpp"]).is_empty());
    }

    #[test]
    fn engineering_units() {
        assert_eq!(eng(0.005, "s/div"), "5 ms/div");
        assert_eq!(eng(2.0, "V"), "2 V");
        assert_eq!(eng(-1.5e-7, "s"), "-150 ns");
        assert_eq!(eng(0.0, "V"), "0 V");
    }
}
