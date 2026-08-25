# ModelControlFlow — Document of Intent

**Status:** Living document. Revision 3.
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

**Revision 3 note:** Intent VI was corrected: Ollama was offered as an *example
of a friction level*, not as an architectural or implementational model, and the
document had begun to treat it as the latter. §VI now says so explicitly, §5
gains an anti-goal against clone-thinking, and every comparison in the document
was rewritten to name the property rather than the product. Revision 3 also adds
§3.16 (prefer substrates a machine can hold to the principles) and §7.19 (the
implementation-substrate question, recorded with the criteria the intents impose
on it and a candidate answer, but deliberately not resolved here).

**Revision 2 note:** Three intents were added — the interface, the serving
model, and the performance mandate. Two of them answered questions this document
had recorded as blocking voids (what "deploy" means; how a human touches the
system). The third — *be the fastest, lightest tool possible* — is the most
disruptive statement made about this project so far. It does not merely add a
goal; it applies downward pressure to every other intent, because rigor,
observability, and universality all have weight. §6.9 through §6.14 exist to
keep that pressure from silently eroding the rest of the document.

---

## 1. What This Project Is

ModelControlFlow (MCF) exists to make the open weights ecosystem *usable by one
person on one machine without that person becoming a full-time operator of it.*

Four things follow from that sentence and they are the whole project:

1. **Acquire** — obtain any model published on Hugging Face, with its
   provenance intact and its licensing legible.
2. **Serve** — host that model as a persistent, dependable local endpoint, on
   the hardware in front of us, with as little ceremony and as little overhead
   as physically possible — or explain precisely why it cannot run.
3. **Judge** — measure what it costs and what it is worth here, on this
   hardware, for the work actually being done, and use those measurements to
   converge on the best available local configuration.
4. **Show** — make all of the above legible through an interface light enough
   to run anywhere, on anything.

The third is the point. Acquisition and serving are table stakes; plenty of
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

### What this implies about the machine

Every byte MCF occupies is a byte not available to a model, and every cycle it
spends is a cycle not spent on inference. MCF is a support structure around the
thing the user actually wants to run. A support structure that consumes what it
supports has failed regardless of how good its features are.

---

## 2. The Founding Intents

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

### V. Interface — "a minimalist, lightweight UI that runs on any system or device"

MCF is not usable if looking at it is a chore. There must be a way to see the
system's state and act on it that imposes no install burden, no framework, no
platform lock, and no meaningful resource cost — reachable from the machine
itself and from whatever device the user happens to be holding. Minimalism here
is not an aesthetic preference; it is the same discipline as §VII applied to the
surface.

### VI. Hosting — "make it easy to host the LLMs it downloads"

The distance between *having* a model and *using* a model should be one command
or one click: a persistent local service, a stable API, models addressed by name
rather than by path, and no requirement that the user understand runtimes,
formats, or flags in order to get a first token.

**On Ollama.** Ollama was cited as an *example of the ergonomic standard*, and
that is the entire extent of the reference. It names a level of friction — near
zero — that MCF must match or beat. It is explicitly **not** a model for MCF's
architecture, implementation, feature set, or engineering choices, and MCF has
no obligation to resemble it in any respect other than being that easy. Where
Ollama's design conflicts with §I, §II, or §VII, MCF diverges without
hesitation; where MCF can be lighter or more rigorous by doing something
entirely different, it should.

Read any comparison in this document accordingly: it is a bar to clear, never a
blueprint to follow.

### VII. Lightness — "the fastest, lightest tool it can possibly be"

Performance is not a late-stage concern to be addressed if there is time. It is
a standing constraint on every design decision, weighted equally with
correctness. Idle cost, memory footprint, startup latency, and the overhead MCF
adds between a request and a token are all budgeted quantities that must be
measured and defended, not incidental outcomes.

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
locally and report the gap. UI unreachable → the service keeps serving.

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

Under §VII this principle is now expensive, and §6.9 governs how it is paid for.
The resolution there reduces the *cost* of observation; it does not reduce the
*obligation* to observe.

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
- **Performance is a tested property.** Under §VII, footprint, startup time, and
  added latency are asserted against budgets by the suite, not eyeballed. An
  unbudgeted performance claim is as unscientific as an unbudgeted accuracy one.

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

