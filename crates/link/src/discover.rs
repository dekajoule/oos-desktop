// SPDX-License-Identifier: GPL-2.0-or-later
//! The scopes on the local network: one mDNS question for `_scpi-raw._tcp.local` (OOS docs/network.md 4.1), asked
//! from an ordinary port, so the responders answer it directly (a legacy unicast answer, RFC 6762 §6.7) and nothing
//! has to listen on port 5353. The question goes out on every IPv4 interface that is up (a scope on a cable straight
//! to the computer is not behind the default route). Every packet is untrusted: names are read with a bound on their
//! length and on the pointers they follow.

use crate::endpoint::{Endpoint, SCPI_PORT};
use std::net::{Ipv4Addr, SocketAddr, UdpSocket};
use std::time::{Duration, Instant};

const SERVICE: &[&str] = &["_scpi-raw", "_tcp", "local"];
const T_A: u16 = 1;
const T_PTR: u16 = 12;
const T_SRV: u16 = 33;
const MDNS: (Ipv4Addr, u16) = (Ipv4Addr::new(224, 0, 0, 251), 5353);

/// A scope that answered: the name it announces (`Rigol DHO924S OOS (123456)`) and where its SCPI port is.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Found {
    pub name: String,
    pub endpoint: Endpoint,
}

/// Asks once on every IPv4 interface and collects the answers for `wait`.
pub fn discover(wait: Duration) -> std::io::Result<Vec<Found>> {
    let sock = socket2::Socket::new(socket2::Domain::IPV4, socket2::Type::DGRAM, Some(socket2::Protocol::UDP))?;
    sock.bind(&SocketAddr::from((Ipv4Addr::UNSPECIFIED, 0)).into())?;
    sock.set_multicast_ttl_v4(255)?;
    let s: UdpSocket = sock.into();
    let q = question(0x4f53);
    let mut sent = 0;
    for ip in interfaces() {
        let s2 = socket2::SockRef::from(&s);
        if s2.set_multicast_if_v4(&ip).is_ok() && s.send_to(&q, MDNS).is_ok() {
            sent += 1;
        }
    }
    if sent == 0 {
        s.send_to(&q, MDNS)?;
    }
    let end = Instant::now() + wait;
    let mut found: Vec<Found> = Vec::new();
    let mut buf = [0u8; 9000];
    loop {
        let left = end.saturating_duration_since(Instant::now());
        if left.is_zero() {
            break;
        }
        s.set_read_timeout(Some(left))?;
        match s.recv_from(&mut buf) {
            Ok((n, SocketAddr::V4(from))) => {
                for f in answers(&buf[..n], *from.ip()) {
                    if !found.contains(&f) {
                        found.push(f);
                    }
                }
            }
            Ok(_) => {}
            Err(e) if matches!(e.kind(), std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut) => break,
            Err(e) => return Err(e),
        }
    }
    found.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(found)
}

/// The IPv4 addresses of the interfaces that are up, loopback included (a simulator on this computer).
fn interfaces() -> Vec<Ipv4Addr> {
    let mut v: Vec<Ipv4Addr> = if_addrs::get_if_addrs()
        .unwrap_or_default()
        .into_iter()
        .filter_map(|i| match i.ip() {
            std::net::IpAddr::V4(a) => Some(a),
            _ => None,
        })
        .collect();
    v.sort();
    v.dedup();
    v
}

/// The question: one PTR of the SCPI service, class IN.
fn question(id: u16) -> Vec<u8> {
    let mut q = Vec::with_capacity(40);
    q.extend_from_slice(&id.to_be_bytes());
    q.extend_from_slice(&[0, 0, 0, 1, 0, 0, 0, 0, 0, 0]);
    for l in SERVICE {
        q.push(l.len() as u8);
        q.extend_from_slice(l.as_bytes());
    }
    q.extend_from_slice(&[0, 0, T_PTR as u8, 0, 1]);
    q
}

