// SPDX-License-Identifier: GPL-2.0-or-later
//! The scope as a USBTMC instrument on its rear USB port (OOS docs/usb-device.md): SCPI in DEV_DEP_MSG_OUT transfers,
//! answers asked for with REQUEST_DEV_DEP_MSG_IN and read until EOM (USBTMC 1.0, 3.2 and 3.3). The protocol is pure
//! and tested here; the USB side is nusb, which needs no libusb and on Windows uses the WinUSB driver the scope's
//! function asks for. The answer's bytes go through the same [`Framer`] as TCP, so the layers above see a line or a
//! block the same way.

use crate::{Endpoint, Error, Framer, Link, Message, MAX_BLOCK};
use std::io::{Read, Write};
use std::time::{Duration, Instant};

const HEADER: usize = 12;
const DEV_DEP_MSG_OUT: u8 = 1;
const REQUEST_DEV_DEP_MSG_IN: u8 = 2;
const DEV_DEP_MSG_IN: u8 = 2;
/// The most asked for in one REQUEST_DEV_DEP_MSG_IN; a longer answer comes in several transfers.
const TRANSFER: u32 = 1 << 20;
/// USBTMC class requests (USBTMC 1.0, table 15) and their status "pending".
const INITIATE_CLEAR: u8 = 5;
const CHECK_CLEAR_STATUS: u8 = 6;
const STATUS_PENDING: u8 = 2;

/// The bulk header: MsgID, bTag, its inverse, then the transfer size and the attributes.
fn header(msg: u8, tag: u8, size: u32, attr: u8) -> [u8; HEADER] {
    let s = size.to_le_bytes();
    [msg, tag, !tag, 0, s[0], s[1], s[2], s[3], attr, 0, 0, 0]
}

/// A DEV_DEP_MSG_OUT transfer with the whole command and EOM, padded to a multiple of 4.
fn out_transfer(tag: u8, data: &[u8]) -> Vec<u8> {
    let mut t = header(DEV_DEP_MSG_OUT, tag, data.len() as u32, 1).to_vec();
    t.extend_from_slice(data);
    t.resize(t.len().next_multiple_of(4), 0);
    t
}

/// The data of one DEV_DEP_MSG_IN transfer answering `tag`, and whether it ends the answer (EOM).
fn in_data(t: &[u8], tag: u8) -> Result<(&[u8], bool), Error> {
    let bad = |what| Error::Malformed { what, got: t.iter().take(16).copied().collect() };
    let h = t.get(..HEADER).ok_or_else(|| bad("a USBTMC transfer shorter than its header"))?;
    if h[0] != DEV_DEP_MSG_IN || h[1] != tag || h[2] != !tag {
        return Err(bad("a USBTMC answer to another request"));
    }
    let size = u32::from_le_bytes([h[4], h[5], h[6], h[7]]) as usize;
    let data = t.get(HEADER..HEADER + size).ok_or_else(|| bad("a USBTMC transfer shorter than it says"))?;
    Ok((data, h[8] & 1 != 0))
}

/// The tags 1..=255 in turn (0 is not a tag).
#[derive(Default)]
struct Tags(u8);

impl Tags {
    fn next(&mut self) -> u8 {
        self.0 = self.0 % 255 + 1;
        self.0
    }
}

/// The whole answer: transfers asked for with `transfer(tag, size)` until one carries EOM, at most `MAX_BLOCK`.
fn answer(tags: &mut Tags, mut transfer: impl FnMut(u8, u32) -> Result<Vec<u8>, Error>) -> Result<Vec<u8>, Error> {
    let mut out = Vec::new();
    loop {
        let tag = tags.next();
        let t = transfer(tag, TRANSFER)?;
        let (data, eom) = in_data(&t, tag)?;
        out.extend_from_slice(data);
        if out.len() > MAX_BLOCK + 64 {
            return Err(Error::Malformed { what: "an answer larger than allowed", got: Vec::new() });
        }
        if eom {
            return Ok(out);
        }
    }
}