**MCF is part of the apparatus it measures.** Its own resource consumption
contaminates its own benchmarks. This is the scientific argument for §VII, and
it is a stronger argument than the ergonomic one: a heavy instrument does not
merely annoy the user, it corrupts the readings.

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

Intent V pushes against this: an interface reachable from any device is an
interface reachable over a network, and a network-reachable control plane that
can pull arbitrary code from the internet is a serious object to leave
unattended. See §6.12.

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
defended explicitly. Under §VII, *performance* now supplies a second stream of
such reasonable decisions — caching, adaptive behaviour, skipped validation —
and §6.13 governs them.

### 3.13 Lightness is a budget, and budgets are defended continuously

Software does not become heavy through a decision; it becomes heavy through
three hundred defensible ones. "Fastest and lightest possible" is therefore
unachievable as an aspiration and achievable only as an accounting discipline.

The spirit:

- Weight is budgeted, measured, and regression-tested like any other property.
  Idle CPU, resident memory, disk footprint, cold-start time, and the latency
  MCF interposes between request and first token are the quantities that matter.
- **Do nothing when nothing is happening.** A management tool spends most of its
  life idle. Idle cost is the number that should embarrass us first, because it
  is pure waste — polling loops, background timers, and always-on watchers are
  the default suspects.
- The overhead of a feature is part of the feature's cost, and features are
  refused on that basis. Refusing a feature is a normal outcome, not a failure.
- Dependencies are weight. Each one is admitted for a stated reason and is
  expected to justify itself against the alternative of not existing.
- **Optimize what is measured, not what is imagined.** §II governs §VII: a
  performance change without a before-and-after under stated conditions is not a
  performance change, it is a guess that also increased complexity. Complexity
  spent on unmeasured speed is the worst trade available to us.

### 3.14 The interface is a window, not an application

The UI's job is to make system state legible and to trigger actions. It is not
where the system lives, does not hold authority, and is not a place where
behaviour hides. Everything the interface can do is something the system can do
without it, and the system remains fully functional with no interface attached
at all.

This is what allows §V and §VII to coexist: the surface can be radically thin
precisely because it carries no logic worth weight. It also follows from §3.2 —
if the window breaks, the machinery keeps running.

Corollary: minimalism is a constraint on *chrome*, never on *truth*. A
minimalist interface shows less decoration, not less information — and never
strips a measurement of the conditions §3.4 requires travel with it.

### 3.15 Ease means fewer decisions, not hidden ones

Intent VI asks for Ollama's ergonomics. The lesson worth taking is that the
common path should require no expertise. The lesson worth refusing is that the
tool should make consequential choices silently on the user's behalf.

The spirit: MCF chooses a sensible default for everything, so nothing blocks the
first token — and every such choice is visible, attributed, explained on demand,
and overridable. The user should be able to ask "why this quantization, why this
context length, why this runtime" at any moment and get MCF's actual reasoning,
including the measurements behind it.

Defaults are a service. Undisclosed defaults are a lie of omission, and §3.4
makes them a scientific problem as well as an ethical one, because a
configuration nobody recorded is a measurement condition nobody can reproduce.

### 3.16 Prefer substrates that let a machine enforce the principles

This document is only as strong as its weakest moment of human discipline. Every
principle here that depends on a person remembering it — §3.1's prohibition on
silent failures most of all — will eventually be violated by someone tired at
the end of a long change.

The spirit: **where a choice of language, structure, or tooling determines
whether a principle is checked by a compiler or merely hoped for, choose the one
that checks.** A design in which unhandled failure is a build error is
categorically better than one in which it is a code review finding, and the
difference compounds over the life of a system that intends never to fail.

This generalizes past the obvious case. It argues for making illegal states
unrepresentable rather than validating against them; for a measurement type that
cannot exist without its conditions attached (§3.4); for provenance that travels
with an artifact by construction rather than by convention (§3.6); and for
budgets asserted by tests rather than watched by humans (§3.13).

The corollary is a real constraint on engineering choice: a substrate that makes
these principles *unenforceable* is a substrate that costs more than it appears
to, however fast or familiar it is. See §7.19.

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
- **Observation costs performance.** See §6.9 — managed, never eliminated.
- **Ease costs transparency.** Every step removed from the user's path is a step
  they no longer see. §3.15 manages this; it does not abolish it.
