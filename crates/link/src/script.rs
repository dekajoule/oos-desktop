// SPDX-License-Identifier: GPL-2.0-or-later
//! A link that plays a written exchange, for the tests of every layer above: each `> cmd` must be what the code
//! sends next, each `< answer` (or [`Message::Block`]) is what it receives. A send that differs fails the test with
//! both texts.

use crate::{Endpoint, Error, Link, Message};
use std::collections::VecDeque;
use std::time::Instant;

#[derive(Clone, Debug, PartialEq)]
pub enum Step {
    Send(String),
    Answer(Message),
}

#[derive(Debug, Default)]
pub struct Script {
    steps: VecDeque<Step>,
}

impl Script {
    /// `text`: one step per line, `> ` for what is sent, `< ` for a line answer; empty lines are skipped.
    pub fn new(text: &str) -> Script {
        let mut steps = VecDeque::new();
        for l in text.lines().map(str::trim).filter(|l| !l.is_empty()) {
            if let Some(c) = l.strip_prefix("> ") {
                steps.push_back(Step::Send(c.to_string()));
            } else if let Some(a) = l.strip_prefix("< ") {
                steps.push_back(Step::Answer(Message::Line(a.to_string())));
            } else {
                panic!("a script line starts with '> ' or '< ': {l}");
            }
        }
        Script { steps }
    }

    /// Adds a block answer.
    pub fn block(mut self, after: &str, payload: Vec<u8>) -> Script {
        self.steps.push_back(Step::Send(after.to_string()));
        self.steps.push_back(Step::Answer(Message::Block(payload)));
        self
    }

    /// Adds more steps in the same form as [`Script::new`].
    pub fn then(mut self, text: &str) -> Script {
        self.steps.extend(Script::new(text).steps);
        self
    }

    /// True when every step was played.
    pub fn done(&self) -> bool {
        self.steps.is_empty()
    }
}

impl Link for Script {
    fn send(&mut self, cmd: &str) -> Result<(), Error> {
        match self.steps.pop_front() {
            Some(Step::Send(want)) if want == cmd => Ok(()),
            other => panic!("sent {cmd:?}, the script wanted {other:?}"),
        }
    }

    fn recv(&mut self, _deadline: Instant) -> Result<Message, Error> {
        match self.steps.front() {
            Some(Step::Answer(_)) => match self.steps.pop_front() {
                Some(Step::Answer(m)) => Ok(m),
                _ => unreachable!(),
            },
            _ => Err(Error::Timeout),
        }
    }

    fn clear(&mut self) -> Result<(), Error> {
        Ok(())
    }

    fn endpoint(&self) -> Endpoint {
        Endpoint::Tcp { host: "script".into(), port: 0 }
    }
}
