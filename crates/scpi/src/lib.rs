// SPDX-License-Identifier: GPL-2.0-or-later
//! What the scope understands (docs/architecture.md 3.2): the table of settings ([`commands`]), the measurements shown
//! ([`measures`]), the typed queries of [`Scope`], and the waveform preamble. Pure: no threads, no clock of its own,
//! no UI. Every SCPI header of the program is written in this crate and nowhere else (ADR 0004).

pub mod commands;
pub mod measures;
mod scope;
mod value;
mod waveform;

pub use commands::{Kind, Row, ROWS};
pub use measures::{Measure, MEASURES};
pub use scope::{Identity, Scope, ScpiError};
pub use value::{short, Value};
pub use waveform::{Preamble, Waveform};