/// The scopes in one answer packet from `from`: each PTR of the service with the port of its SRV (5555 without one)
/// and the address of the SRV target's A record (the sender's address without one).
pub fn answers(p: &[u8], from: Ipv4Addr) -> Vec<Found> {
    let Some(records) = records(p) else { return Vec::new() };
    let lower = |v: &[String]| v.iter().map(|s| s.to_ascii_lowercase()).collect::<Vec<_>>();
    let service: Vec<String> = SERVICE.iter().map(|s| s.to_string()).collect();
    let mut out = Vec::new();
    for r in &records {
        let Rdata::Ptr(instance) = &r.data else { continue };
        if lower(&r.name) != service || instance.len() <= SERVICE.len() || instance[1..] != service[..] {
            continue;
        }
        let srv = records.iter().find_map(|s| match &s.data {
            Rdata::Srv(port, target) if lower(&s.name) == lower(instance) => Some((*port, target.clone())),
            _ => None,
        });
        let (port, target) = srv.map_or((SCPI_PORT, None), |(p, t)| (p, Some(t)));
        let addr = target
            .and_then(|t| {
                records.iter().find_map(|a| match a.data {
                    Rdata::A(ip) if lower(&a.name) == lower(&t) => Some(ip),
                    _ => None,
                })
            })
            .unwrap_or(from);
        out.push(Found { name: instance[0].clone(), endpoint: Endpoint::Tcp { host: addr.to_string(), port } });
    }
    out
}

enum Rdata {
    Ptr(Vec<String>),
    Srv(u16, Vec<String>),
    A(Ipv4Addr),
    Other,
}

struct Record {
    name: Vec<String>,
    data: Rdata,
}

/// The answer, authority and additional records of a response; None for anything malformed.
fn records(p: &[u8]) -> Option<Vec<Record>> {
    let u16_at = |o: usize| p.get(o..o + 2).map(|b| u16::from_be_bytes([b[0], b[1]]));
    if u16_at(2)? & 0x8000 == 0 {
        return None; // a question, not a response
    }
    let qd = u16_at(4)? as usize;
    let rr = u16_at(6)? as usize + u16_at(8)? as usize + u16_at(10)? as usize;
    let mut o = 12;
    for _ in 0..qd {
        o = name(p, o)?.1 + 4;
    }
    let mut out = Vec::with_capacity(rr.min(64));
    for _ in 0..rr.min(256) {
        let (n, next) = name(p, o)?;
        let (t, len) = (u16_at(next)?, u16_at(next + 8)? as usize);
        let d = next + 10;
        let body = p.get(d..d + len)?;
        let data = match t {
            T_PTR => Rdata::Ptr(name(p, d)?.0),
            T_SRV if len >= 7 => Rdata::Srv(u16::from_be_bytes([body[4], body[5]]), name(p, d + 6)?.0),
            T_A if len == 4 => Rdata::A(Ipv4Addr::new(body[0], body[1], body[2], body[3])),
            _ => Rdata::Other,
        };
        out.push(Record { name: n, data });
        o = d + len;
    }
    Some(out)
}

