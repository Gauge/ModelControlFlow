# ModelControlFlow — Document of Intent

**Status:** Living document. Revision 1.
**Nature:** This is a *spirit of the rules* document. It is not a requirements
specification, not an architecture document, and not a backlog. Nothing here is
directly implementable, and that is deliberate. Its job is to be the thing you
consult when a rule is ambiguous, when two rules conflict, or when no rule
exists yet — and to be the source from which real rules are written.

**How to use it:** When writing a specification, a test plan, a lint rule, or a
code review comment, cite the principle it serves. If you cannot find one, that
is a finding — record it in §7 (Voids) rather than inventing intent silently.

**Provenance note:** At the time of this revision the repository contains no
implementation. Section 6 (Conflicts) resolves tensions on principle alone.
Where a resolution says *provisional*, it means exactly that: the first real
implementation that touches the question gets to argue back, and this document
should be amended rather than quietly violated.

---

## 1. What This Project Is

ModelControlFlow (MCF) exists to make the open weights ecosystem *usable by one
person on one machine without that person becoming a full-time operator of it.*

Three things follow from that sentence and they are the whole project:

1. **Acquire** — obtain any model published on Hugging Face, with its
   provenance intact and its licensing legible.
2. **Deploy** — get that model actually running on the hardware in front of us,
   or explain precisely why it cannot.
3. **Judge** — measure what it costs and what it is worth here, on this
   hardware, for the work actually being done, and use those measurements to
   converge on the best available local configuration.

The third is the point. Acquisition and deployment are table stakes; plenty of
tools do them. The reason MCF is worth building is the closed loop: *measure,
compare, select, re-measure.* A version of MCF that downloads and runs models
but cannot tell you which one you should be running has missed its purpose.

### The animating frustration

Choosing a local model today is folklore. People pick based on leaderboard
positions measured on someone else's hardware with someone else's quantization
against benchmarks that do not resemble their work. MCF's contribution is to
replace folklore with measurement taken *here*.

### What this implies about the user

The user is treated as a competent operator with limited attention. MCF does
not hide the machinery; it makes the machinery cheap to ignore when things are
going well and fully legible the moment they are not. We never trade away the
user's ability to understand what happened in exchange for a smoother surface.

---

## 2. The Four Founding Intents

These are the originating statements, restated in the form they will be held to.
Everything downstream is an elaboration of these.

### I. Reliability — "it should never fail"

MCF should be the calm component in the system. It runs on hardware that
throttles, against a network that drops, over a model hub that changes under it,
launching runtimes that segfault. None of that is exceptional; all of it is
Tuesday. MCF's job is to absorb that and remain a coherent, queryable, restartable
system.

### II. Science — "the highest scientific standards"

Every claim MCF makes about a model is a measurement, and every measurement
carries the obligations of a measurement: a stated method, stated conditions,
stated uncertainty, and the ability for someone else to repeat it. Full test
suites are the floor, not the ceiling — they establish that the code does what we
think, which is a prerequisite for believing what it reports.

### III. Custody — "download and deploy any LLM from Hugging Face"

MCF is the single place where models enter this machine, live on it, and leave
it. It owns the lifecycle: fetch, verify, store, convert, serve, evict. "Any"
is an aspiration about *coverage of attempt*, not a promise of success — see §6.3.

### IV. Optimization — "build an optimal local model configuration for this hardware"

MCF profiles the machine, benchmarks candidates on it, and recommends — and
where possible constructs — the configuration that best serves the user's
declared objective. This is the intent that makes MCF a research instrument
rather than a package manager.

---

## 3. Principles

These are the load-bearing beliefs. When a decision is genuinely close, decide
in the direction of the principle.

### 3.1 Failure is a first-class, well-typed outcome

"Never fail" does not mean "never encounter errors." It means **MCF never
becomes unable to tell you what happened.** A model that will not load is not a
bug in MCF; MCF failing to record *why* it would not load is.

Concretely, the spirit is:

- Every failure is caught, classified, attributed to a subsystem, and persisted
  with enough context to reconstruct it without a rerun.
- No failure of a managed thing (a download, a conversion, a benchmark run, a
  served model) may take down the manager.
