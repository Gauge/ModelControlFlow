# Building MCF

| | |
|---|---|
| **Type** | Reference — the workspace, the toolchain, and the checks that gate a change |
| **Version** | 6 |
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
| 7 | [Generated code](#7--generated-code) |
| 8 | [The documents](#8--the-documents) |
| 9 | [The scheduled tiers](#9--the-scheduled-tiers) |
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
| `mcf-core` | The types every rule is enforced through — failure (B-003), `Measurement` (B-005), `Provenance` (B-006), the time model (B-184), configuration identity (B-272), degradation (B-008), build identity — and the machine profiler those types describe (B-013) | — |
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

**One module opts out of `unsafe_code`, and it is the one D4 predicted.**
`crates/mcf-core/src/hardware/nvml.rs` loads the vendor's management library at
runtime and calls it over the C ABI, because F1 established that a device's live
state — available memory, temperature — is reachable no other way, and D25 makes
those readings the difference between a characterized device and an
uncharacterized one. The workspace denies `unsafe_code` as a `deny` rather than
a `forbid` precisely so that this opt-in is possible with its reason written at
the site. Every pointer is null-checked, every status code is checked before its
out-parameter is read, and the library is closed on every path out.

**A lint table is itself a claim, so it is checked.**
`scripts/check-lints-bite.sh` writes each forbidden construct into a copy of
the workspace and requires the build to refuse it — ten constructs, plus a
negative control (`clippy::dbg_macro`, deliberately not enabled) that must
*not* be reported. Without the control, a probe that failed to compile or a
grep that never matched would read as ten clean refusals, which is the
vacuously green suite A19 exists to prevent. It runs in the gating tier and
takes about twelve seconds. A test in `checks/tests/workspace_shape.rs` asserts
separately that the table still claims each lint, because a lint quietly
deleted from the manifest would leave that script checking a shorter list and
still reporting every entry refused.

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

**The first candidate was refused, and the reasoning is the template.** The
record needs a serialization (§3.3, D20), and a general one was the obvious
dependency. It was not admitted. The data model is closed — MCF's own records,
no user-defined shapes, no schema anyone else supplies — so a framework would
bring derive macros, a trait hierarchy and a compile-time cost for a generality
the format will never use, and it would put a third party in charge of an
interface §7.30 makes MCF's to keep stable for ever. What it would have bought
is correctness, and correctness here is a testable property of about three
hundred lines, so it was bought with tests instead (A19).
`crates/mcf-record/src/json.rs` is the result, and its refusals are the
interesting part: a fraction is refused rather than rounded, a duplicate key
rather than resolved, an unpaired surrogate rather than replaced.

That test is the gate B15 asks for. Weight is admitted only against a stated
cost, so the first dependency admitted has to be added to that check to get in,
and the reason it justifies itself against not existing is recorded in the
commit that admits it. This is not an ambition to have no dependencies — §XVI
requires vendoring an entire inference stack (D23, B64), which is a great deal
of weight admitted deliberately. It is the mechanism that keeps the admission
deliberate.

## 7 · Generated code

One file is generated and then committed: `crates/mcf-core/src/failure/
category.rs`, from `doc/taxonomy.md`.

It is **not** regenerated at build time. A build that reads a Markdown file to
decide what compiles is a build with an undeclared input, and §3.12 makes every
input a condition of the artifact. What keeps the two together is a test —
`checks/tests/taxonomy_agreement.rs` — which compares codes, meanings, domains
and axis values in both directions and fails on any difference. Adding a
taxonomy leaf therefore means editing the document and the code in one change,
with a laboratory scenario (A13).

The same test asserts that nothing constructs `internal.unclassified`. The
taxonomy makes it a tracked defect metric with a target of zero rather than a
bucket: every occurrence is a missing category, and adding the category is the
fix.

## 8 · The documents

The format contract in the repository README governs every Markdown file in
`doc/`, and `checks/tests/documents_conform.rs` is what fails when one leaves
it. Twelve checks, in the gating tier: a title, front matter naming type,
version and status, a numeric version, a changelog that is the last section and
that accounts for the version the front matter claims, no derived document
claiming a source version that does not exist, every relative link resolving,
and every `B-*`, `DEC-*`, `§`, rule, resolution, laboratory, milestone and
proposal citation resolving to the document that defines it.

It exists because reading does not catch this class of defect. Its first run
found the intent document's changelog stopped at version 8 while its front
matter claimed version 24, a citation written without its hyphen so that it
named a rule that does not exist, and — before it was written — a register whose
front matter was two versions behind its own changelog and whose header counts
were three revisions stale.

**One clause is deliberately not fully checked.** The contract asks for present
tense outside changelogs, and that resists a machine. Only the constructions the
README names outright are checked; the rest stays a stated `review` obligation.
B16 counts a review check as a cost, and claiming a machine check that is really
a keyword search would be worse than counting it.

## 9 · The scheduled tiers

B38 tiers the suite: the fast hermetic tier gates every change, and the heavy
ones run on a schedule and before a release. Two exist.

```
$ scripts/ci.sh --with-reproducibility     # B-001
$ scripts/ci.sh --with-budget              # B-011
```

**Reproducibility** is §5 above. **The budget tier** measures MCF's own cost
against D24's ceilings and reads each figure the way D27 says: a prohibition at
the maximum, a ceiling on *state* at the maximum, a ceiling on an *event* at the
99th percentile over at least a hundred trials.

Two things it refuses to do are worth knowing before reading its output.

**It asserts only in release.** D24's ceilings are about the artifact MCF ships,
and a debug binary is a different one — larger, slower, different code. The
profile is a condition (§3.4), so a debug run reports every figure and asserts
none. That is why the flag runs `cargo test --release`.

**It asserts only on a quiet machine.** B35 holds that a timing taken under
contention measures the contention, so the tier reads the machine's one-minute
load, records it as a condition, and marks the run **unattributable** rather
than failing it. A budget cannot be violated by somebody else's compile. The
threshold — half a core's capacity free per core — is a judgement rather than a
reading, and it is stated in `mcf_core::hardware::QUIET_PER_CORE` with its
reasoning so that a reader can disagree with it.

An unattributable run is not a pass either. Nothing yet enforces that, because
tier ages are B-185 and a stored baseline is what a regression is measured
against; until those exist the tier reports and does not gate. B20's
before-and-after is not yet possible and the tier says so rather than implying
otherwise.

## Changelog

### Version 6 — the budget tier

§9 added with B-011. The two scheduled tiers now both have flags on the same
command, and the section states the two conditions under which the budget tier
declines to assert anything — the wrong profile and a busy machine — because a
tier that printed figures without saying when it was not judging them would read
as judging them.

### Version 5 — the first dependency is refused, in writing

§6 gains the reasoning that kept the record's serialization in-tree. A section
that only says "there are none" is a section that will one day say "there is
one" with no account of why, and B15's requirement is that each dependency be
*admitted for a stated reason* — which is worth nothing unless a refusal is
stated too.

### Version 4 — one module opts out of `unsafe_code`

Recorded in §4 when B-013 landed. The workspace's `deny` on `unsafe_code` was
written as a `deny` rather than a `forbid` in anticipation of exactly one kind
of work, and this is the first of it: reading an accelerator's live state over
the C ABI. A reader who finds `#![allow(unsafe_code)]` in a tree that denies it
should be able to find out why without reading the diff that added it.

### Version 3 — the documents are checked too

§8 added when B-041 landed. The build document is where the checks that gate a
change are described, and the documentation checks gate a change in exactly the
way the code checks do — they run in the same command and fail it the same way.

### Version 2 — the lints are checked, and one file is generated

Two sections gained substance when B-003 landed. §4 records that the lint table
is a claim and names the script that checks it, including why the negative
control is not optional. §7 is new: it states which file is generated, why it is
committed rather than regenerated at build time, and what keeps it honest.

### Version 1 — the workspace exists

Created with the workspace itself (B-001). The repository had held only intent,
rules, a plan and a register; the moment there is code there is a build, and a
build has conditions — a pinned compiler, a layering, a set of lints, a
definition of what "green" means — that belong in a document rather than in the
memory of whoever set them up.

Written as reference rather than instructions: each section states what the
build asserts about itself and which clause makes that worth asserting, because
a build convention nobody can cite is a convention that erodes.
