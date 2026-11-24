// SPDX-License-Identifier: GPL-2.0-or-later
//! The strip below the waveform, as along the bottom of the scope's screen: a card per channel, the time base, the
//! acquisition and the trigger, and the chosen measurements of the channels that are on. A click on a channel's card
//! switches it, the wheel over a card turns its knob (V/div, s/div, trigger level). Draws from the model and returns
//! actions, as the panels do.

use crate::model::Model;
use crate::panels::{send, word_label};
use crate::units::{eng, step125};
use crate::{theme, widgets};
use eframe::egui::{self, pos2, vec2, Align2, Color32, CornerRadius, Rect, Response, Sense, Stroke, Ui};
use oos_session::{measures, Action, TriggerState, Value};

/// Wheel travel in points for one step of a knob.
const WHEEL_STEP: f32 = 40.0;

/// The cards; `wheel` keeps the wheel travel between frames.
pub fn cards(ui: &mut Ui, m: &mut Model, out: &mut Vec<Action>, wheel: &mut f32) {
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 6.0;
        for ch in 1..=m.channels() {
            let on = m.is_on(ch);
            let scale = m.float("chan.scale", ch);
            let line1 = scale.map_or("…".into(), |s| eng(s, "V/div"));
            let coupling = m.word("chan.coupling", ch).map(word_label).unwrap_or_default();
            let offset = m.float("chan.offset", ch).map(|o| eng(o, "V")).unwrap_or_default();
            let probe = m.float("chan.probe", ch).filter(|p| (*p - 1.0).abs() > 1e-9).map(|p| eng(p, "×"));
            let mut line2 = format!("{coupling}  {offset}");
            if let Some(p) = probe {
                line2 = format!("{line2}  {p}");
            }
            let hint = "Click: on or off · Wheel: V/div";
            let r = card(ui, 150.0, theme::channel(ch), &format!("CH{ch}"), &line1, &line2, !on, hint);
            if r.clicked() && m.value("chan.on", ch).is_some() {
                send(m, out, "chan.on", ch, Value::Bool(!on));
            }
            if let Some(s) = scale {
                if let Some(up) = turned(ui, &r, wheel) {
                    send(m, out, "chan.scale", ch, Value::Float(step125(s, up)));
                }
            }
        }
        ui.add_space(6.0);
        // the time base
        let ts = m.float("time.scale", 1);
        let line1 = ts.map_or("…".into(), |s| eng(s, "s/div"));
        let line2 = m.float("time.offset", 1).map(|o| format!("Delay {}", eng(o, "s"))).unwrap_or_default();
        let r = card(ui, 140.0, theme::ACCENT, "H", &line1, &line2, false, "Wheel: s/div");
        if let Some(s) = ts {
            if let Some(up) = turned(ui, &r, wheel) {
                send(m, out, "time.scale", 1, Value::Float(step125(s, up)));
            }
        }
        // the acquisition
        let a = m.acquisition;
        let line1 = a.and_then(|a| a.sample_rate).map_or("…".into(), |r| eng(r, "Sa/s"));
        let depth = a.and_then(|a| a.depth).map(|d| eng(d, "pts")).unwrap_or_default();
        let mode = m.word("acq.type", 1).map(word_label).unwrap_or_default();
        card(ui, 140.0, theme::TEXT_DIM, "ACQ", &line1, &format!("{depth}  {mode}"), false, "");
        // the trigger
        let src = m.trigger_source();
        let colour = src.map_or(theme::TEXT_DIM, theme::channel);
        let level = m.float("trig.level", 1);
        let slope = m.word("trig.slope", 1).map(word_label).unwrap_or_default();
        let line1 = match (src, level) {
            (Some(s), Some(l)) => format!("CH{s}  {}", eng(l, "V")),
            _ => "…".into(),
        };
        let sweep = m.word("trig.sweep", 1).map(word_label).unwrap_or_default();
        let r = card(ui, 150.0, colour, "T", &line1, &format!("{slope}  {sweep}"), false, "Wheel: trigger level");
        if let (Some(l), Some(s)) = (level, src.and_then(|s| m.float("chan.scale", s))) {
            if let Some(up) = turned(ui, &r, wheel) {
                // a fifth of a division, a tick of the graticule
                let step = s / 5.0;
                let next = ((l / step).round() + if up { 1.0 } else { -1.0 }) * step;
                send(m, out, "trig.level", 1, Value::Float(next));
            }
        }
    });
}

