# ADR 0002: SCPI is the only way to the scope, and the scope measures

- Status: proposed
- Date: 2026-11-24

## Context

OOS offers SCPI (TCP 5555, VXI-11, HiSLIP, and USBTMC over the USB cable), a web remote, MCP and SSH. All SCPI
transports end in one interpreter, and OOS keeps that interface documented and compared from release to release
(OOS ADR 0022, ADR 0023). The web pages and the shell are not a stable interface for programs. The scope already
measures, decodes and computes math on its own records.

## Decision

- The program talks to the scope through SCPI only, over TCP port 5555 or USBTMC. It uses the commands of OOS's
  `docs/scpi.md` at the commit in `OOS_REV`, and nothing else of the scope.
- It shows what the scope answers. It computes only what the display needs (reducing a record to the screen, grid,
  units, cursors in the view); measurements, decoding and math come from the scope.

## Alternatives

- **Reading the web remote's JSON or pictures:** faster to start, but those routes change with the web page.
- **VXI-11 or HiSLIP as the network transport:** VISA programs need them; this program does not, and the raw socket
  is simpler and has no portmapper.
- **Measuring on the PC:** a second implementation that would disagree with the scope's own values.

## Consequences

- Whatever the program can do, a script can do with the same commands; missing commands are added to OOS, not
  worked around here.
- `tools/scpi-used.py` fails when the program uses a header that OOS's `docs/scpi.md` at `OOS_REV` does not list.
