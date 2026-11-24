// SPDX-License-Identifier: GPL-2.0-or-later
//! The worker against a scripted link: what it sends for a set and what it tells the UI.

use oos_link::Script;
use oos_session::{start, Action, Event, Value};
use std::time::Duration;

const NO_ERROR: &str = "> :SYSTem:ERRor?\n< 0,\"No error\"";

#[test]
fn a_set_then_a_refused_set_then_the_link_is_gone() {
    let script = Script::new("> *IDN?\n< OOS,DHO924S,0,oos-ui 0.1.15")
        .then(&format!("> :CHANnel1:SCALe 2E-1\n{NO_ERROR}"))
        .then("> :TIMebase:SCALe 1E9\n> :SYSTem:ERRor?\n< -222,\"Data out of range\"")
        .then(NO_ERROR);
    let (h, rx) = start(move || Ok(Box::new(script)), || {});
    let wait = Duration::from_secs(5);
    assert!(matches!(rx.recv_timeout(wait), Ok(Event::Connected(id)) if id.model == "DHO924S"));
    h.send(Action::Set { key: "chan.scale", ch: 1, value: Value::Float(0.2) });
    h.send(Action::Set { key: "time.scale", ch: 1, value: Value::Float(1e9) });
    match rx.recv_timeout(wait) {
        Ok(Event::Problem { key: Some("time.scale"), text }) => assert!(text.contains("Data out of range"), "{text}"),
        other => panic!("{other:?}"),
    }
    drop(h);
}