- **Lightness costs features.** This is the intended cost, not a regrettable
  one. A tool that keeps every feature proposed to it cannot also be the
  lightest thing it could be, and we would rather be light.

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
- **Not a wrapper that hides the runtime.** MCF makes the runtime unnecessary to
  think about; it never makes it impossible to reach. Under §VI this is the line
  between ergonomics and concealment — see §3.15.
- **Not opinionated about which model you should want.** It is opinionated about
  *knowing* which one you should want, given what you told it you value.
- **Not a chat product.** The interface exists to operate and observe the
  system. A conversation surface is justified only as an instrument — a way to
  exercise a model and capture evidence about it — and must never grow into the
  reason MCF exists. Feature requests that make sense only for a chat app are
  out of scope by construction.
- **Not a platform.** No plugin ecosystem, no extension API, no configurability
  for its own sake. Every generalization is weight (§3.13), and weight is spent
  only where a stated intent demands it.
- **Not a clone of anything.** Ollama, LM Studio, and their peers establish that
  a level of ease is possible; none of them establishes how MCF should be built.
  "Because that is how the other tools do it" is not an argument, and matching a
  competitor's feature is never in itself a reason to carry its weight.

---

## 6. Conflicts Between Stated Intents

The intents are not mutually consistent as written. Each conflict below states
the tension, the resolution, and the confidence in that resolution.

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
§VII makes this harder, not easier: see §6.9.

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
*visible as* a default. §VI raises the priority of choosing it: frictionless
ease is impossible while the tool refuses to have an opinion out of the box.

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

### 6.9 "Fastest and lightest possible" vs. "deep full system logging and telemetry"

**Tension.** This is the sharpest conflict in the document and the one most
likely to be resolved badly under deadline pressure. Intent I asks for pervasive
instrumentation of everything. Intent VII says every cycle and byte MCF spends
is waste taken from the model. Telemetry is, viewed through §VII, the largest
single line item of self-inflicted weight in the design — and it is *always*
running, which makes it the worst kind under §3.13's idle-cost rule.

**Resolution — the obligation to record is absolute; the cost of recording is an
engineering problem, not a licence to record less.** §3.3 stands unmodified.
What §VII changes is that instrumentation must now be *engineered* rather than
merely *added*: cheap enough at the point of capture that it does not deform the
hot path, with cost pushed to writing, aggregation, and query where it is off
the critical path and can be paid lazily or not at all.

Three rules follow, in priority order:

1. **Never drop a record silently to save time.** That is the §3.1 sin
   committed for performance reasons, and performance is not an excuse that
   outranks honesty. If load-shedding is genuinely necessary, the gap is itself
   recorded — §3.2's mark applies to telemetry about telemetry.
2. **Reduce fidelity before reducing coverage.** Sampling rates, resolution, and
   retention are legitimate dials. *Which events exist at all* is not. We would
   rather know that something happened imprecisely than not know it happened.
3. **Verbosity is a dial with an honest floor.** The user may turn detail down
   for speed, and the floor beneath which they cannot go is whatever §3.4 needs
   to keep published measurements reproducible. Below that floor, MCF stops
   publishing numbers rather than publishing unconditioned ones.

**Confidence: high on the ordering, low on the feasibility.** "Deep full system
telemetry with negligible overhead" is a hard engineering target, not a
compromise position, and it may not be fully reachable. If it proves
unreachable, the correct amendment is to *narrow what MCF claims to observe* —
explicitly, here, in this document — never to keep the claim and quietly miss
records.

### 6.10 "Fastest and lightest possible" vs. "highest scientific standards"

**Tension.** Rigor is heavy in a way that is easy to underestimate. Measurement
history accumulates. Provenance chains accumulate. Repeated runs for statistical
confidence cost real time. Validation costs cycles on paths where skipping it
would never be noticed. A tool optimizing purely for speed would keep less,
check less, and repeat less — and would be a worse instrument for every gram it
saved.

**Resolution — §II outranks §VII wherever they meet.** Performance is weighted
equally with correctness (§VII) but *not* above the integrity of what MCF
claims. Concretely: MCF may be fast in how it validates, records, and repeats,
but may not become fast by validating, recording, or repeating less than the
science requires.

