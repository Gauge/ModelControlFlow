# Proposals

| | |
|---|---|
| **Type** | Proposals — features argued in full, not yet accepted |
| **Version** | 12 |
| **Status** | Living |
| **Authority** | Derived from [document-of-intent.md](document-of-intent.md) v14, governed by [rules.md](rules.md) |
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

**Proposals are `PR<n>`, and used to be `P<n>`.** The letter `P` names the five
precedence rules in [rules.md](rules.md), and one letter cannot name two things
a reader has to tell apart — the same bare identifier meant *science outranks
speed* in one document and *the repro bundle* in another. C5 forbids reusing or
renumbering an identifier and permits deprecating one in favour of a **named
successor**, so `P<n>` here is deprecated in favour of `PR<n>`, digit for digit.
A citation made before this change still resolves. Registered as B-353.

## Contents

| # | Proposal | Recommendation | Would land |
|---|---|---|---|
| PR1 | [Customizable workloads](#pr1--customizable-workloads) | **Accepted** — B-204, B-205 | M6 |
| PR2 | [The repro bundle](#pr2--the-repro-bundle) | **Accepted** — B-211, B-212 | M5 |
| PR3 | [Pre-acquisition planning](#pr3--pre-acquisition-planning) | **Accepted** — B-213, B-214, B-215 | M1 · M5 |
| PR4 | [Your own second machine](#pr4--your-own-second-machine) | **Dropped** — subsumed by §6.38 | — |
| PR5 | [Contention diagnosis](#pr5--contention-diagnosis) | **Accepted** — B-216, B-217 | M5 · M6 |
| PR6 | [The stop control](#pr6--the-stop-control) | **Accepted** — registered as B-210 | M2 |
| PR7 | [Longitudinal regression detection](#pr7--longitudinal-regression-detection) | Accept — the one artifact §6.7 names and nothing builds | M8 |
| PR8 | [The stand-in engine](#pr8--the-stand-in-engine) | **Accepted** — B-360, B-361, B-362 | M0 · M2 |
| PR9 | [What serving looks like](#pr9--what-serving-looks-like) | **Accepted** — DEC-001 decided; B-032, B-033, B-034 build it | M2 |
| PR10 | [What a probe is, and when configuration may change](#pr10--what-a-probe-is-and-when-configuration-may-change) | **Accepted** — DEC-024 and DEC-025 decided; B-051–B-060 build it | M3 |
| PR11 | [Prompt analysis: what the model actually received](#pr11--prompt-analysis-what-the-model-actually-received) | **Accepted in part** — the static half, as B-381, B-382, B-383; the behavioural half deferred | M4 |
| PR12 | [Lifting the stand-in's ceiling, and why the cheap version is worse](#pr12--lifting-the-stand-ins-ceiling-and-why-the-cheap-version-is-worse) | **Accepted** — do not lift it; B-366 first, then B-384 measures the usable size | M5 |

---

## PR10 — What a probe is, and when configuration may change

**One line.** A probe is an experiment MCF runs on demand, whose result is a
measurement with conditions and is never a default; and MCF never reconfigures
a model under a user — it reports that its answer would now differ, and
applying that is an act.

**Why this is a proposal.** DEC-024 and DEC-025 gate all of M3, and they are
one question in two halves: what a probe *is* determines whether its result may
change something, and whether configuration may change determines what a probe
result is allowed to be. Arguing them apart produces a probe framework that
cannot say what to do with its own output.

### The claim it enables

**That a measurement taken on MCF is a measurement of the model.** §X's reason
for existing is §3.8's: a misconfigured model is a measurement error. The
concrete case is in the corpus today. Every instruct model there carries
`tokenizer.chat_template` — Qwen3's is forty lines of Jinja — and MCF ignores
all of them: `mcf run --prompt "The capital of France is"` sends raw text to a
model trained to see `<|im_start|>user`. What comes back is a completion from a
model being asked the wrong kind of question, and every number taken on it
would be a number about that mistake.

### What a probe is (DEC-024)

**Four properties, and the first three are already types MCF has.**

1. **Its result is a `Measurement`, not a boolean** (B-051). A probe runs
   trials; what it reports is what was observed across them, with the spread.
   *Three of five stop-token trials stopped* is a different fact from *stopping
   works*, and only the first survives being read six months later.
2. **It carries its conditions** (§3.4). A probe result belongs to a
   *(model, engine, engine build, sampler, seed)* — not to the model. The same
   file through MCF's own engine and through a provisioned one is two
   observations, and the second does not inherit the first's answer.
3. **Its result is a `Capability`, never a default** (B-050). A probe writes to
   the `verified` half; nothing writes a probe result into a configuration
   value. What reads it must ask for it as an observation.
4. **It says how much it cost**, in tokens rather than seconds (B49's shape).

**Which capabilities are probed, and the rule that decides.** A probe earns its
place when a wrong answer to it would corrupt a measurement or a served answer.
That is a test, not a list, and it sorts the open-ended list of §7.24 into two
kinds:

- **Configuring probes**, which change how MCF talks to the model, and are M3's:
  the chat template (B-052), stop conditions (B-056), usable context against
  claimed (B-055). Getting these wrong corrupts everything downstream.
- **Characterizing probes**, which describe what a model can do without
  changing how MCF talks to it: tool calling (B-053), structured output
  (B-054), the modalities of B-057. These are findings about a model, they
  belong beside M6's laboratories, and M3 builds the framework they will use
  rather than the probes themselves.

The division is not a deferral: it is the answer to *what bounds "full"*. §X's
word is bounded by §3.8's reason for it.

**When probing happens: on demand, and never at acquisition.** `mcf probe
<model>` runs them; `mcf run` uses what has been probed and says when nothing
has. Not at acquisition, for three reasons — custody is not evaluation (M1's
job is that the bytes arrived intact); a probe needs an engine, and which
engine is a condition, so a probe at `pull` time would be a result about
whatever engine happened to be installed; and it would pay for every model
somebody never runs.

**What a probe result costs to keep.** It is a measurement with conditions, so
it caches exactly as far as its conditions hold and no further — which answers
§7.24's question about caching across MCF versions without a new rule. A
different engine build is different conditions. MCF does not compare across
them and does not silently reuse.

**Inconclusive** (B-060) is a first-class outcome and is not a negative. A
probe that could not decide reports what it saw and how many trials it took,
and MCF configures nothing from it. §3.18's third state exists because *the
model did not do the thing* and *MCF could not tell* are different facts, and
the second one licenses nothing.

### When configuration may change (DEC-025)

**Never under a user. MCF may learn better; what it does with that is say so.**

A configuration MCF derived carries the probe that set it, when, and under what
conditions (B-059). When MCF's answer *would now differ* — a better probe, a
new engine, a changed default — that is a **divergence**, reported the way
B-058 reports declared-against-verified. Applying it is an act: it happens
because somebody asked, and it is recorded, and after it the conditions have
changed and measurements taken before and after are not comparable — which MCF
already knows how to say.

This is the branch §7.25 feared would let configuration rot, and the fear is
answered by the reporting: a configuration that is out of date is *visible*
rather than silently stale, and updating it is one command. What is refused is
only the silent part. §3.12 keeps its guarantee — a result depends on nothing
hidden — and §3.11 keeps its: nothing changes without deliberation.

**The corollary that matters for M5.** A benchmark run under a configuration
carries that configuration's provenance into its conditions. Yesterday's
benchmark and today's are comparable exactly when the configuration between
them did not change, and MCF can now answer that question rather than assume
it.

### What it costs

The framework is real work: trials, conditions, storage beside the model,
and a surface. The three configuring probes are each an experiment somebody
must design against §3.18's standard. The chat template is the first, because
it is the one whose absence is measurable today.

---

## PR1 — Customizable workloads

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
automatic conversion. That is PR1 plus a convenience, not a different feature,
and it should not be built until slots exist and someone finds authoring them
tedious.

### Open questions

- How large may a slot be before a lab measures something other than what it
  claims? §6.37 says the lab must state what its slot accepts; where exactly
  that boundary sits is unresolved.
- How few workload items make a reportable result? DEC-023's statistics, with a
  much smaller `n`, and the honest answer may be "twelve items is not a suite
  and MCF should say so".
- May a workload move between the user's own machines (PR4) without becoming a
  contribution? Probably yes, and it needs saying explicitly.

### Recommendation

**Accept, at M6, and build the slot into the lab framework rather than after
it.** The retrofit is the expensive order. Scoped this way it is a modest
feature that delivers the whole of the original ambition — measurement of the
work you actually do — without capture, without a content pipeline, and without
turning a designed instrument into a recording of uncontrolled work.

---

## PR2 — The repro bundle

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

With PR2, MCF can say: *this bundle reproduces this number, or tells you exactly
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
path holds the serialization. PR2 is mostly *assembly*, and its cost is dominated
by deciding the format (which §7.30 must decide for contributions anyway).

### Collisions

- **§7.33 is a hard dependency.** A bundle containing only summaries cannot be
  re-analysed, which defeats half its purpose. If DEC-033 chooses summaries, PR2
  degrades to a provenance receipt.
- **A24 and §3.20.** A bundle is a publication the moment it is sent to someone,
  so producing one is not gated but *sending* one is the user's act, and the
  bundle must show what it contains before it leaves — same surface as `mcf
  share --preview`.
- **A25.** A bundle from a private-suite measurement (PR1) would contain user
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

## PR3 — Pre-acquisition planning

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
131 GiB, you have 24." PR3 turns that refusal into a decision aid and moves it
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
memory. MCF already computes this at M1; PR3 computes it for every variant in the
repository without fetching any of them, from file sizes and metadata alone.

**2. Expectation, which is inference from local history and must be labelled as
such.** Once M5 has measured *some* configurations on this machine, MCF can
project a band for unmeasured ones from the relationship between quantization
size and throughput observed here. This is an **estimate** (A20) — clearly
marked, never comparable with a measurement, never promotable — and where MCF
has no local history it says so and offers fitment alone.

**3. A recommendation only if §IV can honestly make one.** Before any local
measurement exists, PR3 states fitment and stops. B34 forbids a foreign number
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
  local history, which would smuggle foreign numbers into a local decision. PR3
  uses this machine's history only.
- **§3.7.** Metadata used for fitment comes from the hub and is untrusted; a
  declared file size that is wrong produces a wrong plan, so the plan is
  re-checked against reality on acquisition and any divergence is a finding.

### Open questions

- How far can a size→throughput relationship be extrapolated on one machine
  before it becomes fiction? Probably not across architecture families, and
  possibly not across quantization schemes.
- Should PR3's prediction be scored against the eventual measurement and
  reported? Yes, almost certainly — a prediction nobody grades is a guess, and
  grading it is nearly free.

### Recommendation

**Accept, and place at M1** for the fitment half, which needs nothing but
metadata and is immediately useful. The expectation half arrives at M5 once
there is local history to project from, and should not be built earlier.

---

## PR4 — Your own second machine

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
— and PR4 is mostly a matter of *scope*, not new machinery. A record is exported
to a file, imported on the other machine, and the comparison runs locally
against both.

The critical constraint, and the reason this is narrow: **a personal transfer is
not a contribution.** It goes machine-to-machine at the user's direction, not to
a public corpus. It therefore does not need §7.27's de-identification (the user
already knows their own hardware), does not enter the crowd-sourced path, and
may include the private suites PR1 produces — which a contribution never could.

Comparison enforces isolation (A8) exactly as it does locally: two machines
differing in accelerator, driver, thermal environment and host memory differ in
four variables, so MCF reports the difference and refuses to attribute it to any
one of them unless the others were held still.

### What it costs

Small if scoped to the user's own machines, because it reuses §XIV's
serialization and §XV's identifiers. It grows expensive the moment it becomes
"compare with anyone", which is the thing it must not become.

### Collisions

- **§5, and this is the one to watch.** PR4 is legitimate because trust and
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

### The honest case against

This is the weakest proposal in this document, and the earlier recommendation
over-sold it. Three problems, stated plainly:

**It requires two machines.** Every other proposal here helps every user. This
one helps users who own and maintain MCF on more than one machine, which is a
minority of a minority.

**Most of its value is obtainable by looking.** Running the same command on two
machines and reading both outputs answers "which is faster" without any feature
at all. What PR4 adds is *rigour* — merging the records, enforcing A8's isolation
check, naming which of the four differing conditions might explain the gap, and
keeping the result in the record rather than in the operator's head. That is
worth something, but it is a smaller something than "you can now compare
machines", which is what the earlier framing implied.

**Its most interesting benefit is a side effect.** The real-hardware validation
datum for DEC-020 is genuinely valuable — but it is valuable *to the project*,
not to the user, and building a user-facing feature primarily to generate
validation data for ourselves is the wrong reason to spend the user's weight
(§3.13, B15).

### Outcome — dropped

**Superseded by §6.38 and D12.** The corpus answers the question PR4 was built
for, and answers it better: many users contributing configurations and
corrections across many hardware profiles builds a picture of what works where,
without requiring anyone to own two machines. A pairwise comparison between two
machines is a sample of two; the corpus is a sample of everyone, and §6.38's
prior-versus-claim distinction is what makes consulting it legitimate.

The DEC-020 argument survives the drop and moves with it: contributed
corrections are also real-hardware data about how configurations behave off the
machine that produced them, which is the validation datum PR4 was going to supply
by hand.

Recorded rather than deleted (C6), so a later "let people compare machines"
arrives as a decision with this reasoning attached.

### Superseded recommendation

**Was: defer, and consider dropping.** It is a thin convenience layer over
machinery M9 builds anyway, so nothing is lost by waiting — and if it is never
built, no intent goes unserved. The condition that would change this: if the
project reaches DEC-020 and cannot answer *how much reality validates the lab*
without it, PR4 stops being a convenience and becomes the cheapest instrument
available for the hardest open question attached to §VIII.

Recorded rather than deleted (C6), so that a later "we should let people compare
machines" arrives as a decision with this reasoning attached rather than as a
new idea.

---

## PR5 — Contention diagnosis

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
machine is busy." PR5 upgrades that from a refusal to a diagnosis: *your
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
that argument; PR5 samples when something happened or somebody asked. If it ever
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
  rather than produce an invalid result, which makes PR5 a precondition for D8's
  quiet-machine check rather than a diagnostic afterthought.

### Recommendation

**Accept, small, at M5**, where invalidation-by-contention first occurs. The
last open question above may promote it: if D8's pre-flight needs to verify the
machine is quiet, PR5 is not optional at all.

---

## PR6 — The stop control

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

## PR7 — Longitudinal regression detection

**One line.** MCF tells you when something on *your* machine got worse, and what
changed at the same time.

### Where this comes from

§6.7 separates tests from benchmarks and then names a third thing in passing: *"a
regression detector built on benchmark results is a third thing again, and its
thresholds are statistical judgments, not assertions."* It is the only artifact
the intent document names and no milestone builds.

The gap matters because the data is already being kept. MCF accumulates
conditioned measurements over months (D6), records MCF's own version and
configuration with each (§3.4), and knows the machine's profile at the time
(§3.8). Everything needed to say *"your throughput dropped 14 % on the 3rd, and
the only thing that changed was your driver"* is in the record, unused.

For a solo operator this may be the single most valuable sentence MCF can
produce, and no leaderboard, corpus or benchmark suite can produce it — it is a
statement about one machine over time, which is the one thing MCF is uniquely
positioned to know.

### How it works

**1. Compare like with like.** A regression is only meaningful between results
sharing an identity (§7.34) — same model, quantization, context, runtime,
workload. Everything else is a different measurement, not a regression.

**2. Detect against noise, not against a threshold.** A drop is a candidate only
when it exceeds the historical spread of that same measurement. §3.27's paired
data helps here: a run whose *internal* ratios held while its absolutes fell
points at the machine rather than the configuration.

**3. Correlate with what changed.** The record already holds MCF's version, the
driver, the runtime build, the thermal baseline and the hardware profile. A
detected drop is reported alongside the diff of everything that changed since
the last comparable result — which is a *hypothesis*, labelled as one, never a
cause.

**4. Report, never gate** (A18). A benchmark has no pass condition, so neither
does this. It is a finding, surfaced when the user looks, and it never fails a
build or blocks a run.

### What it costs

Small. No new measurement, no new instrumentation, no runtime cost — it is a
query over data already stored, run when the user asks or when a new result
lands. Its real cost is statistical judgment: a detector that cries wolf is
worse than none, because it trains the user to ignore it.

### Collisions

- **A18.** Benchmarks are never a gate, and this is built on benchmarks. It
  reports; it does not fail anything.
- **§3.4, A8.** A "regression" across a changed condition is not a regression —
  it is a confounded comparison. §7.45's machine-change question is a hard
  dependency: without knowing what changed, the detector attributes a driver's
  effect to a model.
- **B4, D5.** Detection runs on demand or on new data, never on a timer. It is
  a query, not a monitor.
- **§3.24.** A drop with two prior data points is not a trend, and the report
  says so.

### Open questions

- What statistic distinguishes a real regression from noise, given that §7.7's
  acceptance criteria do not exist yet? Probably answered with DEC-007.
- Does MCF ever surface a regression unprompted, or only when asked? §7.31 asks
  the same question about contribution prompts, and the answer should probably
  match.
- Should improvements be reported too? Almost certainly — a driver update that
  made things 9 % faster is the same finding with the sign reversed, and
  reporting only bad news is its own bias.

### Recommendation

**Accept, at M8**, once there is enough history for a comparison to mean
anything. Blocked in substance on §7.34 (what makes two results comparable),
§7.45 (what changed underneath) and DEC-007 (what counts as a real difference) —
all three of which are already open for other reasons, which is a good sign that
this is a natural consequence of the design rather than an addition to it.

---

## PR8 — The stand-in engine

**One line.** A deliberately slow, obviously correct implementation MCF writes
itself, so that a model no vendored engine will run still runs — and so that the
vendored engine has something to be checked against.

### The claim it enables

Two, and the second is the one that justifies the first.

**Coverage becomes true rather than aspirational.** §III commits MCF to *any*
model and B7 is careful about what that means: a defined actionable outcome for
every reference, not success for every one. Today the outcome for an
architecture or a quantization the vendored engine does not implement is
`engine.unavailable` — honest, and a dead end. With a stand-in it becomes *runs,
slowly, marked*, which is §3.2's degrade-and-say-so applied one level down from
where it was written. The same holds for hardware: on a machine whose
accelerator has no vendorable backend (D23, intent v23), the stand-in runs on the
processor. §3.2 already names that exact case.

**A19 becomes satisfiable.** *Nobody should believe published numbers from
software that cannot demonstrate it computes what it claims.* Against what,
for inference? A second implementation, written to be read rather than to be
fast, is the only answer available: where both run, agreement is evidence about
both, and disagreement is a finding about one of them. That is not a nicety —
it is the difference between MCF trusting its engine and MCF having checked it.

### How it works

A dense, single-threaded, unoptimized implementation in Rust:

- the GGUF and safetensors readers MCF needs anyway for provenance (§3.6) and
  for PR3's pre-acquisition fitment;
- dequantization for the common quantization schemes — each is a short, heavily
  documented piece of bit unpacking;
- the ordinary transformer operations, written straightforwardly: RMSNorm, RoPE,
  attention, the feed-forward block, matmul;
- sampling, which is small.

What it deliberately does **not** have is the entire source of difficulty: no
SIMD, no fusion, no threading, no accelerator path, no memory-layout tricks.
Those are what make an engine fast and what make one a permanent maintenance
obligation as hardware changes. A stand-in has neither.

**The constraint that makes this safe, and it is structural.** *A stand-in
cannot produce a timing.* Not by policy — by type, in the way `Duration<Simulated>`
cannot become a performance number (A11, B37). A throughput figure from a naive
kernel is a measurement of the naive kernel; it says nothing about the model and
nothing about the machine, and publishing one would be worse than publishing
nothing. So the stand-in serves **behaviour-class** laboratories only (B31): did
the tool call parse, did the loop terminate, did the structured output conform,
did the model recover from an error. Those outcomes do not depend on how fast
the arithmetic was.

That constraint also removes the reason the project would drift. There is no
point optimizing something that can never report a speed, so the slippery slope
from *stand-in* to *our own engine* has no first step.

**Comparability takes care of itself.** Intent v16 makes the engine build part of
a configuration's identity (D17), so a stand-in result and a vendored-engine
result are already two configurations rather than two readings of one. Nothing
new is needed to keep them apart; A8 and D17 do it.

**Three states, again.** An artifact runs on the vendored engine, runs on the
stand-in and is marked, or does not run and MCF says why. That is the same shape
D25 gives a device and D29 gives a platform, which is some evidence it is the
right shape.

### What it costs

Real, and bounded in a way a fast engine is not. Perhaps two thousand lines for
the operations, plus the loaders MCF needs regardless, plus a dequantization
routine per scheme. The maintenance is proportional to *architectures*, not to
hardware — a new accelerator costs nothing, which is precisely the treadmill an
own engine would put MCF on.

The honest cost is speed. A naive processor implementation may be two orders of
magnitude slower than a tuned one, which makes a large model on the stand-in a
matter of hours rather than seconds. B49 already anticipates this shape and
makes it tolerable: a behaviour-class run's deadline is a **token budget rather
than a wall clock**, because a task that failed for want of time is a
measurement of the machine. A slow stand-in makes a run long; it does not make
it wrong.

### Collisions

- **B23 — the instrument grows for validity, never for capability.** This is the
  rule that could refuse the proposal, so it is worth answering directly. A
  stand-in makes MCF *able to run more models*, which sounds like capability. But
  what it actually buys is a second implementation to check the first against, and
  a coverage claim that is true rather than nearly true — both of which are
  validity. The timing prohibition is what keeps the two apart: with it, the
  stand-in cannot make MCF's measurements faster, prettier or more numerous, only
  more checkable.
- **§7.4 and the own-engine question.** This is *not* that. §7.4's reading —
  MCF's performance mandate applies to its own overhead, not to the inference
  kernels — is untouched, because a stand-in makes no performance claim at all.
- **A5, A8, D17.** Every stand-in result is degraded and marked, and is a
  different configuration from a vendored-engine result rather than a comparable
  one.
- **B64 — everything MCF runs on, MCF ships.** Satisfied more completely than
  before: the stand-in is MCF's own code, pinned by construction.
- **§XVI.** It reduces the number of cases where a missing component becomes an
  errand, which is what §XVI is for.

### Open questions

- **Which quantization schemes.** Every one MCF's reference model and its
  neighbours use, and then by evidence. A scheme with no dequantization routine
  is a stated absence, not a silent one.
- **Does the stand-in ever verify the vendored engine automatically?** Where both
  can run an artifact, comparing their logits on a fixed input is nearly free and
  is exactly what A19 wants. It probably becomes a laboratory of its own.
- **What tolerance counts as agreement.** Floating-point reduction order differs
  between any two implementations, so this is a real question and it is D19's
  shape: identical inputs do not guarantee identical outputs.

### Recommendation

**Accepted**, with the timing prohibition as a condition of acceptance rather
than a note on it. Registered as B-360 (the stand-in), B-361 (the type-level
prohibition) and B-362 (the cross-check laboratory). The prohibition lands
first — before the thing it constrains exists, for the same reason B-220's
restoration ledger was built before anything was allowed to change the
environment.

---

---

## PR9 — What serving looks like

**One line.** MCF's own line-delimited protocol over the socket it already has,
where every answer carries the conditions that produced it — because the
industry-standard shape has nowhere to put them.

**Why this is a proposal rather than a decision.** DEC-001 is M2's gate and it
asks three things at once: the API surface, the supervision contract when a
runtime dies, and whether two models may be resident together. They are one
question wearing three hats — each answer constrains the other two — so they are
argued together here. Nothing below is built; B-032, B-033 and B-034 are where it
would be, and every one of them also needs an engine (B-320).

### The claim it enables

**That a served answer is as accountable as a measured one.** §3.4 makes a
number without its conditions meaningless, B-073 asks that no view render one
without them, and D19 makes the seed a condition. A serving surface is where
that discipline is most likely to be dropped, because the shape everybody
expects — an OpenAI-compatible `POST /v1/chat/completions` — has no field for
*this came from MCF's stand-in and is marked degraded*, no field for the engine
build that produced it, and no field for the seed. A surface that cannot carry
the mark is a surface that strips it, and B65 and A5 both forbid exactly that.

### How it works

**The protocol is the one `mcf status` already speaks**: one JSON object a line,
over the Unix domain socket in `$XDG_RUNTIME_DIR` (B-036), request in, answers
out. It is extended rather than replaced, so `mcf run` becomes a client of the
daemon and the command an operator types is the same one a script sends.

A generation is **one request and many answer lines**, which is what makes a
first-token latency a thing that can be observed at all (B-035, D24):

```
→ {"v":1,"ask":"generate","model":"owner/model:model.gguf","prompt":"...",
   "limit":128,"seed":7,"sampler":"greedy"}
← {"v":1,"token":"Hel","at":0}
← {"v":1,"token":"lo","at":1}
← {"v":1,"done":{"tokens":2,"stopped":"stop_token",
   "conditions":{"engine":"mcf-standin 0.1.0-m0","seed":7,"sampler":"greedy",
   "model_sha256":"…","context":4096},
   "degraded":"engine.unavailable — no vendored engine runs this artifact"}}
```

Three properties of that shape are load-bearing:

- **The conditions are in the terminating line, not in a header nobody reads.**
  A client that ignores them still has them in the record, because the daemon
  writes the same object it sent (D20).
- **The mark travels with the answer.** `degraded` is absent when nothing is
  degraded and present when something is, which is `Degraded<T>` on the wire
  (B-008): there is no rendering of the answer that does not carry it.
- **A request names its seed and sampler or takes the daemon's stated defaults**,
  which `mcf explain` already prints with their source (§3.15, B-038). Nothing is
  chosen invisibly.

**An OpenAI-compatible surface is a separate, later question, and a lossy one.**
It would make every existing client work, which is a real benefit and the reason
it will keep being asked for. What it cannot do is carry the three fields above.
The honest form is an explicit adapter that **refuses to serve a degraded
result** rather than serving it stripped — which is a decision to take when
somebody has a client to point at it, not now.

### The supervision contract

**A runtime that dies mid-token is a partial answer, a classified failure, and a
daemon that is still up.** In the exchange above, the tokens already sent were
sent: A4 makes nine of ten completions nine data points, and a generation that
died after forty tokens produced forty tokens. So the terminating line becomes:

```
← {"v":1,"done":{"tokens":40,"stopped":"engine_died",
   "failure":{"category":"engine.crashed","attribution":"engine",
   "disposition":"partial","detail":"…","context":{…}},"conditions":{…}}}
```

- **Nothing is retried.** A retried generation is a *different* generation —
  different timing, possibly different tokens — and B2's rule about a retried
  write applies unchanged: silently retrying would put an event in the record
  that did not happen the way it is written.
- **The daemon does not die with its child** (§3.1, §7.1). The engine is a
  supervised subprocess (D32, B-032); the manager holds no state the child's
  death corrupts, which is the same property B-030 already asserts against
  `SIGKILL`.
- **The failure is recorded, not just returned.** A client that hangs up before
  the terminating line still leaves the account behind (A26).

### Simultaneous residency

**One model resident at a time, and switching is recorded.**

The argument is §3.4 rather than memory. Two models resident makes every latency
figure depend on what *else* was loaded, and that is a condition MCF would then
have to capture, state and compare across — for a capability nobody has asked
for yet (§3.13's refusal of generality). One at a time keeps the condition set
small and honest, and a request naming a model that is not resident is answered
after a load whose cost is *stated in the answer's conditions* rather than
hidden in its latency.

What this does **not** settle is DEC-018 — whether the one resident model stays
resident when nobody is looking — which is a different question about idle cost
(§3.13, B-031) and stays open.

### What it costs

- **A protocol MCF owns is a protocol MCF must keep** (§7.30, C5). The version
  field is there from the first line for that reason, and the surface is small:
  four requests today (`status`, `holding`, `stop`, `generate`).
- **No existing client works with it** until somebody writes an adapter. That is
  the real price, and it is paid in the currency §II cares least about.
- **Streaming makes the daemon's one-connection-at-a-time shape visible**: a long
  generation holds the socket. DEC-012 (several clients at once) is where that is
  answered, and until it is, MCF's honest position is one client at a time with
  the bound stated (B7).

### Recommendation

**Accept.** It answers DEC-001's three questions in a way that keeps A5, A22 and
§3.4 intact, it extends a protocol that exists rather than inventing one, and
every part of it is testable without an engine except the parts that are about
an engine. It should not be *built* before there is an engine to supervise: a
supervision contract with nothing to supervise is a claim, and A19 is against
claims.

**Accepted by the operator.** DEC-001 is decided and this is its substance. The
one thing that changed under it since it was written: D38 makes MCF's own engine
the thing being served, so *the engine to supervise* arrives with B-360's
coverage rather than with a vendored one — and the degradation mark the protocol
carries in its terminating line stops being a hypothetical, because every answer
from MCF's own engine has one.


---

## PR11 — Prompt analysis: what the model actually received

**One line.** MCF can already tell how a model wants to be *addressed*; this
proposes the other half — showing a person how a model *receives* the text they
wrote, so that writing a chat template or an agent guidance document becomes an
act of observation rather than an act of guessing.

**Why this is a proposal.** The scenario it serves is concrete and outside
anything the roadmap owns: *how do I craft an accurate chat prompt template
specifically for a given model?* — for OpenCode, or any agentic harness that
ships a guidance document. That is not a benchmarking question and not a
configuration question. It is a question about the boundary between a person's
text and a model's input, and MCF happens to hold every piece of machinery
needed to answer it. Whether that makes it MCF's job is the decision.

It also runs directly at the wall this project keeps hitting. F25, F37 and F38
each established the same thing from a different side: **output quality is no
evidence of implementation correctness in either direction.** A person tuning a
system prompt today does exactly what those findings forbid — edit the text,
read the reply, judge. This proposal is that loop replaced with observation.

### The claim it enables

**That a person can see what the model got, rather than infer it from what the
model said.** Today nobody can. A prompt is written in an editor, passes through
a template, a tokenizer, and a set of control tokens, and arrives as a sequence
of integers that nobody looks at. Every transformation in that chain is a place
where the text a person wrote stops being the text the model reads — and F37 is
the proof that this is not hypothetical: `<|im_start|>` written into a prompt
reached the model as **eight ordinary tokens**, and the resulting table was the
cleanest and most decisive in this repository. MCF's own probe was fooled by it.
A user has strictly less visibility than that probe had.

### What it shows, concretely

Four things, in increasing order of how much they cost to be honest about.

**1 · Segmentation — the prompt as the model receives it.** The text broken
into the tokens it becomes, with the fragments visible. This is the observation
the idea came from: WordPiece exists to handle text a vocabulary does not
contain, and *where it has to* is exactly where a model's grip on the text is
weakest. A domain term that survives as one token and a domain term that
shatters into six subwords are not the same input, and nothing in a person's
editor distinguishes them. `[UNK]` is the extreme case and the easiest to
report; the common case is fragmentation, which is quieter and matters more.

**2 · Cost — what the prompt spends.** Token count against the model's usable
context (B-055). The same guidance document is a rounding error on one model's
window and a tenth of another's, and the number differs by vocabulary, not by
word count. This is arithmetic and needs no judgement.

**3 · Fidelity of the turn — does the template survive assembly?** Which of the
markers a person wrote are real control tokens for this model and which are
ordinary text. MCF already computes this: the marker-survival check exists
because F37 forced it, and B-374 made a turn assemblable from identifiers. This
is the piece that answers the stated scenario most directly, and it is nearly
free.

**4 · Behavioural comparison — does variant A differ from variant B?** Not *is
this prompt good*, but *do these two prompts produce different behaviour from
this model*, measured by something needing no judgement. MCF has three such
measures already: whether the turn ends at the model's own stop token, how long
it runs (F38), and the divergence between two token distributions, which the
oracle already computes for its own purposes.

### Where quantification is honest, and where it is not

This is the crux, and the proposal is worth less than nothing if it gets it
wrong.

**The request is to "quantify a model's evaluation of a prompt." The honest
form of that is a comparison, never a score.** There is no judgement-free
measure of whether a model *understood* a prompt. A19 forbids inventing a
number; A21 separates declared from verified; A7 keeps unknown unknown. A single
"prompt quality score" would violate all three at once — it would be a
fabricated aggregate over things that are not commensurable, and it would be
believed precisely because it is a number. The failure mode is already on
record three times in this repository: **the most decisive-looking result has
been the broken one every time.**

What *is* honest, and is most of what the scenario actually needs:

| Question | Judgement needed | Status |
|---|---|---|
| How does this text segment for this model? | none | the tokenizer already answers |
| How many tokens does it cost, of how many available? | none | arithmetic, plus B-055 |
| Did the markers I wrote survive as control tokens? | none | F37's check, already built |
| Does the assembled turn round-trip? | none | D46's rule, already held |
| Do variants A and B produce different behaviour? | none — it is a comparison | F38's observations |
| Which variant is *better*? | **judgement** | not MCF's to say |

The last row is the boundary, and it is the same boundary D42 draws for probes:
MCF writes the verified half and never the default. Here MCF shows what the
model received and how its behaviour differed; **which prompt to ship is the
person's decision**, and it stays theirs.

### What it costs

Small, relative to what it uses. The tokenizer handles all three schemes
already — unigram, BPE, WordPiece — with the normalizers behind them. The
marker check, the addressing assembly, the turn-length observation and the
distribution comparison all exist and were built for other reasons. What is new
is a presentation surface and a comparison harness. It is not a new subsystem;
it is a view onto subsystems that are already load-bearing.

The real cost is scope discipline. A prompt analyser is the kind of tool that
attracts a score, a grade, a recommendation and a linter, and each of those
would be a claim MCF cannot support.

### What it collides with

**§3.7 — untrusted input.** A prompt under analysis is data, always. It is
tokenized, counted and displayed; it is never executed, and nothing in it may
select a code path. This is the same rule that makes the analyser *useful*: the
thing it exists to show is that user text does not become control tokens.

**F26 — MCF must not parse control tokens out of prompt text.** The analyser
must never lift this rule to make a template "work". Its job is the opposite:
to make visible that a marker a person typed became eight ordinary tokens, so
they stop typing it and address the model properly instead.

**D42 — a probe writes the verified half of a capability and never a default.**
The behavioural comparison is a probe by every one of D42's properties and
should be one, with the same three outcomes. *Inconclusive* will be common here
and must stay first-class: two prompt variants that produce no measurable
difference is a real and useful answer, and is not the same as them being
equivalent.

**B-376 bounds it.** Behavioural comparison needs many generations, and MCF's
own engine is currently the only one a probe can drive. The static half —
segmentation, cost, marker fidelity — needs no engine at all and is unaffected.

### What is unanswered

- **Is this MCF's job?** MCF measures models. This measures the seam between a
  person and a model. The case for yes is that the seam is where §3.8's
  misconfiguration lives, and §X already calls a misconfigured model a
  measurement error. The case for no is that it serves authors rather than
  measurement, and §5 refuses features that only broaden appeal.
- **Where it lands.** The static half could ship beside M3's probes, since it is
  the same machinery pointed at the user's text. The comparison half wants
  M4's window — a segmentation is something you *look at*, and a terminal is a
  poor place to look at one. Splitting it across two milestones is likely right
  and should be decided rather than drifted into.
- **How many models at once.** The stated scenario is one guidance document
  across many models, which argues for comparison across vocabularies as a
  first-class view rather than a later addition. It also multiplies the cost of
  every behavioural measure by the number of models.
- **Whether fragmentation is worth reporting at all**, or only `[UNK]`. That
  fragmentation matters is an intuition, not a finding. It is testable — the
  same prompt with a term that survives whole against one that shatters, the
  behavioural comparison run between them — and it should be tested before it
  is designed around.

### Recommendation

**Accept the static half, defer the behavioural half.** Segmentation, cost and
marker fidelity are cheap, need no engine, need no judgement, and answer most of
the stated scenario on their own — a person who can see that their markers are
not surviving has learned the thing that was actually wrong. The behavioural
comparison is the part that needs B-376, needs care to avoid becoming a score,
and needs the window to be worth looking at. It should be argued separately once
the static half has shown what people do with it.

**Accepted in part, 2026-08-27.** The static half is taken and enters the
backlog as B-381 (segmentation), B-382 (cost against the usable context) and
B-383 (marker fidelity). The behavioural half is **deferred, not refused**: it
needs an engine that can be driven cheaply enough to run variants, it wants a
view that can show a shape rather than a column, and it is the half that could
drift into scoring a prompt — which has no judgement-free measure, and which
the answer on soft qualities has since ruled out for the same reason.


---

## PR12 — Lifting the stand-in's ceiling, and why the cheap version is worse

**One line.** MCF's own engine cannot read the model §XII names, and the
obvious fix — dequantize per use instead of on load — buys the memory back by
spending time the engine does not have; the two halves are one problem and
should be decided together.

**Why this is a proposal.** B-372's open half reads as an increment: hold the
weights quantized, dequantize as you go, ceiling lifted. It is not. It changes
a design choice `llama.rs` documents as deliberate — *dequantizing on load
rather than per token is the one memory-for-time trade here, and it is made for
legibility* — and it changes it in the direction that makes the resulting
capability unusable. B15 admits weight only against a stated cost, and this is
the cost stated.

### What is actually blocked

**The reference model.** 27,320,697,856 parameters dequantized to `f32` is
109.3 GB, against a machine with 91 GiB. `mcf run` refuses up front, naming
both numbers, and the refusal is honest and fast (0.16 s, from a bounded prefix
of the directory). But the artifact §XII names is the one MCF's own engine
cannot touch — and F49's cross-check, which compares MCF against the engine it
provisioned, is therefore unavailable at exactly the size where somebody would
most want it.

### The arithmetic that decides it

MCF's own engine, measured on this machine: **Qwen3-0.6B at Q4_K_M, load plus
nine forward passes, 11.0 s** — call it 0.9 s a pass. The reference model is
**forty-five times larger**, and a forward pass is roughly linear in
parameters:

| | Qwen3-0.6B | the reference model |
|---|---|---|
| forward pass | ~0.9 s | ~40 s |
| one token | ~0.9 s | ~40 s |
| F49's 120-position cross-check | 30 s | **~80 minutes** |

**Dequantize-per-use makes the second column worse, not better.** The current
design pays the dequantization once, at load. Per-use pays it *on every token*,
for every weight — which is the memory-for-time trade run in the direction the
engine can least afford. It would lift the ceiling and produce a capability
nobody can run.

That is the finding: **the memory ceiling and the speed are not two items, they
are one.** B-372 and B-366 (work split across processors without changing the
answer) are the same question asked twice.

### What would actually work

**Quantized arithmetic** — multiply the stored weights against quantized
activations without materializing floats, which is what the provisioned engine
does. It is the only option that improves both halves: the weights stay at
16.5 GB and the multiply gets *cheaper* rather than dearer.

It also collides with something MCF has measured. F29 and F33 record that MCF
multiplies floats where the reference multiplies in quantized arithmetic, and
that this is **why their numbers differ** — the widest arithmetic gap observed
was on a Q2_K file, for exactly this reason. Adopting quantized arithmetic
would narrow the gap MCF's own oracle was calibrated against. That is not an
objection; it is a reason the change must be measured before it ships, and
F49's cross-check is now the instrument for measuring it.

### What it costs

Large, and larger than it looks. `ops::matmul_vec` is a single function and the
call sites are few, which makes the *shape* of the change contained — but the
numerics are the engine, and every threshold in this repository (F27's 0.40
margin, F41's rank of 8, F29's 0.999596 cosine floor) was measured against the
current arithmetic. Changing it invalidates the calibration of every
comparison MCF makes about itself, and re-establishing them is the real cost.

There is also a boundary question. D31 and B65 make the stand-in *slow by
design* and forbid a speed from it, on the reasoning that a legible engine is
worth more than a fast one. Quantized arithmetic is the thing that makes real
engines fast. A stand-in that adopts it is a stand-in drifting toward being an
engine, and §5 refuses features whose justification is a comparison table.

### What is unanswered

- **Is the reference model the right target at all?** D39 already gives MCF a
  provisioned engine that runs it, and F36 measured it doing so. What MCF's own
  engine buys is the *cross-check*, and a cross-check that takes eighty minutes
  may be worth having as a scheduled tier and worth nothing as a command.
- **Would a smaller ceiling do?** Nothing between 0.6B and 27B has been tried.
  A 7B model at Q4 is 4 GB quantized and 28 GB dequantized — under the
  ceiling, and forty seconds a token is a tenth of that. The interesting
  question may be *how large a model can MCF's own engine usefully read*, which
  is a measurement nobody has taken.
- **Does B-366 change the arithmetic?** Threads across sixteen cores could take
  40 s a token toward 3 s, which changes whether any of this is worth doing —
  and B-366 is open and independently justified.

### Recommendation

**Do not lift the ceiling by dequantizing per use.** It is the cheap version
and it makes the capability worse. **Take B-366 first** — it is independently
justified, it does not touch the numerics, and it moves the number that decides
whether lifting the ceiling is worth anything. **Then measure how large a model
MCF's own engine can usefully read**, which is a fact nobody has and which
would tell us whether 27B is the target or whether it never was.

**Accepted, 2026-08-27.** The ceiling is not lifted. B-366 goes first — it
needs no decision, does not touch the numerics, and moves the number that
decides whether the rest is worth attempting — and **B-384** then measures how
large a model MCF's own engine can usefully read, which nobody knows and which
is the fact that says whether twenty-seven billion was ever the target.

## Changelog

### Version 12 — the stand-in's ceiling

PR12. B-372's open half reads as an increment and is not: dequantizing per use
lifts the memory ceiling by spending time the engine does not have. Measured
here — 0.9 s a forward pass on a 0.6B model, so roughly forty on the reference
model, so eighty minutes for the cross-check that is the point of lifting the
ceiling at all. The cheap fix makes that worse rather than better.

The memory ceiling and the speed are one problem. The recommendation is to take
B-366 first, because it moves the number that decides whether the rest is worth
doing, and then to measure how large a model MCF's own engine can usefully
read — which nobody knows.

### Version 11 — prompt analysis

PR11. Raised from the observation that WordPiece exists to handle text a
vocabulary does not contain, and that *where it has to* is where a model's grip
on the text is weakest — which nobody can currently see. The scenario is
crafting a chat template or an agent guidance document for a specific model,
and the argument it has to survive is F25, F37 and F38's shared lesson: output
quality is no evidence of correctness in either direction, and tuning a prompt
by reading replies is exactly the loop those findings forbid.

Its hardest question is the one it was asked — *quantify a model's evaluation
of a prompt*. The honest form of that is a comparison and never a score; a
single "prompt quality score" would be A19's fabricated number, believed
because it is a number. The proposal draws the line explicitly and puts *which
prompt is better* on the person's side of it, which is where D42 already puts
the equivalent decision for probes.

### Version 10 — PR10 accepted

DEC-024 and DEC-025 decided together, because they are one question in two
halves. A probe is an experiment whose result is a measurement with conditions
and never a default; MCF never reconfigures under a user, it reports that its
answer would differ. The open-ended list of §7.24 is bounded by §3.8's reason:
a probe earns its place when a wrong answer to it would corrupt a measurement,
which sorts the list into configuring probes (M3) and characterizing ones (the
framework in M3, the probes beside M6).

### Version 9 — PR9 accepted

The operator accepted it as proposed. DEC-001 is decided; B-032, B-033 and
B-034 build it when there is an engine to serve with, which D38 now says is
MCF's own.

### Version 8 — what serving looks like

PR9 added, recommended for acceptance. DEC-001 is M2's gate and asks three
things at once — the API surface, the supervision contract when a runtime dies,
and simultaneous residency — which are one question wearing three hats, so they
are argued together.

The argument that decides the surface is not ergonomics. The shape everybody
expects has nowhere to put the conditions that produced an answer: no field for
the engine build, none for the seed, and none for *this came from a stand-in and
is marked degraded*. A surface that cannot carry the mark is a surface that
strips it, which A5 and B65 both forbid — so MCF's own line-delimited protocol
over the socket it already has, with the conditions in the terminating line, and
an OpenAI-compatible adapter left as a later and explicitly lossy decision.

The other two follow from rules already written: a runtime that dies mid-token
is a partial answer with a classified failure and a daemon still standing (A4,
§3.1), never a silent retry (B2); and one model resident at a time, because two
makes every latency figure depend on what else was loaded (§3.4).

It should not be built before an engine exists to supervise: a supervision
contract with nothing to supervise is a claim, and A19 is against claims.

### Version 7 — the stand-in engine

PR8 added and accepted. It comes from a question the author asked while DEC-004
was open: whether MCF could write its own stand-ins for the cases a vendored
engine does not cover, so that every model at least runs.

The argument that decides it is not the coverage one, which sounds like
capability and would run into B23. It is that a second implementation is the
only thing an inference engine can be *checked against*, and A19 forbids
believing numbers from software that cannot demonstrate it computes what it
claims. Coverage is what the second implementation also happens to buy.

The condition of acceptance is a prohibition: a stand-in cannot produce a
timing, by type rather than by policy. That keeps B23 satisfied — the stand-in
cannot make MCF's measurements faster or more numerous, only more checkable —
and removes the reason the work would drift into writing an engine, since there
is no point optimizing something that can never report a speed.

### Version 6 — proposals become `PR`, so that `P` means one thing

The letter `P` named both the precedence rules and the proposals, and both
appeared bare in prose: `P2` meant *science outranks speed* in one document and
*the repro bundle* in another. A reader could not tell which was meant, and
neither could a check.

C5 forbids renumbering and permits deprecating in favour of a named successor,
so the proposals take `PR<n>` and `P<n>` here is deprecated with the mapping
stated. The precedence rules keep `P`: they live in [rules.md](rules.md)
alongside `A`, `B` and `C` and are part of that namespace, and the proposals
were the ones borrowing it.

Every citation elsewhere is updated, including those in changelog entries. A
changelog states what changed rather than the words used at the time, and
leaving a deprecated identifier in one would leave a reader with exactly the
ambiguity this change removes.

### Version 5 — PR7 added

Longitudinal regression detection: the "third thing" §6.7 names in passing and
no milestone builds, despite the data for it already being kept. It is the one
statement no corpus or leaderboard can make — what changed on *your* machine —
and it costs almost nothing, being a query over stored results rather than a new
measurement.

### Version 4 — PR4 dropped

Superseded by §6.38: the corpus answers what PR4 was for, without requiring
anyone to own two machines. A pairwise comparison is a sample of two; the corpus
is a sample of everyone.

### Version 3 — four accepted, one revised downward

PR1, PR2, PR3 and PR5 accepted and registered in [backlog.md](backlog.md). PR5's
placement changed on the way: D8 made laboratories exclusive, which means a lab
must begin on a quiet machine or exclusivity is a claim rather than a condition,
so the contention snapshot became a precondition for every lab rather than a
diagnostic convenience.

PR4's recommendation is revised downward to *defer, and consider dropping*. It
requires two machines, most of its value is obtainable by reading two outputs
side by side, and its most interesting benefit — a real-hardware datum for
DEC-020 — serves the project rather than the user, which is the wrong reason to
spend the user's weight. The reasoning is kept rather than the proposal deleted,
so that a later "let people compare machines" arrives as a decision.

### Version 2 — PR1 rescoped, PR6 accepted

PR1 is rewritten from the ground up. The original proposed automatic session
capture and replay, which inverted the instrument: §XIII's laboratories examine
one property under controlled conditions, and captured real work varies
everything at once. Rescoped to a **workload slot** — a lab ships a default and
accepts yours as data — it delivers the whole of the original ambition at a
fraction of the cost and with no content pipeline. The capture version is
recorded as refused, with its reasoning, rather than deleted.

PR6 accepted and registered as B-210.

### Version 1 — six features argued in full

Created so that a feature can be argued before it is committed to. B15 admits
weight only against a stated cost and B32 admits a laboratory only by naming the
claim it enables; a backlog row cannot hold either argument, so proposals were
previously either accepted silently or lost.

The six here came from an audit of what the intent document implies but nobody
owns. Two are notable: PR1 converts the project's central hedge — that the suite
is a proxy for the work — into a measurement of the actual work, and PR6 is
obligatory rather than optional now that §XVII permits MCF to take exclusive
control of hardware.
