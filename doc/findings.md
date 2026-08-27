# Findings

| | |
|---|---|
| **Type** | Record — what a prototype or a run established, and what it changed |
| **Version** | 42 |
| **Status** | Living |
| **Authority** | Reports to [document-of-intent.md](document-of-intent.md) v25; a finding that changes intent is migrated there and cited from here |
| **Registers to** | [backlog.md](backlog.md) |

**A finding is a thing MCF learned by running something, written down with the
conditions it was learned under.** §8 requires that the reasoning behind a
change outlive the change; this file is where the *evidence* behind one lives,
so that an amendment to the intent document can cite something rather than
assert something.

Read it when a decision cites a run, or before proposing to reopen one.

**These are not measurements MCF publishes.** Every figure here was taken on
one machine, by a prototype, outside the exclusive window B35 requires for
timing-class work. They are evidence about *MCF*, adequate for deciding whether
a substrate or a budget is right, and inadequate for any claim about a model or
about hardware. A20's rule that an estimate is never promoted applies to them
in spirit: a prototype's reading is replaced by the suite's, never carried
forward as one.

## Contents

| § | Section |
|---|---|
| 1 | [F1 — The adversarial prototype (§7.19, DEC-019)](#1--f1--the-adversarial-prototype-719-dec-019) |
| 2 | [F2 — The development machine cannot attribute a budget (DEC-051)](#2--f2--the-development-machine-cannot-attribute-a-budget-dec-051) |
| 3 | [F3 — The load average answers the wrong question, and the obvious fix silently could not fail (DEC-051, D30)](#3--f3--the-load-average-answers-the-wrong-question-and-the-obvious-fix-silently-could-not-fail-dec-051-d30) |
| 4 | [F4 — What the new tiers found on their first runs (B-191)](#4--f4--what-the-new-tiers-found-on-their-first-runs-b-191) |
| 5 | [F5 — The cold-start budget is a measurement of the filesystem (B-011, D30)](#5--f5--the-cold-start-budget-is-a-measurement-of-the-filesystem-b-011-d30) |
| 6 | [F6 — The first mutant to survive (B-186)](#6--f6--the-first-mutant-to-survive-b-186) |
| 7 | [F7 — A major page fault is the signal F5 was missing (B-193)](#7--f7--a-major-page-fault-is-the-signal-f5-was-missing-b-193) |
| 8 | [F8 — How far a kernel MCF could maintain is from a specialist's (DEC-004)](#8--f8--how-far-a-kernel-mcf-could-maintain-is-from-a-specialists-dec-004) |
| 9 | [F9 — What a network costs, and what the hub actually does (B-021)](#9--f9--what-a-network-costs-and-what-the-hub-actually-does-b-021) |
| 10 | [F10 — What a machine says when a network is missing (DEC-011, D33)](#10--f10--what-a-machine-says-when-a-network-is-missing-dec-011-d33) |
| 11 | [F11 — A disk fills at the flush, not at the write (B-026)](#11--f11--a-disk-fills-at-the-flush-not-at-the-write-b-026) |
| 12 | [F12 — What an engine would cost, as far as it has been measured (B-320)](#12--f12--what-an-engine-would-cost-as-far-as-it-has-been-measured-b-320) |
| 13 | [F13 — Two writers, one record (DEC-037)](#13--f13--two-writers-one-record-dec-037) |
| 14 | [F14 — What a query over the record costs, and what an SQL engine would cost to ship (B-300, B-042, D6, D20)](#14--f14--what-a-query-over-the-record-costs-and-what-an-sql-engine-would-cost-to-ship-b-300-b-042-d6-d20) |
| 15 | [F15 — What actually needs elevation, asked of a machine (DEC-039, §7.39, §6.32)](#15--f15--what-actually-needs-elevation-asked-of-a-machine-dec-039-739-632) |
| 16 | [F16 — Three defects the real reference model found, and none of them was the one expected (B-213, B-019, §3.7)](#16--f16--three-defects-the-real-reference-model-found-and-none-of-them-was-the-one-expected-b-213-b-019-37) |
| 17 | [F17 — What a hub says when something is not there (DEC-038, §7.38)](#17--f17--what-a-hub-says-when-something-is-not-there-dec-038-738) |
| 18 | [F18 — The wall three findings kept hitting is a packaged toolchain (B-320, B-183, DEC-047)](#18--f18--the-wall-three-findings-kept-hitting-is-a-packaged-toolchain-b-320-b-183-dec-047) |
| 19 | [F19 — Two defects a real model found in an hour, and the tests that could not (B-364, B-365, A19, D38)](#19--f19--two-defects-a-real-model-found-in-an-hour-and-the-tests-that-could-not-b-364-b-365-a19-d38) |
| 20 | [F20 — A second architecture, and two silences that fail differently (B-365, A2, A19, D26)](#20--f20--a-second-architecture-and-two-silences-that-fail-differently-b-365-a2-a19-d26) |
| 21 | [F21 — Six families for a tenth of one model's bytes (B-369, D40, DEC-054, §3.12)](#21--f21--six-families-for-a-tenth-of-one-models-bytes-b-369-d40-dec-054-312) |
| 22 | [F22 — Where a model becomes able to referee an engine (B-370, D40, DEC-054, A19)](#22--f22--where-a-model-becomes-able-to-referee-an-engine-b-370-d40-dec-054-a19) |
| 23 | [F23 — Four expressions where MCF had two, and what no test could have found (B-365, B-370, A7, A19, A21)](#23--f23--four-expressions-where-mcf-had-two-and-what-no-test-could-have-found-b-365-b-370-a7-a19-a21) |
| 24 | [F24 — A third architecture, and three habits that fail three different ways (B-365, B-370, A19, §3.18)](#24--f24--a-third-architecture-and-three-habits-that-fail-three-different-ways-b-365-b-370-a19-318) |
| 25 | [F25 — The broken engine read better than the correct one (B-365, B-368, A19, D38)](#25--f25--the-broken-engine-read-better-than-the-correct-one-b-365-b-368-a19-d38) |
| 26 | [F26 — The oracle found a defect on its first run, and it was an invention of MCF's own (B-368, A19, A21, §3.12)](#26--f26--the-oracle-found-a-defect-on-its-first-run-and-it-was-an-invention-of-mcfs-own-b-368-a19-a21-312) |
| 27 | [F27 — What a coin-flip looks like, and what a defect looks like (B-368, B-365, A19, A21)](#27--f27--what-a-coin-flip-looks-like-and-what-a-defect-looks-like-b-368-b-365-a19-a21) |
| 28 | [F28 — The mask, past the boundary (B-368, F27, A21)](#28--f28--the-mask-past-the-boundary-b-368-f27-a21) |
| 29 | [F29 — The sixth family answers a different question (B-371, DEC-055, B-365, A19, §3.3)](#29--f29--the-sixth-family-answers-a-different-question-b-371-dec-055-b-365-a19-33) |
| 30 | [F30 — Two ways to provision the same component, measured (DEC-052, B-367, D39, A27)](#30--f30--two-ways-to-provision-the-same-component-measured-dec-052-b-367-d39-a27) |
| 31 | [F31 — What the first automated provisioning found in two tries (B-367, DEC-052, A27, §3.15)](#31--f31--what-the-first-automated-provisioning-found-in-two-tries-b-367-dec-052-a27-315) |
| 32 | [F32 — A defect the oracle let through, and the hole it came through (B-364, B-368, F27, A19, A21)](#32--f32--a-defect-the-oracle-let-through-and-the-hole-it-came-through-b-364-b-368-f27-a19-a21) |
| 33 | [F33 — The last schemes, and the widest noise (B-364, B-368, F32, A21)](#33--f33--the-last-schemes-and-the-widest-noise-b-364-b-368-f32-a21) |
| 34 | [F34 — Distributions against distributions (B-373, B-368, F27, F32, F33, A19)](#34--f34--distributions-against-distributions-b-373-b-368-f27-f32-f33-a19) |
| 35 | [F35 — What a load costs, apart from running (DEC-018, D41, B-034, §3.13)](#35--f35--what-a-load-costs-apart-from-running-dec-018-d41-b-034-313) |
| 36 | [F36 — The reference model answers, through an engine that is a process (B-032, B-033, B-367, D39, §XII)](#36--f36--the-reference-model-answers-through-an-engine-that-is-a-process-b-032-b-033-b-367-d39-xii) |
| 37 | [F37 — The first probe found two defects and then refused to answer (B-051, B-052, D42, §3.18, F25, F26)](#37--f37--the-first-probe-found-two-defects-and-then-refused-to-answer-b-051-b-052-d42-318-f25-f26) |
| 38 | [F38 — The probe's answer was upside down, and the decisive column was the broken one (B-052, B-374, D42, §3.18, F25, F37)](#38--f38--the-probes-answer-was-upside-down-and-the-decisive-column-was-the-broken-one-b-052-b-374-d42-318-f25-f37) |
| — | [Changelog](#changelog) |

## 1 · F1 — The adversarial prototype (§7.19, DEC-019)

**What was run.** `prototypes/adversarial` (B-002): interrogate an accelerator
by two routes, supervise four child processes deliberately made to die badly,
classify everything against [taxonomy.md](taxonomy.md), and measure MCF's own
cost against D24.

**Conditions.** One machine: a 16-core AMD Ryzen 9 9950X, 91 GiB of memory, one
NVIDIA GeForce RTX 5080 on driver 610.57.04, Linux 7.1.9, `x86_64-unknown-linux-gnu`.
Rust 1.98.0, release profile. The machine was **not** quiet — it was building
this repository at the same time for one of the runs below, and that run is
reported precisely because the difference is the finding.

**Verdict: D4 is confirmed. §7.19 is closed by DEC-019 without amendment.**

### 1.1 The supervision claim holds, and costs little

Four scenarios, four distinct taxonomy categories, all reached without the
supervisor being affected:

| The child | Category reached | Disposition | Partial output |
|---|---|---|---|
| Is not there to spawn | `engine.spawn.not_found` | `refused` | none |
| Exits 3 before any output | `engine.exit.immediate` | `aborted` | none |
| Prints 18 bytes, then `kill -9 $$` | `engine.exit.signal` | `partial` | **18 bytes kept** |
| Prints 7 bytes, then exits 1 | `engine.exit.midstream` | `partial` | **7 bytes kept** |
| Sleeps 30 s, silent | `engine.hang.no_output` | `aborted` | none |

A3's claim — *the manager survives the managed* — is the one that had to be
demonstrated rather than argued, and it is: the supervisor returns from every
one of these, including the signal death and the hang. A4's claim holds
alongside it: the bytes a child emitted before dying are kept and the
disposition says `partial`, rather than being discarded with the child.

**What this cost in code.** The supervisor is about 180 lines with no
dependency, and the only subtlety is one D4 predicts: the child's output has to
be drained on another thread, or a silent child makes the deadline
unenforceable. That is a concurrency hazard, and it is the kind Rust makes hard
to get wrong by accident.

### 1.2 The C-ABI claim holds, and it matters more than expected

D4's third argument is that accelerator interrogation is "constant C-ABI work,
and Rust pays no tax at that boundary". The prototype probes the same device
twice to test it:

| Route | Answered | What it could not say |
|---|---|---|
| Files the driver publishes (`/proc`, no `unsafe`) | 3 of 5 | device memory, temperature |
| The vendor management library, over the C ABI | **5 of 5** | — |

The file route names the vendor, the model and the driver version and stops.
The vendor library adds the two that are *live state* — 17 094 934 528 bytes of
device memory and 27 °C — and those are precisely the two §3.4 and §3.8 need,
because they vary and they change a result.

**The finding is not that FFI works. It is that the file route is not
sufficient.** A profiler built only on published files would report a device's
identity and be unable to say whether it was thermally throttled or out of
memory, which is exactly the difference §3.8 requires MCF to know between "this
model is slow" and "this machine was busy". DEC-008 has to be decided knowing
that the interesting fields are behind a C ABI.

**The cost was one module and no dependency.** `dlopen`/`dlsym` declared
directly, eight symbols resolved, every pointer null-checked and every status
code checked before its out-parameter is read. `unsafe` is confined to that
module; everything above it sees safe Rust and a classified failure. The
workspace denies `unsafe_code` as a `deny` rather than a `forbid` for exactly
this, and the opt-in carries its reason at the site.

**A boundary this raises for DEC-008 rather than settles.** Intent v23 defers
D23's platform-provided tier for *inference*, on the ground that an unpinned
runtime is an unpinned variable in every result taken through it. Reading a
driver's report of its own state is a different act: nothing MCF publishes is
computed through the library, so there is no result for it to be a variable in.
That distinction is recorded here, not adopted.

### 1.3 The budget is met with room, and it is missing a statistic

| Quantity | D24 | Measured | Verdict |
|---|---|---|---|
| Artifact on disk | ≤ 40 MiB (core binary) | 433 120 B | pass, by two orders of magnitude |
| Resident, nothing loaded | ≤ 20 MiB | ≈ 7.6 MiB | pass |
| Cold start to first response | ≤ 100 ms | median 3.9 ms, p95 9.6 ms, n = 20 | pass |

The subject of the cold-start figure is `mcf --version` — the shortest complete
command MCF has — measured as the whole round trip a shell sees.

**The finding is the statistic, not the numbers.** D24 states *cold start to
first command response ≤ 100 ms* and does not say which statistic that is. On a
quiet machine it does not matter: median 3.9 ms and p95 9.6 ms are both far
under. On a machine that was simultaneously compiling this repository, the same
twenty trials gave a median of 7.2 ms and a **p95 of 165–257 ms** — a passing
median and a failing tail, from one run. D24 specifies "p99" for added latency
and says *the tail is what a user feels*; it specifies nothing for the other
fifteen. §7.50 records the gap and DEC-050 tracks it.

The contended reading is also a demonstration of B35 rather than a defect: a
timing taken under contention measures the contention. B-011's suite has to
either open an exclusive window or mark its results unattributable (B24), and
this run is the evidence that the difference is an order of magnitude rather
than a rounding error.

### 1.4 What the prototype got wrong, which is why it exists

Its first draft measured its own cold start by re-executing itself, and did not
terminate. B18 makes a bug a fixture before it becomes a fix; this one is
recorded here because the lesson generalizes past the bug — a self-measuring
instrument measures itself measuring itself, and B-011's suite will face the
same shape when it measures the daemon's idle cost from inside the daemon.

### 1.5 What is still not measurable

Idle CPU, timer wakeups, memory growth over thirty simulated days and added
request-to-first-token latency are all D24 figures about a **daemon**, and there
is no daemon until M2 (B-030, B-031, B-035). They are reported as *not
measurable at this stage* rather than estimated, because A20 forbids an estimate
that could be read as a measurement and A7 forbids a plausible substitute for
one.

## 2 · F2 — The development machine cannot attribute a budget (DEC-051)

**What was run.** The budget tier (B-011), in release, on the machine MCF is
being written on.

**Conditions.** The same machine as F1: 16 cores, 32 threads. Its one-minute
load average sat between 44 and 47 for the whole session, from work that has
nothing to do with MCF — a language server pool and a long-running application
belonging to the operator. That is not an artefact of measuring; it is what the
machine is.

**What happened.** Every event-class figure came back **unattributable**, which
is what D27 says should happen and is the correct answer. Alongside it, the
readings themselves:

| Figure | Reading | Ceiling | Verdict |
|---|---|---|---|
| Core binary | 548 776 B | ≤ 40 MiB | within |
| Resident memory | 7 655 424 B | ≤ 20 MiB | unattributable |
| Cold start, median of 100 | 8.7 ms | ≤ 100 ms | — |
| Cold start, **p99** of 100 | **42.3 ms** | ≤ 100 ms | unattributable |

The median and the p99 differ by a factor of six on a machine that is simply
being used. F1 saw the same shape at a factor of twenty-five under a compile.
D27's rule — that the tail is what a budget is about — is doing real work here:
a median-based budget would have reported a machine six times better than the
one the operator has.

**The finding is that the tier can assert exactly one figure here, and this is a
gap rather than a defect.** The binary's size is asserted, because a file's
length is read as a single value and does not consult attributability at all —
nothing else running can change it. Everything that goes through a measurement
comes back unattributable, event-class and state-class alike, because D27's rule
is about the run rather than about the kind of figure.

There is no threshold that would fix the event-class half. The machine really is
busy, and a timing taken on it really does measure the contention (B35);
loosening the rule until the reading passed would be choosing the answer.

The state-class half is a smaller and more answerable question, and it is
recorded rather than assumed: a fresh process's resident set is affected by
memory pressure and not by CPU contention, so it is not obvious that the load
average should gate it. D27 did not distinguish, and §7.51 now asks whether it
should.

So MCF currently has no way to assert an event-class budget on a machine
somebody is using, which is most machines. B35 already names the mechanism that
would — an exclusive window, announced, bounded and interruptible — and it is
M6 work (B-181, B-182) built for measuring *models*. Whether MCF's own budgets
should open one, and what a scheduled tier does on a CI runner that is never
quiet, is **DEC-051**.

**What is not in doubt.** The tier is not reporting a failure and calling it a
success, nor the reverse. It reports the readings, states that it is not judging
them and why, and does not refresh its own age (B38). The dishonest outcomes are
the ones this design forecloses.

## 3 · F3 — The load average answers the wrong question, and the obvious fix silently could not fail (DEC-051, D30)

**What was run.** Two experiments, after F2 established that MCF could not
assert an event-class budget on the machine it is written on.

**Conditions.** The same machine as F1 and F2: 16 cores, 32 threads. The load
this time was produced deliberately — thirty-two spinning shell loops, bounded
to the duration of each measurement — because the operator's own workload had
finished and a quiet baseline was available for the first time.

### 3.1 The load average is on the wrong time scale

A hundred `mcf --version` cold starts, quiet and under load, with the measuring
process's own scheduling delay read across the whole measurement:

| | quiet | under load |
|---|---|---|
| Cold start, median | 208 µs | 1 041 µs |
| Cold start, **p99** | **360 µs** | **4 822 µs** |
| Wall time for the whole measurement | 21.8 ms | 139.1 ms |
| Own runqueue wait | 4 148 ns | 15 477 083 ns |
| **Wait as a fraction of the measurement** | **0.019 %** | **11.1 %** |
| **One-minute load average** | **0.29** | **0.29** |

The p99 moves by a factor of thirteen. The load average does not move at all —
not because it is imprecise, but because a **one-minute average cannot answer a
question about a 140-millisecond measurement**. By the time it responds, the
measurement is long over. The scheduling delay moves by a factor of 585 and does
so inside the window it is describing.

This is what D30 rests on, and it also corrects an assumption F1 and F2 were
reasoning under. The cold-start figures those recorded — 6 to 8 ms median — were
themselves contaminated: on a genuinely quiet machine the median is **208 µs**,
about thirty times faster. Every one of MCF's budget figures had been measured on
a machine that was never quiet, and none of them was wrong about passing, but all
of them were wrong about the number.

### 3.2 The obvious implementation could not fail

The first implementation read `/proc/self/schedstat`. It reported
**0.0000 % in both states** — quiet and under thirty-two spinners — which is
exactly what F2's failure looked like from the other side, and it was found by
the negative control rather than by review.

The cause: a process's accounting is its **main thread's**. Every measurement MCF
takes under a test harness runs on a worker thread, and the main thread is
blocked waiting for it, so it never queues. Reading `/proc/thread-self/schedstat`
instead:

| Same 300 ms of work, on a worker thread | quiet | under load |
|---|---|---|
| `/proc/self/schedstat` — the process | 0.0000 % | **0.0000 %** |
| `/proc/thread-self/schedstat` — the thread | 0.0207 % | **50.46 %** |

**The finding is about method as much as about the reading.** A signal that
cannot fail is not a signal, and this one silently could not. What caught it was
running the budget tier under deliberate load and checking that the verdict
*changed* — the negative control B-003's lint check had already established as
necessary, applied to a measurement instead of to a lint. A check that has only
ever been observed to pass has not been observed.

### 3.3 What changed as a result

Every budget figure is now asserted on this machine, quiet, and correctly
refused under load:

| Figure | quiet | under thirty-two spinners |
|---|---|---|
| Core binary | 617 360 B — within | within (unaffected by load) |
| Resident memory | 7 696 384 B — within | within |
| Cold start, p99 | 575 µs — **within** | 21.8 ms — **not attributable** |
| Record write, p99 | 3 446 ns — within | — |

DEC-051's deadlock is gone rather than traded off: B38 and D27 both still hold,
and the tier refreshes its age whenever a reading was clean.

## 4 · F4 — What the new tiers found on their first runs (B-191)

**What was run.** The five tiers B-191 built — property, whole-system, fuzz,
load, soak — and the mutation runner, each on its first pass over the tree they
were written against.

**Conditions.** The same machine as F1–F3: 16 cores, 32 threads, 91 GiB. Debug
profile, because these are tests rather than measurements of the artifact;
`/tmp` is tmpfs, which matters for every figure below that involves a file. No
timing figure appears here: A18 keeps a throughput assertion out of a test tier,
and these tiers assert counts and invariants rather than speeds.

**Why this is a finding and not a commit message.** Four of the six things below
are about *MCF*, and three of those were not visible to any tier that already
existed. The fifth and sixth are about the tiers themselves, and D10's whole
argument for mutation testing is that a suite is a thing that has to be checked.

### 4.1 A condition did not round-trip through the record

The property tier's first run falsified `a_condition_floor_survives_the_record_including_its_unknowns`
at its first case. `encode::conditions` rendered every known condition through
`Display`, so `ConditionValue::Integer(4096)` — a context length, which is the
one §3.4 floor question that is naturally a number — was written as `"4096"` and
read back as `Text("4096")`.

Nine of the ten floor questions never noticed, because nine of them are
naturally strings, and B-007's round-trip test used a floor captured from a live
machine where the numeric question is `Unknown`. That is exactly the gap a
property closes: the existing test asked about the floor MCF happens to produce
today, and the property asks about every floor MCF can represent.

B-007's stated condition — *the §3.4 floor round-trips through the record store
losslessly* — was therefore false, and had been since the capture path was
written. Fixed, with the failing case kept as a fixture.

### 4.2 A replay holds the whole journal, and it costs about a kilobyte an entry

The soak tier, over a hundred thousand appends:

| | |
|---|---|
| Growth of the writer over 100 000 appends | **0 bytes** |
| Resident held by a replay of the same journal | ~100 MB |
| Per entry, for an entry with one integer field | **~0.8–1.0 kB** |

The first number is the leak check and it is asserted. The second is
proportional by construction — `Replay` returns every entry it read — and it is
D20's design rather than a defect: the journal is the record, and the index over
it is derived. It is **reported and not asserted**, because asserting on it
would be asserting that MCF never keeps a record long enough to matter.

It is worth keeping because it puts a number on why D6's derived index exists
(B-042, B-300): at a kilobyte an entry, a machine that has run a few million
trials cannot answer a question by replaying its journal into memory.

### 4.3 A process-wide reading cannot be taken beside another test

The soak tier reported a 70 MB leak in the journal writer. There is no leak: the
harness runs tests in parallel by default, resident memory is a property of the
*process*, and the reading had another test's replay in it.

This is B35's rule — *a reading taken under contention measures the
contention* — arriving at memory instead of at a timing, and the resolution is
the same one D30 reached: separate the reading from the thing that is not being
measured. The tier runs on one thread, and measuring the writer and measuring a
replay are two tests rather than two readings inside one.

### 4.4 The fuzz campaign reaches the accept path

Four parsers, 200 000 cases each by default, nothing found. That sentence is
worth very little on its own — a campaign that refused every input would say the
same thing — so each target counts what it reached:

| Target | Damaged inputs accepted | Refused |
|---|---|---|
| `json::parse` | 11 484 | 89 623 |
| `time::Zone::parse` | 20 637 | 179 363 |
| `journal::replay` | 2 916 | 2 084 |
| `export::read` | 181 | 4 819 |

The two that write files run a fortieth of the cases, which is why their totals
are smaller. `export::read`'s acceptance rate is low by construction: a bundle
states a digest over its own contents, so most damage is refused by the check
the reader exists to make. A target that never accepted anything would fail the
tier rather than pass it.

### 4.5 The mutation runner was judging mutants against the mutants before them

The first working run of `scripts/check-mutants.sh` reported the `export` mutant
as having hung. It does not hang: run alone, a unit test kills it in a tenth of
a second.

What hung was the **digest** mutant, still compiled in. Cargo decides what to
rebuild from modification times, and the runner restored each file by moving
back a backup taken *before* the mutation — so the restored file was older than
the build made from it, cargo considered the crate fresh, and did not rebuild.
The consequence is more general than the symptom: every mutant after the first
was judged against a tree that still carried the earlier mutations, wherever
those lived in a crate that had no other reason to be recompiled. A mutation in
`mcf-core` survived a later mutation in `mcf-record`, because nothing asked for
`mcf-core` again.

That direction of error inflates a score rather than deflating it — extra
breakage makes a mutant easier to kill — which is the kind that a green run
hides. The fix is one `touch` on the restored file.

The reason it took three runs to find is the part worth keeping. The
equivalent-mutant control runs **first**, before anything has leaked, so it
cannot see this class of defect at all. What sees it is a comparison of the
whole copy against the tree at the **end** of the run, which the runner now
makes and which refuses to report a score if a restore did not take. A control
at the start of a run and a verification at the end answer different questions,
and this tier needed both.

### 4.6 The score, and the mutant that hangs

Eleven mutants, each breaking something a rule depends on:

| | |
|---|---|
| Killed | **11** |
| Survived | 0 |
| Invalid (did not compile) | 0 |
| Of those killed, by hanging | 1 |

A 11-of-11 score is not evidence that the suite is complete; it is evidence
about eleven specific claims, chosen because a rule rests on each. What it does
establish is that the tier itself works — the control is not killed, a mutant
that does not compile would be excluded rather than counted, and the run refuses
to report at all if the copy is not the tree again afterwards.

The hanging mutant is the digest one: it takes the room left in a 64-byte buffer
from 64 to 63, so the filling loop eventually takes zero bytes per pass. Every
suite run is bounded, a timeout is confirmed by a second run before it is
believed, and what a hung run leaves behind is reaped — a spinning test process
outliving the tier is a change to the machine A27 does not permit MCF's own
suite to make either.

## 5 · F5 — The cold-start budget is a measurement of the filesystem (B-011, D30)

**What was run.** `scripts/ci.sh --all`, the first invocation that runs every
tier B-191 declares. The budget tier failed: cold start, p99 of 100 trials,
**252 ms** against D24's ceiling of 100 ms, and the reading was judged
*attributable* rather than refused. A later run of the same command, on the same
machine, passed the same figure at 0.58 ms — which is 5.3 below, and is the part
that settles it.

**Conditions.** The same machine as F1–F4. The one condition that turned out to
matter is one nothing was recording: **the repository lives on
`/home/gauge/Content`, which is a `fuseblk` mount**, and the artifact under test
is executed from there. `/tmp` is tmpfs.

### 5.1 The same binary, two filesystems

A hundred spawns of `mcf --version`, release profile, from each location, timed
outside MCF to keep the instrument out of its own finding:

| The binary is read from | median | p99 | minimum |
|---|---|---|---|
| `fuseblk` — where the repository is | 0.362 ms | **416.081 ms** | 0.252 ms |
| `tmpfs` — a copy of the same bytes | 0.222 ms | **0.390 ms** | 0.212 ms |

The medians differ by a factor of 1.6. The **p99s differ by a factor of 1 067**.
The same file, the same machine, the same instant: what differs is the
filesystem the kernel faults the pages in from, and a FUSE filesystem
occasionally takes hundreds of milliseconds to serve one.

D27's choice of the 99th percentile is doing exactly what it was chosen to do —
*the tail is what a user feels* — and what it caught here is real. It is simply
not about MCF.

### 5.2 Two things this says, and neither is that the budget is wrong

**The storage an artifact is executed from is a measurement condition, and MCF
does not record it.** §3.4's floor asks ten questions and none of them is *where
did this come from*. Two runs of one binary, with identical stated conditions,
differ by three orders of magnitude in the statistic D24 is written in. A reader
handed both numbers could not tell which was which, which is precisely what A6
exists to prevent.

**D30's attributability signal cannot see this, by construction.** It reads the
*measuring thread's* time on the runqueue: the question "was this reading
affected by contention for the processor" (F3). During a cold-start measurement
the measuring thread is blocked in `wait4` and is not runnable at all, while the
child faults its pages in from a slow filesystem. The delay signal stays near
zero and the reading is judged clean, which it is — of the thing the signal
measures.

So MCF has a signal for one kind of contamination and no signal for another, and
the tier's only event-class figure happens to be dominated by the second. That
is a gap in the instrument rather than in the number: B-193 registers both
halves, and until it is built the cold-start figure means *what a cold start
costs on this storage*, which is worth knowing and is not what D24 asked for.

### 5.3 The same tier passes and fails on the same machine within the hour

Run again twenty minutes later, with the mount's page cache warm from the run
before it, the tier was green:

| Run | Cold start, median | Cold start, p99 | Verdict |
|---|---|---|---|
| After the mutation tier had written ~10 GB to tmpfs | 152.2 ms | 252.4 ms | **over** |
| With the mount's cache warm | 0.370 ms | 0.582 ms | within |

A factor of four hundred on the median, between two runs of one command on one
machine, with every condition MCF records identical. That is the sharper form of
what 5.2 says: the figure is not merely mis-attributed, it is **not
reproducible**, and P3
puts reproducibility above convenience. A budget that passes or fails according
to what else has been evicting the page cache is not yet a budget.

It is also why the fix is two halves rather than one. Recording the filesystem
would make the two runs legibly different; only the attributability half makes
the second one *refuse* rather than report.

### 5.4 What this does not change

The tier is behaving as designed in the part it can see: the state-class figures
assert, the reading was reported with its statistic and its sample count, and
the failure was loud. B-011 stays in progress, and it now has a stated reason
beyond the missing baseline.

Nor does it change what B-191 established. `scripts/ci.sh --all` ran all ten
tiers; nine of them were green and the tenth failed on a real reading, which is
the outcome a tier exists to produce.

## 6 · F6 — The first mutant to survive (B-186)

**What was run.** `scripts/check-mutants.sh` against a copy of the tree carrying
a twelfth mutant, added to check that B-186's floor actually refuses. The mutant
was chosen because it looked likely to survive: `resident_bytes` multiplies the
kernel's kibibytes by 1024, and the mutant multiplies by 1000.

**Conditions.** The same machine as F1–F5, debug profile, the whole workspace
suite as the judge.

**It survived.** Eleven of twelve killed, 91 %, and the two refusals fired
exactly as intended — below the floor, and below the previous run's score.

### 6.1 What the survivor was hiding

Nothing checked that MCF's resident-memory reading is in bytes. The existing
test asserted a plausibility band — greater than zero, less than 64 GiB — which
a figure 2.4 % wrong passes without difficulty.

That figure is not decorative. `mcf doctor` prints it, and B-011 asserts it
against D24's 20 MiB ceiling. A19 requires that anything reported be tested
against an independently known value, and this one was tested against itself.

The fix is two independent facts about the same quantity: the kernel reports
`VmRSS` in kibibytes, so the value in bytes is a whole number of pages; and
`/proc/self/statm` counts the same pages in a different file. A mutant
multiplying by 1000 lands off the page boundary and dies.

### 6.2 What it says about the number 100 %

The floor is a hundred per cent of a hand-written catalogue, which is a much
weaker claim than a hundred per cent of everything a generator could produce —
and it is the claim worth making. Each entry breaks something a rule in
rules.md rests on, so the score answers *are these twelve claims checked*
rather than *what fraction of arbitrary edits does the suite notice*.

The loop the tier exists for ran in full here: a mutant survived, the claim it
broke got a test, and the mutant joined the catalogue. A tier that reported 91 %
and moved on would have left the same gap with a number attached to it.

## 7 · F7 — A major page fault is the signal F5 was missing (B-193)

**What was run.** Thirty spawns of one binary, twice: once with the file
resident, once with its pages evicted from the cache before each spawn, counting
the major page faults the kernel charged to this process's children.
`scripts/check-fault-signal.sh` is the experiment, and it runs in the gating
tier.

**Conditions.** The same machine as F1–F6. The probe ran from
`$XDG_CACHE_HOME`, which is btrfs; the repository's own filesystem is a FUSE
mount that does not honour the eviction hint, which is why the script tries
several directories and names the one that worked.

### 7.1 The reading

| | Major page faults over 30 spawns |
|---|---|
| Warm — the file resident | **0** |
| Evicted before each spawn | **30** |

One per spawn, exactly, and none at all when the file is where a measurement
wants it. That is what makes the threshold **zero** rather than a judgement like
`TOLERATED_DELAY_PPM`: a major fault is the kernel going to a device, and a
measurement that took one waited on that device.

### 7.2 Why the other signal could not see it

D30's signal reads how long the *measuring thread* was runnable and not running.
During a cold-start measurement that thread is blocked in `wait`: it is not
runnable, so it accrues no delay, and the reading comes back clean however long
the child spent faulting its pages in. F5 is that blind spot with a number on
it — a figure two and a half times its ceiling, judged attributable.

The two signals are not a refinement of one another. They answer different
questions — *was this thread queuing for a processor* and *did this work go to a
device* — and a measurement can fail either. Where both hold, MCF reports the
device, because a busy machine is somebody else's compile finishing and an
artifact that was not resident is a property of where it lives.

### 7.3 What the pair now does

A cold start on slow storage is **refused as unattributable** rather than
reported as over its ceiling, which is what B-193 asked for. Beside it, the
storage the artifact was read from joined the condition floor as its eleventh
question, so the two runs 5.3 above compared — 152 ms and 0.37 ms on one machine
within the hour — are now legibly different rather than mysteriously so, and the
budget tier refuses to compare a reading with a baseline taken from different
storage (A8).

**What is still true and unfixed:** a p99 over a hundred trials moves by about a
quarter between runs on an idle machine, so the cold-start figure is still
recorded rather than judged against its baseline. The ceiling judges it; the
tolerance would fire on the tail.

## 8 · F8 — How far a kernel MCF could maintain is from a specialist's (DEC-004)

**What was run.** `prototypes/kernel-slope`: one 512×512 single-precision matrix
multiply — the operation an inference engine spends nearly all of its time in —
four ways, each of them something MCF could actually write and maintain in safe,
portable Rust. Then the same multiply through a tuned BLAS present on the
machine, single-threaded, for the other end of the slope.

**Conditions.** The same machine as F1–F7, release profile, five trials per
variant, operands generated from a fixed seed so two runs are comparable. Every
variant's product is compared bit-for-bit against the definition before its time
is kept: a kernel that is fast and wrong is not a data point (A19). The machine
was **not** quiet — a one-minute load average of about 34, from an editor's
indexers — and the single-threaded readings came back attributable at around
0.1 % queuing while the threaded one did not, which is D30 doing its job.

### 8.1 The readings

Medians of three runs of the prototype:

| Variant | Median | Against the definition |
|---|---|---|
| Naive — the definition, in the obvious loop order | 226–270 ms | ×1 |
| Reordered — the same arithmetic, loops walking memory forwards | 53–86 ms | ×3–5 |
| Blocked — tiled to keep a working set in cache, written carefully | 126–143 ms | ×1.6–1.9 |
| Blocked and threaded — the same across 32 threads | 19–27 ms | ×9–14 |
| **A specialist's kernel** — OpenBLAS, **one** thread | **1.5–2.5 ms** | **×100–170** |

Two ratios matter and both are measured here rather than assumed:

- The best MCF could do **single-threaded** is 53–86 ms against 1.5–2.5 ms.
  **A specialist's single core is twenty-five to fifty times MCF's best.**
- MCF using **all thirty-two threads** is 19–27 ms, still **about ten times
  slower than one** of that specialist's cores.

### 8.2 The second step of tuning made it worse

The blocked variant — the careful one, the one that looks like optimization — is
consistently **slower** than the one-line loop reorder. Cache blocking without
operand packing and without a register-blocked microkernel adds loop overhead
and address arithmetic for a locality benefit the reorder had already collected
at this size.

That is not a defect in the prototype; it is the finding. From the bottom of
this slope, the *sign* of an optimization is not obvious, and getting it right
means measuring each step on each machine — which is the treadmill §7.4 was
weighing, seen from the first rung.

### 8.3 What the remaining distance is made of

The gap to the specialist is not algorithmic. It is hand-written SIMD
microkernels per instruction set, operand packing into contiguous panels,
prefetch scheduling, and a different code path per generation of processor.
OpenBLAS reports itself here as `DYNAMIC_ARCH Haswell` — a *generic* kernel,
not one tuned for this processor — and it is still fifty times MCF's best.

None of that is reachable from where MCF stands. The workspace denies
`unsafe_code` for the reason §3.16 gives, portable SIMD is not in the stable
standard library, and every new processor and accelerator moves the target.
Owning it would mean owning it for ever.

### 8.4 What this settles

§7.4 stated the likely answer — *MCF's performance mandate applies to MCF's own
overhead, not to the inference kernels* — and refused to settle on a reading
alone. This is the measurement that reading needed, and it points the same way
by one to two orders of magnitude. D32 is the decision.

It also bounds the cost of the *other* half of that architecture. MCF's own
overhead, measured on this machine, is a cold start of a few hundred
microseconds and a record write of a few microseconds (F3, D24). A single 512³
multiply on a specialist's kernel is 1.5 ms — and a real model's forward pass is
thousands of those. Whatever MCF's wrapper costs, it is not where the time goes,
which is exactly why §VII's mandate belongs there and nowhere else.

## 9 · F9 — What a network costs, and what the hub actually does (B-021)

**What was run.** `prototypes/transport-cost/measure.sh`, on 2026-08-25: ask
the real hub for sixteen bytes of a real model over HTTP/1.1 and keep the
headers, then build the smallest program that could do the same thing in three
shapes and measure what each drags in and what it demands of a machine.

**Conditions.** The machine of F1. Rust 1.98.0, release profile, `cargo vendor`
against crates.io as it stood on the day. Everything was built outside this
repository: measuring a candidate is not admitting one. The machine was not
quiet — two other projects held the exclusive window — which does not matter
here, because nothing timed is reported.

**Why it was run.** Four items sat behind the same sentence: *what remains is
the transport*. B-021 has a fetcher with no socket, B-213 has arithmetic with
no metadata to do it on, B-024 has credentials nothing offers, B-029 has two of
three commands. MCF's tree has no third-party code at all, the hub speaks
HTTPS, and no decision had crossed that boundary. An argument about it would
have been an argument about numbers nobody had.

### 9.1 The hub is simpler than feared, and says more than expected

Every line below is a requirement on whatever MCF writes, and every one of them
was a guess beforehand.

| What was asked | What came back | What it settles |
|---|---|---|
| `--http1.1` | `HTTP/1.1 206` | **No HTTP/2 is needed.** The whole h2 stack — framing, HPACK, flow control — is off the list |
| A model file by its repository path | `302` to a signed URL on another host (`us.aws.cdn.hf.co`) | Redirects cross hosts, so a client must follow them **and must not carry the credential across** |
| `Range: bytes=0-15` | `206 Partial Content`, `content-range: bytes 0-15/396705472` | Resumption works, and B-021's `fetch_from` has a real counterpart |
| The same request, headers kept | `x-linked-size: 396705472`, `x-linked-etag: "ac2d977…d524a"` | The **declared size and SHA-256 arrive before the bytes do** — `Entry::declaring` is what the hub already offers |
| The same | `x-repo-commit: 50968a44…` | The revision to pin at acquisition (B-019) is in the response to the download itself |
| Nothing (no credential) | `x-hf-warning: unauthenticated`, `ratelimit-policy: "fixed window";"resolvers";q=3000;w=300` | Throttling is *published*, so `Behaviour::RateLimited`'s hint is real rather than invented |

### 9.2 The cost is cryptography, not protocol

Three shapes, each the smallest program that does the job:

| Shape | Crates | Vendored tree | Not Windows | Rust source | Artifact demands |
|---|---|---|---|---|---|
| A client and its TLS (`ureq`) | 42 | 91 MiB | 19 MiB | 31 MiB | `libc`, `libgcc_s`, the loader |
| TLS only (`rustls` + `ring` + roots) | 27 | 87 MiB | 15 MiB | 28 MiB | the same |
| The machine's own TLS (`native-tls`) | 5 | under 1 MiB | — | — | **`libssl.so.3`, `libcrypto.so.3`** and the three |

Three things fall out of that table.

**Most of a vendored tree is Windows import libraries a Linux build never
compiles.** 72 of the 91 MiB are `windows-sys` and its target crates. Reporting
the headline number would have overstated the cost fivefold, which is how a
real objection gets dismissed for the wrong reason. The honest figure for the
TLS-only shape is **sixteen crates and 15 MiB**: `cc`, `cfg-if`,
`find-msvc-tools`, `getrandom`, `libc`, `once_cell`, `ring`, `rustls`,
`rustls-pki-types`, `rustls-webpki`, `shlex`, `subtle`, `untrusted`, `wasi`,
`webpki-roots`, `zeroize`. Six of those are a few hundred lines each.

**The HTTP client is the cheap half.** Fifteen crates and 4 MiB separate *TLS
only* from *a client and its TLS*. So the question is not whether to write an
HTTP client — that saves little and is easy to test — it is whether to vendor
cryptography at all, and there is no third option: 9.1 says the hub answers
nothing that is not TLS.

**The cheapest thing to vendor is the one thing MCF may not ship.** The
system-TLS shape needs almost nothing and produces a binary that demands
`libssl.so.3` and `libcrypto.so.3` — two libraries [vendored.md](vendored.md)
§3a does not list, whose versions differ between distributions, and which B36
would make the user's errand. It is in the table as the control: the shape that
fails, failing where a check can see it rather than on somebody's machine.

### 9.3 What the byte counts do not show

`ring` is C and assembly, so vendoring it puts **a C compiler in MCF's build**
— visible in the tree as the `cc` crate. The artifact does not gain a
dependency (row one of the table is the evidence), but the *build* gains a
prerequisite, and §3.12's reproducibility claim would then rest on a second
toolchain nothing pins. That is a cost to state rather than to discover, and
it is the strongest argument any pure-Rust provider has.

Against that: **MCF cannot write this.** The rest of the tree is code MCF could
in principle maintain; a TLS 1.3 implementation is not, and A19 forbids
claiming what is not tested. The alternative to vendoring cryptography is
having no network, and having no network is the end of §III.

### 9.4 The tree cannot be trimmed to the platform

The obvious answer to *72 of 91 MiB are Windows import libraries* is to vendor
only what this platform builds. Cargo will not have it: with a vendored source
directory it resolves the whole lock graph before it compiles anything, so a
tree with `windows-sys` removed fails to build on Linux — not because Linux
needs it, but because the resolution does.

Measured rather than assumed, and it changes the number that matters. Admitting
a TLS stack means **about 90 MiB of third-party source in the repository**, not
15. The 15 MiB is what a Linux build *compiles*, which is the right number for
reviewing what MCF ships and the wrong one for what a checkout costs.

That is a cost worth stating in one place rather than discovering in a diff, and
it is the reason B-322's remaining step is a deliberate admission rather than
another commit: everything else the transport needs is written and tested, and
what is left is one struct and a decision about ninety megabytes.

### 9.5 The obvious provider costs a claim MCF already makes

`rustls`'s usual cryptography is `ring`, which is C and assembly. Building the
`tls-ring` shape for `x86_64-unknown-linux-musl` — the target
`scripts/check-from-scratch.sh` uses to put MCF in a container with *nothing*
in it — fails on this machine:

```
error occurred in cc-rs: failed to find tool "x86_64-linux-musl-gcc"
```

So admitting `ring` does not merely add a C compiler to the build (9.3): it adds
a **cross** C toolchain, without which B-183's from-scratch check cannot run at
all. The check already distinguishes *could not be checked* from *passed*
(exit 2), so nothing would be silently lost — but a claim that can only be
tested where a particular toolchain is installed is a claim that gets tested
less.

### 9.6 A provider MCF can build anywhere, and what it costs

`rustls-graviola` is a provider written in Rust and inline assembly, by
`rustls`'s own author. Measured against the same questions:

| | `ring` | `rustls-graviola` |
|---|---|---|
| Crates compiled here | 16 | **14** |
| Vendored tree, filtered to what compiles | 22.9 MiB | **13.0 MiB** |
| C or assembly *source files* in the tree | 93 | **0** |
| Builds for `x86_64-unknown-linux-musl` | no, without a cross toolchain | **yes** |
| A TLS 1.3 session to the real hub | — | **yes**: `HTTP/1.1 200 OK`, 9937 bytes, `TLSv1_3` |

The last row is the one that matters and it was run rather than assumed: a
sixty-line program using `rustls` over a plain socket, against
`huggingface.co`, returning the same listing 9.1 fetched with `curl`.

**And the tree can be filtered after all.** 9.4 said a vendored tree cannot be
*trimmed*, and that is true of deletion — cargo resolves the whole lock graph.
It is not true of **stubbing**: a crate that no target MCF builds ever compiles
can keep its manifest, its licence files and an empty `lib.rs`, with a checksum
file that lists no files. Both the glibc and musl builds then succeed
`--offline --locked`. That is what `scripts/vendor.sh` does, and it is why the
number above is 13 MiB rather than 95.

**What this costs, stated rather than discovered.** `graviola` is young — the
mature choices are `ring` and `aws-lc-rs`, and both are C. MCF is choosing a
newer implementation to keep a check it already makes runnable on any machine,
and the exchange is worth stating: a weakness here is a weakness in what MCF
verifies a hub with. Two things bound it. The provider is one line behind
[`mcf_hub::wire::Wire`], so swapping it is a change to one struct rather than to
the acquisition path. And the digest MCF checks the bytes against arrives with
the listing rather than with the file, so a source that substitutes weights has
to substitute both — which does not make TLS optional, and does mean TLS is not
the only thing standing there.

[`mcf_hub::wire::Wire`]: ../crates/mcf-hub/src/wire.rs

**Verdict: the transport is TLS-shaped.** The evidence says vendor the
cryptography and own the protocol — the same shape D32 settled for inference,
for the same reason: delegate what specialists maintain, own the wrapper that
has to be correct. What MCF writes is the HTTP/1.1 client, because 9.1 shows it
is small, and because the two behaviours that matter — a redirect that must not
carry a credential, and a resumption that must verify — are precisely the ones
the laboratory has to be able to simulate.

Admitting a component is [vendored.md](vendored.md)'s business and B-322 is
where it happens. This finding is the stated reason B15 requires.

## 10 · F10 — What a machine says when a network is missing (DEC-011, D33)

**What was run.** A sixty-line program on 2026-08-25: resolve a name that
exists, resolve one that cannot, and connect to three addresses that answer in
three different ways. What is being measured is not the network — it is *what
the platform tells a program*, which is all MCF ever has.

**Conditions.** The machine of F1, on a working network, Rust 1.98.0. Nothing
timed is a measurement of anything but the platform's own paths.

| Asked | What came back | How long |
|---|---|---|
| Resolve a real name | an address | 60 ms |
| Resolve `this-name-does-not-exist.invalid` | `ErrorKind::Uncategorized`, no errno, *failed to lookup address information* | 0.5 ms |
| Connect to a closed port here | `ErrorKind::ConnectionRefused`, errno 111 | 0.2 ms |
| Connect to `203.0.113.1` (TEST-NET-3) | `ErrorKind::TimedOut`, no errno | the deadline |
| Connect to `10.255.255.1` | `ErrorKind::TimedOut`, no errno | the deadline |

### 10.1 A failed lookup is one observation with three causes

The row that decides DEC-011 is the second. A name that will not resolve
produces `Uncategorized` — **no error kind at all** — whether the cause is a
machine with no network, a machine with no resolver, or a name that does not
exist. The platform does not distinguish them and neither, therefore, can MCF.

That is not a gap to be filled by inference. MCF could *guess* by trying
something else — a second resolver, a known-good address, a ping — and each of
those is a network request nobody asked for, which §3.2 refuses and §3.13's idle
discipline refuses again. So the honest report is the observation: *this machine
could not turn that name into an address*, with the three causes named as what
the sentence does **not** say (A7).

### 10.2 Refused, no route, and silence are three different things

The other rows are worth keeping apart, because they are three different things
for a person to do something about. A refusal means something is there and said
no — there is a working path. No route means this machine cannot get there at
all, which is what an unplugged machine looks like from inside a program.
Silence means MCF's own deadline ended the wait rather than the far end, which
is a statement about MCF's patience and not about the network.

`mcf_hub::wire::Tcp` now says which of the three it saw, and the gating tier
holds two of them without a network: `.invalid` never resolves (RFC 2606), and
a closed loopback port is always refused.

### 10.3 What already works with nothing

The other half of §7.11's question needed no new experiment, because B-183's
from-scratch check answers it every time it runs: `--version`, `licence` and the
whole of `doctor` — the hardware profile, the self-cost measurement over a
hundred process spawns, and the laboratory reproducing every failure MCF claims
to handle — run in a container with **no network interface, no libc, no shell
and no `/etc`**. `list` and `rm` read and move local files and need nothing
either. The only command that needs a network is the one that fetches, which is
the answer §3.2 suggested and nobody had stated.

**Verdict: D33.** Offline is the ordinary case rather than a degraded one, and
what MCF says when a network is needed and missing is what it observed —
never which layer is absent. *No internet* versus *no local network* is a
distinction an operator draws by pointing `--from` at a mirror, which is a
request MCF was asked to make; it is not one MCF asserts by probing, which would
be traffic nobody asked for.

## 11 · F11 — A disk fills at the flush, not at the write (B-026)

**What was run.** Two writes to `/dev/full` on 2026-08-25 — a device every Linux
machine has that accepts everything and stores nothing, answering every write
with `ENOSPC`. One direct, one through a buffered writer.

| Written | What came back |
|---|---|
| `write_all` straight to the device | `ErrorKind::StorageFull`, errno 28, at the write |
| `write_all` through a `BufWriter` | **`Ok(())`** |
| the `flush` that followed it | `ErrorKind::StorageFull`, errno 28 |

**The middle row is the finding.** A small write into a buffer succeeds, because
nothing has reached the filesystem yet. A fetcher that checked its writes and
ignored its flush would have been told the truth and then discarded it — and
would go on to verify a digest over bytes that are not on the disk, rename a
file that was never written, and record an acquisition that did not happen.

MCF's transfer path flushes and classifies both, and `hub/no-room-on-the-disk`
is the scenario that holds it: the laboratory writes a transfer to `/dev/full`
and the outcome is `resource.disk.exhausted` rather than a success or a general
write failure.

**The other half of §3.11 is the number before the transfer.** A file that will
not fit is refused *before* a byte moves, with the arithmetic in the refusal —
what it needs, what is available, what it is short by, and which filesystem. The
platform has that number and the standard library does not expose it, so
`mcf_core::hardware::space` is the second module in the workspace to take the
`unsafe_code` opt-out: one `statvfs` call, a status checked before any field is
read, and `Unknown` wherever it fails, because *MCF could not look* and *there
is no room* are opposite answers (A7). The reading is checked against `df` — a
different program by other people (A12) — within a hundredth of the filesystem's
size, which is what two readings of a number in motion are worth.

**What it does not buy.** Certainty. The room is what the kernel said at the
moment it was asked, and another process can take it a moment later. That is why
both halves exist: the check turns the common case from a surprise into a
refusal, and the classification catches the case the check cannot.

## 12 · F12 — What an engine would cost, as far as it has been measured (B-320)

**What was run.** `prototypes/engine-cost/measure.sh` on 2026-08-25: fetch the
two candidates §XVI's vendoring constraint leaves standing, and measure what
each would be as a *thing to ship* — how much source, under what terms, needing
which toolchain.

**Why the question is not "which is fastest".** D32 settled that MCF delegates
inference; [vendored.md](vendored.md) asks for a compatibility finding before
anything ships; and B15 admits weight against a stated cost. Speed is what an
engine is *for* and is measurable later, on this machine, by the suite MCF is
building. What has to be known first is whether admitting one breaks something
MCF already promises.

### 12.1 What is measured

| | llama.cpp | candle |
|---|---|---|
| What it is | C++ and CMake, one upstream | Rust, no C++ toolchain |
| Licence | MIT | MIT or Apache-2.0 across 152 crates |
| Revision looked at | `d222767c7a651655` | crates.io as it stood on the day |
| Source MCF would ship | **35 MiB** — `ggml`, `src`, `common`, `include`, `vendor` | **119 MiB unfiltered**; the stubbing F9.6 uses would cut it, by how much is unmeasured |
| Lines of C or C++ | **464,000** | 94 C or assembly files inside otherwise-Rust crates |
| Third parties to account for | **one** | **152** |
| Build needs | CMake ≥ 3.14 and a C++ compiler | `cargo`, and a C compiler for a few crates |

The whole llama.cpp checkout is 167 MiB, of which `models/` and `docs/` are 110;
those are not what a vendored engine would carry, and quoting the headline
number would overstate the cost the way F9.2's 91 MiB did.

### 12.2 The shape of the trade, stated before it is made

**One upstream against a hundred and fifty.** llama.cpp is 464,000 lines of C++
from one project, under one licence, at one revision — a great deal of code and
a single thing to account for. candle is a hundred and fifty-two crates, each
its own project with its own terms and its own release cadence, and
[vendored.md](vendored.md) §2 would grow a row for every one. Neither is
obviously the smaller obligation: one is more code to read and less bookkeeping,
the other the reverse.

**The toolchain is where F9.5's lesson applies again.** This machine has `cmake`
and `g++` and **no musl cross toolchain**, which is exactly the condition that
decided the TLS provider: a candidate that cannot be built for
`x86_64-unknown-linux-musl` does not fail a test, it makes B-183's from-scratch
check runnable in fewer places. For a C++ engine that question is open and
serious; for a Rust one it is likely to be the same answer TLS got.

### 12.3 The deciding half, measured in the exclusive window

Both candidates were built. The numbers below were taken with `heavy` holding
the machine, because a build this size run beside somebody else's measurements
spoils them (B35, and section 12 of the build document).

| | llama.cpp | candle |
|---|---|---|
| Builds for `x86_64-unknown-linux-gnu` | **yes** | **yes** |
| What it produces | `libllama.a` 9.9 MiB + `ggml` 2.9 MiB = **12.9 MiB** of static libraries | a 512³ matmul program: **1.6 MiB**, needing only `libc`, `libgcc_s` and the loader |
| Builds for `x86_64-unknown-linux-musl` | not attempted: it is C++, and this machine has no musl C++ compiler | **no** — `onig_sys`, a C library, fails for want of `x86_64-linux-musl-gcc` |
| Crates in the graph | one project | **143**, for `candle-core` *alone* |

### 12.4 The musl question stops separating them

F9.5's lesson was that a candidate which cannot build for musl costs MCF a check
it already makes. That is what chose the TLS provider, and the expectation going
in was that it would choose the engine too — Rust over C++, for the same reason.

It does not, and the reason is worth recording. `candle-core` depends on
`tokenizers`, `tokenizers` depends on `onig`, and `onig` is Oniguruma — a C
regular-expression library. So the pure-Rust candidate needs a C cross toolchain
for the musl target exactly as the C++ one does. Neither keeps B-183's check
runnable on a machine without one, and the difference that decided TLS is not
available here.

**Read F18 after this.** *On a machine without one* is doing more work in that
sentence than it looked like at the time: the toolchain is packaged for this
machine and costs about nine megabytes. What F12 measured is unchanged; what it
concluded — that neither candidate keeps the check runnable — holds only where a
toolchain cannot be had, which turns out not to be here.

There is a second thing in that dependency worth noticing: `tokenizers` is
capability MCF already has. D31 gave MCF its own GGUF reader and its own
tokenizer so that the vendored engine would have something to be checked
against, and admitting `candle-core` would bring a second tokenizer along with
it — weight admitted for something already owned, which is what B15 asks a
reason for.

### 12.5 What this finding does and does not settle

It does not choose. What it establishes is the ground a choice would be made on,
and one expectation it removes:

- **Both build here.** Neither is blocked on this machine's toolchain for the
  ordinary artifact.
- **Neither builds for musl** without a C cross toolchain, so the from-scratch
  container either gains that prerequisite or ships without an engine — and *the
  second option is a difference in what two artifacts can do*, which §3.4 makes
  a condition of every measurement rather than a packaging detail.
- **The footprint is affordable but not free.** 12.9 MiB of static libraries
  against D24's 40 MiB ceiling is a third of it before a linker drops anything,
  and MCF's own binary is 3.3 MiB today.
- **The trade is one upstream against a hundred and forty-three.** That is a
  judgement about what MCF can account for rather than a number, and
  [vendored.md](vendored.md) §1 is where it would be argued.

B-320 is where the choice is made, and it is left open on purpose: this is the
largest single thing MCF will ship, and the register asks for a finding before
an admission rather than after.

## 13 · F13 — Two writers, one record (DEC-037)

**Why it was run.** The daemon exists, and with it MCF has more than one thing
that writes to the record: `mcf pull` records an acquisition, `mcf rm` a
removal, and the daemon its own starting and stopping. D20 makes the journal
*the record*, and §7.37 asks who writes to it and what happens to a write that
loses. Before deciding, it was worth knowing what the platform actually gives
without any coordination at all.

**What was run.** Eight processes appending to one file, each writing one whole
line per entry — the shape `mcf_record::journal` uses, which opens with
`O_APPEND` and issues one `write` per line. 2,000 lines each, at three sizes,
on the two filesystems that matter here: `tmpfs`, where a scratch record goes,
and `btrfs`, where the operator's own lives.

| Line size | Filesystem | Lines expected | Lines found | Interleaved | Short |
|---|---|---|---|---|---|
| 400 B | tmpfs | 16,000 | 16,000 | 0 | 0 |
| 8 KiB | tmpfs | 16,000 | 16,000 | 0 | 0 |
| 128 KiB | tmpfs | 16,000 | 16,000 | 0 | 0 |
| 8 KiB | btrfs | 16,000 | 16,000 | 0 | 0 |

**No line was torn**, at any size, including 128 KiB — far past `PIPE_BUF`,
which is the bound people usually quote for atomic appends and which applies to
pipes rather than to regular files. What the kernel guarantees here is that an
`O_APPEND` write takes the offset and the write together; a short write would
still tear a line, and none occurred.

### 13.1 What this settles and what it does not

It settles the alarming half. Two MCF processes writing one record do not
produce a record nobody can read: `checks/tests/load.rs` now asserts it —
concurrent writers, then a replay that reports no loss and finds every body
whole — and B62's *a replay reports what was lost* has nothing to report.

It does not settle DEC-037, and the residue is precise:

- **Identifiers can collide.** Each writer counts its *own* appends, so two
  entries from two processes can carry the same sequence number and therefore
  the same `EntryId`. Nothing is lost and nothing is unreadable; what is broken
  is the assumption that an identifier names one entry.
- **A short write would still tear a line**, and nothing here forces one. What
  was measured is that it does not happen on these filesystems at these sizes,
  not that it cannot.
- **A network filesystem was not measured.** `O_APPEND` is the classic thing NFS
  does not honour, and a record on a network share is a configuration MCF has
  not been asked about.

**What it argues for.** Not a lock. The measurement says the cheap arrangement
is sound where it has been tried, so the question is narrower than *how do we
coordinate* — it is *who mints an identifier*. A single writer answers it and
costs a running daemon for every command; a writer-scoped identifier answers it
and costs a change to something C5 makes stable for life. DEC-037 is where that
is chosen, and this is the evidence it should be chosen against rather than
guessed at.

## 14 · F14 — What a query over the record costs, and what an SQL engine would cost to ship (B-300, B-042, D6, D20)

**Why it was run.** D6 says the record is a SQLite database. D20, written later,
says the record is an append-only journal with a **derived, rebuildable** index
over it. Both can be true — one of them is the record and the other is the index
— but which is which decides whether MCF vendors 266,000 lines of C, and that is
not a question to settle by preference.

**What was run.** Two halves, one inside MCF and one outside it.

- `cargo test -p mcf-record --release --test how_the_record_grows -- --ignored`
  writes journals of 1,000 to 1,000,000 entries and times what MCF does with
  them.
- `prototypes/record-index/measure.sh` builds the same-sized table in SQLite,
  times two queries against it, and then measures the amalgamation as a *thing
  to ship*: source, terms, compile, and what it does to the musl artifact
  B-183's container runs.

Conditions: Linux 7.1.9, btrfs on NVMe, release profile, one machine, no
exclusive window — these are cost figures about MCF's own code and a candidate
dependency, not measurements MCF publishes (B35).

### 14.1 What a replay costs, and what an index costs instead

| Entries | Journal | Index | Full replay | Build the index | Open a current index | Last 20 of a kind |
|---|---|---|---|---|---|---|
| 1,000 | 0.4 MiB | 32 KiB | 6.4 ms | 10.7 ms | 128 µs | 183 µs |
| 10,000 | 3.8 MiB | 0.3 MiB | 98.7 ms | 103.9 ms | 812 µs | 318 µs |
| 100,000 | 38 MiB | 3 MiB | 777 ms | 988 ms | 9.1 ms | 230 µs |
| 1,000,000 | 381 MiB | 30 MiB | 7.89 s | 10.39 s | 72 ms | 196 µs |

A durable append costs **8.2 µs** and a whole entry is **400 bytes** on disk.
Replay is linear at about **8 µs an entry** — a parse and a `Value` per line.

Two numbers decide the shape. **7.89 s** is what a daemon start would pay to
count a million-entry record, at every start, and 381 MiB is what it would hold
to do it. **72 ms** is what opening the index costs instead, and **196 µs** is
what *the last twenty acquisitions* costs at any size, because the answer is
twenty seeks rather than a history.

A million entries is not hypothetical: M5 writes an entry per trial, and D16
keeps every trial rather than a summary.

### 14.2 What SQLite would buy, and what it would cost

Against the same million rows, through the `sqlite3` binary, including process
start each time:

| | |
|---|---|
| Build the table (WAL, one index) | 2.18 s |
| `count(*)` of one kind | 25 ms |
| Last 20 of one kind, by time | 16 ms |
| The database on disk | 151 MiB, beside the journal it was derived from |

And as a thing to ship:

| | |
|---|---|
| Source | 265,952 lines, 9.2 MiB, one file |
| Terms | public domain |
| Host object | 1.4 MiB, compiled in **51.6 s** by GCC 16.2.1 |
| Static musl artifact | **cannot be built here** — no C cross toolchain, the same wall F12 found for both engine candidates |

### 14.3 What this settles

**The index does not need to be a database, and D6's SQLite is weight MCF
cannot currently pay.** The queries a record actually gets — *the last twenty of
a kind*, *everything since a moment*, *how many of these are there* — are
answered in tens of microseconds by 32 bytes an entry, which is the same order
as SQLite answers them in and needs no C compiler, no cross toolchain and no
second copy of every body. The one thing SQLite would add that the index does
not have is *arbitrary* query — a `WHERE` over the bodies — and nothing in MCF
asks for one yet.

The cost side is not close. Admitting SQLite would put 9.2 MiB of C in the tree,
add 52 s to a cold build, and — on this machine, as it was configured when this
was measured — break B-183's from-scratch container, which runs a **static musl**
artifact with nothing installed. (F18 later found the missing toolchain is
packaged; the source size and the compile time are unaffected, and they are the
larger half of this.) F12 found the same wall for both engine candidates: the musl question
is turning into MCF's real constraint on vendoring, and it is a constraint about
a claim MCF makes rather than about a preference.

**What it does not settle.** SQLite is not refused for ever: if a query nobody
can answer with an offset table turns up, or the musl toolchain arrives with the
engine B-320 admits, this is a cost to pay rather than a rule to keep. What
changes today is D6's *the record is a SQLite database*, which becomes D20's
shape stated once: the journal is the record, and the index over it is derived,
32 bytes an entry, and free to delete.

## 15 · F15 — What actually needs elevation, asked of a machine (DEC-039, §7.39, §6.32)

**Why it was run.** §6.32 requires MCF's privileged surface be an enumerable,
auditable, short list, and §7.39 records that the list does not exist. It cannot
be written from documentation: what an ordinary user may do differs by kernel,
distribution, hardware and how the machine was set up. B-190's helper and
B-180's containment scenario are both blocked on the list, so it was asked.

**What was run.** `prototypes/elevation/measure.sh`, which **changes nothing**:
every row is a read, or a permission test of the kind `open(O_WRONLY)` performs,
or an operation on the probe's own process. A measurement that reconfigured a
shared machine to find out what it could reconfigure would be the ambient
authority §6.32 exists to refuse.

Conditions: Linux 7.1.9 (Fedora 44), uid 1000 in groups `wheel` and `ollama`,
cgroup v2 under a systemd user slice, one NVIDIA and one AMD device present.
**One machine.** Everything below is a fact about this configuration and a
hypothesis about any other, which is the whole reason DEC-039 asks *on which
platforms*.

### 15.1 What the machine said

| Operation | As this user | What wants it |
|---|---|---|
| Pin a process to cores | **permitted** | §6.39's ladder, B-193 |
| Bound this process's memory (`memory.high`) | **permitted** | §6.39, D8 |
| Read per-process accelerator occupancy | **permitted** | PR5, B-216 |
| Read temperature (thermal zone, hwmon) | **permitted** | §3.4's floor, B-013 |
| Raise priority (`nice -5`) | **refused** | §6.39's ladder |
| Real-time scheduling (`SCHED_FIFO`) | needs elevation | §6.39's ladder |
| Set the CPU frequency governor | needs elevation | §6.39, §3.25 |
| Disable SMT, offline a core | needs elevation | §6.39's upper rungs |
| Drop the page cache | needs elevation | a cold-start measurement |
| Set IRQ affinity | needs elevation | quieting a machine |
| Hardware performance counters | user-space events only (`perf_event_paranoid` = 2) | B-012 |
| **Read processor energy (RAPL)** | **needs elevation** | **D11 — energy is first-class** |
| Change accelerator compute or persistence mode | root, per the vendor's own tool | D8's exclusive lab |
| Lock memory | **8 MiB** (`RLIMIT_MEMLOCK`) | pinning weights |

**One row here was an absence rather than a fact** — *no C cross compiler on
this machine* — and F18 priced it: the toolchain is packaged. Nothing else in
this table changes.

**The one that costs something.** D11 makes energy a first-class quantity, and
`intel-rapl`'s `energy_uj` is not readable by an ordinary user on this
distribution — a deliberate change made after the counters were shown to leak
information about what a machine is doing. So *energy per token* is not a figure
MCF can take here without a privileged reader, and D11's claim needs either a
helper, a vendor counter that is readable (the accelerator's own power draw is,
through `nvidia-smi`), or an honest absence.

**Two that cost less than expected.** Core pinning and a memory bound need no
privilege at all: cgroup v2 delegates `cpu`, `io`, `memory` and `pids` to the
user's own slice, so MCF can ask for less of a machine without asking anybody.
And per-process accelerator occupancy is readable, so PR5's contention snapshot
— *what was I competing with* — does not need elevation.

### 15.2 The method has a lesson of its own

The first version of the probe reported that raising priority was **permitted**,
because `nice -n -5 true` exits zero. It does so having failed: the shell's
`nice` reports the exit status of the command it ran, not whether the priority
was applied. Asking the child what its priority actually *became* reverses the
answer.

That is A21 at the level of a shell script — a declaration is not an
observation — and it is the reason this file exists: a list of privileged
operations written from what the tools *said* would have been wrong on its first
row.

### 15.3 What this settles

It gives DEC-039 the list §6.32 asks for, on one platform, split by what it
costs. Four operations MCF wants need nothing. Two — a governor and real-time
scheduling — are the ladder's upper rungs and need a helper or must be dropped.
One — reading energy — is a *read* that needs elevation, which is the awkward
case §7.39 anticipated: a helper that must exist for a counter rather than for
an action.

**What it does not settle.** Every other platform. A Debian machine, a machine
without systemd, a container with no `/sys` write access at all, and macOS and
Windows are each their own answer, which is why DEC-039's answer has a column
per platform (D35) rather than a single list.

## 16 · F16 — Three defects the real reference model found, and none of them was the one expected (B-213, B-019, §3.7)

**Why it was run.** The operator asked which choices were still theirs, and one
of them was which quantization of the reference model to pin. MCF is supposed to
answer that without downloading anything (B-213, PR3), so instead of guessing at
file sizes the planner was pointed at the real repository.

**What was run.** `mcf pull unsloth/Qwen3.8-27B-GGUF` — the listing and the
plan, no bytes fetched — against the real hub, on the machine MCF is developed
on. Conditions: 61 GB of usable host memory, planning context 4096 tokens.

### 16.1 The plan could not be made at all, and the reason was a number

MCF said it could not plan and that *one of them could not be read*. The
repository publishes a `config.json` MCF's own reader refused, and the offending
text was:

```
"rms_norm_eps": 1e-06
```

MCF's JSON reader deliberately refused fractions and exponents. The reasoning
was sound and is still in the module: every quantity MCF *records* is integral
because A6's `Quantity` is `Ord`, and a record that silently rounded would be a
record that lied. What the reasoning missed is that **the same reader reads
documents MCF did not write**, and §3.7 makes those untrusted input rather than
MCF's own format. A reader that refuses valid JSON refuses real models.

The fix keeps both properties apart. A number this format does not carry is read
as `ForeignNumber`, **kept byte for byte**, and is not a quantity to anybody who
asks — `as_integer` is `None`. Nothing outside the reader constructs one, which
is what `checks/tests/no_float_reaches_the_record.rs` now holds. The record
stays integral because nothing writes anything else into it, rather than because
the reader cannot spell it.

### 16.2 The configuration was not where MCF looked

With the file readable, the shape still came back empty: this is a multimodal
repository, so one `config.json` describes several models and the transformer's
fields are under `text_config` while the top level holds the composition. MCF
now looks at the top level and then there — not a guess, but the place the field
is written.

### 16.3 Counting every block would have overstated the cache fourfold

The third is the one that would have produced a *wrong number* rather than no
number, which makes it the worst of the three. The configuration lists its layer
types:

| | |
|---|---|
| Blocks | 64 |
| Of which full attention | **16** |
| Of which linear attention | 48 |
| Key/value heads | 4 |
| Head dimension | 256 |

Only the full-attention blocks hold a key/value cache. Counting all 64 gives 256
KiB per token against the true 64 KiB — 1 GiB of cache at 4096 tokens where the
real figure is 256 MiB. On a 16 GB accelerator that is the difference between a
variant fitting and not.

MCF now counts the blocks the configuration says cache, and refuses to plan for
a configuration that lists layer types of which *none* caches, because that is a
shape MCF does not understand rather than a model that costs nothing.

### 16.4 What the plan says, now that it can be made

All 33 files classified, no bytes fetched. At 4096 tokens on 59.5 GB of usable
host memory every variant fits, which is the honest answer for **host** memory
and not the interesting one: the machine's accelerator holds 16 GB, and the
figures MCF computes make the comparison possible for whoever is choosing.

| Variant | File | Needs at 4096 tokens |
|---|---|---|
| `UD-IQ4_XS` | 14.25 GB | 15.06 GB |
| `UD-Q4_K_S` | 15.36 GB | 16.16 GB |
| `UD-Q4_K_M` | 16.46 GB | 17.27 GB |
| `Q8_0` | 29.05 GB | 29.85 GB |
| `BF16` (two parts) | 54.66 GB | — |

*Needs* is weights plus 256 MB of cache plus D24's stated 512 MB of runtime
overhead. Against 16 GB of accelerator memory, `UD-IQ4_XS` is the largest that
leaves room to work in; `UD-Q4_K_M` does not fit at all once the cache is
counted.

### 16.5 What this says about the method

Three defects, all in code with tests, all found by pointing it at one real
repository for the first time. Two of them produced *no answer*, which A7 made
safe; the third would have produced a **confident wrong one**, which is the
failure this project is organized against.

That is B-029's online tier earning its place — it acquires from the real hub on
a schedule — and an argument for extending it: the online check acquires a
1.2 MB model from a tiny-model repository, which exercises the transfer and not
the *variety* of what a hub publishes. A repository with a nested configuration,
a hybrid attention scheme and an exponent in its metadata is not an edge case;
it is the reference model.

## 17 · F17 — What a hub says when something is not there (DEC-038, §7.38)

**Why it was run.** MCF pins revisions and nothing says what happens when the
pin goes bad underneath it — a repository withdrawn, gated after acquisition,
relicensed, or a tag repointed (§7.38). What MCF is *able* to detect is a
property of the hub rather than of MCF, so it was asked.
`prototypes/upstream-decay/measure.sh`, anonymously, metadata only, no model
downloaded.

### 17.1 What it said

| Asked | Answer |
|---|---|
| A repository that does not exist | **401** |
| A gated repository, its card | 200, with `"gated":"manual"` |
| A gated repository, its file list | 200 |
| A gated repository, one file | 401 |
| A repository that is there | 200 |
| A revision that is not there | 404 |

The body of the first is `{"error":"Invalid username or password."}`.

**The hub does not distinguish *gone* from *private* from *never existed*, and
says so in the least helpful way available: by asking for a password.** That is
not an accident and it is not a defect — telling an anonymous caller that a
private repository exists is a leak — but it decides what MCF can honestly say.
A withdrawn pin and a typo produce the same answer, and no credential MCF could
be given would tell them apart unless the operator happens to have access to the
thing that is gone.

**A gate is visible before it bites.** The model card is readable and carries
`"gated":"manual"`; the file is not. So a repository that becomes gated *after*
acquisition is detectable — the flag changes, the card still answers — which is
the one decay in §7.38's list that MCF can name precisely.

**404 means one thing only: a revision that is not in a repository that is.**
Which makes it the reliable signal for a repointed or withdrawn *revision*, as
distinct from a repository.

### 17.2 The two decays that never refuse anything

A relicensing and a repointed tag are both a field with a different value and a
200 beside it. Nothing fails; nothing is refused; the request an operator makes
succeeds. They are detectable only by MCF **looking and comparing against what
it recorded at acquisition** — which is the same shape B-058 already has for
declared-versus-verified, applied along time instead of across sources.

### 17.3 What this settles

It gives DEC-038 the half that had to be measured, and the answer constrains the
decision rather than confirming it:

- **MCF can detect** a repointed or withdrawn revision (404), a gate that closed
  (the flag), a relicensing (the field), and any change of digest or size in the
  listing.
- **MCF cannot detect** the difference between withdrawn, private and
  never-existed. Anything it says about a 401 must carry that ambiguity, which is
  D33's shape exactly: report what was observed, and name the question being left
  unanswered.

MCF's current refusal on a 401 says *supply a credential*, which is right for a
private repository and misleading for a withdrawn one — advice that cannot work,
offered as though it could. That is corrected where it is written rather than
here.

## 18 · F18 — The wall three findings kept hitting is a packaged toolchain (B-320, B-183, DEC-047)

**Why it was asked.** Three separate findings have run into the same obstacle
and each treated it as fixed. F12 could not build either engine candidate for
`x86_64-unknown-linux-musl` — llama.cpp because it is C++, candle because
`tokenizers` reaches Oniguruma — and concluded that the musl question does not
separate them. F14 found SQLite's amalgamation unbuildable for the same target.
F15 recorded, in passing, that no C cross compiler is on this machine. Three
conclusions rest on an absence that nobody had priced.

**What was run.** `dnf info` and `dnf search`, which query this machine's
package manager and install nothing.

| Package | Download | Installed | What it is |
|---|---|---|---|
| `musl-gcc` | 12.0 KiB | 7.2 KiB | a wrapper that points the host gcc at musl |
| `musl-devel` | 214.6 KiB | 590.2 KiB | headers |
| `musl-libc-static` | 1.7 MiB | 8.0 MiB | the static library |
| `musl-clang` | 12.1 KiB | 7.8 KiB | the same wrapper for clang |

All from Fedora's own repository, from one source RPM, at musl 1.2.5.

**What this establishes and what it very deliberately does not.** It establishes
that a C toolchain targeting musl is **packaged for this machine** and costs
about nine megabytes installed. It does **not** establish that installing it
makes any of the three candidates build: a wrapper that compiles C for musl says
nothing about a C++ project's CMake, about `cc-rs` finding the right linker for
a Rust build script, or about Oniguruma's own configure. Each of those is a
thing to try, and trying it means changing a machine somebody else is using —
which is a decision for its operator rather than for a measurement.

**Why it matters anyway.** F12 left B-320 open partly because *neither candidate
keeps B-183's check runnable on a machine without a C cross toolchain*. That
sentence is true and its weight is different now: the machine can have one for
the price of a small package, so the question becomes *are we willing to require
a toolchain at build time* rather than *is it possible at all*. B36 constrains
what a **user** needs (nothing), not what a **build** needs, and MCF already
requires a pinned Rust toolchain nobody calls a violation.

The counter-argument survives and is worth stating: every build-time requirement
is a thing that breaks on somebody else's machine, and D29's per-platform
artifacts multiply it by the number of platforms. That is an argument about
which engine to admit rather than about whether the wall is real, and B-320 is
where it gets made.

## 19 · F19 — Two defects a real model found in an hour, and the tests that could not (B-364, B-365, A19, D38)

**Why it was run.** The operator's machine holds 107 GB of models acquired
through another tool, and three of them declare the one architecture MCF's
engine implements. Running one is the cheapest possible test of everything
B-364 had just built, and the strongest evidence available short of an oracle:
either the text is coherent or it is not.

**What was run.** `mcf run` against `Mistral-7B-Instruct-v0.3`, 291 tensors,
225 of them `Q4_0` and one `Q6_K`, prompt *The capital of France is*.

**What came back the first time.** `/******/obiubreérceronulusříoid`, from a
prompt MCF said was 26 tokens long. Both halves of that were defects.

### 19.1 The tokenizer used an algorithm no model is tokenized by

MCF segmented text with a Viterbi search over the whole string, maximizing the
sum of the vocabulary's scores. That is SentencePiece's *unigram* algorithm and
it is not what these files are read with. Two things were wrong with it here:

- **The scores are not log-probabilities.** In this vocabulary, `▁The` scores
  −156, `▁capital` −5306, `T` −28479, and every byte-fallback token scores
  **0.0**. They are ranks: for every token, score = −(identifier − 1027). A
  search maximizing the sum therefore preferred byte tokens to every real word,
  which is exactly what the 26 tokens were.
- **The algorithm is a merge, not a search.** Local runtimes tokenize these
  files by seeding a chain with single characters, then repeatedly merging the
  adjacent pair whose combined spelling scores highest, ties to the left. MCF
  now does the same. Where SentencePiece's own Viterbi would disagree with that
  merge is a question for B-368's oracle rather than an assumption here.

Byte fallback also emitted *one* byte of a multi-byte character rather than all
of them, so a space that fell back arrived as a fragment and came back as a
replacement mark. It emits every byte now, and decoding gathers consecutive byte
tokens before reading them as UTF-8.

**Fixed, the prompt is six tokens: `<s> ▁The ▁capital ▁of ▁France ▁is`.**

### 19.2 `Q6_K` decoded every value correctly and put them in the wrong places

With the tokenizer right, the model still produced noise. The single `Q6_K`
tensor in that file is `output.weight` — the projection to logits — so a fault
there turns a correct forward pass into random-looking tokens, which is what it
did.

The format writes the four values a byte-pair produces at **strides of 32**:

```c
y[l +  0] = d * sc[is + 0] * q1;
y[l + 32] = d * sc[is + 2] * q2;
y[l + 64] = d * sc[is + 4] * q3;
y[l + 96] = d * sc[is + 6] * q4;
```

MCF wrote them consecutively. Every value was right and every value was in the
wrong place. `Q2_K` and `Q3_K` had the same class of error in their sub-block
walk — four shifts of the same thirty-two bytes, two runs of sixteen per shift —
and were rewritten against the source at the same time.

**Fixed, the same prompt answers `Paris, but the largest city is Marseille`**,
and *Write one sentence about rain* answers *Rain is a natural phenomenon that
occurs when water droplets fall from the clouds to the earth's*.

### 19.3 The tests could not have caught either, and that is the finding

Both decoders had tests. Both tests passed throughout.

The `Q6_K` test built a block where **every value was the same** and asserted
they were all there. They were — in the wrong order, which a list of identical
values cannot show. A test that cannot distinguish a permutation from the
identity is not testing placement, and placement was the whole of the defect.
The replacement gives every position a distinct value and a distinct scale, and
fails on the old code.

The tokenizer's tests were worse in a subtler way: the laboratory's fixture
vocabulary held **only whole words** — `▁yes`, `▁no`, `▁maybe`. No real
vocabulary looks like that, because every token in one was *built* by merging,
so the intermediate pieces are always present. The fixture could not be
tokenized by the correct algorithm at all, and so it could only be tokenized by
the wrong one. Both fixtures now carry the full ladder from `▁` and single
letters up to each word.

**The general lesson, stated so it can be applied to the next one.** A fixture
MCF writes to test MCF is a fixture shaped by the same misunderstanding as the
code. D26 already says a scenario should build the observable rather than the
cause; this is its sibling for fixtures — *a fixture that cannot be wrong the
way a real file is wrong hides the defect it was built to catch*. Real artifacts
are how that is escaped, and B-368's oracle is how it is escaped systematically.

## 20 · F20 — A second architecture, and two silences that fail differently (B-365, A2, A19, D26)

**What was run.** Qwen3 0.6B, as Ollama already holds it on this machine —
`sha256-7f4030…e1fa`, 28 blocks, 1024 wide, 16 query heads over 8 key/value
heads, a stated head width of 128 that is *not* the embedding divided by the
heads, and a `gpt2` byte-pair vocabulary of 151,936 tokens with 151,387 merges.
The prompt was `The capital of France is`, greedy, seed 0, twelve tokens, run
through `mcf run` on the build in the working tree.

**What it produced, in four states.** The engine was run with each of the two
corrections this finding is about present and absent, and the four outputs are
the finding:

| rotary pairing | per-head query/key norm | what came out |
|---|---|---|
| adjacent pairs (llama's) | not applied | `了吗了吗了吗了吗了吗了吗…` |
| split halves (Qwen's) | not applied | `了吗了吗了吗了吗了吗了吗…` |
| adjacent pairs (llama's) | applied | `the the capital of of the the country of in which the` |
| split halves (Qwen's) | applied | `Paris, and the capital of the United States is Washington,` |

**The first thing this establishes is that the two defects fail differently, and
only one of them looks like a defect.** A missing per-head normalization
collapses the model onto a single token and is unmistakable. A wrong rotary
pairing produces *English* — words in the right proportions, function words in
plausible places, and no answer. Anybody watching the third row without the
fourth beside it would report a small model doing what small models do. The
rotation was already suspected and already documented as the thing to suspect;
what the experiment adds is that its signature is fluency, so *fluent output is
not evidence the rotation is right*. Nothing short of a second implementation or
a known answer distinguishes row three from a model that is simply small (A19).

**The second is that neither defect was a mistake in arithmetic.** Both were
silences.

The rotary pairing is not in the file. GGUF states the base frequency and the
head width and never states which two components of a head turn together, so
every engine that runs these files carries a table from architecture to pairing
under some name, and MCF's absence of one was a table with one entry that never
said so. It is now `rotation_for`, written out, defaulting to llama's, with the
families that want the other named — because being wrong there does not fail, it
produces text.

The per-head normalization was worse, because MCF had the code for it. The
weights are two tensors per block that llama does not carry, the engine asked
for them with `if let Ok(weights) = self.tensor(…)`, and the tensor map was
built from a manifest that did not list them. The lookup failed on every block
of every model, and an `if let Ok` cannot say so. That is A2's silent failure
inside an engine written under A2 — the guard was shaped like an option and was
in fact an error being discarded. Optional tensors are now loaded by asking the
file whether it carries them; a tensor that is there and unreadable is a refusal
rather than an absence, which is the difference between an optional part and a
skipped one.

**What this says about where to look next.** Both defects are of the same
family: something the architecture requires that the *file* does not say and the
*llama* path does not need. That family has more members — gemma3's sandwich
normalizations, llama4's expert routing, the reference model's sixteen
full-attention blocks among sixty-four — and none of them will announce
themselves. The reference implementation of B-368 is the systematic answer; a
real file per family, run and read, is what is available before it.

**What was not established.** Nothing here is a speed, and it cannot be (B65).
Twelve tokens from one prompt at one seed is not a measure of quality, and the
fourth row's claim about Washington is the model's own. That the engine now
produces coherent text for two architectures says that the paths it exercises
are right for these two files; it says nothing about the paths they do not
exercise — long contexts, other quantization schemes, the grouped-query ratios
these two happen not to have.

## 21 · F21 — Six families for a tenth of one model's bytes (B-369, D40, DEC-054, §3.12)

**What prompted it.** The operator observed that development against the
reference model is slow, that §XII's choice of it was made for its quality
rather than for its fitness as a development subject, and that the models
already on this machine should not be used — their state is unknown and they are
all large. F19 had reached for those models precisely because they were already
there, which is how an unexamined convenience becomes a condition of the work.

**What was acquired.** The smallest *trained* model of each family MCF covers or
means to cover, one distinct quantization apiece so that architecture coverage
and quantization coverage come from the same six files. Every one was fetched by
`mcf pull`, digest-verified, and recorded with its provenance.

| family | artifact | quantization | bytes |
|---|---|---|---|
| llama, unigram vocabulary | `Felladrin/gguf-Llama-160M-Chat-v1` | Q4_K | 121,295,296 |
| llama, byte-pair vocabulary | `bartowski/SmolLM2-135M-Instruct-GGUF` | Q8_0 | 144,811,360 |
| qwen3 | `unsloth/Qwen3-0.6B-GGUF` | Q4_K_M | 396,705,472 |
| gemma3 | `unsloth/gemma-3-270m-it-GGUF` | Q6_K | 282,975,264 |
| mixture-of-experts | `RichardErkhov/Isotonic_-_TinyMixtral-4x248M-MoE-gguf` | Q5_K_M | 500,878,816 |
| embedding | `leliuga/all-MiniLM-L6-v2-GGUF` | Q4_0 | 19,699,648 |

**1.4 GB for six families against 16.5 GB for one model.** The two that run
today answer a five-token prompt in 3.9 s (160M) and 15.9 s (0.6B) on this
machine, measured with `time` around `mcf run` at a ten-token budget. These are
*not* speeds in B65's sense and cannot become them — they are the stand-in's own
cost and are recorded here only as the quantity the decision was about: how long
it takes to find out you were wrong.

**The other four refuse, and what they say is the finding's second half.** Each
names exactly what it wanted:

- the byte-pair llama asks for a pre-tokenizer called `smollm`, which MCF has
  not implemented and will not substitute (A7);
- the embedding model carries a third tokenizer scheme, `bert`;
- the mixture-of-experts file has no `blk.0.ffn_gate.weight`, because its gate
  is per-expert;
- gemma3 is an architecture MCF has not been taught, and says which it has.

Four refusals, four different subsystems, four different categories, and not one
of them a crash or a guess. That is the surface working — but note what it also
is: **the refusals are the order of work**, derived from artifacts rather than
from a list somebody wrote down. B-365's remaining sequence is now read off six
files instead of being predicted.

**What this costs, stated because it is not free.** A 135M model cannot referee
the difference F20 turned on. That finding separated a correct engine from a
subtly wrong one because the correct one answered `Paris` and the wrong one
produced fluent English containing no answer; the judgement needed a model good
enough to be right. The corpus buys iteration speed by giving up the oracle that
F20 used, and the only thing that buys it back is a reference implementation to
compare against numerically — which is D39's opening and B-368's item. That is
why DEC-054 moves B-368 ahead of the remaining families rather than after them.

**What was not established.** That these six are the smallest such models, only
that they are the smallest found by asking the hub for each family's known small
releases; a smaller trained gemma3 or mixture-of-experts may exist. Nothing here
measures quality, and the two timings measure this machine and this stand-in.
Whether the four refusals are the *only* things those files need is unknown —
a refusal names the first thing missing, not every thing.

## 22 · F22 — Where a model becomes able to referee an engine (B-370, D40, DEC-054, A19)

**The question.** F21 asserted that a small model cannot tell a correct engine
from a subtly wrong one, and used that to make the conformance corpus depend on
an oracle. The assertion was reasoning, not measurement. This measures it.

**The method.** Two corpus models were run against a correct engine and against
one with a single defect: the rotary pairing swapped, so each family gets the
other convention. That is the defect from F20 that produces *English* rather
than gibberish — the hard case, deliberately. Four prompts with answers a person
knows, greedy, seed 0, fourteen tokens. The engine was rebuilt from a
git-clean tree before each block; a first attempt at this measurement compared
two runs of the same stale binary and produced two identical tables, which is
how the rebuild came to be part of the method.

**Llama-160M-Chat, Q4_K:**

| prompt | correct engine | rotation swapped |
|---|---|---|
| The capital of France is | `Paris.` | `the capital city of Paris.` |
| Water freezes at a temperature of | `100 °C. The water freezes at a temperature` | `120°C. The water dropleases are formed` |
| The largest planet in our solar system is | `Mercury. Mercury is the second-largest planet in our` | `the Sun.` |
| The opposite of hot is | `hot.` | `hot.` |

**Qwen3-0.6B, Q4_K_M:**

| prompt | correct engine | rotation swapped |
|---|---|---|
| The capital of France is | `Paris. The capital of France is also the capital of the Republic of` | `the the capital of of the the country which is the the capital of` |
| Water freezes at a temperature of | `0°C, and the boiling point of water is at 1` | `at 200000000000` |
| The largest planet in our solar system is | `...? A. Mercury B. Venus C. Earth D. Mars` | `called the...? A.. B B.. C..` |
| The opposite of hot is | `cold, and the opposite of cold is hot. So, the opposite` | `a the of the the same as the opposite of cold. So,` |

**At 160M the reader cannot tell which column is the broken engine, and the
reason is not subtlety.** It is that the correct engine's own answers are wrong:
water freezing at 100 °C, Mercury as the largest planet, the opposite of hot
being hot. Three of four prompts are answered incorrectly by a *correctly
implemented* engine. A model that does not know the answer cannot be asked
whether the engine found it, and on one prompt the two engines produce
character-identical output. The one row that looks like a signal — `dropleases`,
which is not a word — is the only one, and one non-word in four prompts is not
something to build a tier on.

**At 0.6B all four are unmistakable, and the correct column is right.** Freezing
at 0 °C, the opposite of hot being cold. The broken column repeats function
words and emits a run of twelve digits. No judgement is required to separate
them.

**So the threshold is between the two, and it is cheap.** 0.6B is 397 MB and
answers in about sixteen seconds on this machine — not the 16.5 GB the reference
model costs. The useful statement is not *small models are unfit* but **a model
must be good enough to be right about the thing being asked**, and that turns
out to start well below a billion parameters.

**This corrects F21 and the decision that cited it.** F21 concluded the corpus
*depends on* an oracle because coherence cannot referee at small sizes. The
dependency is real at the bottom of the corpus and absent one step up. The
oracle is still worth building — it is exact where this is a judgement, it works
on the 160M model where this does not, and it catches defects that leave output
fluent at any size — but it is not a precondition for the corpus to be useful.
B-368 keeps its priority on its own merits and not on this one.

**What was not established.** One defect, one seed, one greedy sampler, four
prompts, two models. That 0.6B suffices *for this defect* is not that it
suffices for every defect; a defect subtler than a swapped rotation may be
invisible at 0.6B and visible only to a numeric comparison. The threshold is a
lower bound on what is needed, never an upper bound on what is enough (A21).

## 23 · F23 — Four expressions where MCF had two, and what no test could have found (B-365, B-370, A7, A19, A21)

**What prompted it.** SmolLM2 refused with `asked for: smollm`, and MCF's
pre-tokenizer table had to grow by one. Rather than infer what `smollm` means
from its name, the reference implementation was read — llama.cpp's
`llama-vocab.cpp`, the `switch` on the pre-tokenizer type and the table that
maps the string in `tokenizer.ggml.pre` onto it. That reading found three
things wrong with what MCF had already shipped.

**One: `qwen2` and `llama-bpe` are not the same expression.** MCF had them as
one, on the strength of how similar they look. They differ in one place:
llama3 takes digits in groups of up to three, `\p{N}{1,3}`, and qwen2 takes them
one at a time, `\p{N}`. A different cut is a different set of merges that can
apply, so a number tokenizes differently under the two.

**Two: `deepseek-llm` was claimed and is not implemented.** MCF listed it in the
same group. Its pre-tokenizer is six expressions, one of which is an explicit
enumeration of several hundred letter ranges. It is now refused by name.

**Three: `default` is not GPT-2's expression.** MCF read a file with no
`tokenizer.ggml.pre` as GPT-2, reasoning that the field postdates the format.
llama.cpp's fallback for that case is a *fourth* pattern — four expressions,
one of which splits on punctuation. A file that does not say is now refused
rather than run through something that resembles what it wants.

**None of the three could have been found by running anything, and the first
one demonstrates why.** Qwen3-0.6B tokenizes `What is 1234 plus 5678` into
thirteen tokens with every digit separate — *both before and after* the fix.
Its merge list contains no merge that joins two digits, so the grouping never
had anything to group and the two cuts agree on this artifact. The defect is
real, it is fixed, and no prompt against this model distinguishes the fixed
version from the broken one. What a name maps to is a fact about somebody
else's software; the only instrument that reads it is somebody reading it
(A21: declared is not verified — and here, *not verifiable by observation*).

**What the corpus did catch.** With `smollm` implemented, SmolLM2 runs — and
the tier reported it as a failure, which is what it is for: the register said
that entry refuses, it no longer does, and good news nobody notices is how a
check stops being read.

**And SmolLM2 corrects F22.** That finding put the floor for refereeing an
engine "between 160M and 0.6B". SmolLM2-135M-Instruct at Q8_0 is *smaller* than
Llama-160M-Chat at Q4_K and answers all three probe questions correctly —
`Paris.`, `0 degrees Celsius.`, `cold.` — where the 160M model got every one of
them wrong. Run with the rotary pairing swapped it gives `Paris.`,
`10 degrees Celsius.`, `10.`: two of three visibly degraded.

So the floor is not a parameter count. **It is whether that artifact, at that
quantization, knows the answer to what is being asked** — and a well-trained
135M model at Q8_0 knows more of it than a poorly-trained 160M model at Q4_K.
F22's measurement stands; its generalization to a size does not.

**A fourth defect, found by the round-trip and not by reading.** Every corpus
vocabulary is now encoded and decoded over awkward text — digit runs,
punctuation against letters, characters that are several bytes, runs of
whitespace, the empty string. The empty string came back wrong for every
unigram vocabulary: adding control-token matching to `encode` had introduced a
walk that skipped an empty remainder, silently turning *encode nothing* from
one token into no tokens. It has a test now.

The round-trip also showed something that is **not** a defect and had to be
told apart from one: `SentencePiece` puts a space in front of every text, so
`decode(encode(t))` is `" " + t` and no decoder can tell that space from one
the text really had. MCF now reads `tokenizer.ggml.add_space_prefix` where a
file states it, and the check allows the convention for exactly the
vocabularies that declare it rather than stripping a leading space and eating a
real one.

**What was not established.** That the four expressions are now right — only
that they are transcribed from a reference that runs these models, and that
three vocabularies round-trip. The transcription itself is unverified in the
sense A21 means, and B-368's oracle is what would verify it: two
implementations agreeing on identifiers for the same text is a check, and one
implementation agreeing with itself is not.

## 24 · F24 — A third architecture, and three habits that fail three different ways (B-365, B-370, A19, §3.18)

**What was added.** Gemma 3, read from llama.cpp's `models/gemma3.cpp` rather
than inferred, and run against the corpus artifact `gemma-3-270m-it` at Q6_K —
18 blocks, 640 wide, 4 query heads over 1 key/value head, a stated head width
of 256, and a unigram vocabulary of 262,144 tokens that declares
`add_space_prefix = false`.

It differs from llama in five places. Two the file states and three it does not,
and that division is the finding.

**What the file states, and is therefore read from the file (§3.18):** two more
normalizations per block — on the way *out* of the attention half and out of
the feed-forward half, which is what makes these blocks a "sandwich" — carried
as `post_attention_norm.weight` and `post_ffw_norm.weight`. MCF applies them
because the tensors are there, so a file of any family carrying them gets them
and a gemma file without them would not.

**What the file does not state, and is therefore a table:** the gated block
activates with `GELU` rather than `SiLU`, the embedding is multiplied by the
square root of its width on the way in, and the rotation pairs halves rather
than adjacent components. None of the three is visible in any tensor or any
metadata key. A `GELU`-gated block and a `SiLU`-gated one have the same tensors
of the same shapes.

**Each was removed in turn, and they fail in three different registers.** The
prompts are `The capital of France is` and `The opposite of hot is`; correct,
the model says `Paris.` and `cold.`

| what was removed | what came out |
|---|---|
| the two sandwich normalizations | `incessant intensive intensiveHighwayHighwayHighway…` |
| `GELU`, using `SiLU` instead | `the 12th city, and the` · `a cold. It's a cold, a` |
| the embedding's scale | `चौ चौ चौلسللسللسللسل…` |

Word salad, fluent-and-wrong, and characters from another script. **Only the
middle one is dangerous**, and it is the one that would survive review: real
words, plausible grammar, and no answer. It is the same signature F20 measured
for the rotary pairing and F22 measured again — and it is now three habits, in
two different families, that produce it. That is no longer a coincidence worth
noting once. *An unobservable habit, got wrong, produces fluent text* looks like
the general case, and the loud failures are the lucky ones.

**Where this leaves the division of labour.** Everything observable is read
from the artifact, which is why gemma3 needed no new code path for its
normalizations — only two more names in the list of tensors MCF will load if
they are there. Everything unobservable is in one table, `architecture.rs`,
which DEC-053 already requires and which the neutrality check already holds to
naming no artifact. Growing a family is now: read the reference, add what the
file cannot tell you, and let the rest be found.

**What was not established.** Gemma 3 alternates local and global attention on a
five-to-one pattern with a different rotary base for each, and this artifact
declares a sliding window of 512 but **no** separate base for the local layers.
MCF therefore uses the one base the file states, for every layer, which is what
the file says and not necessarily what the model was trained with (A7, A21). At
twelve tokens no window truncates anything, so nothing here exercises the
sliding window at all. A file that declares a second base, or a prompt longer
than 512 tokens, is untested ground.

Nor is the 27B variant's attention scale exercised: gemma scales queries by the
inverse root of the head width except at 27B, where it is the inverse root of
the embedding over the heads. On this artifact those coincide with what MCF
already computes.

## 25 · F25 — The broken engine read better than the correct one (B-365, B-368, A19, D38)

**What was added.** A mixture of experts, read from llama.cpp's
`build_moe_ffn`, and run against the corpus artifact `TinyMixtral-4x248M-MoE` at
Q5_K_M — twelve blocks, four experts per block, two used per token.

Notably the file declares `general.architecture = llama`. Whether a block holds
one feed-forward or a stack of them is not a property of the family: it is
`llama.expert_count`, which the file states, so MCF reads the shape from the
count and not from the name (§3.18). Nothing about "mixtral" appears anywhere in
the engine.

**What a mixture adds is a choice.** Each expert is an ordinary feed-forward run
with the same three matrix multiplies. The new part is: score every expert
through a small router, softmax the scores, take the highest two, renormalize
those two so they sum to one, and add their outputs in proportion.

**Two pieces of that were removed in turn, and here is the result that matters.**
The prompts are `The capital of France is` and `The opposite of hot is`.

| version | what came out |
|---|---|
| as written | `Paris, France is Paris, Paris is Paris is` · `hot. The hot is hot, the hot is` |
| the router ignored — experts 0 and 1 every time | `Paris. It is located on the River Seine in` · `hot.` |
| the two weights not renormalized | `France France France France France France` · `hot` |

**The broken version reads better than the correct one.** Ignoring the router
entirely — never scoring an expert, always taking the first two — produces
`Paris. It is located on the River Seine in`, which is a fluent, accurate,
well-formed sentence. The implementation transcribed from the reference produces
a stammer. Anybody tuning this by reading the output would have deleted the
router and called it an improvement.

That is the same phenomenon as F20's rotation, F22's measurement and F24's three
habits, and it is now at its limit: it is not merely that a wrong engine can
look right, it is that **a wrong engine can look better than a right one**, on
the same artifact and the same prompt. Output quality is not evidence about
implementation correctness in either direction. This finding is the case for
B-368 and there is no longer a stronger one to be made.

**What is actually known about this implementation.** That it was transcribed
from the reference rather than inferred; that both the routing and the
renormalization are live, because removing either changes the output; that the
shape is read from the file's own counts and refuses a file that states experts
without stating how many are used. That is not the same as knowing it is right,
and the difference is exactly what an oracle would close (A21).

**Why the artifact is a poor witness, stated because it bears on the above.**
`TinyMixtral-4x248M-MoE` is a merge of four small models rather than a mixture
trained as one, so its own quality is low and its stammer is a plausible thing
for it to do unassisted. A better mixture would make the correct column read
better — and would not change the argument, because the broken column would
still have read fluently.

**What was not established.** The weight scale some mixtures apply
(`expert_weights_scale`), which this file does not declare and MCF therefore
does not apply; expert biases and grouped routing, which belong to other
families; and whether two of four is representative of routing at a realistic
width. Nothing here is a speed (B65).

## 26 · F26 — The oracle found a defect on its first run, and it was an invention of MCF's own (B-368, A19, A21, §3.12)

**What was built.** llama.cpp at commit `925e1179`, built from source as a
development instrument — vendored nowhere, on no MCF command's path, and absent
from every artifact MCF ships (D39's fourth condition). The build queued behind
another project's run on the shared machine rather than taking the window from
it.

**What is compared, and why only this.** Token identifiers. They are integers:
two tokenizers either agree about a list of them or do not, with no tolerance to
argue about and no floating-point arithmetic in the way. Logits are deliberately
not compared yet — a correct implementation can flip an argmax on a near-tie
through nothing worse than a different summation order, so a disagreement there
is a finding to investigate rather than a verdict.

Six texts across five corpus vocabularies: a plain sentence, digit runs,
punctuation against letters with contractions, characters outside ASCII, a tab
and a double space, and a single letter.

**The result: 29 of 30 agreed exactly, and the thirtieth was a defect.**

```
gemma-3-270m-it   on  "tabs\tand  spaces"
    MCF:        2 39218 255968 624 236743 9952
    reference:  2 39218 255968 624 138 35220
```

The two agree for four tokens and part at the double space. Token 138 in that
vocabulary is `'  '` — two literal spaces, marked **user-defined**. MCF was
segmenting it into `▁` and `▁spaces`; the reference matches it whole.

**The cause was a guard MCF invented.** MCF matches user-defined tokens in the
raw text before segmenting, and the code that did it skipped any token shorter
than three bytes, on the reasoning that *a one-character token would match
inside ordinary words*. That reasoning is plausible and is not what any
implementation does. The reference matches every user-defined token whatever its
length, sorted longest-first so a longer token claims its text before a shorter
one can — the sorting MCF already did, with a guard on top that nobody else has.

Gemma's vocabulary carries 163 user-defined tokens, many of them runs of
whitespace. Any text containing a double space tokenized differently in MCF than
in every other implementation of the same file.

**A second thing the reference settled, which MCF had guessed at.** MCF matched
*control* tokens in ordinary text too — `<|im_start|>` typed into a prompt became
the token a chat template uses to start a turn. The reference's default is to
leave control tokens as text and match only user-defined ones, and that is both
the compatible answer and the safer one: a prompt is text somebody typed, and it
should not be able to produce a marker by spelling it. MCF now does the same, and
a surface that applies a chat template will have to ask for the other behaviour
rather than receive it by accident.

**Why this finding matters more than the defect in it.** The defect is small.
What it demonstrates is the thing four previous findings argued for and could
not supply: F20, F22, F24 and F25 each established that output is no evidence
about correctness, and each ended by saying an oracle would settle it. This is
the first check MCF has that can say *wrong* about an engine without a human
judging a sentence — and the first time it ran, on five models it had been
producing plausible text with all day, it found something.

Note also what found it: not the plain sentence, and not the model failing to
say `Paris`. Gemma answers `Paris.` and `cold.` correctly with the defect
present. It was the tab-and-double-space string, which exists in that list
precisely because whitespace is where tokenizers differ.

**What is now known that was not.** MCF's four pre-tokenizer expressions, its
byte-level alphabet, its merge ordering and tie-breaking, its unigram
segmentation, its byte fallback, its space prefix and its beginning-of-text
handling agree with the reference on thirty comparisons across five vocabularies
of two schemes. F23 recorded three transcription defects found by reading and
said plainly that the transcription itself was unverified. That part is now
verified — for these vocabularies and these texts, which is what a check
establishes and not more (A21).

**What was not established.** Nothing about the forward pass: the arithmetic
that F20, F24 and F25 were about is untouched by this, and remains transcribed
and unchecked. Nothing about the embedding family, whose vocabulary MCF still
refuses and which was reported as not compared rather than counted as agreeing
(A4). And nothing about texts unlike these six.

## 27 · F27 — What a coin-flip looks like, and what a defect looks like (B-368, B-365, A19, A21)

**The question this had to answer before the forward pass could be compared at
all.** Greedy generation is deterministic, so two correct implementations should
produce the same tokens from the same model and prompt. Except they need not:
where the best and second-best logits are close enough, a different order of
summation picks a different winner, and neither implementation is wrong. F26
compared identifiers precisely because that comparison has no such problem —
integers agree or they do not. Comparing generated text needs a way to tell a
coin-flip from a defect, and MCF had asserted one was needed without measuring
it.

**The instrument.** `margins`, which reports for every step of a generation the
gap between the chosen token's logit and the runner-up's. The claim it makes
possible is: *a divergence at a small margin is arithmetic; a divergence with
room to spare is a defect.*

**Fifteen comparisons, five models, three prompts, ten tokens each.** Eleven
agreed exactly, token for token. Four diverged, and every one of them looks the
same:

| model · prompt | margin where they parted | MCF chose | reference chose |
|---|---|---|---|
| SmolLM2 · *Water freezes…* | **0.040** | ` freezing` | ` equation` |
| Llama-160M · *Water freezes…* | **0.098** | ` water` | ` free` |
| SmolLM2 · *The capital of France is* | **0.105** | ` the` | ` a` |
| gemma3 · *Water freezes…* | **0.159** | `2` | `0` |

In **every** case the reference's choice was exactly MCF's runner-up, and in
every case the divergence happened at the smallest margin in that whole
generation. That is what a coin-flip looks like: the two implementations
disagree only where the model itself was indifferent.

**And then the defect, which looks nothing like it.** Before the above, gemma3
diverged on *The opposite of hot is* at a margin of **0.775**, choosing `Hot`
where the reference chose `The`. Five to nineteen times the largest noise
margin. That was real, and this is what it was.

Gemma 3 alternates sliding and global blocks — five that see only the last 512
positions, then one that sees everything — and the two kinds **rotate at
different base frequencies**. The corpus artifact declares
`gemma3.attention.sliding_window = 512` and does *not* declare a base for the
sliding blocks. F24 recorded exactly this and called it untested ground: MCF
used the one base the file states, for every block.

What the reference does when that key is absent is use **ten thousand** — a
default in its own header, not anything the file says. So MCF was rotating five
blocks in six at 1,000,000 where every other implementation rotates them at
10,000. The model still answered `Paris.` and `cold.` correctly, which is why
nothing before this caught it.

MCF now reads `rope.freq_base_swa` where a file states it, falls back to ten
thousand where it does not, and applies the window itself — a key at position
`p0` is visible from `p1` only while `p1 - p0` is under the window, which is the
reference's boundary.

**The tolerance, and its honest width.** The check now fails a generation
divergence only when *every* step of it had a margin over **0.50** — above the
largest noise observed by a factor of three, below the one observed defect by a
factor of one and a half. Reintroducing the sliding-window defect makes it fire:
`closest margin 0.95879, over 0.50 — this is not a near-tie`, and the run exits
non-zero after printing its summary.

The threshold is provisional and the reason is worth stating plainly: **a defect
can hide under a near-tie.** If a wrong engine happens to be wrong only where the
model was indifferent, this passes it. What narrows the gap is more comparisons
and more prompts, not a cleverer rule; four noise observations is what this rests
on.

**What is now known.** Across five corpus models, MCF's tokenizer agrees with the
reference on thirty comparisons exactly, and its forward pass agrees on eleven of
fifteen generations token for token, the other four differing only where the
model was indifferent. F20, F24 and F25 each ended by saying the forward pass
was transcribed and unchecked. It is no longer unchecked — for these models,
these prompts, and ten tokens, which is what a check establishes and not more.

**What was not established.** Nothing about long contexts: at ten tokens the
sliding window never truncates anything, so the *mask* MCF now applies is
exercised by nothing here and only the rotary base was measured. Nothing about
the embedding family, still refused. Nothing about sampling other than greedy.
And nothing here is a speed (B65).

## 28 · F28 — The mask, past the boundary (B-368, F27, A21)

**The hole this closes.** F27 fixed gemma3's sliding-window rotary base and
*implemented* the window's mask, then said plainly that at ten tokens the mask
is exercised by nothing. A defect in a boundary nobody crosses is invisible, and
the mask MCF now applies had never once masked anything.

**Two prompts, both past the boundary, chosen for different failure shapes.**
Gemma3's window is 512 positions; five blocks in six see only that far back.

- **682 tokens of one sentence repeated**, then eight generated. The blandest
  possible history: if the mask boundary were off by one, the sliding blocks
  would attend to a slightly different set of near-identical keys — a defect
  with room to hide. Both implementations produce `The quick brown fox jumps
  over the`, token for token.
- **A distinctive fact, then 550 tokens of filler, then a cue** — `My name is
  Konstantin Aurelio Blackwood… [filler] …My name is`. The name sits *outside*
  every sliding block's window and inside the global blocks' view, so the two
  kinds of block must disagree about what the history holds, and only an engine
  that masks the sliding ones and not the global ones recalls it. Both
  implementations produce `Konstantin Aurelio Blackwood. I live`, token for
  token.

Both tokenizations were verified identical before comparing (682 and 621
identifiers), so the generations compare the forward pass and nothing else.

**What this establishes.** MCF's window mask — a key at `p0` visible from `p1`
only while `p1 − p0` is under the window — agrees with the reference's at 682
positions, on a boundary that is actually crossed, in both a history where the
masked keys resemble the kept ones and a history where they do not. The
comparison is available on demand as `MCF_ORACLE_LONG=1 scripts/check-oracle.sh`
and is off by default: the stand-in pays one forward pass per prompt token, and
a tier that costs minutes by default stops being run.

**What was not established.** One window size, one pattern (five sliding to one
global), one model. A file whose window differs from 512 or whose pattern the
file states explicitly exercises arithmetic this did not. Nothing here is a
speed (B65).

## 29 · F29 — The sixth family answers a different question (B-371, DEC-055, B-365, A19, §3.3)

**What was built.** The bert family: a WordPiece tokenizer, a whole-sequence
non-causal forward pass with classic layer normalization and biases throughout,
mean pooling as the file declares, and a new surface — `mcf embed <model>
--text <text>` — because the question these models answer is not `mcf run`'s
question. There is no output head and no next token; there is one vector for
the text, and DEC-055 is resolved by giving that its own verb. The vector goes
first and machine-readable — one JSON line, `{"width":…,"embedding":[…]}` —
and the conditions after, legible: token count, the declared pooling, the unit
normalization, and the same B65 mark every stand-in answer carries.

**Everything transcribed was checked against the reference the same day it was
written**, which is what B-368 existing before B-371 was for.

*The tokenizer, exactly.* Five texts — plain English, punctuation with
contractions, `café naïve 你好`, digits, and invented words that must become
`[UNK]` — agree with the reference identifier for identifier. The normalizer's
scope is stated in the module rather than hidden: accents are stripped by a
Latin fold table because MCF carries no Unicode tables, and a precomposed
accented letter outside Latin passes through where the reference would
decompose it (A21).

*The forward pass, at a measured distance.* MCF dequantizes to floats and
multiplies; the reference multiplies in quantized arithmetic and quantizes the
activations too. The vectors therefore cannot be equal, and how unequal is the
measurement: across five texts, cosine 0.999596 to 0.999811, largest single
component difference 0.006. The oracle's floor is set at 0.999 — under the
observed agreement, and far above what anything structural leaves standing:
swapping one normalization in one layer from LayerNorm to RMS drops cosine to
0.972–0.988, and the tier fires on every text.

*That the vector means something*, which agreement alone does not show: `The
cat sat on the mat` against `A kitten rested on the rug` scores 0.62; each
against `Quarterly revenue grew by twelve percent` scores under 0.03.

**The corpus is six for six.** Every family acquired in F21 now either runs or
embeds through MCF's own engine, and each got there the same way: read the
reference, take from the file everything the file states, put what no file
states where DEC-053 requires, and let the oracle say whether the transcription
is right. Four defects were found on that road (F23 ×3, F26, F27) and none by
reading output.

**What was not established.** One bert model, quantized one way, on short
English-heavy texts. The nomic-bert variant on this machine adds rotary
positions and a gated feed-forward and is not covered by anything here. The
first-position pooling variant is implemented and exercised by no artifact. And
cosine at 0.999 is a floor calibrated on this model at this size — a larger
model's arithmetic gap may sit elsewhere, and the floor would need remeasuring
rather than trusting (A21). Nothing here is a speed (B65).

## 30 · F30 — Two ways to provision the same component, measured (DEC-052, B-367, D39, A27)

**The question.** D39 admits components MCF provisions — installs, builds and
pins itself — and names four conditions: pinned and recorded, reproducible,
contained and reversible, never the baseline. DEC-052 asks what mechanism makes
"controlled" true. Two candidates existed on paper: a managed prefix built with
the host's own tools, and a container. Both have now provisioned the same
component — llama.cpp at commit `925e1179`, the oracle — and the differences
were measured rather than argued.

**Route A: the host's tools into a prefix.** This is how the oracle was first
built (F26): clone pinned, `cmake`, build into `/home/gauge/Content/mcf-oracle`.
It worked, and it failed D39's first condition in a way nobody would have
noticed: **the build used cmake 4.2.1 resolved from a pyenv shim** —
`~/.pyenv/shims/cmake` — while the system package, which is what anyone
recording the environment would have asked `rpm` about, is 4.3.0. The toolchain
that built the oracle was an accident of the operator's Python setup, recorded
nowhere, and different from what every reasonable record would have said it
was. That is "whatever the PATH resolved today", observed in the wild on the
first provisioning MCF ever did.

**Route B: a rootless container over a mounted prefix.** `podman run --rm` on
`fedora:44` (pinned by digest `5a4a491c…`), the source mounted read-only, a
prefix on the content drive mounted for output, the toolchain installed inside
and its exact versions written into the prefix: gcc 16.2.1-2.fc44,
cmake 4.3.0-1.fc44, glibc 2.43-8. The build ran under the shared machine's
exclusive window like any other heavy work.

**What was measured.**

| property | route A (host prefix) | route B (container) |
|---|---|---|
| toolchain | ambient — a pyenv cmake, discovered after the fact | enumerated — image digest + package versions, written into the prefix |
| residue outside the prefix | build used host state; nothing installed | **zero bytes**: container storage byte-identical before and after (`--rm` discards the layer the toolchain was installed into) |
| the artifact on the host | runs (it is a host binary) | **runs, and agrees**: the container-built `llama-tokenize` on the host produces identifier-for-identifier the same output as the host-built one |
| removal | `rm -rf` the prefix | `rm -rf` the prefix; the base image was already present and is shared |
| cost of a rebuild | incremental | the toolchain reinstalls every run, because the layer that held it was discarded |

**What decides it.** Route A's failure is not hypothetical — it happened, on
the first try, silently. Route B's costs are visible and payable: a rebuild
re-resolves packages unless the image is derived and kept, and a binary built
against a container's glibc runs on the host only while the container's glibc
is not newer than the host's — true here by construction, since the image is
the host's own distribution and release, and a boundary any implementation must
check rather than assume.

DEC-052 is therefore resolved: **a controlled environment is a container MCF
drives — rootless, base image pinned by digest, source mounted read-only, a
prefix the operator can point at any drive mounted for output, and the
component's exact package set recorded into the prefix beside what was built.**
The managed prefix survives as the *shape of the output* — everything lands in
one removable directory — and the container is what makes the inputs
enumerable.

**Afterwards, the reversibility claim was exercised rather than asserted:** the
experiment's prefix was removed with one command, and the container store had
nothing to remove.

**What was not established.** Bit-identical rebuilds were not measured — two
builds from the same image and commit may still differ by timestamps, and D39's
second condition asks for "the same environment, or say what differed", which
package-set recording satisfies and binary identity would exceed. GPU and
vendor-runtime provisioning — the case that motivated D39 — adds device access
to the container and was not exercised. And `podman` itself is now a condition:
a machine without it falls back to nothing, because route A is what falling
back looks like.

## 31 · F31 — What the first automated provisioning found in two tries (B-367, DEC-052, A27, §3.15)

**What was built.** `mcf provision <component>`: F30's manual run turned into a
command. A table of components in MCF's source — one entry, the reference
implementation — each pinning an image by digest, a source by commit, a package
list, a configure line and targets. One `podman run --rm` over the pinned
image with a prefix bind-mounted; the recipe written into the prefix as a
script before it runs; the package versions, the landed commit and the log
written into the prefix by the run; `mcf-provenance.json` beside the build and
a `component_provisioned` entry in the record on success. Removal carries a
reason and is recorded before the directory goes.

**The first run failed in 30 seconds, and the failure was MCF's.** Exit 126:

```
Error: cannot chown /home/gauge/Content/mcf-data/containers/storage/overlay/…/merged
to 0:0: … read-only file system
```

Rootless podman keeps its *image store* under `$XDG_DATA_HOME/containers`.
MCF's data home on this machine is the large content drive — the operator's
instruction, and the right place for a prefix — and podman inherited it. That
drive's filesystem presents every file as root-owned and will not perform the
ownership changes an overlay store needs. 196 MB of image were pulled into a
store that could not hold them before the run stopped. F30's manual run had
not hit this only because it ran with the data home unset.

The division that holds: the *prefix* is MCF's and goes where the operator
says; the *store* is podman's, shared with everything else on the machine,
and goes where the platform puts it. `mcf provision` now hands its child the
platform default for the store and the operator's choice for the output. The
misplaced store was removed by hand, and that is the last time it will need
to be.

**The second run failed after the clone, and the failure was git's guard.**
Exit 128: `fatal: detected dubious ownership in repository at '/work/source'`.
The same filesystem, seen from inside the container, presents the freshly
cloned repository as owned by somebody else, and git refuses to operate in a
repository it thinks it does not own. The clone had succeeded and the package
versions were already recorded; the checkout was what refused. The exception
is now passed per invocation — `git -c safe.directory=/work/source` — scoped
to one directory for the life of one command, which is exactly as far as it
should reach.

**Both failures did what A2 asks.** Each named its exit status and its log,
said the prefix was safe to remove and the run safe to repeat, and left the
record untouched — nothing was written as provisioned that was not. What
neither could do was *say* which of the four D39 conditions was in play, and
that is worth noticing: the platform-mechanism category carried both with
distinct details, and a reader of the record would need the log to tell them
apart. Whether provisioning deserves categories of its own is a question for
when there is a second component.

**The third run succeeded, and found the fourth thing.** Image pulled by
digest, packages resolved and recorded exactly — gcc-c++ 16.2.1-2.fc44,
cmake 4.3.0-1.fc44, git 2.55.0-1.fc44, make 4.4.1-12.fc44, glibc 2.43-8.fc44 —
the checkout verified against the pin, three targets built, 656 MB in the
prefix, `mcf-provenance.json` beside the build, a `component_provisioned`
entry in the record, and the container store byte-identical to before (195,424
KB) with nothing created under MCF's data home. Run from the host against the
corpus's embedding model, the provisioned `llama-tokenize` produced identifier
for identifier what the hand-built one did.

*With an incantation.* Without `LD_LIBRARY_PATH` it loaded nothing:

```
error while loading shared libraries: libllama-common.so.0: cannot open shared object file
RUNPATH: [/work/build/bin:]
```

The build was shared, and a shared build bakes the library path *at build time*
into every binary — `/work/build/bin`, which is where the build directory was
inside the container and nowhere on the host. The hand-built oracle had never
shown this because its RUNPATH was a real host path. The artifact was correct,
complete, recorded, and unusable where it landed. The recipe now builds
self-contained (`-DBUILD_SHARED_LIBS=OFF`), the test that holds every recipe
requires it, and the shared build was removed *through MCF* — `mcf provision
--remove … --because "built shared: its RUNPATH names /work/build/bin …"` —
so the record says why a build that worked was thrown away (A27).

**The fourth run is the one that stands.** Self-contained: no `RUNPATH` in the
binary at all, `llama-tokenize` runs bare from the host and agrees with the
corpus, the prefix is 95 MB where the shared build was 656, the oracle tier
finds the provisioned prefix on its own and reports 57 of 57 comparisons in
agreement through it, and podman's store is still 195,424 KB. B-367's three
conditions, each exercised rather than asserted: reproducible from its record
(the recipe, the digest, the commit and the package set are all in the
prefix and the journal), removable without residue (done once, through MCF,
with the reason recorded), nothing outside the environment touched.

**Four findings from one command in one evening, none of them about the
component.** A store that follows the wrong variable, a guard that mistrusts a
mount, a path baked in from the wrong side of a boundary, and — implicit in all
three — that a provisioning which *succeeds* can still hand back something that
does not run. D39's four conditions say what a controlled environment must
guarantee; these are what it took to make one do so on one machine with one
component, and each is now either code or a test.

## 32 · F32 — A defect the oracle let through, and the hole it came through (B-364, B-368, F27, A19, A21)

**What prompted it.** B-364 listed quantization schemes MCF decodes that no
acquired file had ever carried. Five small variants of two corpus models were
acquired to change that — 88 to 113 MB each — and their directories read for
what they actually hold:

| file | tensor types carried |
|---|---|
| Llama-160M IQ4_XS | IQ4_XS ×84, Q8_0, F32 |
| SmolLM2 IQ3_XS | IQ3_S ×30, IQ4_NL ×180, Q8_0, F32 |
| SmolLM2 IQ3_M | IQ3_S ×27, IQ4_NL ×120, Q5_0 ×60, Q4_K, Q8_0, F32 |
| SmolLM2 Q2_K | **Q3_K** ×30, IQ4_NL ×180, Q8_0, F32 |
| SmolLM2 Q3_K_S | Q3_K ×30, IQ4_NL ×180, Q8_0, F32 |

Two things a file name does not say: a "Q2_K" of a 135M model carries no Q2_K
tensor at all — the quantizer falls back to Q3_K at this size — and the IQ3
variants are mostly IQ4_NL. The scheme a file exercises is read from its
directory, not its name (§3.18, again).

**Q3_K was wrong, and it read as gibberish.** `asionally himself He
intoosaicunken's.` where the reference says `Paris. Paris is the political,
cultural`. The reference indexes the 32-byte high-bit plane with the value's
position alone, the same bytes serving both halves of the block, the advancing
mask bit telling them apart; MCF indexed it with `half × 32 + position`, ran
off the end for the second half, and `get` handed back zero for every one of
those, which reads as "inverted". F19 had rewritten this decoder against the
reference and given it a test; the test decoded a block and checked the values
were all there, and they were — a plane read off its end is a plane of zeros,
which a test on a zeroed fixture cannot see. One index, and a real file, was
what it took.

**The oracle passed it, through the hole F27 named.** Its rule was: a
divergence is explained if *any* step in the generation had a margin under
0.50. The broken decoder diverged at step 0 with a margin of 0.449 — and was
excused by a 0.021 at step 4, five tokens into text that was already wrong.
Margins across the broken generation: `0.449 1.94 1.33 1.06 0.021 0.226 2.12
0.104`. Against a healthy file's: `0.573 0.248 0.640 3.38 0.105 0.466 1.75
2.93`. No summary of those two rows separates them. What separates them is
*where* they part.

The rule is now: the margin **at the step where MCF's text stops being a
prefix of the reference's**, and nowhere else. `margins --against` finds that
step. Re-measured under it across eleven files, the noise divergences part at
margins from 0.017 to 0.237; the F27 defect parted at 0.775 and this one at
0.449. The threshold moves from 0.50 to 0.30 — above every noise margin
observed, below both defects, nearer the noise — and the remaining hole is
stated in the same words as before: a defect that happens to diverge at a
genuine near-tie still passes. The gap it lives in has narrowed from
0.159–0.775 to 0.237–0.449, which is what more files do to a threshold.

The instrument found one thing about the comparison itself on its first run:
the shell had been folding newlines away where the instrument folded them to
spaces, so a paragraph break read as a divergence one step early. Both now
apply one rule — runs of whitespace are one space — and the rule is written
once in each place with the other named.

**Where this leaves the oracle.** Eleven files, 102 comparisons, all in
agreement or parting under 0.30; with the plane bug reintroduced, the tier
fails on the first prompt of the first Q3_K file — `at step 0, where they
part, the margin was 0.44906, over 0.30 — not a near-tie`. The five witness
files join the corpus tier under the schemes they actually carry.

**The old test could not have found the plane bug**, and the new one fails on
the old code: every plane bit set, every low bit zero, scales that multiply by
one. Correct decoding is 256 zeros; the old decoder produced −4 for the second
half, because a plane read off its end is a plane of zeros. F19's test had
decoded a zeroed fixture, where a plane of real zeros and a plane read off its
end look the same.

**And a scheme nobody had implemented.** IQ3_M refused: `blk.0.attn_v.weight`
in `unknown type 6`, which is Q5_0. Q5_0 and Q5_1 are now decoded from the
reference — the fifth bit of value `j` sits at bit `j` of the block's word for
the first half and bit `j + 16` for the second, which the reference does in one
shift and MCF spells out — with tests that put a bit on values 0 and 16 and
nowhere else.

## 33 · F33 — The last schemes, and the widest noise (B-364, B-368, F32, A21)

**What was acquired.** Four more witnesses, chosen by what their directories
hold rather than what their names say: `SmolLM2-360M Q5_1` (Q5_1 ×224),
`gemma-3-270m Q4_1` (Q4_1 ×126), `gemma-3-270m Q2_K` (no Q2_K at all: Q3_K,
IQ4_NL and Q5_0 — the second "Q2_K" of a small model to fall back), and
`Qwen3-0.6B Q2_K`, the first file with true Q2_K tensors (×112, beside Q3_K
×84). With these, every scheme a corpus file carries — Q2_K, Q3_K, Q4_0,
Q4_1, Q4_K, Q5_0, Q5_1, Q5_K, Q6_K, Q8_0, IQ4_NL, IQ4_XS, IQ3_S — is decoded by
a path a real file exercises and the oracle compares.

**All four cohere, and the oracle flagged one — at 0.320.** Sixteen files, 138
comparisons, one over the threshold: `Qwen3-0.6B Q2_K` on *The opposite of hot
is* parts at step 3 — `The correct statement` against `The problem is` — with
a margin of 0.320, above the 0.30 that F32 set. Both texts begin `cold.` and
the first token's margin is 3.9. Of the file's other two prompts, one agrees
outright and one parts at step 0 — `the city of Paris` against `Paris` — at
0.196, inside the noise range.

Is that a defect? The evidence says no, and it is worth setting out because the
number is close. The Q2_K decoder is structurally the same as the Q3_K decoder
F32 verified, with the one difference that its low plane really is sixty-four
bytes indexed by half. A broken decoder produced gibberish on every prompt
(F32); this file produces coherent, correct text on all three and parts once,
late, on a plausible alternative. And Q2_K is where the arithmetic gap between
the two implementations is *widest*: the reference multiplies two-bit weights
against eight-bit-quantized activations, MCF multiplies dequantized floats, and
a two-bit weight carries the least information to agree about. The largest
noise margin observed until now, 0.237, was on a Q3_K file. That the coarsest
scheme produces the widest noise is what one would predict.

**So the threshold moves to 0.40**, between this noise (0.320) and the smallest
defect (0.449). The gap it lives in has narrowed again — F27 had 0.159 to
0.775, F32 had 0.237 to 0.449, this has 0.320 to 0.449 — and every file added
narrows it further, because noise and defects are measured in the same unit.
That is the limit of a comparison of *texts*: it reads a distribution through
one sample. The next instrument compares logits (B-373), where a defect is a
different vector and noise is the same vector to within arithmetic, and the two
do not share a scale.

A second witness was sought and did not exist: `SmolLM2-360M Q2_K` carries
Q3_K and IQ4_NL and no Q2_K either, and agrees with the reference on one prompt
and parts on two at 0.059 and 0.029 — noise, and a fourth "Q2_K" file that is
not one. `Qwen3-0.6B Q2_K` stands alone as the true witness.

**What was not established.** That the Q2_K decoder is right — only that it is
structurally the verified decoder's twin, coheres on three prompts, and parts
once at a margin the coarsest scheme would be expected to produce. A21 applies:
stated as evidence, not as a verdict, until the logits comparison exists.

## 34 · F34 — Distributions against distributions (B-373, B-368, F27, F32, F33, A19)

**Why.** Three findings in a row moved the text comparison's threshold inside a
gap that closed with every file — 0.159–0.775, then 0.237–0.449, then
0.320–0.449 — because a text samples a distribution once, and the margin that
excuses a divergence is *MCF's own confidence*, which is the thing under test.
The instrument that does not narrow compares the distributions themselves.

**The instrument.** The reference exposes its distribution through one tool:
`llama-server`, whose completion endpoint returns, for every generated token,
the top-N tokens with their log-probabilities (`n_probs`). The recipe gained
that target and the oracle was re-provisioned. For each model and prompt the
tier starts one server on loopback, asks for ten greedy tokens with the top
twenty at each step, and has MCF's `margins --logprobs-of` print its own
log-softmax for exactly those twenty tokens at the same step. At step 0 the
two contexts are the same tokens by construction (the tokenizer section
already holds that); at the step where the texts part, they are compared only
if both engines reached it through identical token ids — text agreement is not
token agreement, and the first version of this compared MCF's step 0 against
the reference's step 9 and reported gaps of twenty.

**Three statistics were measured before one was chosen**, on a clean engine and
on two engines with known defects, over sixteen files and three prompts:

| engine | largest gap, top 20 | largest gap, top 5 | KL over the top 20 |
|---|---|---|---|
| clean (n = 56) | median 0.35, max **1.85** | median 0.21, max **1.12** | median 0.002, max **0.113** |
| rotation swapped (n = 63) | median 2.75, min 1.09 | median 2.03, min 0.34 | median 0.32, p90 2.6, min 0.002 |
| Q3_K plane bug, Q3_K files | ≥ 12.5 | ≥ 12.5 | min 1.59, median 7.53, n=15 |

**What the numbers say, and what they do not.** On every statistic the
rotation defect's *smallest* value sits inside the clean range: at a step where
the model barely attends to position — the second token of a five-token prompt
on a 135M model — a wrong rotation changes the distribution by less than
quantized-against-float arithmetic does. No statistic separates a subtle
defect at every position, because at some positions there is nothing to
separate. What the tier judges is every prompt of every file, and there the
separation is wide: with the floor at a KL of 0.20 — under twice the clean
maximum — the rotation mutant crosses it on the majority of its 63 comparisons
and the clean engine on none of its 56.

KL was chosen over the two gap statistics for the size of that separation
(the clean maximum is one-third of the defect's median; for the top-five gap it
is one-half) and because it is the quantity the register item named: a defect
is a different vector, noise is the same vector to within arithmetic, and a
divergence between distributions is what measures that. The gap statistics are
still printed beside it, so a reader of a failure sees all three.

**Where the noise is.** The clean engine's largest KL, 0.113, is gemma-3-270m
at Q6_K; the next are the mixture of experts at Q5_K_M and gemma at "Q2_K".
The coarse schemes and the small, wide-vocabulary model are where two
implementations' arithmetic differs most, which is what F33 predicted from the
text comparison. The floor is calibrated on this corpus; a file whose noise
sits above 0.113 would be a new measurement, not a defect, and would say so by
which file and which prompt.

## 35 · F35 — What a load costs, apart from running (DEC-018, D41, B-034, §3.13)

**The question.** §7.18 asks whether a served model stays resident, and frames
it as a cold start on every request against memory held. Before deciding, the
two costs were separated on this machine — one reading each, no window, so the
figures are for scale and not for the record's baselines.

**Through the daemon, loading per request, Qwen3-0.6B at Q4_K_M, a one-token
answer:** 6.54, 6.52, 6.49 s. **The same with the model held after the first
request:** 6.66 s (the load), then 12.28 s and 5.63 s resident. Residency did
not make the request faster, and the second resident run was slower than the
load — the machine is shared and nothing here was attributable, which is what
the outlier says and all it says.

**So the load was measured alone.** `load`, a diagnostic that reads, parses and
dequantizes and prints each:

| model | read | parse | dequantize | on disk |
|---|---|---|---|---|
| Qwen3-0.6B Q4_K_M | 0.112 s | 0.041 s | 0.636 s | 397 MB |
| gemma-3-270m Q6_K | 0.080 s | 0.056 s | 0.293 s | 283 MB |
| Llama-160M Q4_K | 0.034 s | 0.007 s | 0.177 s | 121 MB |

Against that, six forward passes through Qwen3-0.6B in this process — a
five-token prompt and one generated token — take 6.8 s. The load is 0.8 s. On
MCF's own engine a request is the forward passes, and the load is the twelfth
part of the shortest one.

**What this decides, and what it does not.** It decides D41: residency is held,
because it costs nothing idle and hides nothing when stated, and because on a
faster engine the proportions invert — but it is not the saving §7.18 imagined
on the engine MCF has today, and pretending otherwise would be a number
without its conditions. It does not decide anything about two models, about
memory pressure, or about an engine whose forward pass is milliseconds; those
are DEC-009's and B-032's, and this finding is the baseline they will be read
against.

**And what residency costs idle: nothing.** The soak tier idles a daemon for
sixty seconds with the fixture resident after one generation and reads zero
context switches and zero clock ticks of processor time — the same figures as
an empty daemon — with the record unchanged. Residency is memory and nothing
else, which is the condition D41 was decided under.

**What was not established.** These are single readings on a shared machine
without the window; the 12.28 s is unexplained and recorded as such. The
budget tier's first-token figure (B-035) is the one taken under conditions,
and it is on the fixture, where the load is microseconds either way.

## 36 · F36 — The reference model answers, through an engine that is a process (B-032, B-033, B-367, D39, §XII)

**What was built.** An engine adapter for the provisioned `llama.cpp` (F31): a
supervised subprocess per generation, its output streamed to the client as it
arrives, its exit classified into the stage it died in — not started
(`engine.spawn.not_found`, `engine.spawn.refused`), dead before a byte
(`engine.exit.immediate`), dead after part of an answer
(`engine.exit.midstream`), killed (`engine.exit.signal`) — and the daemon
standing after each. The laboratory produces every one of those with a shell
that dies the stated way (D26), and the whole-system tier drives a fake engine
through the daemon: chosen without being asked when it is the only one
provisioned, named in the account, its partial answer kept and its own last
words attached when it dies mid-answer.

**The stated rule for which engine serves** (§3.15): what the client asks for;
else the provisioned engine where exactly one is here; else MCF's own. Two
provisioned pins is refused by name rather than chosen between. The account
says which served — `engine: provisioned llama.cpp @925e1179947e from
<prefix>` — and a provisioned answer carries no degraded mark, because it is a
real engine and D39 says a timing taken under stated conditions through it
would be a measurement.

**Two things the first live run found.** The completion tool's `--log-disable`
silences the completion itself in this build: the first provisioned
generation exited cleanly with an empty answer. And `mcf run` was refusing the
reference model *before* asking the daemon, on MCF's own engine's architecture
check — a check that belongs to the engine that will run, and only that one.
Both are fixed and the whole-system tier holds them.

**Then the reference model answered.** `Qwen3.8-27B-UD-Q4_K_M.gguf` — 27.3
billion parameters, 16.5 GB, the artifact §XII named, refused by MCF's own
engine as an architecture it has not been taught and as 109 GB dequantized
against 63 usable (B-372) — through the provisioned engine, on this machine's
processor, twelve tokens:

```
$ mcf run …/Qwen3.8-27B-UD-Q4_K_M.gguf --prompt "The capital of France is" --limit 12 --engine provisioned
 Paris.
The capital of Germany is Berlin.
  produced 12 token(s); stopped: engine_finished
  engine   provisioned llama.cpp @925e1179947e from …/provisioned/llama.cpp@925e1179947e
```

Ten and a half seconds of wall clock, load included, one reading on a shared
machine without the window — for scale, not for the record (B20). The corpus
model through the same engine: `Paris. The capital of France is also the
capital of the`, twelve tokens, 1.7 s.

**What this closes.** M2's first exit criterion in the form §VI meant it: a
model of the size §XII chose for residency and arbitration, on a machine that
had never served it, from one command. B-032 in both halves — the engine is a
recorded condition, and it is a supervised subprocess. B-033 at the stages a
process can die in. And the path D39 opened: what MCF provisions, MCF drives.

**What was not established.** Nothing timed under conditions — the engine may
be timed, and the budget tier does not time it yet (B-035 measures MCF's own
engine on the fixture, on purpose). Residency for a subprocess engine: this
loads per request, and the daemon's resident model is MCF's own engine's; a
long-lived server child is the shape that would hold a 16 GB model between
requests, and that is a further increment of B-032. One engine, one component,
processor only.

## 37 · F37 — The first probe found two defects and then refused to answer (B-051, B-052, D42, §3.18, F25, F26)

**What was built.** `mcf probe`, the framework D42 describes, and the first
configuring probe. Its question is the chat template, and its observation needs
no judgement: a model addressed the way it was trained ends its turn at its own
stop token, and one addressed wrongly runs to the budget. Five trials per
addressing, forty tokens each, through a named engine.

**The first defect: MCF never used the stop token it had been reading.** Every
generation ran to its budget, because `Request.stop` was empty at both call
sites — `mcf run` and the daemon — while the vocabulary had been carrying
`ending` since the tokenizer was written. A model saying *I am done* was
generated as an ordinary token and passed over. Nothing had noticed, because
nothing had asked *why* a generation ended; the probe asks exactly that, and
found it in its first run. Both sites now pass the model's own end of text.

**The second defect: the answer was an artifact of the instrument.** With the
stop token honoured the probe decided, and said `raw` was best for both
SmolLM2 and gemma3 — with `turns 0 of 5` against `raw 5 of 5`. That is a clean
result and it was worthless. MCF refuses to parse control tokens out of prompt
text — a prompt must not be able to produce a chat marker by spelling one
(F26) — so `<|im_start|>` written into a prompt reaches the model as **eight
ordinary tokens**, and `<start_of_turn>` likewise. The chat addressings were
never applied. The probe had compared raw against a garbled prompt and reported
the garbling as the model's behaviour.

This is F25's lesson arriving from the other side. There, a broken engine read
*better* than a correct one; here, a broken experiment produced a *cleaner*
result than an honest one — `5 of 5` against `0 of 5` is the most decisive
table in this document and it means nothing.

**So the probe now checks that what it sends is what it means.** Each marker is
tokenized and must come back as one token spelling itself. Where it does not,
the addressing is untestable *as text*, and the probe reports inconclusive
naming exactly why. On the corpus today that is every instruct model:

```
SmolLM2-135M   INCONCLUSIVE — chatml cannot be applied: its markers are control
               tokens, and MCF does not parse control tokens out of prompt text
               (F26) … addressing has to be built from token identifiers (B-374)
gemma-3-270m   INCONCLUSIVE — turns cannot be applied: …
Llama-160M     INCONCLUSIVE — no addressing ended at the model's own stop token
               within 40 tokens; a larger budget may decide it
```

**A probe that will not answer is the probe working.** D42 made *inconclusive*
a first-class outcome for exactly this: *the model did not do the thing* and
*MCF could not tell* are different facts, and only the first would license a
configuration. What MCF has learned here is about MCF — its prompt surface
cannot express a chat turn — and B-374 is the item that fixes it: addressing
built from token identifiers rather than from text, which is what a chat
template actually is.

**What was not established.** Nothing at all about how these models prefer to
be addressed; that is what B-374 unblocks. The stop-token fix is verified only
in that generations now end at a model's own end of text where they did not
before — how often, on which models, under which addressing, is the same
unasked question until the probe can ask it.

## 38 · F38 — The probe's answer was upside down, and the decisive column was the broken one (B-052, B-374, D42, §3.18, F25, F37)

**What was built.** B-374: a chat turn assembled from token identifiers rather
than from text. A request now carries either a prompt or a list of identifiers;
MCF builds the identifiers from the model's own markers, taken from the
template in its own file; and what a *user* typed is never among them. This is
what F26 left unbuildable and F37 stopped on — with it, the chat-template probe
could compare addressings for the first time.

It compared them, decided, and was wrong in the exact opposite direction.

**The observation counted silence as success.** SmolLM2-135M, every one of its
five quantizations, reported the same table:

```
im_start…im_end as assistant 0 of 5
raw                          5 of 5 ←
```

The addressing was verified correct before this was believed — the identifiers
were dumped and read back:

```
["<|im_start|>", "user", "Ċ", "What", "Ġis", "Ġthe", "Ġcapital",
 "Ġof", "ĠFrance", "?", "<|im_end|>", "Ċ", "<|im_start|>", "ass", "istant", "Ċ"]
```

That is ChatML, exactly. So the two addressings were sent by hand and what came
back was read rather than counted:

| addressing | tokens produced | ended at | what it said |
|---|---|---|---|
| ChatML | 40 | the budget | "The capital of France is Paris. Paris is a city located in the northern part of the country…" |
| raw | **0** | its own stop token | *nothing* |

The addressing the model answers under scored **zero**. The addressing it
refuses to speak under scored **five of five**. A model addressed in a way it
does not recognise emits its end-of-turn token *first thing* — and the probe's
question, *did it end at its own stop token*, cannot tell that apart from a
finished answer. It is the same token. The whole result was inverted.

**Why it was believed for as long as it was.** Because it was consistent. Five
quantizations of the same model, `5 of 5` against `0 of 5` every time — the
kind of stability that reads as signal. F25 is the same lesson from the other
side (a deleted MoE router read *better* than the correct one) and F37 the same
again (a garbled prompt gave a cleaner table than an honest one). Three times
now the decisive-looking result has been the broken one. **Consistency is not
correctness, and a clean margin is not evidence about anything but itself.**

**The fix is not a threshold.** A trial counts only if the model *said
something and then stopped*: `Trial::Stopped` carries how many tokens preceded
the stop, and zero is a refusal to speak, not a completed turn. The boundary is
not arbitrary — nothing said is nothing said. The silences are kept and printed
rather than folded into the failures (A1), because *the model recognised its
stop token and declined the turn* is a fact worth having:

```
im_start…im_end as assistant 5 of 5 ←   turn ran 10-292 token(s), middle 96
raw                          1 of 5     (ended without saying anything 4 of 5)   turn ran 120
```

That is SmolLM2 after the fix, and it agrees with the file's own declaration.

**The turn lengths are kept, because they were already paid for.** A trial
cannot tell a finished turn from a refusal without counting what preceded the
stop (that *is* the fix), so the count exists whether or not it is recorded —
and throwing it away would be discarding a measurement MCF already made (A1).
It is the observation a stop-condition question is asked of (B-056), and the
first candidate for separating addressings that tie on *did the turn end*
(B-375). The spread above is worth noticing on its own: the same model, the
same addressing, five short factual questions, and turns from 10 tokens to
292. Any budget chosen for *this* probe by looking at one question would have
been wrong for the others — which is how the budget came to be measuring
verbosity in the first place.

**A third defect fell out of the first two: five trials were one trial.** MCF
samples greedily from a fixed seed. Five trials of one question are one
observation written down five times, and `5 of 5` was arithmetic wearing the
costume of evidence (A19). The probe now asks a different short question each
trial, so five trials are five observations of the thing actually being
asked — *does this addressing get an answer out of this model*.

**And the budget was measuring the model's verbosity.** At forty tokens the
correct addressing had not finished; at 160 it still had not; it ends its turn
cleanly at **250**. A budget too small to reach the end of a turn makes every
addressing look identical, so *ran out of budget* was doing the work that *did
not stop* was being credited for. It is 320 now, and B49 holds — the budget is
in tokens, not seconds.

**gemma-3-270m stays inconclusive, and correctly.** Under the broken
observation three addressings tied at `5 of 5`; once silence stopped counting
and the budget was large enough to reach the end of a turn, the tie narrowed to
**two** — so one of the three had been tied on refusals. Two addressings answer
and end the turn equally often, and this observation cannot tell those two
apart. The
tie-break that preferred `raw` was itself a defect — `max_by_key` returns the
*last* maximum, which handed every tie to whichever candidate was listed last,
and `raw` always is. A tie is now `inconclusive` naming the tie, because
choosing on a tie would be MCF reading its own default back as a finding
(§3.15, D42).

**What was not established.** Whether the addressings gemma3 ties under are
genuinely equivalent for it, or whether a sharper question separates them —
that is the next increment, and the probe says so rather than guessing. Nothing
about any model larger than 270M: both models here are small on purpose, and
what a probe learns on a 135M model is about that model. And nothing is
configured by any of this — `mcf run` still sends raw text (§3.8); D42 holds
that a probe writes the verified half of a capability and never a default.

## Changelog

### Version 42 — the probe's answer was upside down

F38. B-374 built: a chat turn assembled from token identifiers, which is what
a chat template actually is. The probe then reported the exact inverse of the
truth — a model that refuses to speak emits its end-of-turn token immediately,
and *did it stop* cannot tell that from a finished answer. Fixed by requiring
the model to have said something, by asking a different question each trial
(greedy sampling made five trials one trial), and by a budget large enough to
reach the end of a turn. B-052 answers for the first time, and agrees with the
file it was checking.

### Version 41 — the first probe found two defects and then refused to answer

F37. `mcf probe` and the chat-template probe. It found that MCF had never used
the stop token it was reading — every generation ran to its budget — and then
that its own first answer was an artifact: MCF cannot put a control token into
a prompt (F26, rightly), so the chat addressings it scored were never applied.
It now reports inconclusive with that reason, which is D42's third state doing
its job.

### Version 40 — the reference model answers

F36. An engine that is a process: the provisioned llama.cpp driven as a
supervised subprocess per generation, its exit classified by the stage it died
in, the daemon standing after each. The reference model — 27B, 16.5 GB, the
artifact §XII named — produced text through MCF in ten and a half seconds.

### Version 39 — what a load costs, apart from running

F35. Loading Qwen3-0.6B costs 0.8 s; the six forward passes of a one-token
request cost 6.8 s. Residency saves the twelfth part of the shortest request on
MCF's own engine, and everything on a faster one. D41 is decided on that.

### Version 38 — distributions against distributions

F34. The oracle compares the reference's top-twenty log-probabilities against
MCF's log-softmax for the same tokens, at the same step, through the same
token ids. Three statistics measured on a clean engine and two known defects;
KL over the top twenty chosen for the width of its separation — clean maximum
0.113 against a subtle defect's median of 0.32 — and the floor set at 0.20.
The instrument's own first failure, comparing step 0 against step 9, is
recorded with it.

### Version 37 — the last schemes, and the widest noise

F33. Four more witnesses close B-364's list: every scheme a corpus file carries
is exercised and compared. The first true Q2_K file parts from the reference
once at 0.320 — evidence says noise, on the coarsest scheme, and the reasoning
is set out — so the threshold moves to 0.40 and the gap it lives in is stated
as the thin thing it has become. A logits comparison is the next instrument.

### Version 36 — a defect the oracle let through

F32. Five small variants acquired to exercise the schemes no file had carried,
and the first thing they found was that MCF's Q3_K decoder read its high-bit
plane off the end — gibberish where the reference says Paris — and the second
was that the oracle had excused it: the broken engine parted from the reference
at step 0 with a margin of 0.449 and was explained by a 0.021 five tokens later.
The rule now takes the margin at the step where they part and nowhere else;
the threshold sits at 0.30. Q5_0 and Q5_1 are decoded, which IQ3_M needed.

### Version 35 — what the first automated provisioning found in two tries

F31. `mcf provision` exists, and its first two runs each found something real.
Rootless podman put its image store under MCF's data home — the content drive,
whose filesystem cannot hold an overlay store — so the store and the prefix now
go to different places on purpose. Then git refused the cloned repository as
foreign-owned from inside the container. Both failures named their exit status
and log and touched nothing in the record.

### Version 34 — two ways to provision, measured

F30. The same component was provisioned through a host prefix and through a
rootless container, and the measurements resolve DEC-052. The host route failed
D39's first condition on its very first use — the oracle was built by a cmake
from a pyenv shim, recorded nowhere, different from what the system says — and
the container route left container storage byte-identical while producing a
binary that runs on the host and agrees with the host-built one exactly.

### Version 33 — the sixth family answers a different question

F29. The bert family embeds, through its own verb — `mcf embed`, vector first
as one JSON line, conditions after — and everything transcribed was checked
against the reference the same day: the tokenizer exactly, the forward pass at
a measured cosine floor of 0.999 that a single swapped normalization falls
through, and the vector's meaning by related-against-unrelated sentences.

The corpus is six for six.

### Version 32 — the mask, past the boundary

F28. The sliding-window mask F27 implemented but could not exercise is now
compared past the boundary: 682 tokens of repetition, and a distinctive name
parked outside the sliding blocks' view with a cue after the filler. Both agree
with the reference token for token. The long comparison is gated behind
`MCF_ORACLE_LONG=1` because the stand-in pays a forward pass per prompt token.

### Version 31 — what a coin-flip looks like, and what a defect looks like

F27. Before the forward pass could be compared at all, a way was needed to tell
two correct implementations disagreeing on a near-tie from one of them being
wrong. That was measured rather than assumed.

Four divergences across fifteen comparisons were noise: margins of 0.040, 0.098,
0.105 and 0.159, in every case the reference choosing exactly MCF's runner-up,
in every case at the smallest margin of that whole generation. The one real
defect diverged at 0.775.

The defect was the sliding-window rotary base. Gemma 3 rotates its sliding
blocks at a different frequency from its global ones, the artifact does not
declare it, and the reference falls back to ten thousand where MCF was using the
million the file states for the others — so five blocks in six were rotating
wrongly. The model still said `Paris.` correctly, which is why nothing before
this caught it. F24 had recorded this as untested ground.

The forward pass is no longer unchecked, for these models and prompts. The
threshold that separates the two cases is provisional, and a defect can still
hide under a near-tie.

### Version 30 — the oracle found a defect on its first run

F26. llama.cpp at a pinned commit, built as a development instrument, compared
against MCF on token identifiers — the one part of an engine that can be checked
exactly, because identifiers are integers.

Twenty-nine of thirty comparisons agreed. The thirtieth was a real defect, and
an invention of MCF's own: a guard that skipped user-defined tokens shorter than
three bytes, on the plausible reasoning that a short token would match inside
ordinary words. No implementation does that, and gemma's vocabulary carries a
token spelled as two literal spaces.

The reference also settled something MCF had guessed: control tokens are left as
text and only user-defined ones are matched, which is both the compatible answer
and the safer one — a prompt should not be able to produce a chat marker by
spelling it.

What found the defect was the tab-and-double-space string, not the sentence.
Gemma answers `Paris.` correctly with the defect present.

### Version 29 — the broken engine read better than the correct one

F25. A mixture of experts runs, read from the reference. The shape comes from
the file's own `expert_count` rather than from the family name — the artifact
declares `llama` and is a mixture of four.

Removing the router entirely, so that every token goes to experts 0 and 1,
produces `Paris. It is located on the River Seine in`. The implementation
transcribed from the reference produces `Paris, France is Paris, Paris is Paris
is`. The broken version reads better.

F20, F22 and F24 each showed that a wrong engine can look right. This shows a
wrong engine looking *better* than a right one, on the same artifact and prompt.
Output quality is not evidence about correctness in either direction, and this
is the strongest case for B-368 that can be made.

### Version 28 — a third architecture, and three habits that fail three ways

F24. Gemma 3 runs, read from the reference rather than inferred. It differs from
llama in five places: two the file states — the normalizations on the way out of
each half of a block — and three it does not — a `GELU` gate, an embedding
scaled by the square root of its width, and the other rotary pairing.

Removing each in turn gives word salad, fluent-and-wrong, and text in another
script. The middle one is the dangerous one, and it is now the third
unobservable habit across two families to fail that way. *Unobservable habit,
got wrong, produces fluent text* looks like the general case rather than a
coincidence.

The division holds up: everything observable is read from the artifact, so
gemma3's normalizations needed no new code path — only two more tensor names.
Everything unobservable is in the one table DEC-053 already governs.

### Version 27 — four expressions where MCF had two

F23. Implementing one more pre-tokenizer meant reading the reference rather than
inferring from a name, and the reading found three defects already shipped:
`qwen2` and `llama-bpe` are different expressions (digits singly against groups
of three), `deepseek-llm` was claimed and is not implemented, and `default` is
not GPT-2's expression but a fourth one.

The first is the instructive one. Qwen3-0.6B tokenizes a numeric prompt
identically before and after the fix, because its merge list never joins digits.
No prompt against that model distinguishes the broken version from the fixed
one. Some facts are about somebody else's software and are read, not measured.

A fourth defect came from the round-trip now run over every corpus vocabulary:
the empty string had stopped encoding as anything for unigram vocabularies, a
regression from adding control-token matching.

And SmolLM2-135M corrects F22's generalization: it is *smaller* than
Llama-160M and answers correctly where that model does not, so the floor for
refereeing an engine is not a parameter count but whether that artifact knows
the answer being asked of it.

### Version 26 — where a model becomes able to referee an engine

F22, which measures what F21 asserted. Two corpus models, run against a correct
engine and one with the rotary pairing swapped — F20's *fluent* defect, chosen
because it is the hard one to see.

At 160M the reader cannot tell the columns apart, because the correct engine
answers three of four prompts wrongly on its own: a model that does not know the
answer cannot be asked whether the engine found it. At 0.6B every prompt is
unmistakable and the correct column is right.

The rule is therefore not *small models are unfit* but *a model must be good
enough to be right about the thing being asked* — which starts below a billion
parameters and costs sixteen seconds. F21's claim that the corpus depends on an
oracle is corrected: the dependency exists at the bottom of the corpus and not
one step up.

### Version 25 — six families for a tenth of one model's bytes

F21. The reference model is 16.5 GB and every engine iteration ran it. Six small
trained models — one per family, one quantization each, 1.4 GB together —
replace it for that job, and §XII is amended (D40) to say which job each
artifact has.

Two of the six run. The other four refuse, each naming what it wants: a
pre-tokenizer, a tokenizer scheme, an expert-routing tensor, an architecture.
Those four refusals are B-365's remaining order of work, read off artifacts
rather than predicted.

The cost is stated rather than glossed: small models cannot referee what F20
turned on, so the corpus depends on B-368's oracle rather than merely preferring
it.

### Version 24 — a second architecture, and two silences that fail differently

F20. Qwen3 needed two things llama does not: a rotary pairing that splits the
head in half rather than taking adjacent pairs, and a normalization of each
query and key head before the rotation. Neither is stated in the file.

Running the engine with each correction present and absent gives four outputs,
and the point of the finding is that they are not four degrees of one failure.
Without the per-head normalization the model emits one token forever. With it
and the wrong rotation, it emits *English* — and an observer without the fourth
row beside it would call that a small model being small.

The normalization was the more serious of the two, because the code for it was
already written: it asked for its weights with `if let Ok(…)`, the manifest
never loaded them, and the error was discarded on every block of every model.
A2 inside an engine written under A2.

### Version 23 — two defects a real model found in an hour

F19. The operator pointed out that their machine already holds a hundred
gigabytes of models, three of which declare the architecture MCF's engine
implements. Running one took an hour and found two defects, neither of which
any test could see.

The tokenizer used SentencePiece's unigram search where these files are read by
a merge algorithm — and in a vocabulary whose byte tokens score zero and whose
words score minus thousands, that search spells every word out one byte at a
time. And `Q6_K` decoded every value correctly into the wrong position, which
in that file corrupts the projection to logits and nothing else.

The third part is the one worth keeping: both had tests, and both tests were
incapable of failing. A uniform block cannot show a permutation, and a fixture
vocabulary of whole words with no intermediate pieces cannot be tokenized by the
correct algorithm at all. A fixture MCF writes to test MCF is shaped by the same
misunderstanding as the code.

### Version 22 — three findings point forward to the one that qualified them

F12, F14 and F15 each recorded that this machine has no C toolchain targeting
musl, and F18 later found it packaged. A reader landing on any of the three
would take the absence for a fact about the world, so each now says where to
read next. Nothing measured is changed — a finding is what was observed when it
was observed — and what each *concluded* is bounded to the condition it was
observed under.

### Version 21 — the wall was a package nobody priced

F18. Three findings — F12 on the engines, F14 on SQLite, F15 on elevation — each
ran into the same absence and treated it as a fact of the world: this machine
has no C compiler targeting musl. Asking the package manager takes a second and
nobody had.

It is packaged: `musl-gcc`, `musl-devel` and `musl-libc-static`, from Fedora's
own repository, about nine megabytes installed. That does not make any candidate
build — a C wrapper says nothing about a C++ project's CMake or about
Oniguruma's configure — and trying it means changing a machine somebody else is
using, which is its operator's call. What it changes is the question B-320 is
answering: not *is this possible* but *are we willing to require a toolchain at
build time*.

### Version 20 — what a hub says when something is gone

F17. §7.38 has asked since it was written what happens when a pinned artifact
decays, and what MCF can detect is a property of the hub rather than of MCF, so
the hub was asked — anonymously, metadata only.

A repository that does not exist answers **401**, with the body *Invalid
username or password*. So *gone*, *private* and *never existed* are one answer,
and no credential MCF could be given tells them apart. A gate, by contrast, is
visible before it bites: the card reads 200 with `"gated":"manual"` while the
file reads 401. And 404 means exactly one thing — a revision that is not in a
repository that is.

The two decays that matter most refuse nothing at all: a relicensing and a
repointed tag are a changed field with a 200 beside it, detectable only by
comparing against what was recorded at acquisition.

### Version 19 — three defects, found by pointing MCF at one real repository

F16. Deciding which quantization of the reference model to pin is the operator's
call, and MCF is supposed to inform it without downloading anything. Pointing
the planner at the real repository for the first time found three defects in
code that had tests.

MCF's JSON reader refused the configuration outright over `1e-06`, because it
was written to keep floating point out of the record — correct property, wrong
mechanism, since the same reader reads documents MCF did not write. The
transformer's fields were under `text_config`, where a multimodal repository
puts them. And counting every block rather than the sixteen that actually cache
would have overstated the KV cache fourfold — the one defect that would have
produced a confident wrong number instead of no number.

All three are fixed, all three have tests, and the plan now classifies all 33
variants without fetching a byte.

### Version 18 — what a machine lets an ordinary user do

F15. §6.32 wants the privileged surface enumerated and §7.39 records that nobody
had enumerated it; documentation cannot, because the answer differs by machine.
So it was asked, without changing anything: pinning cores, bounding memory,
reading temperatures and reading per-process accelerator occupancy need no
privilege; a governor, real-time scheduling, dropping caches and IRQ affinity
do; and reading processor energy — which D11 makes first-class — needs
elevation on this distribution, which is the awkward case §7.39 anticipated.

The probe's own first answer was wrong in the instructive way: `nice -n -5 true`
exits zero having failed, because the shell reports the command's status rather
than whether the priority was applied. Asking the child what it actually ran at
reverses the row. A declaration is not an observation, at the level of a shell
script.

### Version 17 — the index earns its bytes, and SQLite does not

F14. D6 named SQLite the record and D20 later made the record a journal with a
derived index over it; the question of which is which was decided by measuring
both. A replay costs 8 µs an entry — 7.89 s at a million, at every daemon start
— and the derived index answers the same questions in 196 µs from 32 bytes an
entry. SQLite answers them in the same order of time and costs 9.2 MiB of C, 52
seconds of compile, and B-183's static musl container, which cannot be built
here at all for want of a C cross toolchain.

D6 is amended rather than dropped: the journal is the record, the index over it
is derived, and SQLite is a cost to pay if a query arrives that an offset table
cannot answer.

### Version 16 — two writers, one record

F13 added, because the daemon gave MCF a second thing that writes to the record
and §7.37 has never been answered. Eight processes, one file, one whole line per
write, at 400 bytes and 8 KiB and 128 KiB, on tmpfs and on btrfs: sixteen
thousand lines every time, none torn, none interleaved.

That settles the alarming half — two MCF processes do not produce a record
nobody can read, and the load tier now asserts it — and leaves DEC-037's real
question, which turns out to be narrower than it looked. Not *how do we
coordinate writes*, but *who mints an identifier*: each writer counts its own
appends, so two entries can carry the same `EntryId`. Nothing is lost; what is
broken is the assumption that an identifier names one entry.

### Version 15 — the engine's second half, and an expectation removed

F12 completed in the exclusive window. Both candidates build for the ordinary
target; llama.cpp produces 12.9 MiB of static libraries against D24's 40 MiB
ceiling, and a minimal candle program is 1.6 MiB needing only the three
libraries §3a allows.

The result worth recording is the one that did not go as expected. F9.5 chose
the TLS provider on musl: the C candidate could not build for the target
B-183's container uses, and the Rust one could. The same test was expected to
choose the engine — and it does not, because `candle-core` depends on
`tokenizers`, which depends on Oniguruma, which is C. Neither candidate keeps
that check runnable without a cross toolchain, so the engine decision turns on
other things: one upstream against a hundred and forty-three, and a second
tokenizer MCF already owns.

### Version 14 — what an engine would cost, half-measured on purpose

F12 added, and it is the first finding here that reports an *unfinished*
measurement. The cheap half is done: llama.cpp is 35 MiB of one project's C++
under one licence, candle is 152 crates of Rust under many, and this machine has
no musl cross toolchain — the condition that decided the TLS provider in F9.5.

The deciding half needs builds of several minutes on a quiet machine, and the
machine is shared. Rather than guess at the two numbers that matter — whether
either builds for musl, and what each does to D24's 40 MiB ceiling — the finding
says it does not know them, and the script that would fill them in is committed
beside it. A finding that reported an unmeasured half as measured would be the
thing this file exists to prevent.

### Version 13 — a disk fills at the flush

F11 added with B-026. Writing to a full filesystem through a buffered writer
*succeeds*: the buffer takes the bytes and the failure arrives at the flush. A
fetcher that checked its writes and ignored its flush would verify a digest over
bytes that never reached the disk and record an acquisition that did not happen.

Both halves of §3.11 are built on it. A file that will not fit is refused before
a byte moves, with the arithmetic in the refusal rather than a verdict — which
needed the second `unsafe_code` opt-out in the workspace, one `statvfs` call,
checked against `df`. And a filesystem that fills anyway is classified rather
than reported as a general write failure, with `/dev/full` as the scenario.

### Version 12 — what a machine says when there is no network

F10 added, and it closes §7.11 as D33. The measurement that decides it is small
and slightly surprising: a name that will not resolve produces no error kind at
all — `Uncategorized`, no errno — whether the machine has no network, no
resolver, or asked for a name that does not exist. One observation, three
causes, and no way to tell them apart without making a request nobody asked for.

So MCF reports what it saw and names the question it is not answering. The other
three cases *are* distinguishable and are now kept apart, because refused, no
route and silence are three different things for a person to act on. And the
other half of the void needed no experiment: the from-scratch container already
runs everything except acquisition with no network at all.

### Version 11 — the provider that keeps the checks runnable

F9.5 and F9.6 added, and they change the answer 9.4 pointed at. `rustls`'s usual
cryptography is C, and it cannot be built for the musl target B-183's
from-scratch check uses without a cross toolchain nobody has here — so admitting
it would make an existing check runnable in fewer places. A pure-Rust provider
by rustls's own author builds for both targets, compiles fourteen crates rather
than sixteen, and holds a real TLS 1.3 session with the hub, which was run
rather than assumed.

And the ninety megabytes turn out to be avoidable: 9.4 was right that a vendored
tree cannot be *deleted* down to the platform and wrong that it cannot be
filtered. A crate nothing compiles can keep its manifest, its licence and an
empty `lib.rs`. Thirteen megabytes, no C, both targets, offline and locked.

The cost that remains is maturity, and it is stated rather than discovered: MCF
is choosing a young implementation, and it sits behind one trait so that
choosing differently later is one struct.

### Version 10 — what a vendored tree actually costs

F9.4 added after the obvious economy turned out not to exist. Most of a
vendored TLS tree is Windows import libraries a Linux build never compiles, so
the natural move is to trim them — and cargo refuses, because with a vendored
directory it resolves the whole lock graph before compiling any of it. The
honest figure for admitting a TLS stack is therefore about ninety megabytes of
source in the repository rather than fifteen. The fifteen is what gets
compiled, which is the right number for reviewing what MCF ships and the wrong
one for what a checkout costs.

### Version 9 — the transport, measured before it was argued

F9 added. Four backlog items were stopped at the same sentence — *what remains
is the transport* — and the question behind it had never been measured. It is
now: the hub speaks HTTP/1.1, publishes the size and the SHA-256 before the
bytes, serves ranges, and names the revision in the download's own headers.
What MCF lacks is not a protocol, it is TLS, and the honest cost of that is
sixteen crates and 15 MiB rather than the 91 MiB a vendored tree reports —
because most of a vendored tree is Windows import libraries a Linux build never
compiles. The shape that would cost nothing to vendor is the one that demands
`libssl` of the user's machine, and it is in the finding as the control.

### Version 8 — the slope §7.4 was arguing about

F8 added. §7.4 said MCF cannot be faster at matrix multiplication than the
projects that specialize in it, and asked not to be settled on that reading
alone. The prototype measures the slope: the best safe portable Rust it could
maintain is twenty-five to fifty times slower than one core of a *generic*
tuned BLAS on the same machine, and the careful tiling step made it slower than
the one-line reorder — which is what a treadmill looks like from the bottom.

### Version 7 — the signal F5 was missing

F7 added with B-193. A major page fault charged to a child is what tells a
cold-start measurement it went to a device, and the experiment that shows the
signal moving — zero warm, thirty evicted, over the same thirty spawns — runs in
the gating tier, because F3's lesson is that a signal nobody has watched fail is
not known to work.

### Version 6 — the first mutant to survive

F6 added with B-186. The experiment that checked whether the floor refuses
found a real gap while it was at it: nothing tested that MCF's resident-memory
reading is in bytes, and it is a figure `mcf doctor` prints and B-011 asserts
against a ceiling. The mutant is now in the catalogue and the claim has a test.

### Version 5 — a budget that measures the filesystem

F5 added. The first `--all` run failed its cold-start budget by a factor of
two and a half and a later one passed it by two orders of magnitude, on one
machine within the hour; the cause is a `fuseblk` mount whose p99 page-fault
service time is a thousand times its median when its cache is cold. Two things follow and both are
registered as B-193: the storage an artifact is executed from is a condition
nothing records, and D30's signal watches the measuring thread, which for a
cold start is the one thing not doing the work.

### Version 4 — what the new tiers found

F4 added with B-191. Five tiers ran for the first time and four of the six
things they found are about MCF rather than about themselves — a condition that
did not round-trip, a replay whose footprint is proportional to the journal, a
reading that could not be taken in parallel, and a fuzz campaign that had to be
made to say what it reached. The other two are about the tiers, which is what
D10 means by the test of the tests.

### Version 3 — the signal that answers F2, and the one that could not fail

F3 added. It closes what F2 opened and corrects the ground both F1 and F2 stood
on: their cold-start figures were taken on a machine that was never quiet, and
the honest number is thirty times smaller. Neither was wrong about passing; both
were wrong about the number, which is exactly what §3.4's conditions exist to
prevent and what a contaminated one hides.

The second half is a finding about method rather than about MCF. The first
implementation of the new signal read a process's accounting rather than a
thread's, and therefore reported a perfectly clean measurement under any load —
a signal that could not fail. It was caught by running the tier under deliberate
load and checking the verdict *changed*, which is the negative control B-003
established for a lint check, applied to a measurement.

### Version 2 — the budget tier reports that it cannot judge

F2 added. B-011's first run on a real machine established something worth
keeping: the tier's state-class figures assert cleanly, its event-class ones
cannot be asserted at all on a machine somebody is using, and no threshold
fixes that because the machine really is busy. DEC-051 registers the question
rather than the tier quietly loosening until a number passed.

### Version 1 — the adversarial prototype reports

Created to hold F1. B-002's condition is that §7.19 be *amended or confirmed in
writing*, and neither the intent document nor the register is the right place
for the evidence: the first states positions and the second states status, and
a run's conditions and readings are a third kind of thing. Putting them in
either would have made a document say something it is not for.
