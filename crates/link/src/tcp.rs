// SPDX-License-Identifier: GPL-2.0-or-later
//! The SCPI socket of OOS: commands end with `\n`, answers are framed by [`Framer`].

use crate::{Endpoint, Error, Framer, Link, Message};
use std::io::{Read, Write};
use std::net::{TcpStream, ToSocketAddrs};
use std::time::{Duration, Instant};

pub struct Tcp {
    stream: TcpStream,
    framer: Framer,
    ep: Endpoint,
}

impl Tcp {
    pub fn connect(ep: Endpoint, deadline: Instant) -> Result<Tcp, Error> {
        let Endpoint::Tcp { host, port } = &ep else { return Err(Error::NotSupported("not a TCP endpoint")) };
        let mut last = Error::Timeout;
        for addr in (host.as_str(), *port).to_socket_addrs()? {
            let left = left(deadline)?;
            match TcpStream::connect_timeout(&addr, left) {
                Ok(stream) => {
                    stream.set_nodelay(true)?;
                    return Ok(Tcp { stream, framer: Framer::default(), ep });
                }
                Err(e) => last = e.into(),
            }
        }
        Err(last)
    }
}

fn left(deadline: Instant) -> Result<Duration, Error> {
    let d = deadline.saturating_duration_since(Instant::now());
    if d.is_zero() {
        Err(Error::Timeout)
    } else {
        Ok(d)
    }
}

impl Link for Tcp {
    fn send(&mut self, cmd: &str) -> Result<(), Error> {
        let mut b = Vec::with_capacity(cmd.len() + 1);
        b.extend_from_slice(cmd.as_bytes());
        b.push(b'\n');
        self.stream.write_all(&b)?;
        Ok(())
    }

    fn recv(&mut self, deadline: Instant) -> Result<Message, Error> {
        let mut chunk = vec![0u8; 64 * 1024];
        loop {
            if let Some(m) = self.framer.take()? {
                return Ok(m);
            }
            self.stream.set_read_timeout(Some(left(deadline)?))?;
            match self.stream.read(&mut chunk)? {
                0 => return Err(Error::Closed),
                n => self.framer.push(&chunk[..n]),
            }
        }
    }

    fn clear(&mut self) -> Result<(), Error> {
        // the raw socket has no device clear: drop what is buffered and what is already on its way
        self.framer.clear();
        self.stream.set_nonblocking(true)?;
        let mut chunk = [0u8; 4096];
        while let Ok(n) = self.stream.read(&mut chunk) {
            if n == 0 {
                break;
            }
        }
        self.stream.set_nonblocking(false)?;
        Ok(())
    }

    fn endpoint(&self) -> Endpoint {
        self.ep.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::TcpListener;

    #[test]
    fn a_query_and_its_block_over_a_socket() {
        let l = TcpListener::bind("127.0.0.1:0").expect("bind");
        let port = l.local_addr().expect("addr").port();
        let peer = std::thread::spawn(move || {
            let (mut s, _) = l.accept().expect("accept");
            let mut got = [0u8; 16];
            let n = s.read(&mut got).expect("read");
            assert_eq!(&got[..n], b":WAV:DATA?\n");
            s.write_all(b"#14").expect("write");
            s.write_all(b"\x00\x01\n\x03\n").expect("write");
        });
        let deadline = Instant::now() + Duration::from_secs(5);
        let mut t = Tcp::connect(Endpoint::Tcp { host: "127.0.0.1".into(), port }, deadline).expect("connect");
        t.send(":WAV:DATA?").expect("send");
        assert_eq!(t.recv(deadline).expect("recv"), Message::Block(vec![0, 1, b'\n', 3]));
        peer.join().expect("peer");
    }
}
