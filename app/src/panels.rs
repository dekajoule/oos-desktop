// SPDX-License-Identifier: GPL-2.0-or-later
//! The side panels, one line each in [`PANELS`]: the setting keys a panel shows (rows of the table in oos-scpi) and
//! whether it repeats them per channel. A panel draws from the model and returns actions; it never talks to the scope.

use crate::model::Model;
use crate::theme;
use eframe::egui::{self, RichText};
use oos_session::{commands, short, Action, Kind, Value};

pub struct PanelInfo {
    /// Stable: stored with the window layout.
    pub id: &'static str,
    pub title: &'static str,
    /// Keys of rows of the table, in the order shown.
    pub keys: &'static [&'static str],
    /// One group per channel of the model.
    pub per_channel: bool,
}

pub static PANELS: &[PanelInfo] = &[
    PanelInfo {
        id: "channels",
        title: "Channels",
        keys: &["chan.on", "chan.scale", "chan.offset", "chan.coupling"],
        per_channel: true,
    },
    PanelInfo { id: "timebase", title: "Time base", keys: &["time.scale", "time.offset"], per_channel: false },
    PanelInfo {
        id: "trigger",
        title: "Trigger",
        keys: &["trig.sweep", "trig.source", "trig.level", "trig.slope"],
        per_channel: false,
    },
];

/// Draws panel `p`; what the user changes is appended to `out` and marked as sent in `m`.
pub fn draw(ui: &mut egui::Ui, p: &PanelInfo, m: &mut Model, out: &mut Vec<Action>) {
    ui.heading(p.title);
    let chs: Vec<u8> = if p.per_channel { (1..=m.channels()).collect() } else { vec![1] };
    for ch in chs {
        if p.per_channel {
            ui.label(RichText::new(format!("CH{ch}")).color(theme::channel(ch)).strong());
        }
        egui::Grid::new((p.id, ch)).num_columns(2).show(ui, |ui| {
            for key in p.keys {
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
            assert!(!p.keys.is_empty(), "{}: no keys", p.id);
            for k in p.keys {
                let row = commands::row(k).unwrap_or_else(|| panic!("{}: no row {k} in the table", p.id));
                assert_eq!(row.per_channel(), p.per_channel, "{}: {k} and the panel differ on channels", p.id);
            }
        }
    }

    #[test]
    fn engineering_units() {
        assert_eq!(eng(0.005, "s/div"), "5 ms/div");
        assert_eq!(eng(2.0, "V"), "2 V");
        assert_eq!(eng(-1.5e-7, "s"), "-150 ns");
        assert_eq!(eng(0.0, "V"), "0 V");
    }
}
