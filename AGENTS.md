# Instructions for coding agents (and for people)

oos-desktop controls an oscilloscope running [OOS](https://github.com/dekajoule/oos) from Windows, macOS and Linux,
over the network or a USB cable, and shows what it records. These instructions are the same for every agent and every
person; the reasons are in the decision records ([docs/adr](docs/adr/README.md)). Where this file and an ADR differ,
the ADR holds.

## Read first

1. [docs/adr/README.md](docs/adr/README.md): the decisions, one line each. Open the records that touch your change.
2. [docs/architecture.md](docs/architecture.md): the parts, the direction they call in, and where a change goes.
3. The OOS remote interface you use: [docs/scpi.md](https://github.com/dekajoule/oos/blob/main/docs/scpi.md) of OOS at
   the commit in `OOS_REV`.

## Never

- **Reach the scope other than through SCPI** (`crates/link` and `crates/scpi`): no web pages of the scope, no SSH, no
  files copied from it (ADR 0002).
- **Wait for the scope on the UI thread**, and no I/O in `app/` at all: panels return `Action`s, the session worker
  does the talking (ADR 0005).
- **Write a SCPI header outside `crates/scpi/`.** A setting is a row of `commands.rs`, a query a method of `Scope`
  (ADR 0004).
- **Compute in the program what the scope measures.** The program shows the scope's answers; it computes only what the
  display needs (ADR 0002).
- **Add a dependency** without its reason in the pull request and its line in `tools/deps-allowed.txt`; licences must
  be compatible with GPL-2.0-or-later (`tools/check.sh deps`).
- **Put personal data** (serial numbers, MAC or IP addresses of real scopes, host names, user names) into code, tests,
  docs, logs or commits. Tests use made-up identities such as `dho924s-000000`.
- **Bring vendor material into the tree**: no code, binaries, icons or texts of the scope maker's software.
- **Push to main.** Work on a branch and open a pull request; it is merged squashed when CI is green.

## Always

- Keep the direction of the parts: `app` → `session` → `scpi` → `link`, never upwards, `app` not past `session`
  (ADR 0003). `tools/check.sh layers` fails otherwise.
- One place to declare each kind of extension, and a test that names what is missing (table below).
- Every answer of a scope is untrusted input: parse it without panicking, bound every length and every wait, and give
  a malformed answer a test.
- A change that is not meant to change behaviour says `Behaviour: unchanged` in the pull request, and the tests stay
  as they were. A change a user sees gets a line in `CHANGELOG.md` under "Unreleased".
- Reuse before you add: look for code that already does the job and call it. Remove what your change leaves unused in
  the same pull request.
- Docs change in the same pull request as the code they describe: `docs/architecture.md` when a part, a thread or a
  direction changes; `docs/user-guide.md` when a user sees it; a new ADR (status `proposed`) for a decision others
  build on; a change of direction supersedes a record, it does not edit it.
- State how a fact about the scope is known: **tested against `oos-ui --serve`**, **tested on a scope**, or **not yet
  tested on a scope**. Reports of your own work say what ran where.

## Adding something

| Kind | Declare it in | Also touch | The check that fails when a part is missing |
|---|---|---|---|
| setting | a row in `crates/scpi/src/commands.rs` | its key in a panel's list, an exchange test if its answer is unusual | `crates/scpi/src/commands.rs::every_row_is_complete`, `tools/scpi-used.py` |
| measurement | a row in `crates/scpi/src/measures.rs` | nothing else: the measurement panel offers every row | `crates/scpi/src/measures.rs::every_measure_is_complete` |
| query | a method of `Scope` in `crates/scpi/src/scope.rs` | an exchange test, a `Kind` and an `Event` in `session` if it is polled | `tools/scpi-used.py` |
| panel | its line in `PANELS` (`app/src/panels.rs`), a module of its own once it draws more than rows | `docs/user-guide.md` | `app/src/panels.rs::every_panel_is_complete` |
| transport | a module in `crates/link/src/` implementing `Link`, an `Endpoint` variant | `discover()`, its resource string, an exchange test | `crates/link/src/endpoint.rs::every_endpoint_round_trips` |
| dependency | `Cargo.toml` of the crate | its line in `tools/deps-allowed.txt` with the reason | `tools/check.sh deps` |

## Build and check

```sh
tools/check.sh            # every layer CI runs: fmt clippy test layers deps scpi text
tools/check.sh test       # one layer
cargo run -p oos-desktop  # the program
OOS_DIR=../oos tools/sim.sh   # oos-ui --serve of OOS_REV on port 55555, by hand
OOS_DIR=../oos tools/sim.sh cargo test -p oos-session --test sim   # the contract test
```

Run `tools/check.sh` before you say a change is done, and say which layers ran. A compiler warning fails it. Rust code
is laid out by rustfmt with the tree's `rustfmt.toml`. Clippy findings in code you touch are fixed, or allowed at the
place with a reason.

## Style

English in code, comments, docs and commits. Lines of 120 columns. Files LF, UTF-8, a final newline, no trailing
spaces, an `SPDX-License-Identifier: GPL-2.0-or-later` line in every source file. Comments say why, not what. Commit
subjects `Area: what changed, as a statement` (`Link: a block longer than announced is an error`), one topic per
commit, the body says why and how it was tested; every commit is signed off (`git commit -s`). One topic per pull
request, about 400 changed lines.