- Partial success is a real outcome and must be representable. Nine of ten
  benchmark tasks completing is a result with nine data points, not a failure.
- **The forbidden failure mode is the silent one.** A swallowed exception, a
  bare `except: pass`, a default substituted for a missing value without a
  record — these are worse than a crash, because a crash is honest. If we ever
  find ourselves choosing between crashing loudly and continuing quietly, we
  continue *loudly*.

### 3.2 Degrade, don't die — but say so at full volume

The preferred response to trouble is reduced capability with an explicit,
visible statement of what was lost. GPU unavailable → run on CPU and mark every
resulting measurement as CPU-derived. Hub unreachable → serve from local cache
and mark the catalog as potentially stale. Telemetry backend down → buffer
locally and report the gap.

The mark is not optional. A degraded result that is not labelled as degraded is
a corrupted result, and corrupted results are the one thing this project cannot
tolerate, because everything else it produces is built on them.

### 3.3 Observability is the product, not the plumbing

Logging and telemetry are not a debugging convenience bolted on afterward. They
are the substrate that makes claims III and IV possible. A benchmark number
without the conditions under which it was taken is not data. So:

- The system should be reconstructible from its own records. If we cannot answer
  "what was this machine doing when this number was produced," the number is
  weaker than it looks.
- Instrumentation is written with the code it observes, in the same change, by
  the same person. It is never a follow-up ticket.
- Records are structured and machine-readable first, human-readable second.
  Prose logs are a rendering of the record, never the record itself.
- Everything that varies is recorded: hardware state, thermal and power
  conditions, driver and runtime versions, quantization, context length, batch
  shape, MCF's own version and configuration.

### 3.4 A measurement is a claim, and claims carry their conditions

The unit of scientific output in MCF is not a number; it is a number bound to
the conditions that produced it. Any surface that displays a benchmark result
and drops its conditions is doing damage.

The spirit here:

- **Repeatability before precision.** A number we can reproduce to ±10% beats a
  number we can quote to three decimals once.
- **Uncertainty is mandatory.** Single-shot timings are anecdotes. Report spread
  and sample count, or don't report.
- **Isolate the variable.** Comparisons are only meaningful when one thing
  differs. If more than one thing differed, the honest output is "these are not
  comparable," and MCF should be capable of saying that.
- **Null and negative results are results.** "This model does not fit on this
  hardware" and "quantization gave no measurable speedup here" are valuable
  outputs and must be stored and surfaced as such, not discarded as failures.
- **We do not train on the test.** Any accuracy suite MCF uses for selection
  must be protected from the contamination and overfitting that would make its
  numbers meaningless — including MCF's own tendency to tune toward whatever it
  measures.

### 3.5 Tests establish the right to be believed

The test suite's purpose is not defect count. It is credibility: MCF publishes
numbers, and nobody should believe published numbers from software that cannot
demonstrate it computes what it claims.

The spirit:

- Anything that produces a number reported to the user is tested against a case
  where the correct number is known independently.
- Failure paths are tested as rigorously as success paths, because §3.1 makes
  them a feature. Untested error handling is decorative.
- Tests must not require a GPU, a network, or a 70B model to run. The expensive
  paths are tested through seams; the seams are part of the design, not an
  afterthought.
- We test the instrument, then trust the instrument. Simulated hardware,
  synthetic model artifacts, fake hubs, and replayed telemetry are all
  legitimate and expected. A lab has calibration rigs; so do we.
- A bug that escaped becomes a test before it becomes a fix.

### 3.6 Provenance is preserved, never inferred

Every artifact MCF holds knows where it came from: repository, revision,
checksum, license, retrieval time, and every transformation applied since. A
quantized derivative traces back to its source weights.

We do not guess. If provenance is unknown, that is recorded as *unknown* rather
than filled with a plausible value. Inferred metadata that looks like recorded
metadata is a form of the silent failure forbidden in §3.1.

### 3.7 The hub is untrusted input

Hugging Face is a public, mutable, user-populated surface. Model repositories
can carry executable code, deceptive metadata, malformed configs, enormous
files, and licenses forbidding the use being attempted. MCF treats every fetched
byte as hostile until validated.

