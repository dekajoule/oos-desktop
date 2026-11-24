// SPDX-License-Identifier: GPL-2.0-or-later
//! The contract test (docs/architecture.md 6): a session against `oos-ui --serve` of the OOS commit in `OOS_REV`, the
//! demo source behind SCPI without hardware. `tools/sim.sh` starts it and sets `OOS_SIM`; without it the test only
//! says that it did not run.

use oos_session::{open, Action, Endpoint, Event, Value, Watch, ROWS};
use std::time::{Duration, Instant};

#[test]
fn a_session_against_the_simulator() {
    let Some(addr) = std::env::var("OOS_SIM").ok().filter(|s| !s.is_empty()) else {
        eprintln!("not run: OOS_SIM is not set (tools/sim.sh sets it)");
        return;
    };
    let ep = Endpoint::parse(&addr).expect("OOS_SIM is host:port");
    let (h, rx) = open(ep, || {});
    let next = |what: &str| rx.recv_timeout(Duration::from_secs(10)).unwrap_or_else(|_| panic!("no event: {what}"));
    let id = match next("connected") {
        Event::Connected(id) => id,
        other => panic!("{other:?}"),
    };
    assert_eq!(id.maker, "OOS");
    h.send(Action::Watch(Watch { channels: vec![1], settings: true, measures: vec![("vpp", 1)] }));
    // every row answers, for every channel of the model, and a record comes
    let mut seen = std::collections::HashSet::new();
    let mut wave = None;
    let mut vpp = None;
    let end = Instant::now() + Duration::from_secs(20);
    while Instant::now() < end && (wave.is_none() || vpp.is_none() || seen.len() < ROWS.len()) {
        match next("settings and a record") {
            Event::Setting { key, .. } => {
                seen.insert(key);
            }
            Event::Waveform(w) => wave = Some(w),
            Event::Measured { key: "vpp", ch: 1, value } => vpp = vpp.or(value),
            Event::Measured { .. } => {}
            Event::Problem { key, text } => panic!("{key:?}: {text}"),
            other => panic!("{other:?}"),
        }
    }
    let missing: Vec<_> = ROWS.iter().map(|r| r.key).filter(|k| !seen.contains(k)).collect();
    assert!(missing.is_empty(), "rows without an answer: {missing:?}");
    let w = wave.expect("a record of CH1");
    assert!(vpp.is_some_and(|v| v > 0.0), "no peak-peak value of CH1: {vpp:?}");
    assert!(w.codes.len() as u64 == w.preamble.points && !w.codes.is_empty(), "{:?}", w.preamble);
    // a screenshot and the record are written where the user chose
    let dir = std::env::temp_dir().join(format!("oos-desktop-sim-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("temp dir");
    let (png, csv) = (dir.join("s.png"), dir.join("r.csv"));
    h.send(Action::SaveScreenshot(png.clone()));
    h.send(Action::SaveRecords { chs: vec![1], path: csv.clone() });
    let mut saved = Vec::new();
    while saved.len() < 2 {
        match next("files saved") {
            Event::Saved(p) => saved.push(p),
            Event::Problem { text, .. } => panic!("{text}"),
            _ => {}
        }
    }
    assert!(std::fs::read(&png).expect("png").starts_with(b"\x89PNG"));
    let text = std::fs::read_to_string(&csv).expect("csv");
    assert!(text.starts_with("time_s,ch1_V\n") && text.lines().count() > 100, "{}", &text[..text.len().min(200)]);
    let _ = std::fs::remove_dir_all(&dir);
    // a set comes back as the scope's answer
    h.send(Action::Set { key: "chan.scale", ch: 1, value: Value::Float(0.5) });
    let end = Instant::now() + Duration::from_secs(10);
    loop {
        assert!(Instant::now() < end, "the scale did not come back as 0.5");
        if let Event::Setting { key: "chan.scale", ch: 1, value: Value::Float(x) } = next("the new scale") {
            if (x - 0.5).abs() < 1e-9 {
                break;
            }
        }
    }
}
