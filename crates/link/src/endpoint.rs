// SPDX-License-Identifier: GPL-2.0-or-later
//! Where a link goes, written as a VISA-like resource string so that users can paste what they know from VISA:
//! `TCPIP::<host>::<port>::SOCKET` and `USB::<vendor>::<product>::<serial>::INSTR`.

/// The SCPI port of OOS (docs/network.md 4 of OOS).
pub const SCPI_PORT: u16 = 5555;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Endpoint {
    Tcp { host: String, port: u16 },
    Usb { vendor: u16, product: u16, serial: String },
}

impl Endpoint {
    /// Reads a resource string, or a bare `host` / `host:port` as typed into the connect field.
    pub fn parse(s: &str) -> Option<Endpoint> {
        let s = s.trim();
        let parts: Vec<&str> = s.split("::").collect();
        match parts.as_slice() {
            [t, host, port, kind] if t.eq_ignore_ascii_case("TCPIP") && kind.eq_ignore_ascii_case("SOCKET") => {
                Some(Endpoint::Tcp { host: host_ok(host)?, port: port.parse().ok()? })
            }
            [t, vendor, product, serial, kind]
                if t.eq_ignore_ascii_case("USB") && kind.eq_ignore_ascii_case("INSTR") =>
            {
                Some(Endpoint::Usb { vendor: hex(vendor)?, product: hex(product)?, serial: serial_ok(serial)? })
            }
            [one] => match one.rsplit_once(':') {
                Some((host, port)) if !host.contains(':') => {
                    Some(Endpoint::Tcp { host: host_ok(host)?, port: port.parse().ok()? })
                }
                _ => Some(Endpoint::Tcp { host: host_ok(one)?, port: SCPI_PORT }),
            },
            _ => None,
        }
    }
}

impl std::fmt::Display for Endpoint {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Endpoint::Tcp { host, port } => write!(f, "TCPIP::{host}::{port}::SOCKET"),
            Endpoint::Usb { vendor, product, serial } => {
                write!(f, "USB::{vendor:#06x}::{product:#06x}::{serial}::INSTR")
            }
        }
    }
}

fn host_ok(h: &str) -> Option<String> {
    let ok = !h.is_empty()
        && h.len() <= 253
        && h.bytes().all(|c| c.is_ascii_alphanumeric() || c == b'.' || c == b'-' || c == b'_');
    ok.then(|| h.to_ascii_lowercase())
}

fn serial_ok(s: &str) -> Option<String> {
    (!s.is_empty() && s.len() <= 64 && s.bytes().all(|c| c.is_ascii_graphic() && c != b':')).then(|| s.to_string())
}

fn hex(s: &str) -> Option<u16> {
    let s = s.strip_prefix("0x").or_else(|| s.strip_prefix("0X")).unwrap_or(s);
    u16::from_str_radix(s, 16).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_endpoint_round_trips() {
        let all = [
            Endpoint::Tcp { host: "dho924s-000000.local".into(), port: SCPI_PORT },
            Endpoint::Tcp { host: "192.0.2.7".into(), port: 5025 },
            Endpoint::Usb { vendor: 0x1209, product: 0x0001, serial: "DHO900-TEST-0".into() },
        ];
        for ep in all {
            assert_eq!(Endpoint::parse(&ep.to_string()), Some(ep.clone()), "{ep}");
        }
    }

    #[test]
    fn what_people_type_in_the_connect_field() {
        let tcp = |h: &str, p| Some(Endpoint::Tcp { host: h.into(), port: p });
        assert_eq!(Endpoint::parse("192.0.2.7"), tcp("192.0.2.7", SCPI_PORT));
        assert_eq!(Endpoint::parse(" Scope.local:5025 "), tcp("scope.local", 5025));
        assert_eq!(Endpoint::parse("tcpip::scope::5555::socket"), tcp("scope", 5555));
        for bad in ["", "a b", "host:port", "USB::zz::1::x::INSTR", "TCPIP::h::5555::INSTR", "h;rm"] {
            assert_eq!(Endpoint::parse(bad), None, "{bad}");
        }
    }
}
