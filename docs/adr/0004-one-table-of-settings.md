# ADR 0004: One table of settings

- Status: proposed
- Date: 2026-11-24

## Context

Most of what the program changes on a scope is a setting with a set and a query of the same SCPI header: a channel's
scale, the time base, the trigger level. Written by hand in each panel, every setting needs the header twice, a parser,
a range and a widget, and each copy can differ.

## Decision

- `crates/scpi/src/commands.rs` holds one row per setting: a stable key, the SCPI header (with `{n}` for a channel),
  its kind (`Float` with unit and range, `Enum` with its words, `Bool`, `Int`), and the channels it repeats for.
- Sets, queries, parsing, the merging of fast changes in `session` and the widgets in `app` all work from the row.
- A panel lists the keys it shows. A SCPI header appears nowhere outside `crates/scpi/`.
- The key is stored in saved layouts and never changes once released.

## Alternatives

- **A struct field per setting:** typed, but every new setting touches the struct, the parser, the session and a
  panel.
- **Generating the table from OOS's docs/scpi.md:** the document is written for people; the ranges and kinds would
  still need hand-written rules.

## Consequences

- A test (`every_row_is_complete`) checks each row: a header, a kind with a sensible range, a unique key.
- Queries that are not settings (waveforms, screenshots, measurements) stay methods of `Scope`.