The reconciling insight is that these are less opposed than they look, and §3.8
explains why: **MCF's own weight contaminates MCF's own measurements.** A
bloated instrument that steals memory and cycles from the model it is timing
produces worse numbers, not just a worse experience. Lightness is therefore a
*scientific* requirement, not only an ergonomic one — which is why §VII is
weighted equally, and why the two intents mostly pull the same direction in
practice.

Where they genuinely diverge, the cost is disclosed rather than absorbed
silently: an operation that is slow because it is being done properly says so.

**Confidence: high.** The inverse ordering produces a fast tool whose numbers
cannot be trusted, which fails the project's purpose while succeeding at its
constraint.

### 6.11 "Runs on any system or device" vs. "fastest and lightest possible"

**Tension.** Universal reach and minimal weight are the classic opposition in
interface design. The technologies that run everywhere are typically the
heaviest available; the technologies that are lean are typically platform-bound.
Taken naively, §V argues for a shipped browser runtime or a cross-platform app
framework, and §VII forbids exactly that.

**Resolution — universality is bought by making the client thin, not by making
the runtime portable.** The escape from the tradeoff is §3.14: because the
interface holds no logic and no authority, it can be small enough that "runs
anywhere" costs nothing. MCF serves a view; the device already has something
capable of displaying it. Portability comes from *demanding almost nothing of
the client*, and specifically not from bundling a runtime, shipping a framework,
or building per-platform applications.

Two consequences follow, both non-negotiable under §VII:

- **The interface must not be the reason MCF is heavy.** Its idle cost, when
  nobody is looking at it, should be indistinguishable from zero. A UI that
  polls a busy machine every second in order to look responsive is spending the
  user's inference budget on decoration.
- **Headless is the base case, not a mode.** MCF is fully operable with no
  interface running at all (§3.14). The interface is an optional attachment to a
  system that was already complete without it.

**Confidence: high on the strategy, medium on the reach.** "Any device" is
unbounded as stated, and §7.16 records the need to bound it honestly. A phone
browser and a decade-old laptop are reasonable; a smart fridge is not a
commitment we should make.

### 6.12 "Runs on any device" vs. "the user's machine and data are theirs"

**Tension.** Reachability from other devices means the interface is exposed on a
network. That interface controls a service that downloads arbitrary code from
the internet, executes it, and can read what the user sends through the models.
§V's convenience and §3.10's privacy posture point in opposite directions, and
the failure mode is severe rather than merely annoying.

**Resolution — local-only by default; network exposure is an explicit,
informed, revocable act.** Reaching MCF from another device is a capability the
user turns on deliberately, knowing what becomes reachable. It is never the
out-of-box state, and it is never a side effect of enabling something else.
Access to the interface is access to the control plane, and MCF should treat it
with the seriousness that implies rather than the informality typical of
localhost developer tools.

Note this specifically constrains §VI: frictionless must not be read as "as open
as a typical localhost developer service," because MCF's surface is larger than
an inference endpoint — it can acquire and execute code.

**Confidence: high on the default, low on the mechanism.** What authentication
is proportionate for a single-user local tool — and how to add it without
violating §VII or §3.15's ease — is genuinely open. Recorded in §7.17.

### 6.13 "Fastest and lightest possible" vs. "reproducibility over convenience"

**Tension.** §3.12 warns that reproducibility erodes through small reasonable
decisions. §VII generates a steady supply of them: cache the probe result,
reuse the warm process, adapt the batch size to current load, skip the
verification we already did once, keep the previous run's state to avoid a cold
start. Each is a genuine speedup. Together they make results depend on hidden
history — the definition of irreproducible.

**Resolution — optimizations may not introduce undeclared state.** Caching,
reuse, and adaptation are permitted, and are expected under §VII — but anything
that could change a result must be *visible in that result's conditions*
(§3.4). A measurement taken with a warm cache is a different measurement from
one taken cold, and MCF must know which it produced.

Corollary, and the sharpest edge of this: **the benchmark path may not adapt.**
Adaptive behaviour that improves the serving experience destroys the isolation
§3.4 requires of a comparison. What MCF does to be fast for the user, it does
not do while measuring — and §6.2's separability problem is exactly the
mechanism this depends on.

**Confidence: high on the rule, medium on the boundary.** Where legitimate
optimization ends and result-altering hidden state begins will need real cases
to draw precisely.

