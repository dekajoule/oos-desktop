# User guide

## Connecting

**Find** asks the network for scopes running OOS (about two seconds) and looks at this computer's USB; **Found** then
lists them, and a click connects. On the network the scope needs SCPI and mDNS on under System > Network > Remote
services; over a USB cable to its rear port it needs **USB instrument** on there. Windows and macOS need no driver;
Linux needs read and write access to the device (a udev rule, see OOS's docs/usb-device.md).

Or type the scope's address into the field at the top and press Enter or **Connect**: its name (`dho924s-123456.local`,
shown on the scope under System > Network > Remote services), its IP address, or a VISA-like resource string
(`TCPIP::<host>::5555::SOCKET`). The scope must have **SCPI** switched on in Remote services, and the PC must be in its
**Allowed** list if one is set. The bar shows the model and the OOS version once connected.

## The window

- **Run**, **Stop** and **Single** at the top do what the keys on the scope do.
- **Screenshot…** saves the scope's screen as PNG; **Save record…** saves the records of the channels that are on as
  one CSV file (time in seconds, then volts per channel). Both ask where; the bottom line says when the file is
  written.
- The waveform in the middle shows the record of each channel on the scope's screen.
- The panels on the right (**Channels**, **Time base**, **Trigger**) show the scope's settings and change them. A dot
  beside a value means it was sent and the scope has not answered it yet; a value the scope refuses goes back to the
  scope's value, and the reason is shown at the bottom.
- **Measure** shows the measurements the scope makes of each channel that is on, twice a second while the panel is
  open. **Choose** below picks which; a dash means the scope has no value on this record (too few periods, say).

The program remembers the last address.