This is not paranoia about the ecosystem; it is the ordinary posture toward any
remote input. The user's decision to run a given model is theirs to make, and
MCF's obligation is that the decision is *informed and explicit* — never
accidental, never implicit in a convenience default.

### 3.8 The machine is the experimental apparatus

MCF's measurements are only as good as its understanding of what it is
measuring on. Hardware is not a static fact to be read once at install; it is a
time-varying condition — thermal state, power profile, memory pressure,
contention from everything else the user is doing.

The spirit: MCF knows the difference between "this model is slow" and "this
machine was busy." When it cannot tell the difference, it says so rather than
attributing the result.

### 3.9 "Optimal" is meaningless without a stated objective

There is no universally best local model. There is a Pareto frontier across
quality, latency, throughput, memory, power, and disk, and a user-specific
preference that picks a point on it.

MCF's honest job is: **map the frontier faithfully, make the tradeoff legible,
and let the user's declared objective choose.** When MCF recommends, the
recommendation shows its reasoning and its runners-up. A recommendation the user
cannot interrogate is an oracle, and we are not building an oracle.

Corollary: MCF should be able to say "the differences here are within noise,
pick either." Refusing to manufacture a distinction is a feature.

### 3.10 The user's data and machine are theirs

Telemetry serves the user's own optimization loop first. Anything that leaves
the machine does so because the user chose it, knowing what it contains. Local
inference is often chosen precisely for privacy, and a tool that manages local
inference while leaking its contents has betrayed the reason it was installed.

### 3.11 Nothing is destroyed without deliberation

MCF manages objects that are expensive in bandwidth, time, and disk. Eviction,
overwrite, and cleanup are real operations with real cost and are treated as
such: previewed, logged, and where reasonable reversible. Reclaiming disk is
never worth surprising the user.

### 3.12 Reproducibility over convenience, when they conflict

Where a convenience would make a result harder to reproduce — an unpinned
version, an implicit default, an unrecorded environment variable, a silent
auto-upgrade — reproducibility wins. This is the principle most likely to be
eroded by a hundred small reasonable decisions, so it is stated explicitly to be
defended explicitly.

---

## 4. Standing Tensions We Accept

Not every tension resolves. Some are permanent conditions of the problem and
should be *managed* rather than solved. Naming them prevents relitigating them.

- **Rigor costs time.** Proper benchmarking is slow; users want answers now. We
  accept slowness for the numbers we publish, and we are permitted fast, clearly
  labelled *estimates* — provided an estimate can never be mistaken for a
  measurement.
- **Coverage costs sharpness.** Supporting every model format weakens the
  guarantees we can make about any one of them. We accept broad attempted
  coverage with honestly narrow guarantees.
- **Automation costs agency.** The more MCF decides, the less the user
  understands their own stack. We resolve toward explanation over autonomy.
- **Observation costs performance.** See §6.2 — managed, not eliminated.

---

## 5. Anti-Goals

Stating what MCF is *not* protects the intents above from dilution.

- **Not a training or fine-tuning platform.** MCF deploys and evaluates weights;
  it does not produce them. Quantization and conversion are in scope as
  deployment transformations, not as model development.
- **Not a leaderboard.** MCF measures *this* machine. It does not publish
  cross-machine rankings, and it should be actively suspicious of numbers that
  did not originate locally.
- **Not a general orchestrator.** Single machine, single operator. Fleet
  management is a possible future, not a shaping constraint on today's design.
- **Not a model-quality authority.** MCF reports what its suites measure under
  its conditions. It does not pronounce on whether a model is "good."
- **Not a wrapper that hides the runtime.** If the user needs to reach the
  underlying engine, MCF's abstraction has failed, and the escape hatch is part
  of the design.
- **Not opinionated about which model you should want.** It is opinionated about
  *knowing* which one you should want, given what you told it you value.

---

## 6. Conflicts Between Stated Intents

The four founding intents are not mutually consistent as written. Each conflict
below states the tension, the resolution, and the confidence in that resolution.

