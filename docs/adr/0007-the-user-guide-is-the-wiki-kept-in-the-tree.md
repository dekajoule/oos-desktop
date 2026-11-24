# ADR 0007: The user guide is the wiki, kept in the tree

- Status: proposed
- Date: 2026-11-24

## Context

People who use the program look for help on the repository's wiki; OOS's wiki links there. A page edited on the wiki
itself is not reviewed and drifts from the program; a second copy of the same text in `docs/` drifts from the first.

## Decision

- What a user reads lives in `wiki/`, one page per topic: installing, connecting, the window, troubleshooting.
  `.github/workflows/wiki.yml` publishes the folder to the repository's wiki when a change to it reaches main.
- `docs/` is for developers: the architecture and the decisions. A fact has one home; the other place links to it.
- A change a user sees brings its wiki page along in the same pull request (AGENTS.md).
- The wiki is never edited on GitHub: the next publish overwrites it.

## Alternatives

- **A user guide in `docs/`:** readable in the repository, but not where users look, and OOS's wiki could only link
  into the source tree.
- **Editing the wiki on GitHub:** quick, but outside review and CI.

## Consequences

- `docs/user-guide.md` goes; its content is the wiki's pages.
- The same pattern as OOS (its ADR 0035), so the two wikis link to each other plainly.
