// SPDX-License-Identifier: GPL-2.0-or-later
//! Answers out of a byte stream: a line up to `\n`, or a block `#<n><len><payload>` followed by `\n` (OOS
//! docs/scpi.md, Waveform transfer). Every length is bounded, since the bytes come from outside.

use crate::{Error, Message};

/// The longest text answer taken.
pub const MAX_LINE: usize = 1 << 20;
/// The largest block taken (a 125 M-point record in WORD format, with room).
pub const MAX_BLOCK: usize = 300 << 20;

/// Collects bytes and hands out whole answers.
#[derive(Default)]
pub struct Framer {
    buf: Vec<u8>,
}

impl Framer {
    /// Adds bytes as they came.
    pub fn push(&mut self, b: &[u8]) {
        self.buf.extend_from_slice(b);
    }

    /// The next whole answer, `None` while it is incomplete.
    pub fn take(&mut self) -> Result<Option<Message>, Error> {
        if self.buf.first() == Some(&b'#') {
            return self.block();
        }
        match self.buf.iter().position(|&c| c == b'\n') {
            Some(i) => {
                let line: Vec<u8> = self.buf.drain(..=i).take(i).collect();
                let text = String::from_utf8_lossy(&line).trim_end_matches('\r').to_string();
                Ok(Some(Message::Line(text)))
            }
            None if self.buf.len() > MAX_LINE => Err(self.malformed("a line longer than allowed")),
            None => Ok(None),
        }
    }

    /// Drops everything buffered.
    pub fn clear(&mut self) {
        self.buf.clear();
    }

    fn block(&mut self) -> Result<Option<Message>, Error> {
        let Some(&n) = self.buf.get(1) else { return Ok(None) };
        if !(b'1'..=b'9').contains(&n) {
            // `#0` (indefinite length) is not something OOS sends
            return Err(self.malformed("a block without a definite length"));
        }
        let n = (n - b'0') as usize;
        let Some(digits) = self.buf.get(2..2 + n) else { return Ok(None) };
        let len = std::str::from_utf8(digits)
            .ok()
            .filter(|s| s.bytes().all(|c| c.is_ascii_digit()))
            .and_then(|s| s.parse::<usize>().ok())
            .ok_or_else(|| self.malformed("a block length that is not a number"))?;
        if len > MAX_BLOCK {
            return Err(self.malformed("a block larger than allowed"));
        }
        let start = 2 + n;
        if self.buf.len() < start + len {
            return Ok(None);
        }
        let payload = self.buf[start..start + len].to_vec();
        let mut end = start + len;
        // the newline after the block
        if self.buf.get(end) == Some(&b'\n') {
            end += 1;
        } else if self.buf.len() == end {
            return Ok(None);
        }
        self.buf.drain(..end);
        Ok(Some(Message::Block(payload)))
    }

    fn malformed(&mut self, what: &'static str) -> Error {
        let got = self.buf.iter().take(40).copied().collect();
        self.buf.clear();
        Error::Malformed { what, got }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn all(input: &[&[u8]]) -> Vec<Result<Message, String>> {
        let mut f = Framer::default();
        let mut out = Vec::new();
        for part in input {
            f.push(part);
            loop {
                match f.take() {
                    Ok(Some(m)) => out.push(Ok(m)),
                    Ok(None) => break,
                    Err(e) => {
                        out.push(Err(e.to_string()));
                        break;
                    }
                }
            }
        }
        out
    }

    #[test]
    fn lines_and_blocks_split_anywhere() {
        let whole: &[u8] = b"OOS,DHO924S,0,oos-ui 0.1.15\n#15ab\ncd\n1.0E-3\r\n";
        for cut in 0..whole.len() {
            let got = all(&[&whole[..cut], &whole[cut..]]);
            assert_eq!(
                got,
                vec![
                    Ok(Message::Line("OOS,DHO924S,0,oos-ui 0.1.15".into())),
                    Ok(Message::Block(b"ab\ncd".to_vec())),
                    Ok(Message::Line("1.0E-3".into())),
                ],
                "cut at {cut}"
            );
        }
    }

    #[test]
    fn a_block_without_its_newline_waits_for_more() {
        let mut f = Framer::default();
        f.push(b"#13xyz");
        assert!(matches!(f.take(), Ok(None)));
        f.push(b"\n");
        assert_eq!(f.take().ok().flatten(), Some(Message::Block(b"xyz".to_vec())));
    }

    #[test]
    fn broken_headers_and_sizes_are_errors() {
        for bad in [&b"#0abc\n"[..], b"#2x1abc\n", b"#9999999999"] {
            let got = all(&[bad]);
            assert!(matches!(got.last(), Some(Err(_))), "{:?} gave {:?}", String::from_utf8_lossy(bad), got);
        }
        let long = vec![b'a'; MAX_LINE + 1];
        assert!(matches!(all(&[&long]).last(), Some(Err(_))));
    }
}