/// One card of the strip.
#[allow(clippy::too_many_arguments)] // a card is these texts; a struct for them would only be built here
fn card(
    ui: &mut Ui,
    w: f32,
    accent: Color32,
    title: &str,
    line1: &str,
    line2: &str,
    dim: bool,
    hint: &str,
) -> Response {
    let (r, resp) = ui.allocate_exact_size(vec2(w, 52.0), Sense::click());
    if ui.is_rect_visible(r) {
        let p = ui.painter();
        let fill = if resp.hovered() { theme::SURFACE_HI } else { theme::SURFACE };
        let stroke = if resp.hovered() { theme::alpha(accent, 120) } else { theme::BORDER };
        p.rect(r, CornerRadius::same(theme::RADIUS + 2), fill, Stroke::new(1.0, stroke), egui::StrokeKind::Inside);
        let bar = Rect::from_min_size(r.left_top() + vec2(0.0, 8.0), vec2(3.0, r.height() - 16.0));
        let a = if dim { theme::alpha(accent, 70) } else { accent };
        p.rect_filled(bar, CornerRadius::same(2), a);
        let text = if dim { theme::TEXT_FAINT } else { theme::TEXT };
        let sub = if dim { theme::TEXT_FAINT } else { theme::TEXT_DIM };
        let x = r.left() + 12.0;
        p.text(pos2(x, r.top() + 7.0), Align2::LEFT_TOP, title, theme::body(11.0), a);
        if dim {
            p.text(pos2(r.right() - 8.0, r.top() + 7.0), Align2::RIGHT_TOP, "off", theme::body(11.0), sub);
        }
        p.text(pos2(x, r.top() + 21.0), Align2::LEFT_TOP, line1, theme::mono(13.0), text);
        p.text(pos2(x, r.top() + 37.0), Align2::LEFT_TOP, line2, theme::body(11.0), sub);
    }
    if hint.is_empty() {
        resp
    } else {
        resp.on_hover_text(hint)
    }
}

/// Up or down when the wheel over `r` has travelled a step.
fn turned(ui: &Ui, r: &Response, wheel: &mut f32) -> Option<bool> {
    if !r.hovered() {
        return None;
    }
    *wheel += ui.input(|i| i.smooth_scroll_delta.y);
    if wheel.abs() >= WHEEL_STEP {
        let up = *wheel > 0.0;
        *wheel = 0.0;
        Some(up)
    } else {
        None
    }
}

/// The chosen measurements of the channels that are on, as chips.
pub fn measurements(ui: &mut Ui, m: &Model, shown: &[&'static str]) {
    let chs = m.channels_on();
    if chs.is_empty() || shown.is_empty() {
        return;
    }
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing = vec2(6.0, 6.0);
        for &c in &chs {
            for &k in shown {
                let Some(row) = measures::measure(k) else { continue };
                let value = match m.measured(k, c) {
                    Some(Some(x)) => eng(x, row.unit),
                    Some(None) => "—".into(),
                    None => "…".into(),
                };
                chip(ui, theme::channel(c), &format!("CH{c} {}", row.label), &value);
            }
        }
    });
}

fn chip(ui: &mut Ui, colour: Color32, label: &str, value: &str) {
    let p = ui.painter();
    let gl = p.layout_no_wrap(label.to_string(), theme::body(11.0), theme::TEXT_DIM);
    let gv = p.layout_no_wrap(value.to_string(), theme::mono(13.0), theme::TEXT);
    let w = gl.size().x.max(gv.size().x).max(70.0) + 22.0;
    let (r, resp) = ui.allocate_exact_size(vec2(w, 40.0), Sense::hover());
    let p = ui.painter();
    p.rect(
        r,
        CornerRadius::same(theme::RADIUS + 2),
        theme::SURFACE,
        Stroke::new(1.0, theme::BORDER),
        egui::StrokeKind::Inside,
    );
    p.rect_filled(Rect::from_min_size(r.left_top() + vec2(0.0, 7.0), vec2(3.0, r.height() - 14.0)), 2.0, colour);
    p.galley(pos2(r.left() + 12.0, r.top() + 5.0), gl, theme::TEXT_DIM);
    p.galley(pos2(r.left() + 12.0, r.top() + 20.0), gv, theme::TEXT);
    if value == "—" {
        resp.on_hover_text("the scope has no value for it on this record");
    }
}

/// The state of the acquisition as a badge, with its colour: what the scope's top bar shows.
pub fn trigger_badge(ui: &mut Ui, m: &Model) {
    let (text, colour) = match m.acquisition.map(|a| a.trigger) {
        Some(TriggerState::Triggered) => ("T'D", theme::GOOD),
        Some(TriggerState::Auto) => ("AUTO", theme::AUTO),
        Some(TriggerState::Wait) => ("WAIT", theme::WAIT),
        Some(TriggerState::Stop) => ("STOP", theme::STOP),
        None => ("—", theme::TEXT_FAINT),
    };
    widgets::badge(ui, text, colour).on_hover_text("the trigger's state as the scope reports it");
}
