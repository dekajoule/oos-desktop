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

- **The top bar** holds the connection (the dot is green while connected), the run controls and the tools. **Running**
  / **Stopped** is the scope's Run/Stop key: green while it acquires, red when it stopped; a click switches.
  **Single** takes one record on the next trigger, the flash forces one without a trigger, **Auto** lets the scope set
  channels, time base and trigger to the signals, the crossed circle clears persistence and statistics. On the right,
  the badge shows the trigger's state as the scope reports it: **T'D** (triggered), **AUTO**, **WAIT** or **STOP**.
- **The waveform** shows the record of each channel that is on, placed as on the scope's screen: ten divisions of time
  with their times below, eight of level. The tags on the left mark each channel's ground, the **T** on the right the
  trigger level, the triangle at the top the trigger time. Under the pointer the time and each channel's value there
  are shown.
- **Cursors:** the two buttons with lines switch time cursors (two vertical lines: their times, ΔT and 1/ΔT) or level
  cursors (two horizontal lines: the volts of the channel chosen in the side panel, and ΔV). Drag a line to move it;
  the cursors keep their place on the screen when a scale changes.
- **The strip** below the waveform has a card for each channel, the time base, the acquisition (sample rate, memory
  depth, mode) and the trigger. A click on a channel's card switches it on or off; the mouse wheel over a card turns
  its knob: V/div, s/div in 1-2-5 steps, or the trigger level in fifths of a division.
- **The measurements** chosen in the **Measure** panel are shown below the waveform for every channel that is on,
  twice a second; a dash means the scope has no value on this record (too few periods, say).
- **The panels** on the right (**Channels**, **Horizontal**, **Trigger**, **Acquire**, **Measure**) show the scope's
  settings and change them: scale (with buttons for the steps of the knob), offset, coupling, probe, bandwidth limit
  and invert of each channel; scale, delay, expand point and auto roll of the time base; sweep, source, level, slope,
  coupling, holdoff and noise reject of the trigger; mode and averages of the acquisition. A dot beside a value means
  it was sent and the scope has not answered it yet; a value the scope refuses goes back to the scope's value, and the
  reason is shown in the status line (× dismisses it).
- **The camera** saves the scope's screen as PNG, the arrow the records of the channels that are on as one CSV file
  (time in seconds, then volts per channel). Both ask where; the status line says when the file is written.
- **The status line** shows the model, the OOS version and the address, the last message, and how many records a
  second arrive.

## Keys

| Key | Does |
|---|---|
| Space | Run/Stop |
| S | Single |
| F | Force a record |
| C | Clear |

The keys act while no text field has the focus.

## Starting

`oos-desktop <address>` connects at the start, with any address the field takes. The program remembers the last
address, the panel and the measurements chosen.
