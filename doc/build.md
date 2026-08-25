# Building MCF

| | |
|---|---|
| **Type** | Reference — the workspace, the toolchain, and the checks that gate a change |
| **Version** | 1 |
| **Status** | Living |
| **Authority** | Derived from [document-of-intent.md](document-of-intent.md) v24, governed by [rules.md](rules.md) |
| **Registers to** | B-001 in [backlog.md](backlog.md) |

**One command builds it and one command gates it.** `cargo build --locked`
produces the binary; `scripts/ci.sh` decides whether a change may land. Read
this before adding a crate, a dependency or a check — each of those changes
something the workspace asserts about itself, and the assertion is a test that
will fail rather than a convention somebody will notice.

Everything here is M0 (B-001). MCF at this commit builds, reports what built
it, and claims nothing else; `mcf doctor` — the milestone's product — arrives
with B-014.

## Contents

| § | Section |
|---|---|
| 1 | [The toolchain](#1--the-toolchain) |
| 2 | [The crate split](#2--the-crate-split) |
| 3 | [Building](#3--building) |
| 4 | [The gating tier](#4--the-gating-tier) |
| 5 | [Reproducibility](#5--reproducibility) |
| 6 | [Dependencies](#6--dependencies) |
| — | [Changelog](#changelog) |

## 1 · The toolchain

`rust-toolchain.toml` pins an **exact** version, not a channel. §3.12 makes
reproducibility a precedence rule (P3) and §3.4 makes MCF's own version a
condition of every measurement it takes; the compiler is part of both, so
`stable` would quietly build a different instrument every six weeks. The pin
and the workspace's `rust-version` are asserted equal, because a declared
minimum nothing has ever built with is an untested claim (A19).

`rustup` reads the file and installs the pinned toolchain on its own. Nothing
else is required to build: no interpreter, no system libraries, no vendored
tree yet.

Raising the pin is its own commit. It invalidates byte-identical comparison
with everything built before it, and §5 of this file is re-run as part of
making it.

## 2 · The crate split

The split is a **boundary**, not packaging. §3.16 asks that the substrate
enforce the rules rather than that reviewers remember them, and the strongest
form of that above the type level is an edge that does not exist: `mcf-core`
cannot depend on the record store, so no convenience in the record store can
reach back and weaken a type in `mcf-core`.

| Crate | Holds | Depends on |
|---|---|---|
| `mcf-core` | The types every rule is enforced through: failure (B-003), `Measurement` (B-005), `Provenance` (B-006), the time model (B-184), build identity | — |
| `mcf-record` | The journal, and the index derived from it (D20, D6) | `mcf-core` |
| `mcf-lab` | Simulated clock, injected faults, replayable scenarios (§3.17) | `mcf-core`, `mcf-record` |
| `mcf-hub` | Resolving, fetching and pinning artifacts (§III) | `mcf-core`, `mcf-record` |
| `mcf-serve` | The daemon, engine adapters, the serving surface (§VI) | `mcf-core`, `mcf-record` |
| `mcf-bench` | Measurement, and the laboratories that produce it (§II, §XIII) | `mcf-core`, `mcf-record`, `mcf-serve` |
| `mcf-cli` | The headless surface; binary `mcf` (A22) | all of the above |
| `mcf-checks` | Workspace-shape checks. Ships nothing, and nothing depends on it | — |

The table is a rendering of `checks/src/workspace.rs`, which is the
declaration the tests compare the repository against. Both directions are
checked: an **added** edge inverts the layering, and a **missing** one means
the declaration has drifted and can no longer be read as the architecture.

`mcf-bench` depends on `mcf-serve` and not the reverse, so no serving path can
acquire a dependency on the benchmark harness — A18's separation of tests from
benchmarks, drawn in the dependency graph.

## 3 · Building

```
$ cargo build --locked                  # debug
$ cargo build --locked --release        # the artifact D24's budgets govern
$ ./target/release/mcf --version
MCF 0.1.0-m0  (revision unknown, rustc 1.98.0 (88d9e12ae 2026-08-18), target x86_64-unknown-linux-gnu, profile release)
```

`revision unknown` is correct, not a defect. The source revision reaches the
binary through `MCF_BUILD_COMMIT`, set by whatever performs the build, and is
`SourceRevision::Unknown` when nothing set it — A7 forbids filling an unknown
with a plausible value, and B36 forbids requiring `git` to be installed in
order to compile. A release build sets it:

```
$ MCF_BUILD_COMMIT=$(git rev-parse HEAD) cargo build --locked --release
```

**The release profile is part of what is measured.** It aborts on panic, keeps
overflow checks on, and enables fat LTO with one codegen unit. A panic is an
`internal.invariant_violated` the type system was meant to prevent, and
unwinding past one invites the catch-and-continue A2 calls worse than a crash.
Overflow checks stay on because an arithmetic overflow that wraps is a wrong
number, and P1 puts honesty above continuity; the branch they cost is
something D24's budgets are measured against rather than assumed to be free.

Cargo forces the test profile to unwind whatever the dev profile says, so the
suite and the shipped artifact genuinely differ in that one respect. It is why
B-011's budget suite measures the release binary rather than the test one.

## 4 · The gating tier

```
$ scripts/ci.sh
```

Formatting, `clippy -D warnings` over every target, the test suite, and
`cargo doc` with warnings denied. It passes `--offline` rather than merely
expecting no network, so a check that starts reaching out fails here instead of
on an aeroplane (B19).

B38 tiers the suite, and this is only the fast hermetic tier. The heavy tiers —
load, soak, mutation, the full fault matrix — do not exist yet (B-191, B-185,
B-186), and `ci.sh` prints their absence rather than passing silently: "did not
run" read as "passed" is A2's silent failure aimed at the suite.

**Lints are the machine-checked form of rules that would otherwise rest on
review** (B16). The workspace denies `unsafe_code`, `missing_docs`, all of
clippy's `pedantic` set, and the panicking constructs B-003 will replace —
`unwrap`, `expect`, `panic!`, `todo!`, `unimplemented!`, bare indexing. They
are denied from the first commit so that no code is written under the old habit
and migrated later. `clippy.toml` exempts tests, where an assertion that fails
loudly is the honest outcome, and `crates/mcf-core/build.rs` carries the one
documented opt-out: a build script has no MCF failure type available to it and
no record to write to.

## 5 · Reproducibility

```
$ scripts/ci.sh --with-reproducibility     # or scripts/check-reproducible-build.sh
```

B-001's acceptance condition is that `cargo build --locked` reproduces
byte-identically from a clean checkout on the pinned toolchain, and A19 says an
unchecked claim is not one. The script checks it: two checkouts of the
committed tree, at paths of different lengths, with different
`SOURCE_DATE_EPOCH`, `TZ`, umask and target directory, built and compared byte
for byte.

Held still are the things that are genuinely conditions of the artifact — the
toolchain, the lockfile, `MCF_BUILD_COMMIT` — because a build that changed when
those changed would be correct to change.

It is out of the gating tier because two release builds take minutes; it is run
before a release and on demand. Its exit status distinguishes *diverged* (1)
from *could not be checked* (2), since those are different answers.

**`--remap-path-prefix` is not set, and that is a finding rather than an
omission.** Built from the workspace root with no registry dependencies, cargo
passes rustc relative source paths, so no absolute path from the build machine
reaches the binary — the check above is what establishes that, empirically. The
first vendored dependency (§XVI, B64) will need
`--remap-path-prefix=$CARGO_HOME=/cargo`, and this check is what will say so.

## 6 · Dependencies

**There are none.** The workspace compiles from the standard library alone, and
a test asserts it: `the_workspace_declares_no_third_party_dependencies`.

That test is the gate B15 asks for. Weight is admitted only against a stated
cost, so the first dependency admitted has to be added to that check to get in,
and the reason it justifies itself against not existing is recorded in the
commit that admits it. This is not an ambition to have no dependencies — §XVI
requires vendoring an entire inference stack (D23, B64), which is a great deal
of weight admitted deliberately. It is the mechanism that keeps the admission
deliberate.

## Changelog

### Version 1 — the workspace exists

Created with the workspace itself (B-001). The repository had held only intent,
rules, a plan and a register; the moment there is code there is a build, and a
build has conditions — a pinned compiler, a layering, a set of lints, a
definition of what "green" means — that belong in a document rather than in the
memory of whoever set them up.

Written as reference rather than instructions: each section states what the
build asserts about itself and which clause makes that worth asserting, because
a build convention nobody can cite is a convention that erodes.
