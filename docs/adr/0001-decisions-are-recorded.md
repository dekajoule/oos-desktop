# ADR 0001: Decisions that others build on are recorded here

- Status: proposed
- Date: 2026-11-24

## Context

The program is written by people and by coding agents, often in short sessions without the history of earlier ones.
A rule that lives only in a review comment is lost; a rule without its reason is broken the first time it is in the
way.

## Decision

- A decision that others build on (a part, a direction of calls, a format, a dependency, a protocol detail) is a
  record in `docs/adr/`, numbered, from [template.md](template.md), with its line in [README.md](README.md).
- A new record comes in as `proposed`; the maintainer sets `accepted`.
- A change of direction is a new record that supersedes the old one; the old one keeps its text and gets
  `superseded by ADR NNNN`.
- `AGENTS.md` is the short form for daily work; where it and a record differ, the record holds.

## Alternatives

- **Decisions in the wiki or in issues:** not versioned with the code they govern.

## Consequences

- A pull request that decides something brings its record; review can point to it.
