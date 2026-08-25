# Building MCF

| | |
|---|---|
| **Type** | Reference — the workspace, the toolchain, and the checks that gate a change |
| **Version** | 16 |
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
| 4 | [The gating tiers](#4--the-gating-tiers) |
| 5 | [Reproducibility](#5--reproducibility) |
| 6 | [Dependencies](#6--dependencies) |
| 7 | [Generated code](#7--generated-code) |
| 8 | [The documents](#8--the-documents) |
| 9 | [The tiers](#9--the-tiers) |
| 10 | [Ages, and what a release refuses](#10--ages-and-what-a-release-refuses) |
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
| `mcf-standin` | MCF's own implementation of inference: the model-file reader, and the operations that will run one (D31, B-360) | `mcf-core` |
| `mcf-lab` | Simulated clock, injected faults, replayable scenarios (§3.17) | `mcf-core`, `mcf-record`, `mcf-standin` |
| `mcf-hub` | Resolving, fetching and pinning artifacts (§III) | `mcf-core`, `mcf-record` |
| `mcf-serve` | The daemon, engine adapters, the serving surface (§VI) | `mcf-core`, `mcf-record` |
| `mcf-bench` | Measurement, and the laboratories that produce it (§II, §XIII) | `mcf-core`, `mcf-record`, `mcf-serve` |
| `mcf-cli` | The headless surface; binary `mcf` (A22) | all of the above |
| `mcf-checks` | The checks that are about the repository rather than a value — the crate split, the taxonomy agreement, the document contract, the tier register — and the machinery the tiers that are not `cargo test` need. Ships nothing, and nothing depends on it | — |

The table is a rendering of `checks/src/workspace.rs`, which is the
declaration the tests compare the repository against. Both directions are
checked: an **added** edge inverts the layering, and a **missing** one means
the declaration has drifted and can no longer be read as the architecture.

`mcf-bench` depends on `mcf-serve` and not the reverse, so no serving path can
acquire a dependency on the benchmark harness — A18's separation of tests from
benchmarks, drawn in the dependency graph.

`mcf-standin` is an engine rather than an adapter, which is why it is its own
crate and not part of `mcf-serve`: B65 forbids a stand-in from reporting a
speed, and a crate boundary is how a result from it reaches a surface only
through the handle that carries its mark (A5). `mcf-lab` depends on it because
A13 requires a scenario for every category MCF's code constructs, and the
model-file reader constructs two.

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

## 4 · The gating tiers

```
$ scripts/ci.sh
```

Formatting, `clippy -D warnings` over every target, the suite, and `cargo doc`
with warnings denied. It passes `--offline` rather than merely expecting no
network, so a check that starts reaching out fails here instead of on an
aeroplane (B19).

**Five of B38's ten tiers run here**, and they are one `cargo test` because
separating them would make it possible to run some of them and believe the
suite had run: `unit`, `property`, `functional`, `whole-system` and
`fault-injection`. §9 has the table and what each one covers.

**It takes about seven seconds, and the command says so.** Measured rather than
asserted: D24 gives sixteen figures and none of them is a suite time, so there
is no ceiling to compare against — B38 requires the gating tier be fast and says
so qualitatively. The elapsed time is printed at the end of every run, because a
gating tier that grew slowly would otherwise become one people skip without
anybody noticing when.

**What did not run is named.** Every invocation ends with the tiers it did not
run and the flag that runs each, because "did not run" read as "passed" is A2's
silent failure aimed at the suite. Tier *ages* — the stronger form, where a tier
that has not run recently is reported stale — are B-185, and the mutation floor
is B-186; the last line of every run says so rather than implying the list is
complete.

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

**A signal is a claim too.** `scripts/check-fault-signal.sh` spawns one binary
thirty times warm and thirty times with its pages evicted, and requires the
major-fault count to be zero and then non-zero. F3 records MCF shipping an
attributability signal that silently could not fail; this is what keeps the
second one honest. A machine that cannot evict a file's pages — a tmpfs has no
device behind it — reports that it could not demonstrate the signal, which is
not a pass and does not fail the gate.

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

**What the artifact requires of a *machine* is a separate question, and it is
checked too.** B36 refuses to make a missing prerequisite the user's errand, and
B-192's condition is the checkable form of it: the binary's own `DT_NEEDED`
entries must name nothing a stock machine lacks. `crates/mcf-cli/tests/
artifact.rs` reads the ELF rather than asking `ldd` — the requirement is in the
file, and `ldd` answers the different question of what resolves here — and fails
on a stranger, on a baked-in library search path, or on an interpreter that is
not the platform's own. The release artifact needs `libc.so.6` and
`libgcc_s.so.1`; `doc/vendored.md` §3a is where that list lives with its
reasoning, because the first vendored engine is the thing most likely to change
it.

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

## 9 · The tiers

D10 names ten disciplines that share the word "test", and §6.34 resolves the
tension between them: **tier the suites, gate on the fast one, schedule the
heavy ones**. All ten exist (B-191). The table is a rendering of
`checks/src/tiers.rs`, and `checks/tests/tiers_conform.rs` fails the build when
the two disagree — a tier whose command is not in `scripts/ci.sh`, a flag the
script does not accept, a file the register names and the tree does not have, a
gating tier that `#[ignore]`s its own tests, or a tier missing from this
document.

| Tier | Covers | Runs |
|---|---|---|
| `unit` | Logic, in the crate that owns it | gating |
| `property` | Invariants MCF claims universally, over generated inputs | gating |
| `functional` | Behaviour at the API surface, and the rules the workspace enforces about itself | gating |
| `whole-system` | The binary as a process, against a real record, including restart and a kill | gating |
| `fault-injection` | Every failure MCF claims to handle, reproduced from the laboratory's catalogue | gating |
| `fuzz` | The parsers that read bytes MCF did not write | `--with-fuzz` |
| `load` | MCF's claims under many callers at once, against the simulated laboratory | `--with-load` |
| `soak` | Drift over a long run: descriptors, directories, memory | `--with-soak` |
| `performance` | D24's budgets, on the release artifact, read as D27 says | `--with-budget` |
| `mutation` | The test of the tests | `--with-mutation` |

```
$ scripts/ci.sh --with-fuzz --with-load --with-soak
$ scripts/ci.sh --with-budget --with-mutation --with-reproducibility
$ scripts/ci.sh --all
$ scripts/check-tier-ages.sh [--release]     # ages, and the release refusal
```

Reproducibility (§5) is a scheduled check rather than one of D10's tiers, and
keeps its own flag.

### The property tier

Invariants with a quantifier in them: *the reported spread is five values that
were actually observed* (A6), *a journal torn anywhere reports the byte it
stopped at and how much it did not read* (B62), *what MCF did not know comes
back unknown* (A7). Generation is deterministic — a fixed seed set, stated in
`mcf_checks::property::BASE_SEED` — because §3.12 makes reproducibility a
precedence rule and a suite that draws fresh inputs every run gates each change
against a different question. Exploration is the fuzz tier's job.

It found a defect on its first run, which is recorded in the commit that added
it: every known condition was rendered through `Display`, so a context length of
4096 was written to the record as `"4096"` and read back as text.

### The whole-system tier

The binary cargo just built, run as a process against a scratch machine: two
runs appending to one journal, an export read back by the reader another machine
would use, and twelve kills landing wherever they land. Where the end-to-end
boundary falls is **DEC-022 and it is open** — §7.22 asks about a real HTTP
surface, a started engine and a supervised child, and none of those exists at
M0. The tier covers the fourth question, recovery with persisted state, and says
which three it is not answering.

### The fuzz tier

Four parsers read bytes MCF did not write: the record's codec, the journal
replay, the zone file the platform publishes, and a bundle that arrived from
somewhere else. The tier damages known-good inputs rather than generating random
ones — a uniform generator reaches the first error path and stays there — and
asserts only that a parser does not panic, refuses by name, and describes what
it accepted correctly.

Each target counts what it reached and fails if it never once accepted a damaged
input, because a campaign that only exercised the refusal path reports the same
green as one that examined the parser. Measured over the default 200 000 cases:
`json::parse` accepts 11 484, `Zone::parse` 20 637, `replay` 2 916 of its 5 000,
and the bundle reader 181 of its 5 000 — the last low by construction, since a
bundle states a digest over its own contents. About four seconds.

### The load and soak tiers

§6.34 settles what they run against: **the simulated laboratory, not real
weights**, so they stay cheap enough to run often and deterministic enough to
believe.

Load asks whether the laboratory's determinism (B27) and the journal's
completeness (B62) are properties of the code or of there having been one
caller: sixty-four workers, 38 400 scenario runs, 128 000 appends, 64 bundles,
about a second of wall time and fourteen of CPU. Nothing in it times anything —
A18 keeps a throughput assertion out of a test suite. One journal per worker,
because who writes to *one* journal is DEC-037 and it is open (B-332).

Soak looks for drift rather than a wrong answer. A hundred thousand appends grow
the writer by nothing; ten thousand open-append-close cycles leak no
descriptors; twenty thousand scenario runs leave no directories; thirty
simulated days cost nothing, because the lab's clock is supplied rather than
waited on. It runs on **one thread**, and that was found rather than assumed:
resident memory is a property of the process, so a second test allocating in
parallel reads as growth — B35's lesson about contention, arriving at memory
instead of at a timing.

It also produces a number worth keeping: a replay holds about a kilobyte per
entry, because `Replay` returns every entry it read. That is D20's design and it
is what D6's derived index exists to stop growing (B-042, B-300). Reported and
not asserted — asserting on it would be asserting that MCF never keeps a record
long enough to matter. This tier is **not B-148**, the M8 endurance scenario:
days of simulated operation with a daemon, state migration, and a machine that
changes underneath.

### The performance tier

Measures MCF's own cost against D24's ceilings and reads each figure the way D27
says: a prohibition at the maximum, a ceiling on *state* at the maximum, a
ceiling on an *event* at the 99th percentile over at least a hundred trials.

Two things it refuses to do are worth knowing before reading its output.

**It asserts only in release.** D24's ceilings are about the artifact MCF ships,
and a debug binary is a different one — larger, slower, different code. The
profile is a condition (§3.4), so a debug run reports every figure and asserts
none. That is why the flag runs `cargo test --release`.

**It asserts only on a clean reading.** B35 holds that a timing taken under
contention measures the contention, so each measurement is bracketed by a
`Watch` — the kernel's per-thread accounting of how long the measuring thread
was runnable and waiting — and a reading with too much queuing in it is marked
**unattributable** rather than failing. A budget cannot be violated by somebody
else's compile.

This is a question about the *reading* and not about the machine, which is why
it works on a workstation somebody is using: the machine being busy elsewhere
does not contaminate a measurement that was not queuing behind it. MCF tried the
machine-wide load average first and `findings.md` F3 records why that failed —
a one-minute average read 0.29 both on a quiet machine and under thirty-two
spinning processes, because it cannot answer a question about a
140-millisecond measurement. The threshold is one part in a hundred, stated in
`mcf_core::hardware::TOLERATED_DELAY_PPM`, and the two states F3 measured are
three orders of magnitude apart.

**Every figure is also compared with the last one recorded** (B-011, B20). A
ceiling catches a figure that became bad; a baseline catches one that became
worse, which is earlier and more useful. The previous readings live in
`.mcf-tiers/baselines/`, one file per figure, machine-local for the same
reason the tier ages are: a baseline from somebody else's machine is not a
baseline. A comparison is refused rather than made wrong when the two are not
comparable — a different profile is a different artifact (A8) — and a run that
fails leaves the baseline it failed against rather than adopting the worse
number.

**Growing a figure on purpose is a deliberate act, and looks like one.** Delete
that figure's file in `.mcf-tiers/baselines/` and say in the commit what was
bought. B-330's licence text is the first instance: carrying the whole GPL into
the binary so a redistributor conveying it conveys a copy (§4) grew the core
binary by 5.7 %, well past the 2 % tolerance and to 1.6 % of D24's ceiling. The
tier refused it, which is what it is for; accepting it was a sentence in a commit
message rather than a threshold quietly raised.

**Not every figure is judged against its baseline, and the ones that are not say
so.** A18 makes a regression detector a third thing whose thresholds are
statistical judgments rather than assertions:

| Figure | Tolerance | Why |
|---|---|---|
| Core binary | +2 % | The build is reproducible byte for byte, so a file's size does not move on its own. Two per cent tolerates a different inlining decision about the same code and catches a dependency arriving unnoticed |
| Resident memory | +10 % | A fresh process's resident set is nearly deterministic, and the allocator's policy is not MCF's |
| Cold start | not judged | Dominated on some storage by conditions MCF does not record ([findings.md](findings.md) F5). The change is reported; B-193 is what would make it judgeable |

An unattributable run is still not a pass. Nothing yet enforces that, and a
stale-tier refusal now exists to build it on (B-185).

**A reading is judged by two signals, not one** (B-193). The first is D30's:
how long the measuring thread was runnable and not running. The second exists
because the first cannot see a cost paid in another process — during a cold
start the measuring thread is blocked in `wait` while the child faults its pages
in — so a measurement whose work took a **major page fault** is refused as a
reading of the device. [findings.md](findings.md) F5 is the run that established
the gap and F7 is the experiment that shows the new signal moving; the threshold
is zero faults, which is not a judgement but a fact about what a fault is.

The storage an artifact was read from is also a condition now, the floor's
eleventh question, so two runs that differ only in where the binary lives are
legibly different — and the baseline comparison refuses them as incomparable
(A8).

### The mutation tier

D10 calls mutation testing *the test of the tests*: a suite that does not fail
when the code is deliberately broken is a suite that proves nothing.
`scripts/check-mutants.sh` applies a declared catalogue of mutations to a copy of
the working tree — the same shape `check-lints-bite.sh` uses — and requires the
suite to notice each one.

The catalogue is written by hand rather than generated. A generator produces
thousands of mutants, most of them equivalent or unreachable, and each one here
costs a suite run; every entry breaks something a rule depends on, so a
**survivor names a rule nothing is checking**.

**It refuses a score below the floor, and a score below the last one** (B-186,
B20). The floor is `MUTATION_FLOOR_PERCENT` in `scripts/lib-tiers.sh` and it is
**100 %** — a smaller number than it sounds, because it is a claim about the
twelve mutants in the catalogue rather than about every conceivable mutation of
MCF. Each of them breaks something a rule rests on, so a survivor is a gap
rather than a percentage to trade off. The previous score comes from the tier's
own stamp (B-185), which is where a run leaves what the next one compares
against, and a lower score is refused even when it clears the floor.

Three things keep the score honest. **An equivalent mutant is the control**: a
change with no semantic effect must *not* be killed, or the runner cannot tell a
killed mutant from a broken copy and its other results mean nothing. **A mutant
that does not compile is not a result**: it is excluded and named, because the
question is what the *tests* notice and a compiler error is the compiler
noticing. **Every mutated file is compared against a pristine copy at the end**,
because a restore that silently failed would make every judgment after it a
judgment about the wrong code — which is what happened, and what
[findings.md](findings.md) F4.5 records. A control at the start of a run and a
verification at the end answer different questions.

A mutant may also hang: the digest entry takes the room left in a 64-byte buffer
from 64 to 63, so the loop that fills it eventually takes nothing per pass. Each
suite run is bounded at two minutes, a timeout is confirmed by a second run
before it is believed, and anything still executing out of the run's own copy is
killed — a spinning test process outliving the tier is a change to the machine
A27 does not permit MCF's suite to make either.

**Twelve mutants, twelve killed, one of them by hanging; about four minutes.**
The score is not evidence that the suite is complete. It is evidence about
twelve specific claims, chosen because a rule rests on each.

The twelfth is there because it survived. `resident_bytes` converts the kernel's
kibibytes to bytes, and a mutant that multiplied by 1000 instead of 1024 lived
through the whole suite — a figure `mcf doctor` prints and B-011 asserts against
D24's ceiling, resting on nobody having looked (A19). The test that kills it
checks the reading lands on a page boundary and agrees with the second file that
counts the same pages. That loop — a survivor names a gap, the gap gets a test,
the mutant joins the catalogue — is what the floor is for.

## 10 · Ages, and what a release refuses

B38: *a heavy tier that has not run recently is reported as **stale**, never
assumed green — an unstated staleness is A2's silent failure aimed at the
suite.* A green `scripts/ci.sh` says nothing about whether the soak tier has
ever run against this code, so B-185 gives every scheduled tier an age.

```
$ scripts/check-tier-ages.sh
source e18ab72da0a3cff3da509e8bcd210cd24437901018b1ea7d491890836f2d926e

  fuzz         2 hours ago, on this source
  load         2 hours ago, on this source
  soak         2 hours ago, on this source
  performance  3 days ago, on OTHER source (a1c9f0e21b44)
  mutation     2 hours ago, on this source — mutation score: 11 killed of 11 scored
```

Run with no arguments it reports; run `--release` it **refuses**, naming each
tier, its age and the flag that clears it.

**Stale means the source changed, not that a clock advanced.** A maximum age in
days is the obvious design and it needs a number nobody has — D24 states sixteen
figures and none of them is how old a soak result may be, and inventing one is
how a project acquires intent nobody chose (A23). What *does* invalidate a
result is that the code it was taken against is no longer the code in the tree,
and that is decidable: every scheduled tier stamps a digest of the source when
it passes, and a stamp whose digest is not the current one is stale. The
wall-clock age is reported beside it because a reader wants to know when, and it
is not the verdict.

**What the digest covers is what can change a tier's outcome**: the manifests,
the toolchain pin, `crates/`, `checks/` and `scripts/`. Documents are excluded —
a rewritten paragraph cannot change what a soak run does, and treating it as if
it could would make every tier stale after every documentation commit, which is
how a staleness mechanism gets switched off.

**The stamps are machine-local** (`.mcf-tiers/`, untracked). A fresh checkout
has not run anything and says so rather than inheriting somebody else's result,
which is the same reason a contributed measurement is not a local one (B-166,
§XV). They live outside `target/` because a tier's result is about the source
rather than the build directory: `cargo clean` throws away something derived and
should not throw away the evidence that a four-minute tier ran.

A stamp carries anything the tier wants to hand forward. The mutation tier puts
its score there, which is what B-186's floor will compare against and what B20
means by a before and an after.

## Changelog

### Version 16 — the stand-in engine has a crate

§2 gains `mcf-standin` with the reason it is separate: an engine that cannot
report a speed keeps that prohibition through a crate boundary as well as
through a type. `mcf-lab` gains an edge to it, because a scenario that produced
a model-file failure by hand would be a scenario about a mock.

### Version 15 — how a figure is grown on purpose

§9 gains the sentence B-330 needed on its first day: the regression detector
refused a 5.7 % growth in the binary that was bought deliberately, and the way
to accept one is to delete the baseline and say what it bought. A threshold
raised quietly would have been the other option, which is the one B20 exists to
prevent.

### Version 14 — a reading is judged by two signals

§9's performance section rewritten where it described B-193 as owed. The
storage an artifact is read from is a condition, and a measurement that went to
a device for bytes is refused rather than asserted. §4 gains the check that
shows the new signal failing, which is the only way to know it works.

### Version 13 — the budget tier has a before

§9's performance section gains B-011's last half. The tier asserted each figure
against D24's ceiling and had nothing to compare it with; it now keeps the
previous reading beside the tier ages and refuses a figure that got worse by
more than a stated tolerance, printing both readings and both sets of
conditions. Which figures are judged and which are only reported is a table,
because a detector that cries wolf is one people switch off.

### Version 12 — what the artifact requires of a machine

§6 gains B-192's half of B36: the workspace has no build-time dependencies and
the artifact has three run-time ones, each of them part of what a Linux machine
is. Checked by reading the binary rather than by asking a tool, with a control
that watches the predicate reject something.

### Version 11 — the mutation score has a floor

§9's mutation section rewritten with B-186. The tier reported a score and now
refuses one below the floor or below the last run's, which needed B-185's stamp
to have somewhere to keep the previous number. The catalogue gained a twelfth
mutant, and how it got there is the part worth reading: it survived, so the
claim it broke got a test.

### Version 10 — the tiers have ages

Section 10 added with B-185, and §9's commands gain the age check. The tiers existed
and nothing said whether any of them had run against the code in the tree. What
makes one stale is the part worth reading: the source it ran against, not a
clock, because no clause states how old a soak result may be and the thing that
actually invalidates one is decidable.

### Version 9 — all ten tiers exist

§4 and §9 rewritten with B-191. The suite had five of D10's ten disciplines and
`ci.sh` printed the absence of the others; it now has all ten, and the table in
§9 is a rendering of `checks/src/tiers.rs` that a test compares this document
against in both directions. What the new tiers found is recorded with them: a
condition that did not round-trip, a replay whose footprint is proportional to
the journal, and the reason the soak tier runs on one thread.

### Version 8 — the budget tier judges the reading, not the machine

§9 rewritten where it described the attributability signal. The load average
answered a question about the machine on a time scale that could not see the
measurement; the per-thread scheduling delay answers the question that was
actually being asked. `findings.md` F3 holds the evidence, including the defect
in the first implementation of the replacement.

### Version 7 — the gating tier's time is reported

§4 gains the measured figure. B-015's condition asked for the suite to run
inside a budget DEC-016 was to set; DEC-016 closed with D24, and none of D24's
sixteen figures is a suite time. Reporting the number is what can honestly be
done — asserting it would need a figure nobody has stated, and inventing one to
satisfy a clause is how a document acquires intent nobody chose (A23).

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
