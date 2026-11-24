# oos-desktop

Control and view an oscilloscope running [OOS](https://github.com/dekajoule/oos) from Windows, macOS and Linux: the
live waveforms of its channels, Run/Stop and Single, the settings of channels, time base, trigger and acquisition,
cursors, the scope's measurements, screenshots and records as files; over the network (SCPI on port 5555) or a USB
cable to the scope's rear port (USBTMC).

**State:** it has run against a DHO924S with OOS over the network and over USB from a Mac, and in CI on Windows, macOS
and Linux; USB from Windows and Linux is not yet tried with it. What is built and what is planned:
[docs/architecture.md](docs/architecture.md), section 10.

## Use

**The [wiki](https://github.com/dekajoule/oos-desktop/wiki)** is the user guide: installing, connecting, the window,
troubleshooting. In short:

```sh
cargo run --release -p oos-desktop
```

then **Find**, or the scope's name or address in the field at the top. The scope needs SCPI (network) or USB
instrument (USB cable) switched on under System > Network > Remote services.

## Develop

Read [AGENTS.md](AGENTS.md) first: it is the same for people and coding agents. The parts and where a change goes are
in [docs/architecture.md](docs/architecture.md), the decisions in [docs/adr](docs/adr/README.md).

```sh
tools/check.sh                                               # what CI runs
OOS_DIR=../oos tools/sim.sh cargo test -p oos-session --test sim   # against oos-ui --serve
```

## Licence

GPL-2.0-or-later, as OOS. See [LICENSE](LICENSE).