### 6.14 Frictionless ease vs. "the hub is untrusted" and "no invented defaults"

**Tension.** Tools that achieve this level of ease do so substantially by
deciding for the user:
which quantization, which context length, which runtime, and an implicit trust
decision about the artifact. §3.7 requires informed and explicit consent before
running untrusted code; §6.5 forbids inventing an objective; §3.15 forbids
hidden choices. Naively applied, those principles reintroduce exactly the
friction §VI exists to remove.

**Resolution — automate the choice, surface the record, gate only what is
irreversible or dangerous.** MCF picks defaults freely and without prompting for
everything reversible and benign — quantization, context, runtime, placement —
because that is what §VI asks for and §3.15 permits, provided the choice is
recorded, attributed, and explained on demand.

The line is drawn at *category*, not at *frequency*: decisions that execute
untrusted code (§6.4), consume large irrecoverable resources, expose the system
to a network (§6.12), or destroy existing artifacts (§3.11) are asked, every
time, no matter how much friction it adds. Everything else flows.

The user should never be stopped to be *informed*; they should only be stopped
to *authorize*. Information is delivered by the record, which they can consult
whenever they care.

**Confidence: high.** This is the reading that lets §VI and §3.7 coexist without
either being reduced to a slogan, and it locates the friction where it buys
something real. Note it is also a place where MCF should be *better* than the
tools cited as its ease benchmark, not merely equal to them: matching their
friction while exceeding their honesty is the whole ambition of §VI.

---

## 7. Voids — Where Intent Is Missing or Underdetermined

These are questions the stated intents do not answer and cannot be derived from.
Each will be answered by someone; this section exists so that it is answered
*deliberately, and recorded here*, rather than settled accidentally by whoever
writes the code first.

They are ordered roughly by how much downstream design they block. **Numbers
reflect order of discovery; position reflects blocking priority** — a void keeps
its number for life so it can be cited stably, but may be moved up the list as
its urgency becomes clear.

### 7.1 ~~What "deployed" actually means~~ — **RESOLVED in Revision 2**

Intent VI answers this: deployment means **persistent local hosting** — a
long-lived service, a stable API, models addressed by name.
MCF is therefore a **daemon** with clients attached to it, not a command-line
instrument that exits. This settles the shape of nearly every reliability
question in §3.1: MCF is a process that must survive indefinitely, supervise
child runtimes, and recover across restarts. It also makes §3.13's idle-cost
rule central rather than incidental, since a daemon's dominant state is idle.

*Now-open sub-questions:* what API surface is offered (OpenAI-compatible,
Ollama-compatible, both, native), what the supervision contract is when a served
runtime dies, and how many models may be resident simultaneously (§7.9).

### 7.2 The objective function — **blocking §IV**

§6.5 defers the definition of "optimal." Someone must eventually state how
quality, latency, throughput, memory, power, and disk trade against one another,
and how a user expresses their own weighting. Without this, the optimization
intent cannot be implemented, only gestured at. §VI raises its urgency: a tool
that is frictionless by intent must ship a default opinion, and §6.5 requires
that opinion be stated rather than emergent.

### 7.3 What "quality" is measured against — **blocking §IV**

Intent IV says "accuracy benchmarks" without saying accuracy at what. Public
academic suites are contaminated and often unrepresentative of real use. The
user's own work is representative but has no ground truth. A stronger model as
judge introduces its own biases and a dependency MCF may not want. How MCF
measures *quality* — as opposed to *speed*, which is comparatively easy — is the
single hardest unanswered question in the project, and the credibility of Intent
IV rests entirely on it.

### 7.4 Engine ownership: does MCF perform inference, or delegate it? — **blocking §VI and §VII**

Newly urgent, and arguably now the most consequential unanswered question in the
document. §VII's "fastest possible" reads as an argument for owning the
inference path; §VI's frictionless breadth and §III's "any model" read as an
argument for delegating to mature runtimes.

The honest reconciliation is almost certainly that **MCF's performance mandate
applies to MCF's own overhead, not to the inference kernels** — MCF cannot be
faster at matrix multiplication than the projects that specialize in it, and
attempting to be would sacrifice §III's coverage for a loss. Under that reading
§VII means: *the lightest possible wrapper, adding the least possible latency
between request and token, over the fastest available engine — and knowing
empirically which engine that is on this hardware, which is precisely what §IV
is for.*

