# Building MCF

| | |
|---|---|
| **Type** | Reference — the workspace, the toolchain, and the checks that gate a change |
| **Version** | 38 |
| **Status** | Living |
| **Authority** | Derived from [document-of-intent.md](document-of-intent.md) v44, governed by [rules.md](rules.md) |
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
| 11 | [A machine with nothing on it](#11--a-machine-with-nothing-on-it) |
| 11a | [The real hub](#11a--the-real-hub) |
| 12 | [A machine with something else on it](#12--a-machine-with-something-else-on-it) |
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
| `mcf-standin` | MCF's own implementation of inference: the model file, the operations, the forward pass, the tokenizer (D31, B-360) | `mcf-core` |
| `mcf-helper` | The privileged helper: three named operations, performed and then gone — binary `mcf-helper` (B-190, D35, §6.32) | `mcf-core` |
| `mcf-hub` | Resolving, fetching and pinning artifacts, and the interface a source of them answers (§III) | `mcf-core`, `mcf-record` |
| `mcf-lab` | Simulated clock, injected faults, replayable scenarios (§3.17) | `mcf-core`, `mcf-helper`, `mcf-hub`, `mcf-record`, `mcf-serve`, `mcf-standin` |
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

`mcf-helper` links `mcf-core` and nothing else, and nothing links it back
except the laboratory that reproduces its failures. §6.32 asks for a privileged
surface that is *auditable*, and a program which runs with rights the daemon
does not have should be readable in one sitting; the daemon starts it as a
process rather than calling into it, which is what keeps the daemon
unprivileged (`checks/tests/the_daemon_holds_no_privilege.rs` holds both
halves).

`mcf-standin` is an engine rather than an adapter, which is why it is its own
crate and not part of `mcf-serve`: B65 forbids a stand-in from reporting a
speed, and a crate boundary is how a result from it reaches a surface only
through the handle that carries its mark (A5). `mcf-lab` depends on it, and on `mcf-hub`, because A13 requires a scenario for
every category MCF's code constructs — the model-file reader and the reference
parser construct four between them, and a scenario that built those failures by
hand would be a scenario about a mock (D26).

## 3 · Building

```
$ cargo build --locked                  # debug
$ cargo build --locked --release        # the artifact D24's budgets govern
$ ./target/release/mcf --version
MCF 0.1.0-m0  (revision unknown, rustc 1.98.0 (88d9e12ae 2026-08-18), target x86_64-unknown-linux-gnu, profile release)
```

**There is a second artifact, and D29 makes that normal rather than exceptional.**
`x86_64-unknown-linux-musl` links statically: no interpreter, no libc to
resolve, nothing to find on the machine it lands on. The target triple is
already a §3.4 condition, so it is a different artifact rather than a different
build of the same one, and B-183's from-scratch check — section 11 — is what it
exists for.

```
$ rustup target add x86_64-unknown-linux-musl
$ cargo build --locked --release --target x86_64-unknown-linux-musl -p mcf-cli
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

**Two modules opt out of `unsafe_code`, and both say why at the site.**
`crates/mcf-core/src/hardware/nvml.rs` loads the vendor's management library at
runtime and calls it over the C ABI, because F1 established that a device's live
state — available memory, temperature — is reachable no other way, and D25 makes
those readings the difference between a characterized device and an
uncharacterized one. The workspace denies `unsafe_code` as a `deny` rather than
a `forbid` precisely so that this opt-in is possible with its reason written at
the site. Every pointer is null-checked, every status code is checked before its
out-parameter is read, and the library is closed on every path out.

The second is `crates/mcf-core/src/hardware/space.rs`, one `statvfs` call, and
its reason is §3.11: a download that would exhaust the disk is meant to be a
decision rather than a surprise, and a decision needs the number *before* the
download. The standard library does not expose it. The struct is declared here
rather than taken from a binding crate — eleven integers, named as the manual
page names them so the layout is checkable rather than trusted — and the reading
is `Unknown` wherever the call fails, because *could not look* and *no room* are
opposite answers. It is checked against `df` (A12), and
[findings.md](findings.md) F11 has the measurement that made it necessary: a
buffered write to a full filesystem *succeeds*, and the failure arrives at the
flush.

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

**There is one: a TLS client.** Everything else compiles from the standard
library alone, and a test asserts exactly that —
`the_workspace_takes_exactly_what_the_register_admits` names the three crates
`mcf-hub` may take and fails on a fourth in any manifest, in any table. The
admission is B-322, the stated reason B15 requires is
[findings.md](findings.md) F9, and the compatibility findings are
[vendored.md](vendored.md) §2.

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

**The second candidate was admitted, and the shape of the decision is the
point.** MCF could not reach an encrypted hub, and §III requires that models
enter this machine. F9 measured the boundary rather than arguing about it: the
hub speaks HTTP/1.1 and serves ranges, so the protocol is MCF's own and written;
what was missing was cryptography, which nobody here is going to write.

Three things about how it went in. The provider was chosen by what it costs —
the usual one is C and cannot build for the musl target the from-scratch check
uses, so it would have made an existing check runnable in fewer places (F9.5).
The tree is 18 MiB rather than 95 because crates no target compiles are stubbed
down to a manifest and a licence, which cargo accepts and deletion does not
(F9.4, F9.6). And the source is *in the repository*: every build here runs
`--offline --locked`, so a dependency resolved from a registry at build time is
a dependency nobody pinned. `scripts/vendor.sh` builds that tree and checks both
targets against it; `scripts/check-vendored-terms.sh` reads what every crate
declares and gates the build on it.

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

**One thing in this tier is a measurement rather than a budget.** `cargo test
-p mcf-record --test how_the_record_grows` writes journals of a thousand to a
million entries and prints what a replay, an index build, an index open and a
query cost on this machine. It asserts one fact — that the index and the journal
agree about what happened — and gates on none of the timings, because A18 keeps
a measurement out of a pass condition. It rides here because it needs the same
quiet machine everything else in this tier needs, and its figures are what
[findings.md](findings.md) F14 was written from: re-run it when somebody
doubts that the derived index still earns its bytes.

**Growing a figure on purpose is a deliberate act, and looks like one.** Delete
that figure's file in `.mcf-tiers/baselines/` and say in the commit what was
bought. B-330's licence text is the first instance: carrying the whole GPL into
the binary so a redistributor conveying it conveys a copy (§4) grew the core
binary by 5.7 %, well past the 2 % tolerance and to 1.6 % of D24's ceiling. The
tier refused it, which is what it is for; accepting it was a sentence in a commit
message rather than a threshold quietly raised.

The second instance is larger and the procedure did not change. The TLS stack
B-322 admitted grew the core binary from 780 KiB to 3.3 MiB — 356 %, against a
2 % tolerance, and 8.3 % of D24's ceiling. That is what reaching an encrypted
hub costs, it was measured before it was bought
([findings.md](findings.md) F9), and the tier refusing it is the tier working:
a 356 % growth nobody had to acknowledge is exactly what B20 is written
against. Two other figures moved with it and are recorded rather than accepted:
resident memory by 5.7 % (7.8 MiB, well inside its ceiling), and the cold-start
p99 from 875 µs to 1.7 ms — which the tier does not judge against a baseline,
for the reason D27 gives, and which its ceiling still passes.

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

## 11 · A machine with nothing on it

```
$ scripts/ci.sh --with-from-scratch     # or scripts/check-from-scratch.sh
```

§6 checks what the artifact *requires* by reading its dependency list. This
checks the same claim the only way that settles it: it builds an image `FROM
scratch` — the statically linked binary and **nothing else**, no libc, no shell,
no package manager, no `/etc`, no `/tmp` — and runs MCF in it.

All of it runs. `--version`, `licence`, and the whole of `doctor`: the hardware
profile, the self-cost measurement over a hundred process spawns, and the
laboratory reproducing every failure MCF claims to handle. What the container
lacks, MCF reports rather than assumes — the accelerator comes back *attempted,
uncharacterized (missing memory, thermal)* because the vendor library is not
there to load, which is D25 and A5 behaving exactly as written on a machine
that genuinely lacks something.

**And it reaches a first token**, which is B-183's condition. The container has
no toolchain to build a model with and no network to fetch one over, so the
machine that has a toolchain writes one out — `cargo run -p mcf-lab --example
write-fixture` — and the image is handed the bytes. MCF's own stand-in (D31)
reads it and produces text: the whole path, file to vocabulary to forward pass
to sampler to tokens, with nothing installed.

**What that token is and is not.** It is what B36 claims — MCF needs nothing
from the machine — demonstrated rather than argued. It is not a real model and
it can never be a speed (B65): the fixture has four tokens and one block, and
what a real artifact does here is B-019's, which needs fifteen gigabytes and
somebody's decision to spend them. The other platforms D29 names each need a
machine to run this on.

## 11a · The real hub

```
$ scripts/ci.sh --with-online           # or scripts/check-online.sh
```

Everything else about acquisition runs against a hub the laboratory holds on the
loopback address, because B19 requires M1's suite to run with no network. That
is right, and it leaves one thing unchecked: whether the hub MCF was written
against behaves the way MCF believes.
[findings.md](findings.md) F9 answered that once by hand; this is the repeatable
form, and it is scheduled rather than gating for the same reason the fuzz tier
is — a gate that needs a network is a gate that fails on a train.

It acquires a 1.2 MiB GGUF from a repository of deliberately tiny models: real
weights, a real LFS digest, a real redirect to a CDN. Then it lists what is
held, removes it, purges it, and reads the record for both events. A check that
downloaded a 27 GiB model to prove a transfer works would be one nobody runs;
one that downloaded nothing would prove nothing.

**And it runs what it acquired, with the engine MCF wrote.** Everywhere else
the stand-in runs a fixture the laboratory built — MCF checking its own
arithmetic against its own file. Here it reads a 260-thousand-parameter model
from a real publisher and produces text: *. It was a big, shiny blueb*. That is
D31's claim against weights MCF did not write, and A19's reason for wanting it —
nobody should believe numbers from software that cannot demonstrate it computes
what it claims, and the first thing to demonstrate is that it computes anything
at all on somebody else's bytes. The check asserts the answer carries its mark
and its sentence about what it cannot be (B65).

**And it plans for a repository nobody would download to test with**, which
costs a listing and a `config.json` and no bytes at all. A transfer exercises
the wire; it says nothing about the *variety* of what a hub publishes.
[findings.md](findings.md) F16 found three defects the first time the planner
met the reference repository — a configuration MCF could not parse, one MCF was
not looking in the right place for, and a key/value cache overstated fourfold —
all in code that had tests. The check now asserts that every variant the
repository publishes is classified, because a plan missing a row nobody
mentioned is worse than no plan (A1).

**Nothing here is timed.** The hub is somebody else's machine on somebody else's
network, and A6 would want conditions MCF cannot state for any number taken
across it.

## 11b · Where the models go

MCF keeps its **record** where the platform says a user's data belongs —
`$XDG_DATA_HOME/mcf`, falling back to `$HOME/.local/share/mcf`, refusing to
invent a location when neither is set (A7). The record is small.

**Models are a separate choice, because they are not small.** `MCF_MODELS` is an
ordered list of absolute paths, separated the way every path list on this
platform is separated. The first is where a new acquisition goes; all of them
are searched for what is held:

```
$ export MCF_MODELS=/home/gauge/Content/mcf-data/models:/home/gauge/models
$ mcf pull unsloth/Qwen3.8-27B-GGUF:Qwen3.8-27B-UD-Q4_K_M.gguf
$ mcf pull owner/other:model.gguf --into /home/gauge/models
```

An environment variable rather than a configuration file: §5 refuses MCF a
configuration language, and a path list in a variable is the platform's idiom
rather than a language. Where a machine wants it to persist, a shell profile is
how a machine persists an environment variable.

`mcf doctor` prints the stores in order, says which one new models go to, and
gives the free space of each — including for a store that does not exist yet, by
asking the filesystem that would hold it.

**On this machine** the first store is on the content drive, which has 21 TB
free against the home partition's few. That matters beyond convenience: the
drive an artifact was read from is a condition of every measurement taken
against it (B-193), and the two drives here are not the same kind of device.

## 11c · The conformance corpus

```
$ scripts/ci.sh --with-corpus           # or scripts/check-corpus.sh
```

The engine is developed against the smallest *trained* model of each family MCF
covers or means to cover, one distinct quantization apiece, so that architecture
and quantization coverage come from the same handful of files (D40, DEC-054).
Six of them are 1.4 GB against the reference model's 16.5 GB — which is the
point: the size that makes residency a real problem is the size that makes every
engine iteration slow.

**Each entry declares what MCF does with it today, and the check fails both
ways.** A model that ran and now refuses is a regression. A model that refused
and now runs is *also* reported, because the entry is then out of date and
somebody should say which family MCF covers. A check that quietly accepted good
news would be a check that stops being read.

**A refusal is checked for what it says.** Today four of the six refuse, and
each names a different missing thing — a `smollm` pre-tokenizer, a `bert`
tokenizer scheme, a per-expert gate where MCF looked for `ffn_gate.weight`, and
an architecture MCF has not been taught. That list is B-365's order of work,
read off artifacts rather than predicted, and a refusal that stopped naming what
it wanted would have lost the thing that made it useful.

**Where the corpus is.** `MCF_CORPUS`, or the store MCF itself would use. It is
scheduled rather than gating because a gate that needs 1.4 GB of models is a
gate that fails on a fresh clone; `mcf pull` the artifacts named in
[findings.md](findings.md) F21 to have it.

**Nothing here is timed and nothing here may be** (B65). The figures in F21 and
F22 are recorded as what the decision was about, not as properties of any model.

## 11d · Against a reference implementation

```
$ scripts/ci.sh --with-oracle           # or scripts/check-oracle.sh
```

**Why this exists, in one measurement.** [findings.md](findings.md) F25 removed
the expert router from MCF's mixture-of-experts entirely and the model produced
`Paris. It is located on the River Seine in`; the correct implementation
produced `Paris, France is Paris, Paris is Paris is`. The broken engine read
*better* than the right one. Four findings now say the same thing from different
directions (F20, F22, F24, F25): **output quality is not evidence about
implementation correctness, in either direction.** Every family MCF's engine
covers was transcribed from somebody else's source and is unverified in A21's
exact sense. This is what verifies it.

**It starts with the tokenizer because that part can be exact.** Identifiers are
integers. Two tokenizers either agree about them or do not — no tolerance, no
floating-point arithmetic in the way. It is also the part F23 found three
defects in, every one of them by reading rather than running, and every one of
them still unverified by anything but that reading.

**The forward pass is compared too, against a tolerance that was measured.**
Greedy generation is deterministic, so two correct implementations should agree
— except where the best and second-best logits are close enough that a different
summation order picks a different winner. F27 measured which is which: the four
noise divergences sat at margins of 0.040, 0.098, 0.105 and 0.159, in every case
with the reference choosing exactly MCF's runner-up and in every case at the
smallest margin of that whole generation. The one real defect sat at 0.775.

So a generation that differs fails when the margin **at the step where the
two texts part** is over 0.40 — above every noise margin observed (0.017 to
0.320, the widest on a Q2_K file), below both defects observed (0.449, 0.775). F27's first rule took the
smallest margin *anywhere* in the generation, and F32 found what that let
through: a broken decoder that parted at step 0 with 0.449 and was excused by a
0.021 five tokens later. `margins --against "<reference text>"` finds the
parting step. The threshold is provisional in one direction only: **a defect
that parts at a genuine near-tie still passes**, and what narrows that is more
files rather than a cleverer rule.

`cargo run -p mcf-standin --example margins -- <model> "<text>"` is the
instrument, and is worth running by hand whenever a divergence appears.

**Distributions are compared too, and this is the verdict that does not
narrow** (B-373, F34). The provisioned `llama-server` returns the reference's
top twenty log-probabilities at every step; `margins --logprobs-of` prints
MCF's log-softmax for the same tokens at the same step; the two are compared
at step 0 for every prompt and at the parting step when both engines reached
it through the same token ids. The statistic is the KL divergence of the
reference from MCF over those twenty, floor 0.20 — a clean engine's maximum
across sixteen files is 0.113, a swapped rotation's median is 0.32. The two
gap statistics are printed beside it on a failure. `MCF_ORACLE_SECTIONS=
distributions` runs only this section, which is how its floor was measured.

**Embeddings are compared too, at their own measured floor.** An embedding
model's vectors cannot equal the reference's — MCF multiplies dequantized
floats where the reference multiplies in quantized arithmetic — and F29
measured the gap: cosine 0.9996–0.9998 across five texts. The floor is 0.999,
which a single normalization swapped in a single layer falls through (0.972).
Models are discovered by whether `mcf embed` serves them.

**The reference is a development instrument and is not vendored.** Nothing in it
ships, nothing in it is on the path of any MCF command, and MCF's own engine
runs with none of it present — D39's fourth condition. What MCF may *provision*
for itself is DEC-052 and is not settled; until it is, this check asks for a
build that is already there, names the pinned commit it was written against, and
says so when the build is at a different one.

```
$ mcf provision llama.cpp
```

That is the whole of it (B-367): a rootless container from an image pinned by
digest, the source cloned at the pinned commit, the build landing in a prefix
under MCF's data home — `mcf/provisioned/llama.cpp@925e1179947e/` — with the
exact package versions and the script that ran written beside it, and the
provisioning recorded. The tier looks there first. `MCF_ORACLE` still names a
checkout built by hand, which F30 measured as the route that silently picks up
whatever the PATH resolves; it is a fallback, not a peer. The corpus comes from
`MCF_CORPUS` or MCF's own store. Scheduled rather than gating, because it needs
both.

## 11e · Provisioning a component

```
$ mcf provision --list
$ mcf provision llama.cpp [--into <root>]
$ mcf provision --remove llama.cpp --because "<why>" [--into <root>]
```

**What may be provisioned is a table in MCF's source**, not a language (§5):
one entry today, the reference implementation. Each entry pins an image by
digest, a source by commit, a package list, a configure line and a target list,
and the tests hold every entry to that.

**What a run does.** Writes the recipe as a script into the prefix — what ran
is part of what is recorded — then one `podman run --rm` over the pinned image
with the prefix bind-mounted: install the packages, record their versions,
clone, check out the pin, verify the checkout landed on it, configure, build.
Success writes `mcf-provenance.json` beside the build and a
`component_provisioned` entry to the record; failure names the exit status and
the log, and the prefix is safe to remove and the run safe to repeat.

**Where things live, and why two places.** The *prefix* goes under MCF's data
home — on this machine the large drive — or wherever `--into` says. Podman's
*image store* does not follow: rootless podman keeps it under `XDG_DATA_HOME`,
and the first provisioning ever run failed pulling the image because that
drive's filesystem will not do what an overlay store needs (F31). MCF hands the
child the platform default for the store and the operator's choice for the
output. The store is podman's and shared; the prefix is MCF's and removable.

**Removal carries its reason** (§3.11, A27) and is recorded before the
directory goes: a recorded intention beside a still-present prefix beats a
removed prefix nobody wrote down. The base image stays — it is shared with
everything else on the machine that uses it.

**What is built is self-contained.** A shared build bakes the *container's*
library path into every binary — `/work/build/bin`, a directory that exists
nowhere on the host — and the first provisioned oracle loaded nothing without
`LD_LIBRARY_PATH` (F31). The recipe builds static, and the test that holds
every recipe requires it: what is provisioned runs where it lands.

**A machine without `podman` is refused by name**, with the platform package
to install. MCF will not fall back to the host's tools: F30 measured what that
route does.

## 12 · A machine with something else on it

A machine that hosts several projects with heavy test workloads — as the one MCF
is developed on does — cannot run four suites at once and get four suites four
times slower. It gets four suites that measure each other, and the one that
suffers most is the one whose figures are timings.

**Every scheduled tier runs inside an exclusive window** where
`~/.local/bin/heavy` is on the path:

```
$ heavy status                 # who has the machine, and who is waiting
$ heavy log 20                 # the last twenty windows
```

`scripts/ci.sh` does this itself — there is nothing to remember — and runs the
tiers plainly where no such tool exists, because a machine without it is a
machine with one project on it and MCF does not require a tool it does not ship
(B36's habit, aimed at a developer's machine rather than a user's).

**The gating tier deliberately does not take the window.** It is seconds long,
and a five-second check queued behind a five-minute mutation run is a check
people stop running.

**The budget tier is the one that needs it most**, and the reason is already in
the rules rather than in convenience: B35 holds that a timing taken under
contention measures the contention, and D30 makes MCF *refuse* such a reading
rather than report it. Without the window that tier does not merely run slower —
it declines to assert, and its age does not refresh (B38). The window is what
makes its figures assertable at all, which is the same thing B-181 asks for
inside MCF for timing-class laboratories: an exclusive window, announced and
bounded.

Each tier states a deadline in minutes. It is a bound on a hung run rather than
an estimate: a window nobody gives back is the failure the tool exists to
prevent.

**A window that never comes is reported, not fatal.** A project on this machine
can hold the window for twelve hours, and a run that asked for the mutation tier
during one used to die at the first timeout: `errexit` ended it, the tiers after
it never ran, nothing said which, and the output stopped mid-sentence. That is
the silent partial A4 forbids, in MCF's own build script. Now a tier that cannot
get the machine is named at the end under **asked for and could not run**, the
remaining tiers are attempted, and the last line says both things:

```
ci: the gating tiers are green in 36s; 1 scheduled tier(s) could not get the machine
```

The tier's age is left exactly as stale as it was, which is what
`scripts/check-tier-ages.sh --release` refuses on (B38, B-185) — so nothing is
lost by not waiting, and nothing is claimed either.

**How long to queue is `MCF_WINDOW_WAIT_SECONDS`** (default 1800). It is an
environment variable rather than a flag because it is a property of the machine
rather than of the run: on a machine nobody else uses it is irrelevant, and on
this one an overnight run wants hours.

## Changelog

### Version 30 — where the models go

`MCF_MODELS` holds an ordered list of stores; the first takes new acquisitions
and `--into` overrides for one. The record stays where the platform keeps a
user's data, because the record is small and the models are not. Written down
because the drive an artifact was read from is part of what a timing taken
against it means (B-193).

### Version 29 — a real model, run by the engine MCF wrote

The online check now runs what it acquires. The stand-in has only ever been
exercised against a fixture the laboratory writes, which is MCF checking its own
arithmetic against its own file; this reads a real publisher's weights end to
end and asserts text comes out with its mark on it.

### Version 28 — a window that never comes

Section 12 gains what the machine taught it. A scheduled tier that cannot get
the exclusive window is now reported rather than fatal: it used to end the whole
run at the first timeout, so the tiers behind it never ran and nothing said so.
The run now names what could not run, leaves those ages stale, and says both
halves in its last line. `MCF_WINDOW_WAIT_SECONDS` decides how long a run is
willing to queue.

### Version 27 — the online tier meets a repository with a shape

Section 11a gains the half a transfer cannot cover: the online check now plans
for the reference repository as well as acquiring from a tiny one. It fetches no
bytes to do it, and it asserts that every variant published is classified —
which is exactly what was silently untrue until F16.

### Version 26 — a first token on a machine with nothing on it

Section 11 gains the row it has been missing: the from-scratch container now
runs `mcf run` and gets text out. The model is a four-token fixture the
laboratory writes out on the host, because the container has neither a toolchain
to build one nor a network to fetch one — which is the point of the exercise.

That is B-183's condition met on this platform, with both caveats stated where
the figure is: the fixture is not a model, and a stand-in's answer is never a
speed.

### Version 25 — a ninth crate, and the only one with rights

`mcf-helper` joins the table: three named operations from D35's list, performed
by an executable that exits. It links `mcf-core` and nothing else, nothing links
it back but the laboratory, and the daemon starts it as a process rather than
calling into it — which is the arrangement that keeps the daemon unprivileged
rather than the promise that it is.

### Version 24 — the record's own growth, measured on a schedule

The performance tier gains `how_the_record_grows`, which is not a budget: it
prints what a replay and the derived index cost at four sizes of record and
asserts only that the two agree. It is in this tier because it wants the same
quiet machine, and it is in the documentation because its figures are the ones
D6's amendment rests on ([findings.md](findings.md) F14).

### Version 23 — the second `unsafe` opt-out

§4 gains it: one `statvfs` call, so that a download which would exhaust the disk
is refused with the arithmetic rather than discovered at ninety per cent (§3.11,
B-026). The pair now reads as the policy it is — `deny` rather than `forbid`,
two sites, each with its reason where a reader will find it.

### Version 22 — the second figure grown on purpose

§9's paragraph on growing a figure deliberately gains its second instance, and
it is a big one: the TLS stack grew the core binary by 356 %. The number is
worth having in the build document rather than only in a commit message,
because the pair now makes the point better than either did alone — 5.7 % for a
licence text and 356 % for a network, both refused by the same detector, both
accepted the same way, and neither by moving a threshold.

### Version 38 — distributions against distributions

The oracle compares the reference's top-twenty log-probabilities against MCF's
at the same step (F34). `MCF_ORACLE_SECTIONS` selects sections for measuring.

### Version 37 — the threshold at 0.40

F33's Q2_K witness parts once at 0.320, on the coarsest scheme, where the
arithmetic gap is widest; the threshold moves to 0.40 and the corpus tier
gains four scheme witnesses.

### Version 36 — the margin where they part

The oracle's generation rule takes the margin at the step where MCF's text
stops being a prefix of the reference's, not the smallest margin anywhere
(F32). Threshold 0.30.

### Version 35 — provisioning is a command

`mcf provision` (B-367): the oracle is built by MCF itself, in a container from
a pinned image at a pinned commit, into a removable prefix, and recorded. The
oracle tier looks at the provisioned prefix first. F31 records the one thing
the first run found: podman's image store must not follow MCF's data home onto
a filesystem that cannot hold it.

### Version 34 — the sixth family, and its own verb

`mcf embed <model> --text <text>` exists (B-371, DEC-055): one JSON line first,
conditions after. The oracle grew an embedding comparison at a cosine floor
measured before it was set, and the corpus tier a third state — `embeds`,
expecting the width the file declares.

### Version 33 — the forward pass, against a measured tolerance

The oracle now compares generations as well as identifiers. What makes that
possible is F27's measurement of what a near-tie looks like against what a
defect looks like, rather than an assumption about it. Both defects the oracle
has found are recorded, and reintroducing either makes the tier fire.

### Version 32 — against a reference implementation

`--with-oracle` added (B-368). MCF's tokenizer against llama.cpp's, at a pinned
commit, over texts chosen for where tokenizers differ. Identifiers are integers,
so the comparison is exact and a disagreement is a defect rather than a
judgement — which is what four findings in a row have been asking for.

### Version 31 — the conformance corpus

`--with-corpus` added (B-370). The engine's development subject is no longer the
reference model: §XII is amended by D40, and the corpus of six small trained
models is what an iteration runs. The tier checks each entry against what the
register says it does, in both directions, and reads a refusal for what it
names.

### Version 21 — the real hub, on purpose

Section 11a added with B-029's online half. Everything about acquisition is tested
against a hub the laboratory holds, which is what B19 asks and what leaves one
thing open: whether the real hub behaves the way MCF believes. F9 answered that
by hand once; `--with-online` is the repeatable form, acquiring a 1.2 MiB model
that is really a model, from the hub that really serves it.

### Version 20 — the first dependency

§6 stops saying *there are none*. A TLS client is vendored, in the one crate
that reaches a hub, and the check that used to assert emptiness now asserts the
list: three crates named, and a fourth in any manifest fails until somebody puts
it in the register too.

The section keeps the first candidate's refusal beside the second's admission,
because the pair is the template: what it would have bought, what it would have
cost, and which way that came out. Two scripts arrived with it —
`scripts/vendor.sh`, which builds the tree deterministically and checks both
targets against it, and `scripts/check-vendored-terms.sh`, which reads what
every vendored crate declares and gates on it.

### Version 19 — the second candidate

§6 gains the TLS question. The first candidate was refused and the reasoning is
the template; this one is *measured and undecided*, which is a third state worth
keeping distinct from both. What MCF lacks to reach a hub is not a protocol —
that is written and tested — but cryptography nobody here should hand-write, and
the price is about ninety megabytes of vendored source rather than the fifteen a
Linux build compiles. Recorded here so the number is in the build document
before it is in a diff.

### Version 18 — the machine is shared

Section 12 added. Several projects run heavy suites on this machine, so every
scheduled tier now takes an exclusive window where one is available. The budget
tier is the reason it matters rather than a nicety: D30 refuses a reading taken
while something else had the processor, so without the window that tier declines
to assert and its age does not refresh.

### Version 17 — a machine with nothing on it

Section 11 added with B-183's checkable half, and §3 gains the statically
linked artifact it runs. The dependency list said MCF needs nothing unusual;
this runs it in a container that has nothing at all, which is the difference
between a claim about a file and a claim about what happens.

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