Because the repository holds no implementation at this revision, *no resolution
here is arbitrated by working code.* They are arbitrated by which reading keeps
the project coherent. Every one is provisional in the sense that a real
implementation may reveal a better answer — but they are binding until amended,
not optional.

### 6.1 "Never fail" vs. "highest scientific standards"

**Tension.** A system engineered to never stop is under constant pressure to
paper over anomalies. Science requires that anomalies be preserved and
confronted. Robustness and honesty pull in opposite directions at exactly the
moment something goes wrong.

**Resolution — honesty wins; "never fail" is redefined as "never lose
information."** MCF's reliability guarantee is about the *manager's* continuity,
never about the *managed thing's* apparent success. It is always correct for MCF
to report that a run failed, a number is untrustworthy, or a comparison is
invalid. It is never correct for MCF to substitute, smooth, retry-until-pretty,
or omit in order to preserve an appearance of success.

**Confidence: high.** This is the load-bearing resolution of the whole document.
Inverted, MCF becomes a machine that produces confident wrong numbers, which is
worse than no MCF at all. Every other resolution here defers to it.

### 6.2 "Deep full system telemetry" vs. "performance measurement"

**Tension.** Intent I demands pervasive instrumentation. Intent IV demands
accurate performance numbers. Instrumentation perturbs what it measures — the
observer effect is not a metaphor here; per-token hooks and continuous hardware
polling are measurable overhead.

**Resolution — the measurement path is separable from the operational path, and
the cost of observation is itself measured and reported.** Deep telemetry is the
default for operation. Benchmark execution runs under a declared, reduced,
recorded instrumentation profile, and that profile is part of the result's
conditions (§3.4). MCF must be able to quantify its own overhead; an
uncharacterized instrument is not a scientific one.

Note this does not weaken §3.3 — nothing about the *outcome* of a run goes
unrecorded. What is reduced during measurement is high-frequency sampling, not
record-keeping.

**Confidence: high on the principle, low on the mechanism.** How to make the
paths separable without two divergent code paths — the classic source of
"it works in benchmark mode" bugs — is a genuine design problem, not a solved one.

### 6.3 "Any model on Hugging Face" vs. "never fail"

**Tension.** The hub is unbounded, heterogeneous, and changes without notice.
Some models cannot run on given hardware, some are gated, some are broken, some
require trusting remote code. Universal coverage and universal success cannot
both hold.

**Resolution — "any" governs *attempt and diagnosis*, not *success*.** MCF
undertakes to (a) accept any hub reference without special-casing, (b) reach a
*defined, actionable outcome* for every one, and (c) never be damaged by a
hostile or malformed one. "This will not run here, because the weights need 48GB
and you have 24GB" is a complete success of Intent III. A hang, an
unclassified crash, or a corrupted local state is a failure of it.

The commitment is therefore: **no unhandled outcomes**, not **no unsuccessful
outcomes**.

**Confidence: high.** The alternative reading — that MCF promises every model
runs — is not achievable by any tool and would make the project dishonest by
construction.

### 6.4 "Never fail" vs. "untrusted hub input"

**Tension.** Many hub models require executing code from the repository to load.
Maximum compatibility argues for running it; reliability and safety argue for
never doing so. Refusing breaks Intent III; accepting silently violates §3.7.

**Resolution — untrusted execution is possible but never implicit.** The user
may choose it, per artifact, with the reason and the risk stated, and the choice
is recorded in provenance. MCF's own integrity does not depend on that code
behaving: it must be contained such that hostile or broken model code cannot
corrupt MCF's records or state.

**Confidence: high on policy, medium on degree.** How strong the containment must
be — process isolation, sandboxing, something heavier — is a real open question
whose answer depends on how much a compromise could cost. Recorded in §7.

### 6.5 "Optimal for the hardware" vs. "there is no single optimum"

**Tension.** Intent IV is phrased as if a single best configuration exists. It
does not; optimality is multi-objective and preference-dependent (§3.9).

**Resolution — MCF optimizes against a *declared* objective and refuses to
invent one.** It maps the frontier and characterizes tradeoffs regardless. When
asked to recommend without a stated objective, it uses an explicit, visible,
overridable default preference — never a hidden one. Every recommendation
carries its objective and its alternatives.

