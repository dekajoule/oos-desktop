# Connecting

## Find

**Find** at the top asks the network for scopes running OOS (about two seconds) and looks at the computer's USB;
**Found** then lists them, and a click connects. A scope can show more than once: on its network cable, on the USB
cable as a network, and on the USB cable as an instrument. Each works.

## By address

Type into the field at the top and press Enter or **Connect**:

- the scope's name, `dho924s-123456.local` (System > Network > Remote services shows it on the scope),
- its IP address, `192.0.2.7`,
- or a VISA-like resource string: `TCPIP::<host>::5555::SOCKET`, `USB::0x1209::0x0001::<serial>::INSTR`.

The bar shows the model and the OOS version once connected; the dot is green while the connection holds.

## What the scope needs

On the scope, under **System > Network > Remote services**:

| Way | Switch on | Also |
|---|---|---|
| Network | **SCPI** (and **mDNS**, for Find and the name) | if **Allowed** lists addresses, the computer's must be among them |
| USB cable, as an instrument | **USB instrument** | Windows: **USB network** off (below) |
| USB cable, as a network | **USB network** | then the scope is at `192.168.235.1`, as on the network |

The USB functions are in OOS from its next release on and are off by default ([OOS: Remote
control](https://github.com/dekajoule/oos/wiki/Remote-Control)).

## USB on each system

- **macOS:** nothing to install. Tried with this program.
- **Windows 10 and 11:** nothing to install: the scope asks Windows for its WinUSB driver. **With USB network on as
  well, Windows sees only the network** and not the instrument: switch USB network off to use the instrument on
  Windows ([OOS #570](https://github.com/dekajoule/oos/issues/570)). Not yet tried with this program.
- **Linux:** the program needs read and write access to the scope's USB device. Once, as root, a udev rule:

  ```sh
  echo 'SUBSYSTEM=="usb", ATTR{idVendor}=="1209", ATTR{idProduct}=="0001", MODE="0660", TAG+="uaccess"' \
    | sudo tee /etc/udev/rules.d/70-oos.rules
  sudo udevadm control --reload
  ```

  then plug the cable in again. Linux binds its own `usbtmc` driver to the scope; oos-desktop takes the interface over
  while it is connected. Not yet tried with this program.

One program at a time can use the scope over USB: close pyvisa or another program first.