That reading is stated here as the likely answer, not as a resolution, because
it decides the project's architecture and deserves to be decided deliberately
rather than inherited from a paragraph in §7.

### 7.19 Implementation substrate — **blocking, and coupled to §7.4**

*Recorded in Revision 3. Placed here because it cannot be separated from the
question above it.*

No stated intent names a language, runtime, or structural approach, and none
should — that is a technical decision, not an intent. What belongs in this
document is the **criteria the intents impose on that decision**, so that it is
made against them rather than against familiarity or momentum.

The intents constrain the choice as follows:

- **§I and §3.16** — the dominant class of daemon failure is memory and
  concurrency error. A substrate that makes those *impossible* rather than
  *unlikely* is worth a great deal here, because §3.1's prohibition on silent
  failure is exactly the kind of rule that erodes under human discipline and
  holds under machine enforcement. Explicit, non-ignorable error handling is
  worth more to this project than almost any other property.
- **§VII** — no interpreter, no dominant runtime, no unavoidable idle work, a
  small resident footprint with nothing loaded, and fast cold start. §7.16's
  budgets, once they exist, are the real test; a substrate that cannot plausibly
  meet them is disqualified regardless of other merits.
- **§III, §IV and §7.4** — hardware probing, accelerator interrogation, and
  driving inference engines all mean talking to C interfaces constantly.
  Friction at that boundary is a recurring tax on the project's central work,
  not an occasional inconvenience.
- **§II and §3.5** — the test and simulation discipline the science requires
  must be *pleasant enough to actually maintain*. A substrate that makes
  fake hardware, synthetic artifacts, and replayed telemetry painful will
  quietly erode §3.5, and §3.5 is what earns MCF the right to be believed.
- **§3.13** — dependencies are weight, so ecosystem maturity matters in a
  specific and slightly unusual way: what counts is having good *small* pieces
  available, not a large framework that solves everything at a cost.
- **§V and §6.11** — the interface is a thin client over a service, so this
  decision governs the daemon. The surface has its own, much lighter, answer.

**The likely answer, stated as a candidate rather than a resolution.** The
combination of §I's reliability mandate and §3.16's enforcement principle points
away from C++ and toward a memory-safe systems language with no runtime — with
Rust the obvious candidate, because it is the one where "every failure is
explicitly handled" is a property the compiler checks rather than a rule the
reviewer remembers. C++ can reach the same performance and the same footprint,
but reaches the same *reliability* only through sustained discipline, and this
project has declared reliability its first intent. A garbage-collected language
is a weaker fit against §VII's idle-cost rule and against the C-interop tax
above, though not an absurd one.

That reasoning is recorded here, not resolved, because it is exactly the kind of
decision this document exists to inform rather than to make. It also depends on
§7.4: if MCF ever owned inference kernels, the calculus changes substantially.

**What would settle it:** §7.16's budget numbers, and a small adversarial
prototype of the least pleasant part of the system — probing a GPU, supervising
a child runtime that is deliberately made to die badly, and recording both under
§3.1 — built more than once if necessary. Choosing this by argument alone would
violate §3.13's own rule that we optimize what is measured rather than what is
imagined.

### 7.5 Retention, scope, and residency of telemetry

§6.8 splits system telemetry from content, but leaves open: how long is anything
kept, how much disk may telemetry consume, what happens when that budget is
exhausted (and how §3.1 and §6.9 forbid that from being a silent drop), whether
anything may ever leave the machine, and whether the user can inspect and purge
what MCF holds about them. §3.10 implies strong answers but does not supply them.

### 7.6 Reproducibility guarantee level

§3.12 asserts reproducibility wins, without saying reproducible *to what
tolerance* and *across what changes*. Same machine, same day? Same machine after
a driver update? A different machine of the same model? This determines how much
environment must be captured and pinned, which is a large cost either way, and
§6.13 cannot draw its boundary precisely until this is answered.

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

### 7.9 Resource arbitration and concurrency — **sharpened by §VI**

Models are enormous relative to available memory and disk. Who decides what is
resident? What happens when a benchmark and a served model both want the GPU, or
when a download would exhaust the disk mid-flight? §3.11 forbids surprising
destruction but does not say who arbitrates.