**Confidence: high on the reasoning, medium on the framing.** There may be a
defensible default objective (something like "best quality that meets an
interactive latency threshold within available memory") that serves most users
well. Choosing it is deferred, not denied — but a default must always be
*visible as* a default.

### 6.6 "Build an optimal model" vs. "not a training platform"

**Tension.** Intent IV says *build*. §5 says MCF does not produce weights.

**Resolution — "build" means *compose and configure*, not *train*.** Selecting a
base model, choosing quantization and format, sizing context, tuning runtime and
sampling parameters, and validating the resulting configuration is construction
enough to satisfy the intent. Weight-modifying procedures — fine-tuning,
distillation, merging, pruning — are out of scope at this revision.

**Confidence: medium.** This is the resolution most likely to be revisited. If
local fine-tuning becomes routine on consumer hardware, the anti-goal in §5 is
the thing that should give way, and it should give way by amendment here rather
than by drift.

### 6.7 "Full test suites" vs. "benchmarks are the product"

**Tension.** Both are called "tests" and they are opposite in kind. Unit tests
must be fast, deterministic, hermetic, and pass. Benchmarks are slow,
stochastic, hardware-bound, and *have no pass condition* — a benchmark that
"fails" has usually just told you something true.

**Resolution — these are two separate systems and are never conflated.** Tests
gate correctness of the code and must be deterministic and green. Benchmarks
produce measurements and are never a gate on correctness. A regression detector
built on benchmark results is a third thing again, and its thresholds are
statistical judgments, not assertions.

**Confidence: high.** Conflating them produces flaky suites that get ignored,
which destroys §3.5's credibility argument.

### 6.8 "Deep full system logging" vs. "the user's data is theirs"

**Tension.** Total observability points at recording prompts, completions, and
usage patterns. Privacy points away from it. Accuracy benchmarking needs
model outputs; those outputs may contain the user's most sensitive content.

**Resolution — record exhaustively about the *system*, minimally about the
*content*, and never conflate the two.** Metrics, timings, resource states,
configurations, and error conditions are recorded in full by default. Prompt and
completion content is a distinct category with its own explicit, separately
governed retention. Benchmark suites — whose content is fixture data, not user
data — may be recorded in full, and this distinction is exactly why suite data
and user traffic must be structurally separated rather than separated by
convention.

**Confidence: high on the split, low on the defaults.** What is retained by
default for user traffic, and for how long, is unresolved. See §7.

---

## 7. Voids — Where Intent Is Missing or Underdetermined

These are questions the stated intents do not answer and cannot be derived from.
Each will be answered by someone; this section exists so that it is answered
*deliberately, and recorded here*, rather than settled accidentally by whoever
writes the code first.

They are ordered roughly by how much downstream design they block.

### 7.1 What "deployed" actually means — **blocking**

Intent III says "deploy" without defining the end state. Is a deployed model a
persistent OpenAI-compatible endpoint? A process MCF supervises for its
lifetime? An ephemeral load for the duration of a benchmark? Something MCF hands
off to and forgets? The answer determines whether MCF is a long-lived daemon or
a command-line instrument, and almost every reliability question in §3.1 depends
on which. **Nothing about MCF's shape can be settled before this is.**

### 7.2 The objective function — **blocking §IV**

§6.5 defers the definition of "optimal." Someone must eventually state how
quality, latency, throughput, memory, power, and disk trade against one another,
and how a user expresses their own weighting. Without this, the optimization
intent cannot be implemented, only gestured at.

### 7.3 What "quality" is measured against — **blocking §IV**

Intent IV says "accuracy benchmarks" without saying accuracy at what. Public
academic suites are contaminated and often unrepresentative of real use. The
user's own work is representative but has no ground truth. A stronger model as
judge introduces its own biases and a dependency MCF may not want. How MCF
measures *quality* — as opposed to *speed*, which is comparatively easy — is the
single hardest unanswered question in the project, and the credibility of Intent
IV rests entirely on it.

### 7.4 Trust boundary and containment strength

§6.4 establishes that untrusted model code is opt-in and contained, but not how
strongly. What is the actual threat model — accidental breakage, or a
deliberately malicious model repository? The answer sets the engineering cost of
every acquisition path.

### 7.5 Retention, scope, and residency of telemetry

§6.8 splits system telemetry from content, but leaves open: how long is anything
kept, how much disk may telemetry consume, what happens when that budget is
exhausted (and how §3.1 forbids that from being a silent drop), whether anything
may ever leave the machine, and whether the user can inspect and purge what MCF
holds about them. §3.10 implies strong answers but does not supply them.

### 7.6 Reproducibility guarantee level

§3.12 asserts reproducibility wins, without saying reproducible *to what
tolerance* and *across what changes*. Same machine, same day? Same machine after
a driver update? A different machine of the same model? This determines how much
environment must be captured and pinned, which is a large cost either way.

### 7.7 Scientific acceptance criteria

Intent II invokes "the highest scientific standards" without operationalizing
it. What makes a benchmark result publishable by MCF's own standards — minimum
sample count, maximum variance, required warm-up, thermal steady-state
requirements, what invalidates a run retroactively? Until this exists, §3.4 is
aspiration rather than a rule anything can be checked against.

### 7.8 Hardware scope

Which accelerators, vendors, and runtimes are in scope, and what happens on
hardware MCF does not recognize? §3.2 says degrade and label — but the boundary
between "supported and characterized" and "will attempt, uncharacterized" is
undrawn, and Intent IV's meaning changes completely depending on where it falls.

### 7.9 Resource arbitration and concurrency

Models are enormous relative to available memory and disk. Who decides what is
resident? What happens when a benchmark and a served model both want the GPU, or
when a download would exhaust the disk mid-flight? §3.11 forbids surprising
destruction but does not say who arbitrates. This is where the ugliest
reliability bugs will live.

### 7.10 Failure taxonomy

§3.1 requires every failure to be classified, but the classification scheme does
not exist. It needs to be designed once, deliberately, because it will appear in
logs, telemetry, tests, the recommendation engine, and the user interface — and
retrofitting it later will be miserable.

### 7.11 Offline and degraded-network operation

Local inference is frequently chosen for disconnected environments. How much of
MCF works with no network at all? §3.2 suggests "most of it, loudly labelled,"
but this has never been stated as an intent and deserves to be.

### 7.12 The user surface

No stated intent describes how a human interacts with MCF — CLI, TUI, local web
interface, API, or several. §3.3's insistence that observability is the product
implies the interface's primary job is *making system state legible*, which is a
strong constraint on the answer, but the answer itself is absent.

### 7.13 State, versioning, and migration

MCF accumulates a catalog, a measurement history, and configuration that must
survive its own upgrades. Historical measurements taken under an older MCF are
scientifically delicate: are they still comparable after a change to the
measurement code? §3.4 implies they may not be, which implies MCF's own version
is part of every result's conditions and that some upgrades must invalidate
history. Nobody has said so.

### 7.14 Definition of done

Neither "reliable" nor "scientific" nor "optimal" has a stated threshold. What
state would let us say MCF works? Without an answer, §3.5's test suites have no
target to be complete against, and the project has no way to distinguish
progress from motion.

### 7.15 Success beyond the author

The stated intents are written for one operator on one machine. Whether MCF is
meant to be usable by others — and therefore whether documentation,
installation, and interface stability are goals or incidental — is unstated. It
changes what "highest standards" costs.

---

## 8. Amending This Document

- Intent changes when the *reasoning* changes, not when the code does. Code that
  diverges from this document is either a bug or an argument; both require the
  divergence to be raised here explicitly.
- Every conflict resolution in §6 is falsifiable by implementation experience.
  When implementation contradicts a resolution, amend the resolution and record
  what taught us better — do not leave the document standing while the code
  disagrees with it.
- When a void in §7 is filled, move it into §3 or §6 as a principle or a
  resolution, and note where the real specification now lives. §7 should shrink
  over time; if it does not, we are building on undeclared assumptions.
- New voids are added the moment they are noticed — including by a subagent, a
  code review, or a failed design discussion. An unrecorded void is how a
  project acquires intent nobody chose.
