# Findings

| | |
|---|---|
| **Type** | Record — what a prototype or a run established, and what it changed |
| **Version** | 3 |
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
| 3 | [F3 — The load average answers the wrong question (DEC-051, D30)](#3--f3--the-load-average-answers-the-wrong-question-and-the-obvious-fix-silently-could-not-fail-dec-051-d30) |
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

## Changelog

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