Persistent hosting makes this immediate rather than eventual: a daemon serving
one model while benchmarking another is the *normal* case, not an edge case, and
§3.8 says the contention will corrupt the measurement unless MCF governs it.
Whether MCF may refuse to benchmark while serving — or must — is unstated. This
is where the ugliest reliability bugs will live.

### 7.10 Failure taxonomy

§3.1 requires every failure to be classified, but the classification scheme does
not exist. It needs to be designed once, deliberately, because it will appear in
logs, telemetry, tests, the recommendation engine, and the user interface — and
retrofitting it later will be miserable.

### 7.11 Offline and degraded-network operation

Local inference is frequently chosen for disconnected environments. How much of
MCF works with no network at all? §3.2 suggests "most of it, loudly labelled,"
but this has never been stated as an intent and deserves to be. Intent V adds a
wrinkle: an interface reachable from other devices assumes a local network even
when there is no internet, and those two conditions should not be conflated.

### 7.12 ~~The user surface~~ — **PARTIALLY RESOLVED in Revision 2**

Intent V answers the character of the interface — minimal, lightweight,
device-agnostic — and §6.11 and §3.14 answer its strategy: a thin client over a
service that is complete without it. §7.1 confirms a daemon underneath.

*Still open:* whether a command-line surface exists alongside the visual one and
whether it is the primary or secondary control path; whether the API MCF exposes
for serving is the same API its own interface consumes (a strong simplifying
answer, and cheaper under §VII, but it couples the two); and what happens when
more than one client is attached at once.

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
changes what "highest standards" costs. Intent V's "any device" hints outward
without committing.

### 7.16 The performance budget itself — **blocking §VII**

§3.13 requires that weight be budgeted, and §3.5 requires those budgets be
tested — but no numbers exist. "Fastest and lightest possible" is unfalsifiable
as written and therefore cannot be defended, only invoked. What is the target
idle CPU? Resident memory with nothing loaded? Cold start? Acceptable added
latency between an inbound request and the engine's first token? Installed
footprint?

Until real numbers exist, §VII is a mood rather than a constraint, and §3.13's
regression tests have nothing to assert against. This is the highest-leverage
void to close, because it converts the newest intent from rhetoric into
something that can be enforced.

Related and equally unstated: **what "any device" bounds mean** (§6.11) — the
oldest, weakest client MCF commits to serving usefully. Without it, §V is also
unfalsifiable.

### 7.17 Authentication and the trust posture of the control plane

§6.12 establishes local-only by default with deliberate exposure, but not what
protects MCF once exposed. A single-user local tool has no obvious identity
model, and §VII and §3.15 both resist adding one. What proportionate protection
looks like — and whether the serving API and the control API deserve *different*
answers, since one is far more dangerous than the other — is open.

### 7.18 What happens to a served model when the user stops looking

A persistent host must decide about idleness: does a loaded model stay resident
indefinitely, holding memory the user might want back, or unload after some
period and pay a cold start on next use? §3.13 says idle cost should embarrass
us; §VI says the first token should be immediate; §3.11 says nothing is
destroyed without deliberation. All three bear on this and none of them decides
it, and whatever is chosen becomes a measurement condition under §3.4 — a
response time is a different number depending on whether the model was resident.

---

## 8. Amending This Document

- Intent changes when the *reasoning* changes, not when the code does. Code that
  diverges from this document is either a bug or an argument; both require the
  divergence to be raised here explicitly.
- Every conflict resolution in §6 is falsifiable by implementation experience.
  When implementation contradicts a resolution, amend the resolution and record
  what taught us better — do not leave the document standing while the code
  disagrees with it.
- When a void in §7 is filled, mark it resolved in place, note what answered it,
  and migrate the substance into §3 or §6 as a principle or a resolution. §7
  should shrink over time; if it does not, we are building on undeclared
  assumptions.
- New voids are added the moment they are noticed — including by a subagent, a
  code review, or a failed design discussion. An unrecorded void is how a
  project acquires intent nobody chose.
- **New intents are integrated, not appended.** A statement of intent added
  later is not additive by default: it may contradict resolutions already made,
  and it may answer voids already recorded. Adding one means re-reading §6 and
  §7 in its light. Revision 2 is the worked example — three sentences added six
  conflicts and closed two blocking voids.
