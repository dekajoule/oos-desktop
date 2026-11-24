# Installation

There are no ready-made downloads yet. oos-desktop is built from its source with Rust, which takes a few minutes the
first time.

## 1. Rust

Install Rust with [rustup](https://rustup.rs) (version 1.92 or newer; `rustup update` brings an older one up to date).

- **Windows:** rustup asks for the Visual Studio Build Tools ("Desktop development with C++"); install them when it
  asks.
- **macOS:** the Xcode command line tools (`xcode-select --install`).
- **Linux:** a C compiler (`build-essential` or `gcc`) and the usual desktop libraries, which a desktop with X11 or
  Wayland has. Saving files uses the desktop's file chooser portal (`xdg-desktop-portal`).

## 2. Build and start

```sh
git clone https://github.com/dekajoule/oos-desktop.git
cd oos-desktop
cargo run --release -p oos-desktop
```

`cargo run --release -p oos-desktop -- <address>` connects at the start, with any address the connect field takes
([[Connecting]]). The program itself is `target/release/oos-desktop` (`oos-desktop.exe` on Windows) and can be started
from there next time.

## 3. Update

```sh
git pull
cargo run --release -p oos-desktop
```

The program remembers the last address, the panel and the measurements chosen.
