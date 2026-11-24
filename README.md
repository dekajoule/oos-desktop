# oos-desktop

Control and view an oscilloscope running [OOS](https://github.com/dekajoule/oos) from Windows, macOS and Linux: the
live records of its channels, Run/Stop/Single, and the settings of channels, time base and trigger, over the network
(SCPI on port 5555). USB (USBTMC) follows once OOS has its USB function.

**State:** a first working version, tested against OOS's simulator (`oos-ui --serve`), **not yet tested on a scope**.
What is built and what is planned: [docs/architecture.md](docs/architecture.md), section 10.

## Use

```sh
cargo run --release -p oos-desktop
```

Type the scope's name or address into the field at the top ([docs/user-guide.md](docs/user-guide.md)). The scope needs
SCPI switched on under System > Network > Remote services.

## Develop

Read [AGENTS.md](AGENTS.md) first: it is the same for people and coding agents. The parts and where a change goes are
in [docs/architecture.md](docs/architecture.md), the decisions in [docs/adr](docs/adr/README.md).

```sh
tools/check.sh                                               # what CI runs
OOS_DIR=../oos tools/sim.sh cargo test -p oos-session --test sim   # against oos-ui --serve
```

## Licence

GPL-2.0-or-later, as OOS. See [LICENSE](LICENSE).
