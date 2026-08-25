# Proposals

| | |
|---|---|
| **Type** | Proposals — features argued in full, not yet accepted |
| **Version** | 1 |
| **Status** | Living |
| **Authority** | Derived from [document-of-intent.md](document-of-intent.md) v8, governed by [rules.md](rules.md) |
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
| P1 | [Bring your own work](#p1--bring-your-own-work) | Accept — highest value in this document | M6 |
| P2 | [The repro bundle](#p2--the-repro-bundle) | Accept — cheap, and §II is built on it | M5 |
| P3 | [Pre-acquisition planning](#p3--pre-acquisition-planning) | Accept | M1 |
| P4 | [Your own second machine](#p4--your-own-second-machine) | Accept, narrow scope | M9 |
| P5 | [Contention diagnosis](#p5--contention-diagnosis) | Accept, small | M5 |
| P6 | [The stop control](#p6--the-stop-control) | Accept — smallest item here, and the most missed | M2 |

---

## P1 — Bring your own work

**One line.** The user points MCF at transcripts of what they actually do, MCF
turns them into a private, verifiable laboratory, and every subsequent
measurement is about *their* work rather than a proxy for it.

### The claim it enables

Today MCF can say: *this model completes tasks from suite `core` 71 % of the
time.* §3.19 concedes what that is worth — "resemblance is not the same as
reality, and an agentic suite is still a proxy" — and §6.28 says the same thing
about aggregate data from other machines.

With P1, MCF can say: *this model completes tasks drawn from your own work 64 %
of the time, and this other one 71 %.* That is not an incremental improvement in
validity. It is the difference between a proxy and the thing itself, and it
resolves the residual §7.3 complaint that the suite resembles the work only to
the extent the suite does.

It also does something no aggregate can: **it is uncontaminated by
construction.** §7.3 and §6.30 both worry that published tasks are eventually
trained on. A task derived from a user's own transcripts, never published, is
outside that failure mode permanently — and §7.23 already lists user-derived
tasks as the "maximally valid, ungradable without effort" option nobody owns.

### How it works

The hard part is not capture; it is **grading**. §3.19 requires checkable
outcomes and B13 forbids grading by another model. A raw transcript has no
checkable outcome. So the design turns on converting transcripts into tasks that
*do*, and being honest about the ones it cannot convert.

**1. Capture.** MCF already sits in the serving path. A session may be marked
for capture — explicitly, per session, never by default (§3.10) — recording the
request, the tool calls, the tool results, and the final state. This lands in the
content store, which is separate from the record and never contributable (A25).

**2. Classify by gradability.** Each captured session sorts into one of three:

| Class | Definition | Gradable |
|---|---|---|
| **Outcome-checkable** | The session ended in a verifiable state change: a file written, a value computed, a command that exited zero, a structured output that parses against a schema | Yes, automatically |
| **Assertion-gradable** | No state change, but the user can state a checkable property: "the answer contains this identifier", "the output is valid JSON with these keys", "it called `search` before `summarize`" | Yes, once the user writes the assertion |
| **Ungradable** | Prose, tone, judgment, taste | No — and MCF says so rather than admitting a model judge |

The third class is the honest one. §7.3 leaves open whether MCF measures
non-agentic quality at all, and P1 does not answer it: those sessions are stored,
counted, and reported as *"41 % of your captured work is not gradable by this
method"*, which is itself a finding a user should know.

**3. Replay as a lab.** A gradable session becomes a task in a private lab: the
tool implementations are replaced with sandboxed recordings of what the real
tools returned (A14 — the model under test never touches anything real), the
starting state is reconstructed, and the assertion becomes the grade. Everything
except the model is held still (B12).

**4. Report as a distribution** (B12), with the same failure taxonomy §6.17
requires, and with a loud statement of the corpus's size and shape — twelve tasks
from one afternoon is not a suite, and MCF should say so rather than producing a
confident percentage from it.

### What it costs

- **The largest single feature in this document.** Capture, classification, an
  assertion authoring surface, deterministic tool replay, and corpus statistics.
- **It touches the serving path**, which §3.13 guards jealously. Capture must be
  off by default and cost nothing when off, or B4 is violated.
- **It puts the user's most sensitive content into MCF's custody** — §3.10's
  central worry, made concrete. The content store's separation (A25) stops it
  leaving, but MCF is now holding it, which raises DEC-005's retention question
  from theoretical to urgent.
- **Assertion authoring is user work**, and a feature that requires the user to
  write assertions will be used by a fraction of the people who install it.

### Collisions

- **§3.10 and A25.** Captured content is user content: separate store, separate
  retention, never contributable, never in an export. The measurements *derived*
  from it (success rates, failure classes) are contributable; the tasks are not.
  §6.30's outcomes-not-artifacts rule already draws exactly this line.
- **B13, no model judges.** The assertion-gradable class exists precisely so
  that "the user knows what right looks like" is captured as a *check* rather
  than delegated to another model.
- **§5, not a chat product.** Capture is an instrument for producing tasks, not
  a conversation feature. If it grows session management, editing or sharing of
  transcripts, it has crossed the line.
- **A10, never train on the test.** A user's private suite is used for selection.
  Nothing here tunes MCF's defaults toward it, and the selection/validation split
  must extend to a private corpus.

### Open questions

- How many tasks constitute a usable private suite, and what does MCF say when
  the corpus is too small? DEC-023's statistics apply, with a smaller `n`.
- Do private tasks rot as the user's work changes? A corpus captured in March
  measuring a model in September may be measuring the wrong thing.
- Can a private suite be *exported* to another of the user's own machines
  (P4) without becoming a contribution? Probably yes, and it needs saying.

### Recommendation

**Accept, and place at M6** as a second laboratory once the framework (B-111)
exists. It is the highest-value feature in this document because it converts the
project's central hedge — *the suite is a proxy* — into a statement about the
user's actual work, and it is the one feature no aggregate, website or
leaderboard can ever provide.

---

## P2 — The repro bundle

**One line.** One file that contains everything needed to reproduce one claim,
so that "prove it" has an answer that is not a conversation.

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

**Accept at M2**, with the daemon. It is the smallest item here, the most
frequently missed in tools of this kind, and it becomes obligatory rather than
optional the moment §XVII lets MCF take exclusive control of hardware.

---

## Changelog

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
