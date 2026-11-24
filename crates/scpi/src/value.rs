// SPDX-License-Identifier: GPL-2.0-or-later
//! A setting's value, read from and written as SCPI text by the kind of its row.

use crate::commands::Kind;

#[derive(Clone, Debug, PartialEq)]
pub enum Value {
    Float(f64),
    Bool(bool),
    /// One of the words of an `Enum` row, in its short form (`NORM` for `NORMal`).
    Word(&'static str),
}

impl Value {
    /// The value as the set command writes it.
    pub fn to_scpi(&self) -> String {
        match self {
            Value::Float(x) => format!("{x:E}"),
            Value::Bool(b) => (if *b { "1" } else { "0" }).to_string(),
            Value::Word(w) => w.to_string(),
        }
    }

    /// Reads a query's answer for a row of kind `kind`; None when it does not fit.
    pub fn parse(kind: &Kind, answer: &str) -> Option<Value> {
        let a = answer.trim().trim_matches('"');
        match kind {
            Kind::Float { .. } => a.parse::<f64>().ok().filter(|x| x.is_finite()).map(Value::Float),
            Kind::Bool => match a.to_ascii_uppercase().as_str() {
                "1" | "ON" => Some(Value::Bool(true)),
                "0" | "OFF" => Some(Value::Bool(false)),
                _ => None,
            },
            Kind::Enum(words) => words.iter().find(|w| word_matches(w, a)).map(|w| Value::Word(short(w))),
        }
    }
}

/// The short form of a SCPI word: its upper-case letters and digits (`NORMal` -> `NORM`, `CHANnel1` -> `CHAN1`).
pub fn short(word: &'static str) -> &'static str {
    let n = word.bytes().take_while(|c| !c.is_ascii_lowercase()).count();
    &word[..n]
}

/// An answer names a word when it is its short form, its long form, or anything between, in any case.
fn word_matches(word: &'static str, a: &str) -> bool {
    let (s, a) = (short(word), a.to_ascii_uppercase());
    a.len() >= s.len() && a.len() <= word.len() && word.to_ascii_uppercase().starts_with(&a)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn answers_read_by_kind() {
        let f = Kind::Float { unit: "V", min: -1.0, max: 1.0 };
        assert_eq!(Value::parse(&f, "5.000000E-01"), Some(Value::Float(0.5)));
        assert_eq!(Value::parse(&f, "nan"), None);
        assert_eq!(Value::parse(&Kind::Bool, "ON"), Some(Value::Bool(true)));
        assert_eq!(Value::parse(&Kind::Bool, "2"), None);
        let e = Kind::Enum(&["AUTO", "NORMal", "SINGle"]);
        assert_eq!(Value::parse(&e, "NORM"), Some(Value::Word("NORM")));
        assert_eq!(Value::parse(&e, "normal"), Some(Value::Word("NORM")));
        assert_eq!(Value::parse(&e, "NOR"), None);
        assert_eq!(Value::parse(&e, "NORMALS"), None);
    }

    #[test]
    fn values_as_sent() {
        assert_eq!(Value::Float(0.5).to_scpi(), "5E-1");
        assert_eq!(Value::Bool(false).to_scpi(), "0");
        assert_eq!(Value::Word("POS").to_scpi(), "POS");
    }
}
