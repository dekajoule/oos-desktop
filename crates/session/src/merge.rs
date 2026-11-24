// SPDX-License-Identifier: GPL-2.0-or-later
//! Actions waiting in the queue, merged: of several sets of one setting only the newest is sent (a knob turned fast
//! gives one set, not twenty), and everything else keeps its order.

use crate::Action;

pub fn merge(waiting: Vec<Action>) -> Vec<Action> {
    let mut out: Vec<Action> = Vec::with_capacity(waiting.len());
    for a in waiting {
        if let Action::Set { key, ch, .. } = &a {
            out.retain(|b| !matches!(b, Action::Set { key: k, ch: c, .. } if k == key && c == ch));
        }
        if matches!(a, Action::Watch(_)) {
            out.retain(|b| !matches!(b, Action::Watch(_)));
        }
        out.push(a);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Value, Watch};

    fn set(key: &'static str, ch: u8, x: f64) -> Action {
        Action::Set { key, ch, value: Value::Float(x) }
    }

    #[test]
    fn the_newest_set_of_a_setting_wins_and_the_order_stays() {
        let got = merge(vec![
            set("chan.scale", 1, 0.1),
            Action::Stop,
            set("chan.scale", 2, 0.2),
            set("chan.scale", 1, 0.3),
            Action::Watch(Watch::default()),
            Action::Watch(Watch { channels: vec![1], settings: true, measures: vec![] }),
        ]);
        assert_eq!(
            got,
            vec![
                Action::Stop,
                set("chan.scale", 2, 0.2),
                set("chan.scale", 1, 0.3),
                Action::Watch(Watch { channels: vec![1], settings: true, measures: vec![] }),
            ]
        );
    }
}
