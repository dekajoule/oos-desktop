# ADR 0006: Rust and egui, one binary per system

- Status: proposed
- Date: 2026-11-24

## Context

The program runs on Windows, macOS and Linux. Users should not need a runtime, a VISA installation or a USB driver.
OOS is written in Rust, and its people know it.

## Decision

- Rust (stable) and egui through eframe for the window; one binary per system, no runtime.
- USB through nusb, which needs no libusb; the scope's USBTMC function announces WinUSB to Windows, so no driver is
  installed.
- Few dependencies, each with its reason in `tools/deps-allowed.txt` and a licence compatible with GPL-2.0-or-later.

## Alternatives

- **Python with Qt and pyvisa:** quick to write, but a large installer, and VISA or a driver tool on Windows.
- **Tauri (Rust with a web front end):** could share code with the web remote, but brings a browser engine and a
  second language.

## Consequences

- An immediate-mode UI: the window is drawn from the `Model` every frame, which suits a live waveform and keeps state
  in one place.
- Native look is traded for one look on all three systems, close to the scope's own.
