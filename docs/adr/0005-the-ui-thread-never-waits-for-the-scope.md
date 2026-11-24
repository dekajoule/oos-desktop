# ADR 0005: The UI thread never waits for the scope

- Status: proposed
- Date: 2026-11-24

## Context

A query to a scope takes from a millisecond (a setting on the local network) to seconds (a deep record over USB). egui
draws every frame from the UI thread; a frame that waits for the scope freezes the window, and a lost link would hang
it.

## Decision

- All I/O runs in the session's worker thread. The UI sends `Action`s through a channel and takes `Event`s at the start
  of each frame; the worker asks for a new frame after each event.
- `app/` does no I/O of its own: no sockets, no USB, no files except through eframe's storage and the save dialog's
  result handed to the worker.
- A control shows the value the scope confirmed, and the value being sent while it waits.

## Alternatives

- **async with a runtime:** the work is one connection with a strict order of commands; a thread and two channels say
  that plainly and need no runtime.
- **Blocking calls with short timeouts:** still freezes the window for the length of the timeout.

## Consequences

- Fast knob turns are merged in the worker (the newest value of a key wins), so the scope gets one set, not twenty.
- The worker is the one place that orders commands, which makes `*OPC?` and the error queue easy to keep right.
