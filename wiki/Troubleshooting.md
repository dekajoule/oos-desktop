# Troubleshooting

**Find finds nothing.** The scope needs **mDNS** and **SCPI** on (System > Network > Remote services), and the
computer must be on the same network segment: mDNS does not pass routers. Type the scope's IP address instead; the
scope shows it under Remote services.

**It connects, then "lost".** The scope's **Allowed** list may not include the computer, or **SCPI** was switched
off. A scope that was switched off or restarted needs a new **Connect**.

**A value jumps back.** The scope refused it; the status line says why (the range of a setting depends on others,
such as the offset on the scale).

**USB: the scope is not listed.**
- **USB instrument** must be on, on the scope.
- Windows: switch **USB network** off; with both on, Windows sees only the network
  ([OOS #570](https://github.com/dekajoule/oos/issues/570)).
- Linux: the udev rule of [[Connecting]], then plug the cable in again.
- Another program (pyvisa, a VISA tool) may have the scope's USB interface: close it.

**Saving does nothing on Linux.** The save dialog comes from the desktop's file chooser portal; install
`xdg-desktop-portal` and the one of your desktop (`-gtk`, `-kde`, ...).

Something else? Open an issue at [oos-desktop](https://github.com/dekajoule/oos-desktop/issues) with the system, the
OOS version (the status line shows it) and what happened.
