# Installation

## Download

The [releases](https://github.com/dekajoule/oos-desktop/releases) have the program ready to start, for Windows (x86_64),
macOS (Apple silicon and Intel in one file) and Linux (x86_64). `SHA256SUMS` beside them has their checksums.

- **Windows:** unpack the `.zip` and start `oos-desktop.exe`. The program is not signed: when SmartScreen warns, choose
  **More info** > **Run anyway**.
- **macOS:** unpack the `.tar.gz` (a double click) and start `oos-desktop` from the Terminal. The program is not
  signed: macOS refuses it the first time; allow it under System Settings > Privacy & Security (**Open Anyway**), or
  once in the Terminal: `xattr -d com.apple.quarantine oos-desktop`.
- **Linux:** `tar -xzf oos-desktop-*-linux-x86_64.tar.gz` and start `./oos-desktop`. It needs a desktop with X11 or
  Wayland; saving files uses the desktop's file chooser portal (`xdg-desktop-portal`).

`oos-desktop <address>` connects at the start ([[Connecting]]).

## From the source

Or build it yourself with Rust, which takes a few minutes the first time.

### 1. Rust

Install Rust with [rustup](https://rustup.rs) (version 1.92 or newer; `rustup update` brings an older one up to date).

- **Windows:** rustup asks for the Visual Studio Build Tools ("Desktop development with C++"); install them when it
  asks.
- **macOS:** the Xcode command line tools (`xcode-select --install`).
- **Linux:** a C compiler (`build-essential` or `gcc`) and the usual desktop libraries, which a desktop with X11 or
  Wayland has. Saving files uses the desktop's file chooser portal (`xdg-desktop-portal`).

### 2. Build and start

```sh
git clone https://github.com/dekajoule/oos-desktop.git
cd oos-desktop
cargo run --release -p oos-desktop
```

`cargo run --release -p oos-desktop -- <address>` connects at the start, with any address the connect field takes
([[Connecting]]). The program itself is `target/release/oos-desktop` (`oos-desktop.exe` on Windows) and can be started
from there next time.

### 3. Update

```sh
git pull
cargo run --release -p oos-desktop
```

The program remembers the last address, the panel and the measurements chosen.
