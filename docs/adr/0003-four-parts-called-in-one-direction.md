# ADR 0003: Four parts, called in one direction

- Status: proposed
- Date: 2026-11-24

## Context

A desktop program for an instrument mixes four concerns: moving bytes (two transports), the instrument's language,
keeping a connection busy without blocking, and drawing. Mixed together they make each change touch everything, and
make the parts impossible to test without a scope and a window.

## Decision

- Four crates: `link` (bytes and framing, one module per transport), `scpi` (the scope's language, pure), `session`
  (one worker thread per connection, the poll plan), `app` (egui, panels, drawing).
- Calls go down only: `app` → `session` → `scpi` → `link`; `session` may also use `link` (discovery, opening).
  `app` uses no crate below `session`; what it needs of their types, `session` re-exports.
- `tools/check.sh layers` reads the crates' dependencies and fails on any other edge.

## Alternatives

- **One crate with modules:** the direction would rest on discipline alone.
- **More crates (one per transport, one for the model):** more ceremony than the size of the program needs today; a
  part that grows can be split later without changing the direction.

## Consequences

- `link`, `scpi` and `session` are tested without a window, and the first two without threads.
- A command-line tool or a second front end can be built on `session` alone.
