# User guide

## Connecting

Type the scope's address into the field at the top and press Enter or **Connect**: its name (`dho924s-123456.local`,
shown on the scope under System > Network > Remote services), its IP address, or a VISA-like resource string
(`TCPIP::<host>::5555::SOCKET`). The scope must have **SCPI** switched on in Remote services, and the PC must be in its
**Allowed** list if one is set. The bar shows the model and the OOS version once connected.

## The window

- **Run**, **Stop** and **Single** at the top do what the keys on the scope do.
- The waveform in the middle shows the record of each channel on the scope's screen.
- The panels on the right (**Channels**, **Time base**, **Trigger**) show the scope's settings and change them. A dot
  beside a value means it was sent and the scope has not answered it yet; a value the scope refuses goes back to the
  scope's value, and the reason is shown at the bottom.

The program remembers the last address.