/// A USBTMC interface of a device: its number and its bulk endpoints.
struct Found {
    number: u8,
    out_ep: u8,
    in_ep: u8,
}

fn tmc_interface(dev: &nusb::Device) -> Option<Found> {
    let cfg = dev.active_configuration().ok()?;
    cfg.interface_alt_settings().find(|i| i.class() == 0xfe && i.subclass() == 0x03).and_then(|i| {
        let bulk = |dir| {
            i.endpoints()
                .find(|e| e.transfer_type() == nusb::descriptors::TransferType::Bulk && e.direction() == dir)
                .map(|e| e.address())
        };
        Some(Found {
            number: i.interface_number(),
            out_ep: bulk(nusb::transfer::Direction::Out)?,
            in_ep: bulk(nusb::transfer::Direction::In)?,
        })
    })
}

/// The USBTMC instruments on this computer's USB, for [`crate::discover`].
pub fn list() -> Vec<crate::Found> {
    use nusb::MaybeFuture;
    let Ok(devs) = nusb::list_devices().wait() else { return Vec::new() };
    devs.filter(|d| d.interfaces().any(|i| i.class() == 0xfe && i.subclass() == 0x03))
        .map(|d| crate::Found {
            name: d.product_string().unwrap_or("USBTMC instrument").to_string(),
            endpoint: Endpoint::Usb {
                vendor: d.vendor_id(),
                product: d.product_id(),
                serial: d.serial_number().unwrap_or("").to_string(),
            },
        })
        .collect()
}

pub struct Usbtmc {
    iface: nusb::Interface,
    number: u8,
    writer: nusb::io::EndpointWrite<nusb::transfer::Bulk>,
    reader: nusb::io::EndpointRead<nusb::transfer::Bulk>,
    tags: Tags,
    ep: Endpoint,
    framer: Framer,
}

fn usb(e: impl std::fmt::Display) -> Error {
    Error::Io(std::io::Error::other(e.to_string()))
}

impl Usbtmc {
    pub fn open(ep: Endpoint) -> Result<Usbtmc, Error> {
        use nusb::MaybeFuture;
        let Endpoint::Usb { vendor, product, serial } = &ep else {
            return Err(Error::NotSupported("not a USB endpoint"));
        };
        let info = nusb::list_devices()
            .wait()
            .map_err(usb)?
            .find(|d| {
                d.vendor_id() == *vendor
                    && d.product_id() == *product
                    && (serial.is_empty() || d.serial_number() == Some(serial.as_str()))
            })
            .ok_or(Error::Closed)?;
        let dev = info.open().wait().map_err(usb)?;
        let f = tmc_interface(&dev).ok_or(Error::NotSupported("a USB device without a USBTMC interface"))?;
        let iface = dev.detach_and_claim_interface(f.number).wait().map_err(usb)?;
        let writer =
            iface.endpoint::<nusb::transfer::Bulk, nusb::transfer::Out>(f.out_ep).map_err(usb)?.writer(64 << 10);
        let reader = iface.endpoint::<nusb::transfer::Bulk, nusb::transfer::In>(f.in_ep).map_err(usb)?.reader(64 << 10);
        Ok(Usbtmc { iface, number: f.number, writer, reader, tags: Tags::default(), ep, framer: Framer::default() })
    }

    /// One class request on the interface, its answer's first byte (the USBTMC status).
    fn class_in(&self, request: u8, value: u16, length: u16) -> Result<Vec<u8>, Error> {
        use nusb::MaybeFuture;
        let req = nusb::transfer::ControlIn {
            control_type: nusb::transfer::ControlType::Class,
            recipient: nusb::transfer::Recipient::Interface,
            request,
            value,
            index: self.number as u16,
            length,
        };
        self.iface.control_in(req, Duration::from_secs(2)).wait().map_err(usb)
    }
}

