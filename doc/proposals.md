# Proposals

| | |
|---|---|
| **Type** | Proposals — features argued in full, not yet accepted |
| **Version** | 2 |
| **Status** | Living |
| **Authority** | Derived from [document-of-intent.md](document-of-intent.md) v9, governed by [rules.md](rules.md) |
| **Feeds** | [backlog.md](backlog.md) on acceptance · [roadmap.md](roadmap.md) for placement |

**Why this document exists.** B15 admits weight only against a stated cost, B32
admits a laboratory only by answering *what claim can MCF make once this exists
that it cannot make now*, and §5 refuses features whose only justification is a
comparison table. A one-line backlog row cannot carry that argument. Each
proposal here states the claim it enables, how it works concretely, what it
costs, what it collides with, and what remains unanswered — so that accepting it
is a decision and refusing it is a recorded one (C6).

**Nothing here is accepted.** A proposal enters [backlog.md](backlog.md) only
when its recommendation is taken, and a refused proposal stays here with the
reasoning, so it is not re-proposed later as an oversight.

## Contents

| # | Proposal | Recommendation | Would land |
|---|---|---|---|
| P1 | [Customizable workloads](#p1--customizable-workloads) | Accept, rescoped — build the slot into the lab framework | M6 |
| P2 | [The repro bundle](#p2--the-repro-bundle) | Accept — cheap, and §II is built on it | M5 |
| P3 | [Pre-acquisition planning](#p3--pre-acquisition-planning) | Accept | M1 |
| P4 | [Your own second machine](#p4--your-own-second-machine) | Accept, narrow scope | M9 |
| P5 | [Contention diagnosis](#p5--contention-diagnosis) | Accept, small | M5 |
| P6 | [The stop control](#p6--the-stop-control) | **Accepted** — registered as B-210 | M2 |

---

## P1 — Customizable workloads

**One line.** A laboratory ships a default workload and accepts yours, so that
*"does this model do the thing I need"* becomes a measurement rather than a
guess.

### Where this comes from

MCF's purpose is to say which model to run (§IV), which requires measuring
quality — the question §7.3 called the hardest in the project. §IX answered it
with *checkable tasks*, and D2 has since made the answer plural: quality is a
set of separately-measured properties, one per laboratory, each run only where
the model is capable of it (§3.23).

That leaves one question unanswered: **where does a laboratory's workload come
from?** §7.23 lists four sources — published benchmarks (comparable, but
contaminated over time), authored for MCF (controlled, but unvalidated as
representative), procedurally generated (fresh, but of uncertain realism), and
derived from the user's own use case (*"maximally valid, ungradable without
effort"*). The fourth is the one nobody owns, and this proposal is that fourth
option, scoped to what it is actually worth.

§3.19 states the reason plainly and then states its own limit: the benchmark
should resemble the work, and *"resemblance is not the same as reality."* A
classification lab shipping MCF's example labels measures classification in
general. The same lab holding your labels measures the thing you will actually
do with the model.

### What it is, and what it deliberately is not

**It is a workload slot** (B42, §6.37). A laboratory declares what its slot
accepts — documents, schemas, labels, constraints, task definitions — and the
user supplies data. [labs.md](labs.md) names the slot for every candidate lab;
the most valuable are L12's labels, L10's documents, L7's schemas, L13's test
suites and L8's tool schemas.

**It is not a programming interface.** No user code, no plugin, no lab API.
B32's prohibition is unaffected, and the boundary is the difference between
*configuring an instrument* and *building one*.

**It is not session capture.** An earlier version of this proposal had MCF
record real sessions from the serving path and convert them into tasks
automatically. That version is refused, below.

### How it works

**1. A lab declares its slot** in machine-readable form: what a workload item
contains, how many are needed for a result to be reportable, and — the critical
part — **how an item is graded**. A lab that cannot grade what you supply
refuses it at load rather than producing an ungradable run (B42).

**2. The user supplies a workload file.** For L12, a list of examples and their
correct labels. For L7, schemas and example inputs. For L13, specifications and
the tests that check them. This is data authoring, and it is the honest cost of
this feature: fifteen minutes of your effort buys a measurement about your work.

**3. The lab runs identically to its default configuration.** Same conditions,
same instrumentation profile, same sandbox, same distribution reporting. Only
the workload differs, which is exactly the isolation §3.4 wants.

**4. The result is marked non-comparable and local, at the point of production**
(B42). It is not contributable — it is your content (A25), and a score against a
workload nobody else can see is uninterpretable to them. Locally it is the most
valuable number MCF can produce.

### What it costs

Modest, and mostly in the framework rather than per lab. A slot declaration
format, a loader with validation, a grading contract each lab implements
anyway for its default workload, and the marking that keeps custom results out
of contributions. If the lab framework (B-111) is built with a slot from the
start, per-lab cost is close to zero; retrofitting it later is a rewrite of
every lab.

### Collisions

- **§3.4, isolation.** A custom workload is one variable changed, which is
  clean. Comparing *your* result against *someone else's default-workload*
  result is not, and B42's production-time marking is what prevents it.
- **A25, §3.10.** Workload files are user content: stored in the content store,
  never contributed, never in an export.
- **B32, not a platform.** Data in, not code. If the slot ever acquires
  expressions, conditionals or callbacks, it has become the lab API §5 refuses.
- **A10, never train on the test.** A private workload is used for selection.
  MCF's own defaults must never be tuned toward it.

### What is refused, and why it is recorded

**Automatic session capture.** The earlier version had MCF record flagged
sessions in the serving path, classify them by gradability, and replay them with
recorded tool responses. It is refused on three grounds, kept here so it is not
re-proposed as an oversight (C6):

1. **It inverts the instrument.** §XIII's laboratories are designed instruments
   that examine one property under controlled conditions. Real captured work
   varies instruction style, context, difficulty and tool availability at once —
   the least isolated measurement obtainable, and the opposite of a microscope.
2. **It puts the user's most sensitive content in MCF's custody** to produce a
   result a fifteen-minute workload file produces without it.
3. **The cost is disproportionate.** Capture, classification, and deterministic
   tool replay is a large subsystem; the slot is a file format.

A narrower descendant may deserve reconsideration later: showing a captured
session *as a drafting aid* while the user authors a workload item, with no
automatic conversion. That is P1 plus a convenience, not a different feature,
and it should not be built until slots exist and someone finds authoring them
tedious.

### Open questions

- How large may a slot be before a lab measures something other than what it
  claims? §6.37 says the lab must state what its slot accepts; where exactly
  that boundary sits is unresolved.
- How few workload items make a reportable result? DEC-023's statistics, with a
  much smaller `n`, and the honest answer may be "twelve items is not a suite
  and MCF should say so".
- May a workload move between the user's own machines (P4) without becoming a
  contribution? Probably yes, and it needs saying explicitly.

### Recommendation

**Accept, at M6, and build the slot into the lab framework rather than after
it.** The retrofit is the expensive order. Scoped this way it is a modest
feature that delivers the whole of the original ambition — measurement of the
work you actually do — without capture, without a content pipeline, and without
turning a designed instrument into a recording of uncontrolled work.

---

## P2 — The repro bundle

**One line.** One file that contains everything needed to reproduce one claim,
so that "prove it" has an answer that is not a conversation.

### Where this comes from

§II is the intent that makes MCF worth trusting: *"every measurement carries the
obligations of a measurement — a stated method, stated conditions, stated
uncertainty, and the ability for someone else to repeat it."*

Three of those four are already built. The record holds the conditions (§3.3),
`Measurement<T>` cannot exist without its spread (A6), and every lab declares
its method (B30). **The fourth is not built by anything on the roadmap.** There
is no artifact MCF produces that another person can take and use to repeat a
result. "Someone else can repeat it" is currently a property of the design
rather than a thing you can hand over.

That gap is easy to miss because §XIV looks like it fills it. It does not: a
contribution is *many rows, stripped and aggregated, for a corpus*. A bundle is
*one claim, complete, for a person* — including the things a contribution
deliberately removes, like the full hardware identity and the raw samples. They
are opposite artifacts serving opposite purposes, and the same intent asks for
both.

### The claim it enables

§II says every measurement carries "the ability for someone else to repeat it."
Nothing in the roadmap currently produces the artifact that makes that true. The
record holds the conditions; the mockups render them; but there is no single
thing a user can hand to a colleague, attach to a bug report, or keep for
themselves that says *here is the claim, and here is everything required to
check it.*

With P2, MCF can say: *this bundle reproduces this number, or tells you exactly
why your machine cannot.* That closes the gap between §II's promise and what MCF
actually ships, and it does so for one claim at a time rather than by publishing
a database.

### How it works

A bundle is a single file containing:

| Part | Content | Why |
|---|---|---|
| **The claim** | The measurement, its value, `n`, spread, and the statement being made | §3.4 |
| **The method** | Which lab, which version, which parameters, which statistic | §3.4, B30 |
| **The conditions** | Full §3.4 floor: hardware, driver, runtime build, quantization, context, KV precision, thermal state, instrumentation profile, MCF version, clock source | A6, D9 |
| **The raw samples** | Every trial, not the summary | §7.33 — and the reason it matters |
| **Provenance** | The artifact's full chain, including transformations (§3.6) | A7 |
| **The identifier** | §XV's identifier for the configuration | B33 |
| **A verification manifest** | What a reader should re-run, and what constitutes agreement | §II |

`mcf verify <bundle>` on another machine reproduces the configuration (via the
identifier), re-runs the method, and reports agreement or divergence with the
conditions compared side by side — which is exactly the M9 mockup's divergence
report, driven by a file instead of an identifier.

### What it costs

Small. Every component already exists or is already required: the record holds
the conditions, §XV holds the identifier, D6 holds the samples, §XIV's export
path holds the serialization. P2 is mostly *assembly*, and its cost is dominated
by deciding the format (which §7.30 must decide for contributions anyway).

### Collisions

- **§7.33 is a hard dependency.** A bundle containing only summaries cannot be
  re-analysed, which defeats half its purpose. If DEC-033 chooses summaries, P2
  degrades to a provenance receipt.
- **A24 and §3.20.** A bundle is a publication the moment it is sent to someone,
  so producing one is not gated but *sending* one is the user's act, and the
  bundle must show what it contains before it leaves — same surface as `mcf
  share --preview`.
- **A25.** A bundle from a private-suite measurement (P1) would contain user
  content. It must not: the bundle carries the *outcome and method*, and where
  the method is a private task it says so and is not portable. That is a real
  limitation and should be stated rather than engineered around.

### Open questions

- One format for both a bundle and a contribution, or two? One is cheaper and
  makes §7.30's schema work count twice.
- Does a bundle carry the *weights*? Almost certainly not — it carries the
  identifier that fetches them — but that makes verification network-dependent,
  which §7.11 cares about.

### Recommendation

**Accept, and place at M5**, when the first defensible number exists. It is
cheap, it is the most direct expression of §II in the whole project, and
building it at M5 forces the record to be complete enough to reconstruct a claim
— which is B21's reconstruction requirement pointed at measurements instead of
failures.

---

## P3 — Pre-acquisition planning

**One line.** Before downloading anything, answer *what of this will run here,
and what should I expect* — across every quantization the repository offers.

### Where this comes from

§III commits MCF to *any* model on the hub, and §6.3 reads "any" as governing
attempt and diagnosis rather than success: every reference reaches a defined,
actionable outcome. The M1 mockup honours that — it says "needs 131 GiB, you
have 24" and refuses cleanly.

But it says so **after resolving the reference**, and for a large repository
that means MCF already knows the file sizes while the user is still deciding
whether to spend the bandwidth. The information needed to answer *"will this run
here"* arrives before the download and is currently used only to justify a
refusal, never to inform a choice.

§XII sharpened this into a practical problem. The reference model publishes
roughly twenty quantizations from 1-bit to BF16. Choosing among them by
downloading candidates is tens of gigabytes per guess, and nothing in MCF
currently helps — despite MCF holding both the arithmetic and, after M5, a
history of what similar configurations actually did on this machine.

### The claim it enables

The M1 mockup already refuses honestly *after* resolving a reference: "needs
131 GiB, you have 24." P3 turns that refusal into a decision aid and moves it
before the bandwidth is spent.

MCF can then say: *this repository publishes twenty quantizations; seven fit
here with headroom, four fit without room for your context, nine do not fit;
based on what you have measured on this machine, expect roughly this throughput
band from each of the seven.* That is the §IV frontier, sketched before
acquisition, from measurements MCF already owns.

This matters more with the reference model than it did before: §XII names a
model published across roughly twenty quantizations from 1-bit to BF16, and
choosing among them blind means downloading tens of gigabytes to find out.

### How it works

**1. Fitment, which is arithmetic and exact.** Weights bytes + KV cache at the
requested context + runtime overhead versus available accelerator and host
memory. MCF already computes this at M1; P3 computes it for every variant in the
repository without fetching any of them, from file sizes and metadata alone.

**2. Expectation, which is inference from local history and must be labelled as
such.** Once M5 has measured *some* configurations on this machine, MCF can
project a band for unmeasured ones from the relationship between quantization
size and throughput observed here. This is an **estimate** (A20) — clearly
marked, never comparable with a measurement, never promotable — and where MCF
has no local history it says so and offers fitment alone.

**3. A recommendation only if §IV can honestly make one.** Before any local
measurement exists, P3 states fitment and stops. B34 forbids a foreign number
choosing a local configuration, and B29 forbids generalizing from one model.

### What it costs

Small, with one caveat: the projection model in step 2 is a model, and a wrong
one produces confident bad guidance. It must be conservative, band-shaped rather
than point-shaped, and validated against subsequent real measurements — the same
discipline §6.16 applies to the laboratory, applied to a prediction.

### Collisions

- **A20.** The projection is an estimate. If it ever appears beside a
  measurement without its label, this feature has done net harm.
- **B34, §5.** The temptation is to project from contributed data instead of
  local history, which would smuggle foreign numbers into a local decision. P3
  uses this machine's history only.
- **§3.7.** Metadata used for fitment comes from the hub and is untrusted; a
  declared file size that is wrong produces a wrong plan, so the plan is
  re-checked against reality on acquisition and any divergence is a finding.

### Open questions

- How far can a size→throughput relationship be extrapolated on one machine
  before it becomes fiction? Probably not across architecture families, and
  possibly not across quantization schemes.
- Should P3's prediction be scored against the eventual measurement and
  reported? Yes, almost certainly — a prediction nobody grades is a guess, and
  grading it is nearly free.

### Recommendation

**Accept, and place at M1** for the fitment half, which needs nothing but
metadata and is immediately useful. The expectation half arrives at M5 once
there is local history to project from, and should not be built earlier.

---

## P4 — Your own second machine

**One line.** Compare this machine with another machine you own, using the same
suites and your own record — the only cross-machine comparison that does not
violate §5.

### Where this comes from

§5 refuses cross-machine rankings, and §6.28 sharpened the reason: MCF is not
suspicious of foreign *data*, it is suspicious of foreign *conclusions*. Numbers
from a stranger's machine travel without their conditions and were taken by
someone whose method you cannot inspect.

Neither objection survives when both machines are yours. Same operator, same MCF
version, same suites, conditions recorded on both, nothing to trust. The general
prohibition exists for reasons that are absent in this specific case, which is
what makes it worth carving out rather than an exception being smuggled in.

There is a second origin, and it may matter more. §6.16 rests MCF's credibility
on a laboratory and then admits the risk: *"a simulator built from our own
assumptions tests our assumptions, not the world."* DEC-020 asks how much real
hardware is needed to validate it and has no answer. A user with two machines,
running the same scenarios under one operator, is the cheapest real-hardware
validation datum the project can obtain — and it arrives as a side effect of a
feature they wanted anyway.

### The claim it enables

§5 refuses cross-machine rankings because they travel without their conditions
and were taken by strangers. Neither objection applies to *your own two
machines*: same operator, same suites, same MCF version, conditions recorded on
both, no trust required.

MCF can then answer questions a solo operator genuinely has and currently
cannot: *is my laptop good enough for this, or do I need the desktop? What does
the 4090 actually buy me over the integrated GPU for this model? Which of my
machines should host this?* These are §IV questions with the hardware as the
variable instead of the model, and MCF is already recording everything needed to
answer them.

It also produces something scientifically useful for free: **a second point on
the reality-versus-lab curve** (§6.16, DEC-020). Two machines under one
operator, running the same scenarios, is the cheapest real-hardware validation
available.

### How it works

MCF already has the mechanism — §XIV's contribution format and §XV's identifier
— and P4 is mostly a matter of *scope*, not new machinery. A record is exported
to a file, imported on the other machine, and the comparison runs locally
against both.

The critical constraint, and the reason this is narrow: **a personal transfer is
not a contribution.** It goes machine-to-machine at the user's direction, not to
a public corpus. It therefore does not need §7.27's de-identification (the user
already knows their own hardware), does not enter the crowd-sourced path, and
may include the private suites P1 produces — which a contribution never could.

Comparison enforces isolation (A8) exactly as it does locally: two machines
differing in accelerator, driver, thermal environment and host memory differ in
four variables, so MCF reports the difference and refuses to attribute it to any
one of them unless the others were held still.

### What it costs

Small if scoped to the user's own machines, because it reuses §XIV's
serialization and §XV's identifiers. It grows expensive the moment it becomes
"compare with anyone", which is the thing it must not become.

### Collisions

- **§5, and this is the one to watch.** P4 is legitimate because trust and
  conditions are not in question. If it ever grows a "compare with a friend's
  machine" affordance, the leaderboard objection returns in full and §6.28's
  contribute-outward-decide-inward rule is what stops it.
- **B34.** A measurement from the user's second machine is still not a local
  measurement, and must not feed a recommendation *for this machine*. It informs
  a decision about *which machine to use*, which is a different question and
  should be a different surface.
- **A25.** A personal transfer may carry private-suite content, so it is not the
  contribution format with a flag flipped — it is a different export with
  different rules, and conflating them would be the exact defect A25 exists to
  prevent.

### Open questions

- Is "machine identity" stable enough to compare against itself over time? A
  driver update makes yesterday's machine a different apparatus, which §7.13's
  comparability logic already knows how to express.
- Does this need a decision about whether the two records merge or stay
  separate? Probably separate, with comparison as a read-time operation.

### Recommendation

**Accept, narrowly, at M9**, alongside the exchange machinery it reuses. Guard
the scope in writing: the moment it generalizes past machines a single operator
controls, it becomes the thing §5 refuses.

---

## P5 — Contention diagnosis

**One line.** A one-shot answer to *what is competing for this machine right
now*, so that B24's "unattributable" verdict comes with a name attached.

### Where this comes from

§3.8 makes the machine part of the experimental apparatus and states the
obligation directly: *"MCF knows the difference between 'this model is slow' and
'this machine was busy'. When it cannot tell the difference, it says so rather
than attributing the result."* B24 encodes that as a verdict — *unattributable*
— and the M5 mockup shows it in action: a run invalidated because an unrelated
process held the accelerator for four minutes.

The refusal is correct and incomplete. Having been told a measurement is
unattributable, the operator's next question is always *by what?* — and MCF is
the only thing positioned to answer, because it was watching the machine at the
moment it happened and nothing else was.

D8 raised the stakes. A laboratory now owns the machine and may be greedy, which
means a lab must *begin* on a quiet machine or its exclusivity guarantee is
fiction. Something has to establish quiet before a run starts, and that
something is this proposal.

### The claim it enables

B24 already requires MCF to say "I cannot tell whether the model is slow or the
machine is busy." P5 upgrades that from a refusal to a diagnosis: *your
accelerator is 71 % occupied by another process; your memory is 94 % committed;
your accelerator has been thermally throttled for the last four minutes.*

The value is asymmetric and worth noting. When a measurement is invalidated by
contention (the M5 mockup shows exactly this), the user's next question is
always *by what?* — and MCF is the only thing in a position to answer, because
it was watching the machine at the moment it happened.

### How it works

An on-demand snapshot, never a monitor. When a run is invalidated, or when the
user asks, MCF samples: per-process accelerator occupancy, memory pressure,
thermal and clock state against their steady-state baselines, and any of its own
activity. The result attaches to the invalidation record so the finding survives
(§3.1), rather than being a transient thing on a screen.

**It is emphatically not a monitor.** B4 refuses ambient sampling and D5 settled
that argument; P5 samples when something happened or somebody asked. If it ever
acquires a background loop, it has become the observability platform §5 refuses.

### What it costs

Very small, and much of it is already required: B-013's hardware profiler and
D11's thermal reads exist for other reasons. The genuinely new part is
per-process attribution, which is platform-specific and may require the
privileged helper (A26) on some systems — which makes it a small consumer of
work §XVII already justifies.

### Collisions

- **B4, D5.** On demand only. This is the rule most likely to be eroded here,
  because a diagnosis is more useful with history, and history means sampling.
  It should be refused: the M5 mockup's invalidation record already captures the
  contention *window*, which is the history that matters.
- **§3.10.** Naming another process names what the user is doing. The snapshot
  stays local, never enters a contribution (A25), and should probably be
  redacted by default in anything rendered beyond the operator.

### Open questions

- How much per-process attribution is available without privilege, per platform?
  Feeds DEC-039.
- Does MCF ever *act* on this — refusing to start a lab because the machine is
  busy? D8 makes a lab exclusive, so probably yes: it should refuse to begin
  rather than produce an invalid result, which makes P5 a precondition for D8's
  quiet-machine check rather than a diagnostic afterthought.

### Recommendation

**Accept, small, at M5**, where invalidation-by-contention first occurs. The
last open question above may promote it: if D8's pre-flight needs to verify the
machine is quiet, P5 is not optional at all.

---

## P6 — The stop control

**One line.** One command, one button: stop everything, unload every model,
release the accelerator, and say what was stopped.

### The claim it enables

Nothing scientific. This one is about the operator at 2am whose machine is
unusable, and it is the smallest item in this document.

It nevertheless has a rule behind it. §3.11 says nothing is destroyed without
deliberation, and §3.1 says no failure is silent — but neither says the user can
*reliably reclaim their own machine*, and a tool permitted by §XVII to take
exclusive accelerator access, lock pages and raise priorities absolutely owes
them a way to make it stop. **The more MCF is allowed to take, the more it owes
a reliable release.**

### How it works

`mcf stop` and an equivalent control in the window (A22 — parity). It:

1. Refuses new work immediately.
2. Interrupts any running laboratory, preserving the partial result and marking
   it incomplete (§3.1, B35) rather than discarding it.
3. Drains and terminates supervised runtimes, escalating from a polite request
   to a kill with a stated deadline.
4. Releases every held resource: accelerator contexts, locked pages, exclusive
   modes, raised priorities, and anything the privileged helper set (A26) —
   restoring a performance governor it changed is part of stopping, not a
   nicety.
5. Writes a record of what was stopped and what was preserved.
6. Reports what it could not release, and why, rather than claiming success.

Step 6 is the one that makes this a real feature rather than a convenience.
`--force` skips the graceful escalation; nothing skips step 6.

### What it costs

Nearly nothing to build, and it pays for itself in the lab: a reliable stop path
is a *testable* stop path, and every fault-injection scenario that kills a child
mid-work is exercising the same machinery (B18).

### Collisions

- **§3.1.** A partial result is preserved and marked, never discarded because
  the run was cancelled. Cancellation is not failure and is not success.
- **A26.** Releasing privileged state is part of stopping. A stop that leaves a
  performance governor pinned has not stopped.
- **§3.11.** Stopping destroys no artifact — it releases resources — so it is
  not a gated category, and requiring confirmation to stop would be a small
  cruelty.

### Open questions

- Does `mcf stop` stop the daemon, or stop the daemon's *work*? Almost certainly
  the work, with the daemon surviving to report what it did — which is §6.1's
  "never lose information" applied to the act of stopping.
- Is there a dead-man's variant: if the control plane is wedged, what stops
  MCF then? A supervised daemon should be killable by the operating system
  without leaving the machine in a held state, and that is a lab scenario
  (§7.21) rather than a feature.

### Recommendation

**Accepted, at M2**, with the daemon, and registered as B-210. It is the
smallest item here, the most frequently missed in tools of this kind, and it
becomes obligatory rather than optional the moment §XVII lets MCF take exclusive
control of hardware.

---

## Changelog

### Version 2 — P1 rescoped, P6 accepted

P1 is rewritten from the ground up. The original proposed automatic session
capture and replay, which inverted the instrument: §XIII's laboratories examine
one property under controlled conditions, and captured real work varies
everything at once. Rescoped to a **workload slot** — a lab ships a default and
accepts yours as data — it delivers the whole of the original ambition at a
fraction of the cost and with no content pipeline. The capture version is
recorded as refused, with its reasoning, rather than deleted.

P6 accepted and registered as B-210.

### Version 1 — six features argued in full

Created so that a feature can be argued before it is committed to. B15 admits
weight only against a stated cost and B32 admits a laboratory only by naming the
claim it enables; a backlog row cannot hold either argument, so proposals were
previously either accepted silently or lost.

The six here came from an audit of what the intent document implies but nobody
owns. Two are notable: P1 converts the project's central hedge — that the suite
is a proxy for the work — into a measurement of the actual work, and P6 is
obligatory rather than optional now that §XVII permits MCF to take exclusive
control of hardware.