/// A name at `o`, with compression pointers followed backwards only and at most 255 bytes; the offset after it.
fn name(p: &[u8], mut o: usize) -> Option<(Vec<String>, usize)> {
    let (mut labels, mut end, mut size) = (Vec::new(), None, 0usize);
    loop {
        let len = *p.get(o)? as usize;
        match len {
            0 => return Some((labels, end.unwrap_or(o + 1))),
            l if l & 0xc0 == 0xc0 => {
                let to = (l & 0x3f) << 8 | *p.get(o + 1)? as usize;
                if to >= o {
                    return None; // a pointer forward or to itself: a loop
                }
                end.get_or_insert(o + 2);
                o = to;
            }
            l if l < 64 => {
                size += l + 1;
                if size > 255 {
                    return None;
                }
                labels.push(String::from_utf8_lossy(p.get(o + 1..o + 1 + l)?).into_owned());
                o += 1 + l;
            }
            _ => return None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A response as OOS sends it: PTR, then SRV, TXT and A as additional records, with name compression.
    fn response(port: u16, with_a: bool) -> Vec<u8> {
        let mut p = vec![0x4f, 0x53, 0x84, 0, 0, 0, 0, 1, 0, 0, 0, if with_a { 3 } else { 2 }];
        // answer: _scpi-raw._tcp.local PTR "Rigol DHO924S OOS (000000)"._scpi-raw._tcp.local
        let svc = p.len();
        for l in SERVICE {
            p.push(l.len() as u8);
            p.extend_from_slice(l.as_bytes());
        }
        p.push(0);
        p.extend_from_slice(&[0, 12, 0, 1, 0, 0, 0, 10]);
        let inst = "Rigol DHO924S OOS (000000)";
        p.extend_from_slice(&((inst.len() + 3) as u16).to_be_bytes());
        let inst_at = p.len();
        p.push(inst.len() as u8);
        p.extend_from_slice(inst.as_bytes());
        p.extend_from_slice(&[0xc0, svc as u8]);
        // SRV of the instance: port, target dho924s-000000.local
        p.extend_from_slice(&[0xc0, inst_at as u8, 0, 33, 0x80, 1, 0, 0, 0, 10]);
        let host = "dho924s-000000";
        p.extend_from_slice(&((6 + 1 + host.len() + 2) as u16).to_be_bytes());
        p.extend_from_slice(&[0, 0, 0, 0]);
        p.extend_from_slice(&port.to_be_bytes());
        let host_at = p.len();
        p.push(host.len() as u8);
        p.extend_from_slice(host.as_bytes());
        p.extend_from_slice(&[0xc0, (svc + 15) as u8]); // "local" inside the service name
                                                        // TXT, ignored
        p.extend_from_slice(&[0xc0, inst_at as u8, 0, 16, 0x80, 1, 0, 0, 0, 10, 0, 1, 0]);
        if with_a {
            p.extend_from_slice(&[0xc0, host_at as u8, 0, 1, 0x80, 1, 0, 0, 0, 10, 0, 4, 192, 0, 2, 7]);
        }
        p
    }

    #[test]
    fn a_scope_with_its_port_and_address() {
        let got = answers(&response(5555, true), Ipv4Addr::new(198, 51, 100, 1));
        let want = Endpoint::Tcp { host: "192.0.2.7".into(), port: 5555 };
        assert_eq!(got, vec![Found { name: "Rigol DHO924S OOS (000000)".into(), endpoint: want }]);
    }

    #[test]
    fn without_an_a_record_the_sender_is_the_scope() {
        let got = answers(&response(6000, false), Ipv4Addr::new(198, 51, 100, 1));
        assert_eq!(got[0].endpoint, Endpoint::Tcp { host: "198.51.100.1".into(), port: 6000 });
    }

    #[test]
    fn the_question_asks_for_the_scpi_service() {
        let q = question(1);
        assert_eq!(&q[..12], &[0, 1, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0]);
        assert!(q.ends_with(b"\x05local\x00\x00\x0c\x00\x01"));
    }

    #[test]
    fn broken_packets_give_nothing_and_do_not_panic() {
        let from = Ipv4Addr::LOCALHOST;
        let good = response(5555, true);
        for cut in 0..good.len() {
            let _ = answers(&good[..cut], from);
        }
        // a pointer to itself, a question instead of a response, a label longer than 63
        let mut looped = good.clone();
        looped[12] = 0xc0;
        looped[13] = 12;
        for bad in [looped, question(1), vec![0, 0, 0x84, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0x50, b'a']] {
            assert!(answers(&bad, from).is_empty());
        }
    }
}