impl Link for Usbtmc {
    fn send(&mut self, cmd: &str) -> Result<(), Error> {
        let mut data = cmd.as_bytes().to_vec();
        data.push(b'\n');
        let t = out_transfer(self.tags.next(), &data);
        self.writer.write_all(&t)?;
        self.writer.flush_end()?;
        Ok(())
    }

    fn recv(&mut self, deadline: Instant) -> Result<Message, Error> {
        if let Some(m) = self.framer.take()? {
            return Ok(m);
        }
        let (writer, reader) = (&mut self.writer, &mut self.reader);
        let bytes = answer(&mut self.tags, |tag, size| {
            let left = deadline.saturating_duration_since(Instant::now());
            if left.is_zero() {
                return Err(Error::Timeout);
            }
            writer.write_all(&header(REQUEST_DEV_DEP_MSG_IN, tag, size, 0))?;
            writer.flush_end()?;
            reader.set_read_timeout(left);
            let mut r = reader.until_short_packet();
            let mut t = Vec::new();
            r.read_to_end(&mut t)?;
            r.consume_end().map_err(usb)?;
            Ok(t)
        })?;
        self.framer.push(&bytes);
        self.framer.take()?.ok_or(Error::Malformed { what: "an answer that is not whole", got: bytes })
    }

    fn clear(&mut self) -> Result<(), Error> {
        self.framer.clear();
        self.class_in(INITIATE_CLEAR, 0, 1)?;
        let end = Instant::now() + Duration::from_secs(2);
        while Instant::now() < end {
            let s = self.class_in(CHECK_CLEAR_STATUS, 0, 2)?;
            if s.first() != Some(&STATUS_PENDING) {
                return Ok(());
            }
            std::thread::sleep(Duration::from_millis(20));
        }
        Err(Error::Timeout)
    }

    fn endpoint(&self) -> Endpoint {
        self.ep.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn in_transfer(tag: u8, data: &[u8], eom: bool) -> Vec<u8> {
        let mut t = header(DEV_DEP_MSG_IN, tag, data.len() as u32, eom as u8).to_vec();
        t.extend_from_slice(data);
        t.resize(t.len().next_multiple_of(4), 0);
        t
    }

    #[test]
    fn a_command_is_one_transfer_with_eom_padded_to_four() {
        let t = out_transfer(7, b"*IDN?\n");
        assert_eq!(&t[..HEADER], &[1, 7, 0xf8, 0, 6, 0, 0, 0, 1, 0, 0, 0]);
        assert_eq!((&t[HEADER..HEADER + 6], t.len()), (&b"*IDN?\n"[..], 20));
    }

    #[test]
    fn an_answer_over_several_transfers_ends_at_eom() {
        let mut tags = Tags::default();
        let parts: [(&[u8], bool); 3] = [(b"#15ab", false), (b"cd", false), (b"e\n", true)];
        let mut i = 0;
        let got = answer(&mut tags, |tag, size| {
            assert_eq!(size, TRANSFER);
            let (d, eom) = parts[i];
            i += 1;
            Ok(in_transfer(tag, d, eom))
        })
        .expect("answer");
        assert_eq!(got, b"#15abcde\n");
        let mut f = Framer::default();
        f.push(&got);
        assert_eq!(f.take().expect("frame"), Some(Message::Block(b"abcde".to_vec())));
    }

    #[test]
    fn an_answer_to_another_tag_or_shorter_than_it_says_is_an_error() {
        let mut tags = Tags::default();
        assert!(answer(&mut tags, |tag, _| Ok(in_transfer(tag.wrapping_add(1), b"x\n", true))).is_err());
        let mut short = in_transfer(tags.0 % 255 + 1, b"abcdef\n", true);
        short.truncate(HEADER + 3);
        assert!(answer(&mut tags, |_, _| Ok(short.clone())).is_err());
        assert!(answer(&mut tags, |_, _| Ok(vec![2, 1])).is_err());
    }

    #[test]
    fn tags_skip_zero() {
        let mut t = Tags(254);
        assert_eq!((t.next(), t.next(), t.next()), (255, 1, 2));
    }
}
