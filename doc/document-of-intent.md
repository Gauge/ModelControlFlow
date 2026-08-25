# Document of Intent

| | |
|---|---|
| **Type** | Intent — the spirit of the rules |
| **Version** | 10 |
| **Status** | Living |
| **Authority** | Source. Every other document in `doc/` derives from this one and is corrected when it changes, never the reverse. |
| **Derives** | [rules.md](rules.md) · [roadmap.md](roadmap.md) · [backlog.md](backlog.md) · [mockup/](mockup/) |

**What this document is.** The thing to consult when a rule is ambiguous, when
two rules conflict, or when no rule exists yet — and the source from which real
rules are written. It is not a requirements specification, not an architecture
document, and not a backlog. Nothing here is directly implementable, and that is
deliberate.

**How to use it.** When writing a specification, a test plan, a lint rule or a
review comment, cite the clause it serves. If no clause fits, that is a finding:
record a void in §7 rather than inventing intent silently. Enforceable rules
derived from these clauses live in [rules.md](rules.md), one per clause or
better, each carrying a check.

**Standing conditions on everything below.**

- The repository holds no implementation. §6 resolves tensions on principle
  alone, and every resolution is falsifiable by implementation experience: the
  first real code that touches a question may argue back, and this document is
  amended rather than quietly violated (§8).
- Clause numbers are stable for life. A clause keeps its number so it can be
  cited stably, even when its position or status changes.
- History lives in §9 and nowhere else. Every clause here states the present
  position; how it came to be held is a changelog entry.

## Contents

| § | Section | What it holds |
|---|---|---|
| §1 | [What This Project Is](#1-what-this-project-is) | The six verbs, and which of them are the point |
| §2 | [The Founding Intents](#2-the-founding-intents) | I–XVII, the originating statements |
| §2.1 | [Settled Decisions](#21-settled-decisions) | Questions once open, now answered, with their reasoning |
| §3 | [Principles](#3-principles) | The load-bearing beliefs |
| §4 | [Standing Tensions](#4-standing-tensions-we-accept) | Permanent conditions, managed rather than solved |
| §5 | [Anti-Goals](#5-anti-goals) | What MCF is not |
| §6 | [Conflicts Between Stated Intents](#6-conflicts-between-stated-intents) | Where intents disagree, and how each is resolved |
| §7 | [Voids](#7-voids--where-intent-is-missing-or-underdetermined) | Questions the intents do not answer |
| §8 | [Amending This Document](#8-amending-this-document) | How intent changes |
| §9 | [Changelog](#9-changelog) | What changed, when, and why |

---

## 1. What This Project Is

ModelControlFlow (MCF) exists to make the open weights ecosystem *usable by one
person on one machine without that person becoming a full-time operator of it.*

Six things follow from that sentence and they are the whole project:

1. **Acquire** — obtain any model published on Hugging Face, with its
   provenance intact and its licensing legible.
2. **Serve** — host that model as a persistent, dependable local endpoint, on
   the hardware in front of us, with as little ceremony and as little overhead
   as physically possible — configured to expose everything the model can
   actually do — or explain precisely why it cannot run.
3. **Analyse** — put the model on the bench: a range of purpose-built
   laboratories, each instrumented for the question it asks, each producing
   evidence about a different quality of the thing being tested.
4. **Judge** — measure what it costs and what it is worth here, on this
   hardware, for the work actually being done, and use those measurements to
   converge on the best available local configuration.
5. **Share** — carry results off this machine deliberately, with their
   conditions attached, so that many machines' evidence can become something no
   single machine could produce; and accept a configuration identified
   elsewhere and reproduce it here exactly.
6. **Show** — make all of the above legible through an interface light enough
   to run anywhere, on anything.

The third and fourth are the point. Acquisition and serving are table stakes;
plenty of tools do them. The reason MCF is worth building is the closed loop —
*instrument, measure, compare, select, re-measure* — and the fact that the loop
runs **here**, on the hardware the answer is actually for. A version of MCF that
downloads and runs models but cannot tell you which one you should be running
has missed its purpose.

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
system. This claim is established by §VIII rather than by observation: every
failure MCF says it survives is a failure the laboratory can produce on command.

### II. Science — "the highest scientific standards"

Every claim MCF makes about a model is a measurement, and every measurement
carries the obligations of a measurement: a stated method, stated conditions,
stated uncertainty, and the ability for someone else to repeat it. Full test
suites are the floor, not the ceiling — they establish that the code does what we
think, which is a prerequisite for believing what it reports. §VIII is the
elaboration of that floor into the project's central discipline.

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

**On Ollama.** Ollama is cited as an *example of the ergonomic standard*, and
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

### VIII. Verification — "full system unit tests and simulated lab tests"

Confidence in MCF is established **before** deployment, in a laboratory, rather
than inferred **during** deployment, from observation. Every environment MCF must
survive is constructed deliberately and deterministically: absent hardware,
hostile hubs, dying runtimes, exhausted disks, stalled networks, thermal
ceilings. The suite exercises the whole daemon, not merely its parts.

This intent is the counterweight to the deliberate de-prioritization of ambient
telemetry in §3.3. A system that declines to watch itself in production must be
able to *reproduce* itself on demand, and this is the trade being made
knowingly: **rigor moves from the observatory to the laboratory.**

### IX. Workflow evaluation — "benchmark models on the work they will actually do"

The quality that matters is not the quality measured by academic suites. It is
whether a model can **do the work**: follow instructions, produce parseable
structured output, retrieve from a long context, call tools correctly, recover
from its own errors, and know when to stop.

MCF measures models on tasks of that shape, in a controlled environment, with
outcomes that can be checked.

**There is no single such shape, and this intent does not name one.** Agentic
workflow — multi-turn, tool-calling, self-correcting — is the shape that matters
most to this project's author and is therefore the first laboratory built. It is
not the definition of quality, because a model that cannot call a tool is not
thereby a bad model; it is a model that is *not applicable* to that measurement
(§3.23). A model that extracts fields flawlessly and has never seen a tool
schema is excellent at the work somebody actually needs done.

Quality is therefore **plural**: a set of measured, separately-reported
qualities, each produced by a laboratory built for one of them (§XIII), each run
only where the model is capable of it (§X), and each mattering only to the
extent the user's work resembles it (§3.19). MCF never reduces them to a score.

### X. Capability discovery — "identify and automatically configure full model capabilities"

Getting a model to merely produce tokens is the easy half. Getting it to produce
tokens *the way it was designed to* — correct chat template, tool-calling format,
stop conditions, context window, vision or embedding or reasoning modes,
structured-output support — is where local deployment usually goes wrong,
silently, in ways that make a capable model look mediocre.

MCF determines what each model can do and configures it accordingly, without
requiring the user to know. **A misconfigured model is a measurement error**
(§3.8), so this intent is a precondition for §IX and §IV being honest, not
merely a convenience.

### XI. Both surfaces — "a lightweight UI as well as a headless mode"

MCF is fully operable with no interface attached, and equally usable with one.
Neither is a degraded version of the other.

This ratifies what §3.14 and §6.11 already held — the interface is a window over
a service complete without it — and adds one requirement those did not state:
**parity.** Every capability is reachable headlessly; the interface introduces no
action that exists only there. See §6.21.

### XII. The reference model — "use Qwen3.8-27B for all initial testing"

Early work needs something real to be developed against. The stated choice is
**`unsloth/Qwen3.8-27B-GGUF`** (https://huggingface.co/unsloth/Qwen3.8-27B-GGUF),
and it is the first intent in this document that names a specific artifact
rather than a property of the system.

What the repository *declares*, recorded as declaration rather than fact
(§3.7, §3.18): a 27B model, licensed Apache-2.0, derived from
`Qwen/Qwen3.8-27B`, published in GGUF form across quantizations from roughly
1-bit through BF16. MCF has verified none of it, and will not until §X probes
it. That is not a caveat about this repository in particular; it is the posture
toward every repository, and the reference model earns no exemption from it by
being the reference.

Three properties make it a *good* fixture, and they are the reason to record
this as intent rather than as a note:

- **It is the hard provenance case, not the easy one.** A third-party
  requantization is a derivative whose chain runs through one publisher's
  pipeline back to another's weights. §3.6 demands exactly that chain be
  preserved, and building against this artifact exercises the requirement from
  the first commit rather than discovering it later against something simpler.
- **Its breadth of quantizations is a frontier in itself.** One model, one
  machine, twenty quantizations is the cleanest possible §3.4 comparison: a
  single variable, many points. §IV's frontier work has a subject before any
  second model exists.
- **27B is the useful size.** Large enough that residency (§7.18), arbitration
  (§7.9) and memory pressure are real problems on consumer hardware rather than
  hypothetical ones; small enough that one machine can hold it. A 7B model would
  have let those questions stay theoretical.

What this intent does **not** license is set out in §6.22 and §6.23. In
summary: no code path may behave specially because an artifact is the reference
model, the test suite may not depend on it, and nothing measured on it
generalizes to models in general.

### XIII. Analysis — "an analysis tool as well as a hosting platform"

MCF is a bench, not only a host. It carries a range of **diagnostic
laboratories**, each built for one question about a model, and each instrumented
as deeply as *its own question* requires: a latency lab watches different things
than a tool-calling lab, a context-degradation lab different things again, and
none of them should be forced through one generic instrumentation profile that
serves none of them well.

The unit of work is therefore the **lab**: a named, versioned, reproducible
experiment with its own apparatus, its own telemetry, its own outputs, and its
own statement of what it does and does not establish. §IX's agentic suite is the
first of them, not the whole of them.

This is a substantial enlargement of §IV and it comes with a specific danger:
"deep telemetry, custom per lab" is the phrase that grew into the ambient
telemetry D5 refused, and "a wide range of labs" is the phrase that grows into
the platform §5 refuses. §6.24 and §6.26 draw those lines. The short form is
that **telemetry inside a lab is not ambient — it is the experiment** — and that
labs are built in-tree, one at a time, each justifying itself by the validity it
adds.

### XIV. The shared record — "collected in a local SQLite database that can be shared"

Diagnostic results are durable, queryable and portable: a local database that
belongs to the user, and that the user can **choose to contribute** so that many
machines' evidence aggregates into something no single machine can produce — a
picture of how models behave across real hardware, under stated conditions.

The aggregation happens elsewhere. MCF's obligation is to produce a contribution
that is honest, conditioned and safe to publish: every row carries the conditions
that produced it (§3.4), and no row carries anything the user did not intend to
send (§3.10).

This intent inverts §5's suspicion of foreign numbers and must be read
precisely, which §6.28 does. MCF still decides **locally** and still refuses to
let a number measured on someone else's machine choose a configuration on this
one. What changes is that MCF now *contributes* to a corpus rather than
pretending the corpus should not exist — and the thing it contributes is the
thing most public leaderboards lack, which is the conditions.

### XV. Reproduce by identifier — "paste an identifier, get exactly that"

A user who has found, elsewhere, the model and configuration that suits them
should be able to hand MCF a single identifier and receive **exactly that**: the
same weights, the same quantization, the same context, the same runtime and
sampling parameters — or a precise statement of why this machine cannot
reproduce it.

*Long-term context, deliberately out of scope:* the intended source of such
identifiers is a website the author hosts, which aggregates contributed results
(§XIV) and helps people choose. **That website is not part of this project.**
MCF's side of the contract is the whole of MCF's obligation: emit an identifier
for a configuration it holds, and reproduce a configuration from an identifier it
is given.

An identifier is an inbound instruction from outside the machine, which makes it
§3.7's problem, and it names a configuration MCF has not measured, which makes it
§3.18's problem. §6.29 resolves both, and the resolution is the same one this
document reaches every time: **an imported configuration is a declaration until
this machine verifies it.**

### XVI. Self-contained — "no prerequisite downloads"

A user obtains MCF and runs it. Nothing else. No runtime to install first, no
interpreter, no toolchain, no framework, no separately-fetched inference engine,
no "first install these three things." Whatever MCF needs in order to do what it
claims, MCF ships.

This is an ergonomic intent with a scientific consequence, which is why it
belongs here rather than in a packaging document: **a dependency the user
installs is a dependency MCF did not pin.** Their compiler, their Python, their
CUDA toolkit, their libc — each is an unrecorded variable in every measurement
MCF takes on that machine (§3.12, §3.4). Shipping the dependency is how MCF
knows what it ran.

The cost is real and is accepted: a larger artifact, a build that vendors more,
and work MCF cannot delegate to a package manager. §6.31 records what this does
to §7.4's engine question and to §VII's budget, and the short form is that
**§VII's budget describes MCF's behaviour, not its download size** — a static
binary that idles at nothing and starts in milliseconds honours §VII whether it
is 12 MiB or 400.

Where a capability genuinely cannot be shipped — a vendor driver, a kernel
feature, hardware that is not present — MCF does not send the user on an errand
as a substitute for handling it. It states what is missing, what is therefore
unavailable, and continues without it (§3.2).

### XVII. Full utilization — "use everything this machine has, including admin rights if needed"

MCF is permitted to use the machine to its limit when the work calls for it:
exclusive access to an accelerator, locked pages, pinned cores, raised
priorities, direct thermal and power interfaces, performance governors — and
the elevated privileges some of those require.

This intent exists because of §3.8. A measurement taken by a process that cannot
see the machine's thermal state, cannot stop another process from stealing the
accelerator, and cannot read the power draw is a measurement with unknown
conditions. **The privilege is not for convenience; it is for the validity of
the reading**, and it is the difference between "this model is slow" and "this
machine was throttling."

The danger is obvious and is not waved away. A privileged daemon that fetches
code from the internet, executes model-supplied code, and may be reachable over
a network is the most dangerous object this document has described. §6.32 draws
the boundary, and it is severe: **privilege is per-operation, never ambient, and
never anywhere near untrusted code.**

## 2.1 Settled Decisions

Questions that §7 once held open and that are now answered. Their substance
lives here rather than in §7, per §8: a void that stays recorded after it is
answered makes the open list dishonest. The void numbers remain citable and are
indexed at the end of §7.

### D1 — MCF is a daemon *(answers §7.1)*

Deployment means persistent local hosting: a long-lived service, a stable API,
models addressed by name. MCF is therefore a process with clients attached, not
a command-line instrument that exits.

This settles the shape of nearly every reliability question in §3.1 — MCF
survives indefinitely, supervises child runtimes, and recovers across restarts —
and it makes §3.13's idle-cost rule central rather than incidental, because a
daemon's dominant state is idle.

*Still open beneath it:* which API surface is offered, the supervision contract
when a served runtime dies, and how many models may be resident at once (§7.9).

### D2 — Quality is plural, measured per workflow, and never reduced to a score *(answers §7.3)*

Quality is not one quantity. It is a set of separately-measured, separately-
reported qualities, each established by a laboratory built for one workflow
class (§XIII), each graded by a checkable outcome (§3.19), each run only where
the model is capable of it (§3.23), and each mattering in proportion to how much
the user's work resembles it.

**Why plural rather than singular.** An earlier reading made agentic task success
*the* definition of quality. That reading fails on contact with the ecosystem:
many models have no tool-calling capability at all, and scoring them near zero
on an agentic suite says nothing about them except that the suite was the wrong
instrument. Measuring a model on a workflow it was never built for produces a
number that is precise, reproducible and meaningless — the §6.1 failure mode
reached by a new route.

**What survives from the earlier answer, unchanged.** Checkable outcomes remain
the grading mechanism everywhere, which is what sidesteps the three traps §7.3
identified: contamination, because a verifiable task needs no ground-truth
corpus and is hard to memorise; unrepresentativeness, because a workflow lab
resembles the work by construction; and judge dependency, because outcomes are
checked rather than graded by another model (B13).

**What follows and is binding.** MCF publishes a *profile*, never a score. There
is no weighted average across laboratories, no overall rating and no ranking —
§3.9 already forbids collapsing a multi-objective frontier into one number, and
this is the same prohibition applied to quality. Which qualities matter is the
user's declared objective (§6.5), not MCF's opinion.

*Still open beneath it:* which laboratories exist and in what order (§7.29);
what each suite contains (§7.23); whether qualities with no checkable outcome —
prose, tone, taste — are measured at all, declined, or admitted with a declared
judge (§7.3 residual); and contamination remains reduced rather than eliminated.

### D3 — Both surfaces are first-class; headless is primary *(answers §7.12)*

Every capability is reachable with no display attached. The interface is a
client of the same API a script uses, and introduces no action that exists only
there (§XI, §6.21).

Three intents converge on this, which is about as settled as this document gets:
§XI requires parity, §VII prefers not paying for two implementations, and §VIII
can only test what is reachable headlessly.

### D4 — The substrate is Rust *(answers §7.19)*

**Why it follows from the intents,** recorded so the reasoning survives the
decision:

- **§I and §3.16 decide it.** The dominant class of daemon failure is memory and
  concurrency error, and Rust makes those largely impossible rather than merely
  unlikely. More importantly, §3.1's prohibition on silent failure — this
  document's central rule, and exactly the kind that erodes under human
  discipline — becomes a property the compiler checks rather than one a reviewer
  remembers. That is §3.16 applied to the largest available decision.
- **§VII permits it.** No interpreter, no garbage collector, no runtime, a small
  static binary, negligible idle footprint, fast cold start.
- **§III, §IV and §7.4 favour it.** Hardware probing, accelerator interrogation
  and driving inference engines are constant C-ABI work, and Rust pays no tax at
  that boundary. This is where garbage-collected alternatives lose specifically
  for this project, whatever their other merits.
- **A useful accident:** Hugging Face's own `safetensors` and `tokenizers` are
  Rust libraries, so §III's acquisition layer builds on first-party code.

**Why not C++.** C++ reaches the same performance and the same footprint; this
is not a performance argument. It reaches the same *reliability* only through
sustained discipline, and reliability is this project's first stated intent. The
tiebreaker is §3.16, not speed.

**Costs, accepted.** Slower to write. Async Rust is genuinely complex and will
be felt in the concurrent-download and streaming paths. The model conversion and
quantization ecosystem is Python, treated as supervised subprocess tools — which
D1's daemon architecture wants anyway.

**Conditional on §7.4.** This reasoning assumes MCF wraps inference engines
rather than implementing kernels. If that changes, the substrate question
reopens with it.

**Validated, not assumed.** §3.13 requires optimizing what is measured rather
than what is imagined, so this decision is confirmed by a small adversarial
prototype: probe an accelerator, supervise a child runtime deliberately made to
die badly, record both under §3.1, and measure the result against §7.16. If that
goes badly, this entry is amended rather than defended.

### D5 — Confidence comes from the laboratory, not from ambient telemetry *(answers §6.9)*

MCF establishes confidence **before** deployment, in a laboratory, rather than
inferring it **during** deployment, from observation (§VIII, §3.17). Continuous
sampling, always-on tracing and metric streams are refused by default; recording
happens at events, not on a timer.

**What is untouched by this, and it is the important half:** the record (§3.3)
is not telemetry and is not negotiable. Measurement conditions, failure context
and provenance stay whole, §3.1's prohibition on silent failure stands
absolutely, and the §3.4 floor is a floor rather than a setting. The two ideas
are separated precisely so that a decision about one cannot be misapplied to the
other.

**Why this is coherent rather than a compromise:** production observation and
laboratory reproduction buy the same good, and the lab buys it deterministically,
cheaply, and before release rather than after. The cost is real and is recorded
in §6.15 and §6.16.

### D6 — The record is a SQLite database *(follows from §XIV)*

The record — measurements, capability verdicts, failure records, provenance,
lab results — lives in a single embedded SQLite database on the user's machine.

**Why it follows.** §XIV requires the record be portable, and a single file is
the most portable artifact there is. §II requires it be queryable, and the
alternative to a query language is a query language written badly. §VII permits
it: an embedded engine with no server, no daemon of its own and no idle cost.
D4 makes it cheap, since the binding is a C-ABI library Rust drives without
tax. §3.3's "structured and machine-readable first" is satisfied by
construction rather than by discipline.

**What the decision drags with it, recorded rather than discovered later:** a
schema is a public interface the moment it is shared (§7.30), migrations become
a correctness problem the moment measurements must survive them (§7.13), and a
single file is a single point of corruption — which is a §3.1 obligation, not a
footnote.

**The content store is not this store.** §6.8 requires prompt and completion
content live separately from the system record. That separation is what makes
§XIV safe: the shareable database contains no user content *because content was
never in it*, not because an export filter removed it. §6.27 turns on this.

### D7 — MCF is meant to be used by people other than its author *(answers §7.15)*

§XIV and §XV describe a user who downloads MCF, contributes results, and pastes
an identifier they found elsewhere. That is not the author.

**What this settles:** documentation, installation and interface stability are
*goals* rather than incidental. A tool distributed to strangers cannot rely on
its operator knowing what its author knew, and §3.15's "explained on demand" is
owed to somebody who has never read this document.

**What it costs:** breaking changes acquire a cost they did not have; §7.13's
migration problem becomes other people's data; and §7.17's authentication
question stops being hypothetical, because the machines MCF runs on are no
longer all owned by someone who understands the risk.

**What it does not license:** MCF does not become a product with a support
surface, a plugin ecosystem or a configuration language. §5 stands. Being usable
by others is a quality bar, not a mandate to generalize.

### D8 — Laboratories run exclusively, and may be greedy *(resolves most of §7.9)*

A diagnostic laboratory owns the machine for the duration of its run. No user
traffic is served, no second laboratory runs beside it, and within its run the
lab may take whatever resources accuracy requires — all of the accelerator,
locked memory, pinned cores, raised priority.

**Why this is the rigorous answer rather than the convenient one.** §3.8 holds
that contention corrupts a measurement and §3.4 requires one variable to differ.
A lab sharing a machine with a served model is measuring the pair, not the
model. Exclusivity removes the single largest confound available, and it removes
it structurally rather than by correction: there is nothing to subtract because
there was nothing else running.

**What this settles in §7.9.** The question *may MCF refuse to benchmark while
serving, or must it* is answered: **it must.** Arbitration between a lab and the
serving path is not a scheduling problem to be solved; it is a state machine
with one occupant.

**What it costs, and where it is paid.** §VI promises a persistent, dependable
endpoint, and a lab suspends it. That conflict is §6.33, and the resolution is
that suspension is *explicit, announced, bounded and never silent* — the daemon
stays up and answers, the endpoint reports why it is unavailable and for
roughly how long, and no request is quietly dropped or slowed into a timeout.

**What greed does not license.** A lab is greedy *while running* and costs
nothing when it is not (§3.22, B30). Greed is a property of an experiment
somebody started, never of MCF.

### D9 — The time model

Time is one of the conditions every measurement carries, so it is standardized
rather than left to whichever call was convenient:

- **Durations come from a monotonic clock,** always. A duration is never
  computed by subtracting wall-clock readings, because wall-clock steps, drifts
  and is adjusted underneath a running process.
- **Records are timestamped in UTC,** stored with the local offset alongside
  rather than baked in, so a record is both comparable across machines and
  legible about where it was taken.
- **The laboratory uses a simulated clock** (§3.17), and a result knows which
  clock produced it. A duration from simulated time is never a performance
  number (A11).
- **Clock anomalies are events, not corrections.** A backward step or a large
  forward jump during a measurement invalidates that measurement loudly rather
  than being smoothed away (§6.1).
- **Contributed timestamps are coarsened** to whatever §7.27 decides, because
  precise timing is an identifier.

### D10 — The application is tested; the model is put in a laboratory

Two disciplines that share the word "test" and are never conflated (§6.7), each
with its own standard:

**MCF's own code is tested exhaustively, in tiers.** Unit and property tests for
logic; functional tests for behaviour at the API surface; whole-system tests
across the process boundary with real persistence and restart; fault-injection
tests from the laboratory's catalogue (§3.17); load and soak tests for a daemon
that must run for months; performance tests asserting §3.13's budgets; fuzz
tests wherever untrusted bytes enter (§3.7); and **mutation testing as the test
of the tests** — a suite that does not fail when the code is deliberately broken
is a suite that proves nothing, and §3.5's credibility argument rests on knowing
the difference.

**Models are not tested; they are measured** in laboratories built for one
question each (§XIII). A lab has no pass condition, produces a distribution
rather than a verdict, and never gates a build (§6.7).

The tiering is a consequence of §3.5's rule that the suite runs on a laptop,
offline, in seconds: the fast hermetic tier gates every change, and the heavy
tiers — load, soak, mutation, full fault matrix — run on a schedule and before a
release. Neither tier is optional, and a heavy tier that has not run recently is
reported as such rather than assumed green.

### D11 — Energy and thermal state are first-class measurements

Power draw, energy per token, and thermal state are recorded and reported
alongside latency and throughput, not treated as exotic extras. For local
inference they are frequently the deciding quantity: a configuration that is 8 %
faster and 40 % hungrier is a different choice on a laptop than on a
workstation, and §3.9's frontier already lists power as an axis it never
explained how to populate.

**Measurement fidelity varies by platform and is stated rather than assumed.**
Where an accelerator or CPU exposes a real interface, MCF reads it. Where it
exposes an estimate, MCF records it as an estimate (A20). Where nothing is
available, energy is `unknown` and stays unknown (A7) — never modelled from
utilization and presented as though it were measured.

Reading these interfaces is one of the concrete reasons §XVII exists, and
polling them is instrumentation: it happens inside a laboratory, for the
duration of a run, and never as an ambient sampler (B30, D5).

### D12 — Import is convenience; the correction is a byproduct

Handing MCF an identifier and getting that exact configuration running is the
whole of §XV's obligation. Nothing about it is conditional on measuring
anything: paste, resolve, host, done.

Where diagnostics *do* run and change something, the change is recorded — the
identifier imported, the configuration as declared, the configuration after
correction, which probe forced each change, and both hardware profiles. Same
starting point, different hardware, observed delta, and it costs nothing because
the user was running diagnostics anyway.

**This is a byproduct and is never allowed to become the point.** It may not
gate an import, slow one, or make one conditional. A user who imports a
configuration, hosts it and never measures anything has used §XV exactly as
intended.

### D13 — Four tiers, and the ordering is a correctness requirement

Work on a model proceeds in tiers, cheapest first:

| Tier | Establishes | Cost | Gate |
|---|---|---|---|
| **Smoke** | It loads and emits a token | seconds | none |
| **Calibration** | The configuration is *right* — template, stop conditions, tool format, usable context, sane sampling | minutes | none |
| **Characterization** | What it costs here — throughput, latency, memory, energy, thermal | tens of minutes | none |
| **Evaluation** | What it is good at — the behaviour laboratories | hours to days | verified capability (§3.23) |

**The ordering is forced, not chosen.** §X holds that a misconfigured model is a
measurement error, so an evaluation run on an uncalibrated configuration spends
a day measuring the misconfiguration rather than the model. Calibration is cheap
and protects everything downstream, which makes running it first a correctness
property rather than a courtesy.

Two further consequences. Calibration does not merely *detect* (§3.18); it
*adjusts*, and the adjusted configuration is what later tiers measure — with
every adjustment carrying the provenance of the probe that forced it (B10).
And characterization feeds D14: a machine's measured rate is what turns a
laboratory's work estimate into a duration.

### D14 — Runs are bought with time, and time is estimated from what we have measured

Nobody runs everything. The full product of models, configurations,
laboratories and trials is unbounded, so **selection is a first-class feature
rather than an afterthought**, and the interaction is inverted: the user spends
a budget rather than picking a list.

- **A laboratory declares its work**, in units it can count — trials, sweep
  points, tokens to generate, documents to process — not in minutes, which it
  cannot know.
- **The machine supplies the rate**, from the characterization tier (D13). Work
  × rate is a duration, which is why the tiers run in that order.
- **The estimate is a band and never a point** (§3.4), it is an *estimate*
  (A20) and can never be mistaken for a measurement, and where no local history
  exists MCF says so rather than guessing — a corpus prior may fill the gap,
  labelled as such (§6.38).
- **Every estimate is scored against what actually happened**, and the error is
  tracked. A laboratory whose estimates are consistently wrong is a finding
  about that laboratory, and an approximator whose error grows is a finding
  about the approximator.
- **A budget produces a proposal, not a silent truncation.** Given two hours MCF
  states what it will run, what it is leaving out, and why — because a selection
  that quietly drops work reads as coverage it never had (§3.1).
- **Results are anytime.** A laboratory reports as it goes, so a run stopped
  early keeps what it produced, marked incomplete (§3.1). This is what makes a
  laboratory measured in days usable by somebody with an afternoon.

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

### 3.3 The record is obligatory; ambient telemetry is not

Two ideas hide under the word "observability" and they are not the same thing.
Keeping them separate is what allows §VII to be honoured without damaging §II:

- **The record** is the durable evidence attached to things that happened: the
  conditions of every measurement (§3.4), the classification and context of
  every failure (§3.1), the provenance of every artifact (§3.6), and the
  configuration in force when each occurred. The record is **not optional and
  not reducible.** It is small, it is written at moments that matter rather than
  continuously, and §II is built directly on top of it. A benchmark number
  without its conditions is not data; a failure without its context is the
  silent failure §3.1 forbids.
- **Ambient telemetry** is continuous observation of a running system —
  high-frequency sampling, always-on tracing, metric streams, the machinery of
  operational monitoring. This is **deliberately de-prioritized.** It is the
  largest source of permanent idle cost in a daemon (§3.13), its value is
  largely diagnostic, and §VIII supplies that diagnostic value more cheaply and
  more rigorously.

The spirit of what remains:

- Record at **events**, not on a **timer.** Something happening is a reason to
  write; time passing is not. A background sampler that runs whether or not
  anything is occurring is the default suspect under §3.13.
- Records are structured and machine-readable first, human-readable second.
  Prose logs are a rendering of the record, never the record itself.
- Everything that varies *and could change a result* is recorded: hardware
  state, thermal conditions, driver and runtime versions, quantization, context
  length, batch shape, MCF's own version and configuration. This list does not
  shrink under §VII — it is the §3.4 floor, and it is captured deliberately at
  measurement time rather than harvested from a continuous stream.
- Verbosity above that floor is a dial the user controls, defaulting low.

What is given up here is the ability to answer arbitrary retrospective questions
about a running system. That is a real loss, accepted knowingly in §6.15.

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
  synthetic model artifacts, fake hubs, and replayed conditions are all
  legitimate and expected. A lab has calibration rigs; so do we. Under §VIII this
  is the project's primary means of knowing anything, not a supporting practice
  (§3.17).
- **Coverage is whole-system, not merely unit-level.** A suite that proves every
  function correct in isolation and never exercises the daemon end to end has
  tested the parts and not the thing. The behaviours that matter here —
  supervision, recovery, contention, degradation — exist only between
  components.
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
defended explicitly. Under §VII, *performance* supplies a second stream of such
reasonable decisions — caching, adaptive behaviour, skipped validation —
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

### 3.17 The laboratory is how we know things

MCF's confidence in itself comes from **reproducing conditions deliberately**,
not from watching itself in production. Every environment MCF must cope with —
hardware it does not have, a hub that misbehaves, a disk that fills, a runtime
that dies mid-token, a thermal ceiling, a network that stalls at 90% — is
something the test apparatus can *construct on demand*, deterministically, on a
laptop, in seconds.

This is what makes §I's "never fail" a claim rather than a hope. A failure path
that has never been exercised is a guess about how the system behaves, and §3.5
already says untested error handling is decorative. The laboratory is how every
one of those paths gets exercised without waiting for the world to supply the
conditions.

The spirit:

- **Every failure MCF claims to handle has a simulation that produces it.** The
  failure taxonomy (§7.10) and the lab's fault catalogue should be the same
  list, checked against each other. A category with no simulation is an untested
  claim.
- **Determinism is a feature of the lab, not of the world.** Simulated time,
  simulated hardware, injected faults, and replayable scenarios exist so that a
  failure found once can be reproduced exactly, forever.
- **Diagnosis happens by reproduction.** When something goes wrong in real use,
  the goal is not to have already logged enough to explain it — that is the
  telemetry strategy §3.3 declines — but to reconstruct it in the lab from the
  failure record and pin it with a permanent scenario. The bug becomes a lab
  fixture before it becomes a fix.
- **The lab is production code.** It is held to the same standards as everything
  else, because everything else is believed on its authority. A sloppy simulator
  produces confident wrong results exactly as §6.1 warns.
- **The lab does not get to grade itself.** See §6.16 — simulated confidence is
  worth only as much as its fidelity to real hardware, and that fidelity must
  itself be measured.

### 3.18 Capabilities are measured, not believed

§3.6 forbids inferring provenance. Intent X appears to require inferring
capabilities. The resolution that keeps both is to **stop guessing and start
testing**: what a model can do is established by *asking it to do the thing and
observing the result*, not by trusting a config field, a filename, a model card,
or a family resemblance.

This is the right posture independently of the conflict, because hub metadata is
frequently absent, stale, copy-pasted, or wrong, and §3.7 already says the hub is
untrusted input. A chat template that claims tool support proves nothing; a model
that emits a well-formed tool call proves something.

The spirit:

- **Three states, never confused: *declared*, *verified*, and *unknown*.**
  Declared is what the artifact claims. Verified is what MCF observed. Unknown is
  unknown, and is recorded as such rather than filled with a plausible default —
  the §3.6 rule, applied to a new category.
- **Divergence between declared and verified is a finding**, recorded and
  surfaced. It is often the most useful thing MCF can tell a user about a model.
- **Capability probes are experiments** and inherit §3.4 whole: a method, a
  result, conditions, and a record. They are cheap, bounded experiments, but they
  are experiments.
- **Configuration derived from a capability carries that capability's
  provenance.** If MCF set a parameter because a probe said so, the parameter
  knows that, and the user can ask.

### 3.19 The benchmark should resemble the work

A measurement's validity comes from its relationship to the thing the user
actually cares about. MCF exists to help someone choose a model to *use*, so its
evaluations should look like use: multi-turn, tool-calling, instruction-bound,
format-constrained, error-prone, and scored on whether the task got done.

This is also the answer to the contamination problem §3.4 raises. A task with a
*checkable outcome* — the tool call parsed, the file was written, the value
matched, the loop terminated — needs no ground-truth corpus to grade and is far
harder to have memorised than a multiple-choice benchmark. Verifiable tasks are
worth more to this project than famous ones.

The corollary is a warning: resemblance is not the same as reality, and an
agentic suite is still a proxy. §6.17 records what that proxy costs.

### 3.20 Publication is irreversible, so it is deliberate

Every other destructive act in this document is bounded: an evicted model can be
re-fetched, a deleted record was a copy, a wrong configuration can be replaced.
**Publication is the one act MCF cannot undo.** A row that leaves this machine is
gone in the sense that matters — it may be copied, indexed, aggregated and
retained by people who never asked and cannot be reached.

The spirit:

- **Contribution is opt-in, per share, and never a side effect.** No feature is
  ever enabled in a way that starts sending. There is no telemetry that is on by
  default and no "help us improve" default.
- **The user sees exactly what leaves, before it leaves.** Not a description of
  the categories; the rows.
- **What can be inferred is part of what is sent.** A hardware profile, a set of
  model choices and a timestamp identify a machine and often a person. Treating
  a field as harmless because it is not a name is the reasoning that makes
  de-identification fail.
- **Irreversibility is stated at the moment of the decision**, not buried in a
  document the user will read later, because §3.11's deliberation requirement
  means nothing if the user does not know the act cannot be taken back.

### 3.21 A configuration from elsewhere is a claim, not a result

§3.18 established that a *capability* is measured rather than believed. §XV
introduces the same problem one level up: a configuration arriving by identifier
carries a claim — *this works, and works well* — that this machine has not
tested.

The spirit is identical, and so is the answer. An imported configuration is
**declared** until this machine verifies it: MCF reproduces it exactly, says so
plainly, and treats every number attached to it as somebody else's until it has
taken its own. A configuration that cannot be reproduced here is a *finding*,
recorded and surfaced — often the most useful thing the exchange can tell
anyone, because it is evidence about how far a result travels between machines.

### 3.22 Instrumentation is scoped to the question it serves

A laboratory instruments as deeply as its question requires, and no more — and
that instrumentation exists **inside the lab, for the duration of the
experiment.** It does not leak into the serving path, does not run when no
experiment is running, and does not become the daemon's ambient condition.

Two consequences that keep §XIII from re-opening what D5 settled:

- **Deep instrumentation is a property of an experiment, not of MCF.** The idle
  daemon does nothing (§3.13) whether MCF carries three labs or thirty.
- **Instrumentation is a measurement condition.** A result records the profile
  it ran under, because a heavily instrumented timing and a lightly instrumented
  one are different measurements (§3.4, §6.2) — and where the question is a
  *timing*, the profile is reduced and the observer effect is characterized
  rather than hoped away.

### 3.23 A model is measured only where it is capable; not applicable is not zero

§X establishes what a model can do. §XIII builds laboratories that measure how
well it does those things. The rule that joins them: **a laboratory runs against
a model only where the capability it depends on has been verified present, and
reports *not applicable* everywhere else.**

A model with no tool-calling capability scoring 4 % on an agentic suite has not
been measured badly; it has not been measured at all. Publishing that 4 %
alongside another model's 71 % is a comparison between a measurement and an
artefact of the wrong instrument — precise, reproducible, and meaningless.

The spirit:

- **Four outcomes, never three.** *Measured*, *not applicable* (the capability
  is verified absent), *unknown* (§X could not establish it), and *failed* (the
  capability is present and the model did badly). Collapsing the middle two into
  the last is the error this principle exists to prevent.
- **Not applicable is information.** "This model cannot do this" is a §3.4 null
  result and is reported as one, because a user choosing a model for extraction
  is well served by knowing it has no vision and does not need one.
- **No profile is complete, and completeness is not the goal.** A model measured
  in four laboratories and inapplicable to six has a four-laboratory profile.
  MCF states the coverage rather than implying the absent measurements were
  losses.
- **A user's declared workflow decides which laboratories matter**, and MCF says
  when it has no evidence about the thing the user cares about — which is more
  useful than evidence about six things they do not.

### 3.24 Absence of evidence is not evidence of absence

Once MCF holds a corpus (§XIV), a new failure mode becomes available to it: the
confident negative drawn from silence. *"No successful reports on hardware like
yours"* is a statement about who has bothered, not about what is possible.
Reporting follows popularity, novelty and enthusiasm — never coverage.

The spirit:

- **Unreported and unsupported are different words, and MCF uses the right one.**
  A corpus-derived statement says what the corpus contains, never what reality
  permits.
- **A corpus statement carries its sample.** "Two reports, both failures" and
  "four hundred reports, all failures" are different claims and read
  differently. A statement resting on nothing says so.
- **Nothing is hidden on corpus grounds.** An unreported option is ranked lower,
  annotated, and still reachable. Quietly removing it is §3.1's silent omission
  arriving through the recommender instead of an exception handler.
- **The corpus's own bias is a stateable property.** Which hardware is
  over-represented and where the evidence thins out are answerable questions,
  and they belong beside the answers drawn from it.

### 3.25 Reversibility is the boundary of autonomy

MCF is permitted to improve things, and §XVII permits it to reach outside its
own process to do so. The line that keeps that from becoming licence is not a
list of allowed actions; it is a property of the action:

- **What MCF owns, it changes freely.** Its own configuration, its own process,
  the parameters of a model it is serving. No permission is needed to choose a
  quantization.
- **What MCF can restore, it may change with permission**, and it restores it —
  always, including after a crash. A performance governor, a process's run
  state, a scheduling priority, an exclusive device mode. Each such change is
  recorded, is a measurement condition (§3.4), and is reversed when the work
  that needed it ends.
- **What MCF cannot restore, it does not change.** Installing, uninstalling,
  upgrading a driver, editing another program's data, terminating a process
  holding unsaved work, modifying weights. Irreversibility is the disqualifier,
  and no argument about benefit overrides it.

The practical test is a question with one right answer: *if this run were
interrupted at the worst possible moment, could the machine be returned to how
it was found?* Where the answer is no, the action is out of scope regardless of
how much faster it would make the measurement.

This is also why the stop control (§3.1, §6.33) is not a convenience: a system
that alters its environment owes an unwind path, and a system that cannot unwind
should not have altered anything.

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
- **Observation costs performance.** Resolved rather than merely managed — see
  §6.9 and D5. The residual tension is §6.15: reduced observation costs
  retrospective diagnosis.
- **Good evaluation is expensive evaluation.** Agentic benchmarks are long,
  multi-turn, and must be repeated to mean anything. There is no cheap version
  that is also honest; §6.17 manages the cost, and §4's rigor-costs-time rule
  applies with more force than anywhere else in the document.
- **Simulated confidence is not real confidence.** §VIII buys determinism and
  breadth at the price of fidelity. §6.16 manages this; nothing abolishes it.
- **Ease costs transparency.** Every step removed from the user's path is a step
  they no longer see. §3.15 manages this; it does not abolish it.
- **Sharing costs privacy, and no amount of care abolishes it.** A record
  detailed enough to be scientifically useful is detailed enough to identify the
  machine that produced it. §3.20 and §6.27 manage this; nothing eliminates it,
  and the only complete protection is not contributing.
- **Breadth of analysis costs lightness.** Every laboratory is weight, and §XIII
  asks for many. §6.26 makes each pay for itself in validity, which slows their
  arrival deliberately.
- **Lightness costs features.** This is the intended cost, not a regrettable
  one. A tool that keeps every feature proposed to it cannot also be the
  lightest thing it could be, and we would rather be light.

---

## 5. Anti-Goals

Stating what MCF is *not* protects the intents above from dilution.

- **Not a training or fine-tuning platform.** MCF deploys and evaluates weights;
  it does not produce them. Quantization and conversion are in scope as
  deployment transformations, not as model development.
- **Not a leaderboard.** MCF measures *this* machine, and no number measured
  on another one may choose a configuration on this one (§6.28). MCF
  *contributes* conditioned results outward under §XIV and consumes none of the
  aggregate as authority. It does not compute, host or display a ranking.
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
  only where a stated intent demands it. §XIII's laboratories are built in-tree,
  one at a time, each admitted for the validity it adds (§6.26) — a range of
  labs is not an ecosystem of them.
- **Not an agent framework.** MCF runs agentic tasks to *measure models*, not to
  help users build agents. The harness is an instrument (§6.18). Every feature
  that would make it a better agent platform and not a better measuring device
  is out of scope by construction.
- **Not an observability platform.** MCF does not observe *itself* beyond the
  record §II requires: no dashboards, no metric streams, no trace backends, none
  of the apparatus of production monitoring (D5). Deep instrumentation of the
  *model under test*, inside a lab, for the duration of an experiment, is a
  different thing and is the product (§XIII, §3.22). The distinction is the
  subject: MCF is not the specimen.
- **Not the website.** §XV names an external site that aggregates contributed
  results and helps people choose. MCF does not build it, host it, depend on it
  being reachable, or degrade in usefulness without it. MCF's obligations are
  two: emit an identifier for a configuration it holds, and reproduce a
  configuration from an identifier it is given.
- **Not a data broker.** MCF holds the user's record on the user's machine and
  contributes only what the user sends, once, deliberately (§3.20). It does not
  collect, does not aggregate other users' data locally, and has no interest in
  the user beyond the measurement.
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

**Confidence: high on the principle, medium on the mechanism.** Making the paths
separable without two divergent code paths — the classic source of "it works in
benchmark mode" bugs — is a real design problem. D5 shrinks it considerably:
with ambient telemetry refused by default, the gap between operational and
measurement instrumentation is small, because the operational path is already
quiet. The two profiles are near enough to converge, which is the cleanest
available answer to this conflict.

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

### 6.9 Lightness vs. deep telemetry

**Tension.** §I asks for pervasive instrumentation of everything; §VII holds
that every cycle MCF spends is taken from the model.

**Resolution — §VII wins on the continuous-observation axis, and §VIII supplies
the confidence telemetry would have bought.** Stated in full as **D5** in §2.1,
including what the decision explicitly does not touch: the record, the
prohibition on silent failure, and the §3.4 floor.

**Confidence: high.** The two things traded — production observation and
laboratory reproduction — buy the same good, and the lab buys it
deterministically and before release. The costs are §6.15 and §6.16.

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

### 6.15 Reduced observation vs. "it should never fail"

**Tension.** §I demands that MCF cope with everything and always be able to say
what happened. The conventional way to honour that is deep production
telemetry, and D5 declines it. When something goes wrong on the user's machine
in a way the laboratory did not anticipate, MCF has less to look at than a
heavily instrumented system would.

**Resolution — diagnosis moves from *observation* to *reproduction*, and the
failure record is the bridge between them.** The failure record (§3.1) is
explicitly not what was reduced: classification, context, configuration, and
the conditions in force are all still captured at the moment of failure. What
MCF gives up is the *surrounding stream* — the ability to ask arbitrary
retrospective questions about what the system was doing for the ten minutes
beforehand.

The strategy that replaces it: a failure record rich enough to **reconstruct the
scenario in the lab**, where it can be reproduced deterministically, examined
with unlimited instrumentation at zero production cost, and pinned with a
permanent test (§3.17). Observation is expensive and always-on; reproduction is
free and on demand.

This imposes a genuine requirement rather than an aspiration: **the failure
record's sufficiency is measured by whether the lab can rebuild the failure from
it.** When it cannot, that is a defect in the record, and the fix is more context
at the failure site — never a return to ambient streaming.

**Confidence: high on the strategy, medium on the residual risk.** The honest
cost is stated in §6.16: failures arising from conditions nobody thought to
simulate are exactly the ones this strategy handles worst, and they are also the
most interesting ones. This is the price of §VII, knowingly paid.

### 6.16 Simulated confidence vs. reality

**Tension.** §VIII rests MCF's credibility on a laboratory, and a laboratory is
software someone wrote. **A simulator built from our own assumptions tests our
assumptions, not the world.** Real GPUs fail in ways nobody models; real drivers
have undocumented behaviour; real thermal throttling is messier than any curve
we would write. A green suite against a simulated universe can produce total
confidence and zero validity — which is precisely the §6.1 failure mode, arrived
at from a new direction.

Combined with §6.15, this is the sharpest residual risk in the project: MCF is
choosing to know itself through a model of the world rather than through
observation of the world, and it must not fool itself about the difference.

**Resolution — the lab establishes correctness; only real hardware establishes
belief, and the two are never conflated.**

- **Simulated tests gate the code.** They are deterministic, exhaustive, fast,
  and must be green. They prove MCF behaves correctly *given* the conditions
  described.
- **Real-hardware runs validate the lab.** Periodically, and on every substantive
  change to a simulated component, the simulation's predictions are checked
  against the real thing. A simulator whose fidelity is unmeasured is an
  unqualified instrument, and §3.4 already forbids trusting one of those.
  Divergence is a finding about the simulator, and it is recorded.
- **No performance number ever comes from simulation.** §IV's measurements are
  taken on real hardware, always. The lab tests *behaviour*, never *speed* —
  simulated timings are fiction, and publishing one would violate §6.1 outright.
- **Reality outranks the lab.** When they disagree, the world is right and the
  simulator is defective. That direction is never reversed to preserve a green
  suite.
- **Fidelity is bounded and stated.** The lab should be explicit about what it
  does *not* model, so that confidence is claimed only where it was earned.

**Confidence: high on the ordering, low on the sufficiency.** How much real
hardware validation is enough, and how often, is genuinely unresolved and
recorded as §7.20. It is the question that determines whether §VIII is rigor or
theatre.

### 6.17 Agentic benchmarking vs. "isolate the variable" and reproducibility

**Tension.** §3.4 demands isolated variables, repeatability, and stated
uncertainty. Agentic workflows are the least isolated, least repeatable
measurement available: multi-turn runs compound variance at every step, a single
early misstep changes everything downstream, tool environments carry state, and
success is often bimodal rather than normally distributed. A single agentic run
tells you close to nothing, and averaging a handful of them can be worse than
useless because the distribution is not the shape averaging assumes.

**Resolution — agentic results are *distributions*, never scores, and the
environment is pinned even though the model is not.**

- **Sample counts are large enough for the shape of the outcome, and the shape
  is reported.** Success rate over n trials with its spread — not a number. §3.4's
  "uncertainty is mandatory" rule is doing real work here, not ceremony.
- **Everything except the model is held still.** The task, the tool
  implementations, the environment's starting state, the seeds where seeding is
  possible, the harness version, the sampling parameters. This is where §VIII
  pays off unexpectedly: the laboratory MCF already needs (§3.17) is the same
  machinery an agentic environment requires — deterministic, constructible,
  replayable. **The agentic benchmark environment and the test laboratory should
  be the same apparatus**, and it would be a mistake to build two.
- **Irreducible stochasticity is reported, not engineered away.** Temperature
  zero is not a fix; it is a different, less representative experiment. If two
  models are within noise, §3.9 already requires MCF to say so rather than
  manufacture a ranking.
- **A failed agentic run is data** (§3.4), and *how* it failed — wrong tool,
  malformed call, loop, early stop, gave up — is more informative than the pass
  rate. Failure taxonomy (§7.10) applies to models under test, not only to MCF.

**Confidence: high on the framing, medium on the statistics.** How many trials,
and what test distinguishes a real difference from noise, is a genuine
statistical question this document cannot answer by assertion. Recorded as
§7.23.

### 6.18 An agentic harness vs. lightness and "not a platform"

**Tension.** §IX requires MCF to run agents: a loop, a tool registry, tool
implementations, an execution environment, a scoring layer. That is a substantial
subsystem, and §5 says MCF is not a platform while §3.13 says every
generalization is weight.

**Resolution — the harness is an instrument, and instruments are built to the
minimum that makes the measurement valid.** It exists to exercise models under
controlled conditions, not to be useful for building agents. The test is
directional and should be applied ruthlessly: *does this make the measurement
more valid, or does it make the harness more capable?* Only the first justifies
weight.

Two structural consequences: it shares the §VIII laboratory rather than
duplicating it (§6.17), and under §3.13's idle rule it costs nothing when no
benchmark is running — a benchmark subsystem that consumes resources during
ordinary serving would be the worst kind of weight this document knows how to
describe.

**Confidence: high.** The failure mode here is well understood and easy to name:
harnesses of this kind grow into frameworks. §5's new anti-goal exists to make
that growth require an argument.

### 6.19 Automatic configuration vs. "provenance is never inferred" and "no hidden choices"

**Tension.** §X wants MCF to work out what a model can do and set it up
correctly, unattended. §3.6 forbids filling unknown metadata with plausible
values. §3.15 forbids consequential choices the user cannot see. Naively, "detect
and configure automatically" is exactly the inference §3.6 prohibits — and it
would contaminate the record, because a guessed capability that looks like a
known one corrupts every measurement taken under it.

**Resolution — §3.18: detection is *measurement*, not inference, and
configuration is *derived, recorded, and overridable*.** MCF may configure
anything automatically provided it can say, for every setting, whether the value
came from a declaration, from an observation, or from a default — and which
probe, if any, established it.

This is the reading that satisfies both intents rather than trading one off:
empirical capability detection is *more* rigorous than trusting metadata, not
less, so §X and §3.6 turn out to be allies once "identify" is read as "test"
rather than "assume." Where a capability cannot be established either way, it is
unknown, MCF says so, and it does not quietly pick a value that makes the model
appear to work.

**Confidence: high on the principle, medium on the cost.** Probing every
capability of every model is not free, and §7.24 records the unresolved question
of when probes run and how their cost is bounded.

### 6.20 Executing agentic tool calls vs. "the hub is untrusted"

**Tension.** §IX requires that a model under test emit tool calls which are then
*executed*. The model is an artifact of unknown quality fetched from an untrusted
source (§3.7), and it produces instructions that MCF acts on. Whether the
model is malicious or merely bad barely matters — an incompetent agent deleting
files is the same outcome as a hostile one.

**Resolution — the benchmark environment is a sandbox by construction, and no
benchmark tool ever touches anything real.** Tools available to a model under
test operate on constructed, disposable state within the §VIII laboratory. No
benchmark tool reaches the user's filesystem, the network, MCF's own records, or
the serving path — not by policy or by configuration, but because those
capabilities are not present in the environment to begin with.

This is a case where the strict answer is also the cheap one: §6.17 already
requires a pinned, reconstructible environment for validity, and a pinned
environment is inherently a contained one. **The rigor requirement and the safety
requirement have the same implementation**, which is the strongest possible
argument for it.

Note the §6.4 distinction still applies and is separate: *loading* a model may
require executing repository code, which is the user's explicit decision.
*Benchmarking* a model executes the model's outputs, which is never a decision
the user should have to think about, because the answer is always the sandbox.

**Confidence: high.** Any weaker answer makes §IX unsafe to run unattended, and
an evaluation suite that cannot run unattended will not be run.

### 6.21 A UI and a headless mode vs. "the interface is a window"

**Tension.** Minimal, since §3.14 and §6.11 already resolved most of it — but §XI
phrases headless as a *mode*, and this document has held that headless is the
*base case* and the interface an optional attachment. The difference matters:
"mode" implies two supported configurations that could diverge, which is how
interfaces quietly acquire exclusive features.

**Resolution — one system, two equally complete access paths, with the headless
path primary by construction.** The service is the system (§3.14). Both surfaces
are first-class in *capability*, and neither is degraded — that is §XI's real
requirement and it is accepted. But the ordering is not symmetric: **the
interface may not be the only way to do anything.**

Concretely: every action is available without a display attached; the interface
is a client of the same interface a script would use, which under §VII is also
the cheapest possible implementation; and a feature that would be awkward to
express headlessly is a design problem to solve, not a reason to make it
visual-only.

**Confidence: high.** It is also self-enforcing in a way worth noting: §VIII's
whole-system tests exercise MCF headlessly, so a capability reachable only
through the interface is a capability the laboratory cannot test — which §3.5
already forbids.

### 6.22 A reference model vs. "any model" and the hermetic suite

**Tension.** §XII names one artifact as the subject of early work. Two rules
push back. §III and §6.3 commit MCF to accepting *any* hub reference without
special-casing, and a system developed against one model acquires quiet
dependencies on its format, its family and its template without anyone
deciding to. §3.5 is sharper still: tests must not require a GPU, a network or
a large model, and a 27B fixture is precisely the dependency that rule forbids.

**Resolution — the reference model is a fixture for the *instrument*, never a
dependency of the *suite*, and never a case in the code.**

- **The suite stays hermetic.** Unit, whole-system and laboratory tests run on a
  laptop, offline, with no accelerator and no real weights, against synthetic
  artifacts and the simulated hub (§3.17). Nothing in §XII changes that, and a
  test that cannot run without downloading 27B of weights is a defect regardless
  of how convenient it was to write.
- **Where the reference model is legitimately used:** benchmarking (§IV),
  capability probing (§X), agentic evaluation (§IX), real-hardware validation of
  the laboratory (§6.16), and the §7.19 substrate prototype. All of these are
  activities that require real weights by their nature. None of them gate a
  build.
- **No special-casing, ever.** A code path that behaves differently because an
  artifact happens to be the reference model is a defect, not an optimization.
  The honest test is mechanical: substituting a different model must change what
  is measured and nothing about how MCF behaves.

**Confidence: high.** The distinction it rests on — §6.7's separation of tests
from benchmarks — is already load-bearing elsewhere in this document, and this
is the same line drawn through a new question.

### 6.23 One reference model vs. generalization

**Tension.** §IV exists to tell a user which model to run, which is a claim
about models in general. §3.4 warns specifically about MCF's own tendency to
tune toward whatever it measures. Develop the whole instrument against one
artifact and two failures follow: MCF becomes correct about that model rather
than correct in general, and nobody notices, because the only thing measuring
MCF is the thing MCF was built around.

**Resolution — one model is enough to build an instrument and never enough to
generalize from, and MCF must say which it is doing.**

- **Instrument development is single-model work and is honest about it.** During
  early milestones, results measured on the reference model characterize *the
  instrument* — that it records conditions, classifies failures, reproduces
  runs. They characterize the model only incidentally.
- **No §IV recommendation is made from a single model.** A recommendation
  requires alternatives by construction; §3.9's frontier has one point until a
  second model exists, and a frontier with one point is not a frontier. MCF
  should refuse rather than rank a field of one.
- **Breadth is a prerequisite for a generality claim, and its extent is
  unresolved** — recorded as §7.26.
- **The §3.4 anti-overfitting rule applies to MCF itself.** Tuning a default
  until the reference model looks better is training on the test, whoever is
  doing the tuning and however reasonable each individual adjustment seemed.

**Confidence: high on the principle, medium on the discipline.** This is the
resolution most likely to be violated accidentally rather than deliberately,
because every individual act of tuning toward the one model in front of you is
locally sensible. It is the §3.12 failure mode — erosion by a hundred small
reasonable decisions — pointed at a new target.

### 6.24 Deep per-lab telemetry vs. the decision to refuse ambient telemetry

**Tension.** D5 refuses continuous observation and makes "record at events, not
on a timer" the rule. §XIII asks for deep telemetry, custom-built per lab. Read
naively, the newer intent reverses the older decision, and reversing it silently
is exactly how a project acquires the idle cost §VII exists to prevent.

**Resolution — telemetry inside a laboratory is not ambient telemetry; it is the
experiment.** The two are distinguished by *when they run* and *what they watch*,
and both distinctions are absolute:

- **When.** Lab instrumentation exists for the duration of an experiment the
  user started. D5's rule — an idle daemon does approximately nothing — is
  untouched, and carrying thirty labs must cost exactly as much at idle as
  carrying none (§3.22).
- **What.** Ambient telemetry watches *MCF*. Lab telemetry watches the *model
  under test*. MCF is not the specimen, and the anti-goal in §5 now says so
  explicitly.
- **The serving path never carries it.** A lab may instrument an inference run
  it owns; it may not instrument the endpoint a user's application is talking to.

**Confidence: high.** This is a real distinction rather than a semantic escape:
the cost D5 refused was permanent and unattributable, and the cost §XIII asks
for is bounded, attributable to a run, and paid only by someone who asked for a
measurement.

### 6.25 Deep instrumentation vs. the validity of what it measures

**Tension.** §6.2 already holds that instrumentation perturbs what it measures.
§XIII makes that worse on purpose: a lab designed to watch a model closely is a
lab that changes what the model's timings look like. Taken naively, the deepest
labs produce the least trustworthy numbers.

**Resolution — the instrumentation profile is part of the result, and
timing-class results are taken under a reduced one.**

- **Every result records the profile it ran under.** A heavily instrumented
  timing and a lightly instrumented timing are different measurements and are
  never compared (§3.4, A8).
- **Timing-class measurements run reduced,** and the residual overhead is
  characterized and reported. §IV's numbers are not permitted to come from a
  deeply instrumented run.
- **Behaviour-class measurements may instrument freely.** Whether a tool call
  parsed, whether the loop terminated, whether the model recovered from an error
  — none of these are perturbed by watching them, which is precisely why the
  deepest labs should be the ones asking behavioural questions.
- **A lab states which class it is.** A lab that cannot say whether its outputs
  are timing-class or behaviour-class has not been designed yet.

**Confidence: high on the split, medium on the boundary.** Some questions are
both — time-to-first-tool-call is a timing about a behaviour — and those need
the reduced profile and should say so.

### 6.26 A range of laboratories vs. "not a platform"

**Tension.** §XIII asks for many labs, each custom. §5 refuses a plugin
ecosystem and §6.18 holds that an instrument grows for validity and never for
capability. "A wide range of diagnostic tooling" is the exact phrase from which
extension APIs grow.

**Resolution — labs are in-tree, first-party, and admitted one at a time against
a stated question.** There is no lab API, no third-party lab, no discovery
mechanism and no configuration language for labs. Each lab is code in this
repository, held to every rule that governs the rest of it (§3.17), and each is
admitted by answering one question: *what claim can MCF make after this lab
exists that it cannot make now?*

A lab that makes MCF more capable of running experiments in general, rather than
capable of making a specific new claim, is refused — and the refusal is
recorded, so that the same proposal does not return as an oversight.

**Confidence: high on the rule, medium on the pressure it will take.** This is
the anti-goal most likely to erode, because each individual lab will look
obviously worth having.

### 6.27 Contribution vs. "the user's data and machine are theirs"

**Tension.** §3.10 holds that anything leaving the machine does so because the
user chose it, knowing what it contains, and that a tool managing local
inference while leaking its contents has betrayed the reason it was installed.
§XIV asks for data to leave.

**Resolution — the split §6.8 already requires is what makes contribution safe,
and the same implementation satisfies both.** The contributable database
contains no prompt or completion content *because content was never written to
it* (D6), not because an export filter removed it. A filter can be misconfigured;
a store that never held the data cannot leak it.

On top of that structural guarantee:

- **Contribution is opt-in, per share, never default and never a side effect**
  (§3.20).
- **The user sees the rows that leave, not a description of them.**
- **De-identification is treated as hard, because it is.** A hardware profile, a
  timestamp and a set of model choices identify a machine. What is stripped,
  coarsened or withheld is §7.27 and is unresolved.
- **Publication is irreversible and is stated as such at the moment of the
  decision** (§3.20).

**Confidence: high on the structure, low on the sufficiency.** The structural
guarantee is strong and cheap. Whether what remains is de-identified *enough* is
a genuine open question and is recorded rather than assumed away.

### 6.28 Crowd-sourced data vs. "not a leaderboard"

**Tension.** §5's anti-leaderboard rule exists because cross-machine rankings are
folklore: numbers taken on someone else's hardware, with someone else's
quantization, against benchmarks that do not resemble your work. §XIV asks MCF
to feed exactly such an aggregate, and §XV asks it to act on the result.

**Resolution — MCF contributes outward and decides inward, and the two never
cross.**

- **No foreign number ever chooses a local configuration.** §IV's
  recommendations are made from measurements taken here, on this machine, under
  §3.4's conditions. An aggregate may tell a user what to *try*; only a local
  measurement tells them what to *run*.
- **What MCF contributes is what leaderboards lack: the conditions.** A row
  without its hardware, driver, quantization, context, harness version and MCF
  version is not contributable, because it is exactly the kind of number this
  project exists to replace.
- **MCF neither computes nor displays a ranking.** Aggregation happens off the
  machine, in a system that is not part of this project.
- **The suspicion stands, and is now precise.** MCF is not suspicious of foreign
  *data*; it is suspicious of foreign *conclusions*. Conditioned observations
  from many machines are evidence. A ranking derived from them is somebody
  else's opinion.
- **Refined by §6.38.** The corpus may narrow what MCF measures locally. It may
  never supply a number MCF reports about this machine.

**Confidence: high.** This reading strengthens the original anti-goal rather
than weakening it: the reason cross-machine rankings are folklore is that they
travel without their conditions, and §XIV's entire contribution is to make the
conditions travel.

### 6.29 An identifier from outside vs. everything this document says about trust

**Tension.** §XV asks MCF to accept an identifier from an external source and
reproduce a configuration from it. That identifier is untrusted input (§3.7), it
asserts capabilities MCF has not observed (§3.18), it carries provenance MCF did
not witness (§3.6), and acting on it consumes bandwidth and disk and may execute
repository code (§6.14). Four rules point at it at once.

**Resolution — an identifier is a *request to reproduce*, and reproduction is
followed by local verification before anything is believed.**

- **Resolution is gated.** Importing an identifier downloads weights and may
  execute repository code, so it passes the same gates any acquisition does
  (§6.4, §6.14). Being pasted from a website earns it nothing.
- **The configuration is declared, never verified, on arrival** (§3.21). Every
  parameter it sets is attributed to the identifier, and every number that
  travelled with it is somebody else's measurement until MCF takes its own.
- **Exact reproduction is the obligation; identical results are not.** MCF
  reproduces the weights, quantization, context, runtime and parameters, then
  measures. If this machine produces different numbers, that is a *finding* —
  evidence about how far results travel — not a failure of the import.
- **Failure to reproduce is a first-class outcome** (§6.3): "this identifier
  names a configuration needing 48 GiB and you have 24" is a complete, useful
  answer.
- **An identifier binds a configuration, not a promise.** It is a name for a
  reproducible setup. Anything it claims about quality is a claim.

**Confidence: high on the posture, medium on the mechanism.** What an identifier
actually contains, and what makes one trustworthy enough to resolve, is §7.28.

### 6.30 Publishing results vs. keeping the suite uncontaminated

**Tension.** §3.19 and §7.3 hold that MCF's evaluations stay honest partly
because its tasks are not famous. §XIV publishes results from those tasks, and a
public corpus of task results is a map of the tasks. Contamination is not
hypothetical here — it is the predictable consequence of success.

**Resolution — contribute *outcomes*, never *artifacts*, and rotate what
becomes public.**

- **Task content is not contributable.** Scores, failure classifications,
  conditions and distributions leave; prompts, tool definitions, environment
  fixtures and model outputs do not.
- **The procedurally generated portion of the suite carries the weight**
  (§7.23): what is generated fresh per trial cannot be memorised from a
  published result, and its contamination-detection role (comparing generated
  against fixed instances) survives publication intact.
- **Contamination exposure is a property of a task and is recorded.** A task
  whose results have been published for long enough is retired or regenerated
  rather than quietly kept.

**Confidence: medium.** This is the newest and least tested of these
resolutions, and the mechanism — how long is too long, what rotation costs in
comparability — is unresolved and recorded as part of §7.23.

### 6.31 Self-contained vs. delegating inference, and vs. the weight budget

**Tension.** §7.4's likely answer is that MCF wraps mature inference engines
rather than implementing kernels, and D4 accepted Python conversion tooling as
supervised subprocesses. §XVI forbids making the user fetch either. Meanwhile
§VII asks MCF to be the lightest thing it can be, and shipping an engine — or
several, per accelerator vendor — is not light.

**Resolution — MCF ships what it needs, and §VII's budget governs *behaviour*,
not *download size*.**

- **The budget is about what MCF costs while running:** idle CPU, resident
  memory, cold start, interposed latency. A static binary that idles at nothing
  and starts in milliseconds honours §VII at 12 MiB or at 400 MiB. §7.16 must
  therefore budget installed footprint *separately* and generously, and say so,
  rather than letting one number pretend to govern both.
- **Delegation survives; the errand does not.** MCF may drive an engine it did
  not write. It may not require the user to obtain that engine.
- **Anything on the common path is vendored or reimplemented.** Where a
  transformation genuinely requires an external toolchain, the honest options
  are to vendor it, to reimplement the narrow part MCF needs, or to refuse the
  feature — never to emit an instruction to go install something.
- **Absent platform capabilities are not errands.** A missing vendor driver or
  absent hardware is stated, the dependent capability is marked unavailable, and
  MCF continues (§3.2).

**Confidence: high on the principle, medium on the cost.** How large the
artifact becomes when several accelerator backends ship together is unknown, and
if it becomes absurd the honest amendment is per-accelerator builds — still
self-contained, still no errand — rather than quietly asking the user to
install something.

### 6.32 Elevated privilege vs. untrusted code, network exposure, and the user's machine

**Tension.** §XVII asks for admin rights where they buy measurement validity.
§3.7 says the hub is hostile, §6.4 permits executing model repository code,
§6.12 permits network exposure, and §3.10 says the machine is the user's. A
privileged daemon that fetches code from the internet, runs model-supplied code,
and listens on a network is the most dangerous object this document describes.

**Resolution — privilege is per-operation, minimal, auditable, and structurally
unreachable from anything untrusted.**

- **The daemon does not run privileged.** Elevation belongs to a small, separate,
  auditable helper that performs a named operation and exits. MCF's long-lived
  process holds no ambient privilege, so a compromise of the control plane is
  not a compromise of the machine.
- **Nothing untrusted ever runs privileged.** Model repository code (§6.4) and
  models under test (§6.20) execute in the sandbox, unprivileged, always. There
  is no configuration that relaxes this, because there is no legitimate reason
  to want it.
- **The privileged surface is an enumerable list**, not a capability. Reading
  power and thermal counters, setting a performance governor, requesting
  exclusive accelerator access, locking pages, pinning cores — each is a named
  operation with a bounded effect, and the list is short enough to audit and
  short enough to publish.
- **Elevation is a gated category** (A16): asked, stated, recorded, and never a
  side effect. What it buys is stated in the same breath: *this reading is
  unavailable without it, and here is what MCF will report instead.*
- **MCF works without privilege, and says what it lost.** Degraded is a
  first-class state (§3.2). An unprivileged MCF is a less precise instrument,
  not a broken one, and every measurement it takes is marked accordingly.
- **Privilege and network exposure are never simultaneously implicit.** Exposing
  the control plane (§6.12) while privileged operations are available is the
  worst combination available, and it requires its own deliberate act.

**Confidence: high on the structure, medium on the mechanism.** Which operations
truly require elevation differs by platform and is recorded as §7.39; a helper
that turns out to need broad rights for a narrow job should be reconsidered
rather than granted.

### 6.33 Exclusive laboratories vs. a persistent, dependable endpoint

**Tension.** D8 gives a laboratory the whole machine. §VI promises a persistent
local service and §I promises MCF is the calm component. A user whose endpoint
stops answering because a lab started has been failed by both.

**Resolution — suspension is a declared state, never an outage.**

- **The daemon stays up and keeps answering** — about itself. A request arriving
  during a lab receives an immediate, explicit refusal naming the lab, the
  reason, and the expected remaining time. It is never queued into a timeout,
  never silently slowed, and never dropped.
- **Starting a lab while serving is a decision the operator makes**, with what
  will be suspended stated before it begins.
- **Bounded by construction.** A lab declares a maximum duration and is stopped
  if it exceeds it, because an unbounded suspension is indistinguishable from an
  outage.
- **Interruption is allowed and honest.** The operator may stop a lab to reclaim
  the machine; the partial result is preserved and marked incomplete (§3.1),
  never discarded and never reported as complete.

**Confidence: high.** The alternative — labs sharing with serving — buys
availability by destroying the validity the lab exists to produce, which is
§6.1's trade in a new costume.

### 6.34 Exhaustive application testing vs. a suite that runs in seconds

**Tension.** §3.5 requires the suite run on a laptop, offline, fast enough to
run constantly. D10 asks for load, soak, mutation and full fault-matrix testing,
none of which is fast. Mutation testing in particular is quadratic-feeling work:
it re-runs a suite once per mutant.

**Resolution — tier the suites, gate on the fast one, schedule the heavy ones,
and report the age of every tier.**

- **The gating tier stays hermetic and fast** and is what §3.5 was describing.
- **Heavy tiers run on a schedule and before every release**, and their results
  carry a timestamp. A tier that has not run recently is reported as stale, not
  assumed green — an unstated staleness is the silent failure §3.1 forbids,
  aimed at the suite instead of the system.
- **Mutation score is a tracked quantity with a floor**, treated like any other
  budgeted property (§3.13): it may not regress silently.
- **Load and soak run against the simulated laboratory**, not against real
  weights, so they remain cheap enough to run often and deterministic enough to
  believe (§3.17).

**Confidence: high.**

### 6.35 Measuring power vs. the observer effect and platform variance

**Tension.** D11 wants energy recorded. Reading power counters means polling,
polling is the ambient sampling D5 refused, and on some platforms the "reading"
is a vendor's model rather than a measurement.

**Resolution — power is sampled inside a laboratory only, at a declared rate
recorded as a condition, and its provenance is stated.**

- **Only during a run**, never as a background sampler (B30, D5). Idle MCF reads
  no counters.
- **The sampling rate is a measurement condition** (§3.4), because a 10 Hz
  sample and a 1 Hz sample produce different energy integrals.
- **Measured, estimated, or unknown** (A20, A7). A vendor's modelled figure is
  recorded as an estimate and never presented as a reading, and a platform with
  no interface yields `unknown` rather than a number derived from utilization.
- **The cost of sampling is characterized** like any other instrumentation
  overhead (§6.2), and energy figures taken under a heavy profile are not
  compared with those taken under a light one (A8).

**Confidence: high on the rule, low on cross-platform comparability.** Whether
an NVIDIA board-level reading and an Apple package-level reading can ever be
compared is genuinely unclear, and the honest default is that they cannot.

### 6.36 Plural quality vs. a user who wants an answer

**Tension.** D2 refuses to produce a score, and §VI asks MCF to be frictionless.
A user facing eleven laboratories and a coverage table has been handed a
research project, not an answer, and §3.15's "fewer decisions, not hidden ones"
cuts against making them assemble the verdict themselves.

**Resolution — MCF ranks against a *declared workflow*, never in general.**

- **The user says what they do**, or accepts a visible default (§6.5). That
  declaration selects the laboratories that matter and the weights among them.
- **Within a declared workflow, MCF answers plainly**: this configuration, this
  reasoning, these runners-up (§3.9). The friction §VI refuses is being forced
  to interpret raw laboratory output, and this removes it.
- **Across workflows, MCF refuses.** There is no general ranking, because the
  question "which model is better" has no referent once quality is plural.
- **Coverage travels with the answer.** A recommendation states which
  laboratories informed it and which the candidates were inapplicable to, since
  a confident answer resting on two of eleven measurements is a different claim
  from one resting on nine.

**Confidence: high.** This is §6.5 applied one level up: MCF refuses to invent an
objective, and "which qualities matter to you" is part of the objective.

### 6.37 Customizable laboratories vs. comparability and contribution

**Tension.** §XIII invites users to customize generalized laboratories against
their own use cases. §3.4 requires that a comparison hold everything but one
variable still, and §XIV wants results contributed to a shared corpus. A result
produced by a workload only one user has is comparable with nothing anyone else
holds.

**Resolution — customization is a first-class feature whose results are
first-class and *local*.**

- **A laboratory ships a default workload and accepts a replacement.** Results
  from the default are comparable and contributable; results from a replacement
  are neither, and are marked so at the point of production rather than at the
  point of export.
- **Locally, a custom result is the *most* valuable kind.** It answers "does this
  work for what I actually do", which no shared corpus can. §3.19's whole
  argument favours it.
- **A custom workload is never contributed**, because it is the user's content
  (A25) and because a score against an unseen workload is uninterpretable to
  anyone else — it would degrade the corpus rather than enrich it.
- **The customization surface is a workload slot, not a programming interface.**
  A user supplies data, schemas, labels, constraints, documents, tasks. They do
  not supply code, and B32's prohibition on a lab API is unaffected.

**Confidence: high on the split, medium on the surface.** How much a workload
can be replaced before a laboratory is measuring something other than what it
claims is a real boundary, and the honest answer is that a lab must state what
its workload slot may contain and refuse what it cannot grade.

### 6.38 A corpus that guides local decisions vs. "decide inward"

**Tension.** §6.28 resolved that MCF contributes outward and decides inward, and
B34 forbids a foreign number choosing a local configuration. But the reason to
accumulate a corpus is that it eventually knows something no single machine
does — which configurations work on which hardware. Refusing to consult it makes
§XIV a donation with no return; consulting it naively makes MCF the leaderboard
§5 refuses.

**Resolution — the corpus narrows the search; local measurement decides.** The
distinction is between a *prior* and a *claim*, and it is absolute:

- **Permitted: shaping what MCF tries.** Ordering candidates, pruning a search
  space, warning that a configuration has no working reports on hardware like
  this, seeding a local sweep from one that worked elsewhere, and supplying a
  first duration estimate where local history is absent (D14). None of these
  assert anything about this machine; they decide where to spend measurement
  effort, which is strategy rather than result.
- **Forbidden, unchanged: reporting a foreign number as a local one.** No
  measurement MCF publishes about this machine originates anywhere but this
  machine (B34).
- **Every corpus-derived statement is labelled, sampled and overridable**
  (§3.24), and is visibly distinguishable from a local measurement.
- **Guidance is never a gate.** Arithmetic may refuse — a model that does not fit
  does not fit. The corpus may only advise, because it is evidence about other
  machines and the user's machine is the one in the room.

**Why this is coherent rather than a loophole.** §IV's obligation is to
recommend from measurement taken here. Nothing in that obligation requires MCF
to choose *what to measure* blindly, and choosing well is what makes a slow,
honest instrument usable: six plausible configurations is an evening, twenty is
a week.

**Confidence: high on the distinction, medium on the surface.** Keeping a prior
visibly separate from a result, on a small screen, over time, under pressure to
simplify, is the practical difficulty — and the first place it will erode is a
sorted list whose ordering nobody explains.

### 6.39 "Improve anything that makes sense" vs. the user's machine

**Tension.** §IV asks MCF to improve local model use and §XVII permits it to use
the machine fully. Taken at its widest — clearing competing processes, retuning
the system, doing whatever makes the number better — this collides with §3.10
(the machine is the user's), §3.11 (nothing destroyed without deliberation) and
§5 (not a general system tool). And the scientific case for it is real: §3.8
says contention corrupts a measurement, and D8 already gives a laboratory
exclusive use of MCF's own resources. Asking the rest of the machine to stand
aside is the same argument continued.

**Resolution — a ladder, and MCF climbs only as far as the user has agreed.**

1. **Report.** Name what is competing, always, at no cost and no permission
   (§3.8, B24). This alone answers most of the need: a user told that three
   processes hold 40 % of the accelerator can act themselves.
2. **Wait.** Defer a run until the machine is quiet, with a stated timeout. Zero
   risk, and the default behaviour for a laboratory that requires quiet.
3. **Ask, then suspend and restore.** With per-run approval of a named list, MCF
   may *suspend* processes and restore them when the run ends — including after
   a crash. Suspension is reversible; that is the entire reason it is permitted
   where termination is not (§3.25).
4. **Never terminate, and never touch what was not approved.** Killing a process
   may destroy unsaved work, which is §3.11's prohibition without the
   deliberation, and MCF is not positioned to know what it is discarding.

Two conditions on the whole ladder. **Everything MCF changes about the
environment is a measurement condition** (§3.4) — a number taken with the
user's browser suspended is a different number, and a result that hides that is
corrupt. And **everything is restored**, which makes the stop control (§6.33)
part of this feature rather than adjacent to it.

**Confidence: high on the ladder, low on the appetite.** Whether anyone actually
wants step 3, given that step 1 lets them do it themselves with full knowledge,
is unknown — and building 3 before 1 has proven insufficient would be spending
weight on a guess.

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

### 7.2 The objective function — **blocking §IV**

§6.5 defers the definition of "optimal." Someone must eventually state how
quality, latency, throughput, memory, power, and disk trade against one another,
and how a user expresses their own weighting. Without this, the optimization
intent cannot be implemented, only gestured at. §VI raises its urgency: a tool
that is frictionless by intent must ship a default opinion, and §6.5 requires
that opinion be stated rather than emergent.

### 7.4 Engine ownership: does MCF perform inference, or delegate it? — **blocking §VI and §VII**

Arguably the most consequential unanswered question in this document. §VII's "fastest possible" reads as an argument for owning the
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

### 7.5 Retention and residency of the record

D5 removes most of this void by removing most of the data: with ambient
telemetry refused, there is no metric stream to size, age out or budget. What
remains is the record itself — measurement history, failure records, provenance —
which is small, durable and scientifically valuable, so the question is *how
long must we keep* rather than *how aggressively do we discard*, and §7.13's
comparability problem dominates it.

Still open: whether anything may ever leave the machine, whether the user can
inspect and purge what MCF holds about them, and what happens if the record's
disk budget is exhausted (§3.1 and §6.9 forbid that being a silent drop).

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

### 7.9 Resource arbitration and concurrency — **largely resolved by D8**

Models are enormous relative to available memory and disk. Who decides what is
resident? What happens when a benchmark and a served model both want the GPU, or
when a download would exhaust the disk mid-flight? §3.11 forbids surprising
destruction but does not say who arbitrates.

D8 answers the sharpest half: a laboratory owns the machine, so MCF **must**
refuse to serve while measuring, and arbitration between them is a state machine
rather than a scheduler. What remains is everything outside a lab — a download
that would exhaust the disk mid-flight, several clients of a served model, two
models resident at once — and §7.37's question of who writes to the record.

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

### 7.20 How much reality validates the lab — **blocking §VIII**

§6.16 requires that simulated confidence be checked against real hardware, but
not how much, how often, or against what. What fraction of the suite must have a
real-hardware counterpart? On which hardware, given that MCF is meant to run on
machines we do not own (§7.8)? What divergence between simulation and reality is
tolerable before the simulator is declared defective?

Until this is answered, §VIII is an assertion rather than a discipline, and
§6.16's low confidence rating stands. It is the highest-leverage void attached
to §VIII, in the same way §7.16 is for §VII.

### 7.21 What the laboratory is obliged to simulate

§3.17 says every failure MCF claims to handle has a simulation that produces it,
which makes the lab's fault catalogue and the failure taxonomy (§7.10) the same
list — and neither exists yet. Someone must decide the scope: hardware absence
and variety, accelerator failure modes, hub misbehaviour (malformed, gated,
hostile, truncated, mutated-under-us), disk exhaustion, network stall and
partial transfer, runtime death at every lifecycle stage, thermal throttling,
memory pressure, contention, clock and time anomalies, upgrade and migration.

Two second-order questions ride on it: whether simulated time is required (§3.17
implies yes, and it is a structural decision, not a testing convenience), and
what the lab explicitly declines to model, since §6.16 requires that boundary be
stated so confidence is claimed only where earned.

### 7.22 What "full system" testing means for a daemon

§3.5 requires whole-system coverage, but the end-to-end boundary is undrawn.
Does a full-system test drive the real HTTP surface? Start a real inference
engine, or a simulated one? Cross a process boundary into a supervised child?
Exercise restart and recovery with persisted state?

The answers determine whether the suite can honestly claim to test the
behaviours that only exist between components — supervision, recovery,
contention, degradation — which are precisely the ones §I is about, and which
unit tests structurally cannot reach.

### 7.23 The agentic suite: contents, statistics, and honesty over time — **blocking §IX**

§7.3 is answered in kind but not in substance. What tasks? Drawn from published
agentic benchmarks (comparable, but contaminating over time), written for MCF
(honest, but unvalidated and laborious), procedurally generated (fresh and
uncontaminatable, but of uncertain difficulty and realism), or derived from the
user's own work (maximally valid, ungradable without effort)?

Riding on it, and equally unresolved:

- **How many trials, and what counts as a difference.** §6.17 requires
  distributions rather than scores but cannot supply the statistics. Without an
  answer, §3.9's "these are within noise, pick either" cannot be said honestly,
  and §7.7's acceptance criteria have nothing to bind to for agentic runs.
- **What a task's tools are, and how difficulty is calibrated** — a suite every
  model passes and a suite every model fails are equally uninformative.
- **How the suite stays uncontaminated** as it ages, per §7.3.
- **What an agentic run costs**, since §4 concedes that good evaluation is
  expensive, and §7.9's arbitration question becomes acute when a benchmark
  occupies the machine for hours.

### 7.24 The scope and cost of capability probing — **blocking §X**

Intent X says "full capabilities" without bounding *full*. The plausible list —
chat template correctness, tool-calling format and reliability, structured
output, context length actually usable versus claimed, vision, embeddings,
reasoning modes, multilingual, stop-condition behaviour, prompt-format
sensitivity — is open-ended, and each entry is a probe someone must design and
validate against §3.18's measurement standard.

Also unresolved: **when probing happens** (acquisition, first load, on demand,
lazily as needed), **what it costs** in time and resources under §VII, whether a
probe's result may be cached across MCF versions given §7.13's comparability
problem, and what MCF does when a probe is *inconclusive* rather than
positive or negative — the third state §3.18 requires but does not describe how
to act on.

### 7.25 Whether automatic configuration may change under a user

§X implies MCF improves its configuration decisions over time — better probes,
better defaults, better hardware knowledge. §3.12 and §6.13 say results depending
on hidden history are irreproducible, and §3.11 says nothing changes without
deliberation.

So: may MCF silently reconfigure a model the user has been using, if it learns
something better? If it does, yesterday's benchmark and today's are not
comparable and §3.4 is violated. If it never does, the configuration rots.
Neither branch has been chosen, and the choice determines whether an
auto-configured setting is a decision or a live value.

### 7.26 The reference set — how much breadth a generality claim requires

§6.23 holds that one model builds an instrument and never supports a claim about
models in general, without saying what does. Someone must decide what the
reference *set* becomes and when: how many models, chosen along which axes —
family, format, size, quantization lineage, instruction-tuning style — and at
what point in the roadmap the second and third join.

The axes are not equally informative, and choosing badly is cheap to do and
expensive to discover. Three models from one family test less than two from
different ones. A second GGUF derivative tests less than a first safetensors
one, because format handling is where §III's coverage claim actually lives.

Riding on it: whether §IV may make any recommendation before the set exists
(§6.23 says no, but not how large "exists" is), and whether the laboratory's
real-hardware validation (§7.20) needs the same breadth or a different one —
they are asking different questions of the same weights.

### 7.27 What a contribution contains, and whether it can be de-identified

§6.27 requires that the user see what leaves and that de-identification be
treated as hard. It does not say what is actually sent. Someone must decide
which fields are contributable, which are coarsened (an accelerator model rather
than a serial number; a bucketed driver version rather than an exact one), and
which are withheld entirely.

The hard part is not the obvious identifiers; it is that the useful fields are
the identifying ones. Hardware, driver, thermal behaviour, model selection and
timing together fingerprint a machine, and a contribution stripped until it is
anonymous may be stripped until it is useless. Where that line falls, and
whether MCF should say plainly that contribution is not anonymous rather than
implying a protection it cannot deliver, is unresolved.

Riding on it: whether a contributor can be linked across contributions, and
whether that is a feature (longitudinal data from one machine is more valuable)
or a defect (a stable identity is an identity).

### 7.28 What an identifier is

§XV requires MCF to resolve an identifier into an exact configuration, and §6.29
governs the trust posture without saying what the thing *is*. Open: whether it
is content-addressed over the configuration it names, whether it is resolvable
offline or requires a lookup, what it binds (weights, quantization, context,
runtime, sampling parameters, harness version — all of them?), what happens when
part of what it names no longer exists on the hub, and whether it carries
provenance of its own or is merely a key into somebody else's table.

Two properties are worth wanting and may conflict: an identifier short enough to
paste, and one self-describing enough to resolve without trusting a server.

### 7.29 Which laboratories exist, and what makes one worth building — **drafted in [labs.md](labs.md)**

§XIII asks for a range of labs and §6.26 requires each to justify itself by the
claim it enables. Neither says which labs, in what order, or what the first
three are. The plausible list is long — latency and throughput, context
degradation, tool-calling reliability, structured-output conformance,
instruction adherence, refusal and safety behaviour, quantization damage,
prompt-format sensitivity, long-run stability, memory behaviour under pressure —
and each is real work held to §3.17's standard.

[labs.md](labs.md) drafts a catalogue — twenty candidate laboratories in four
families, with the first three named — which converts this void from *unasked*
to *unratified*. What remains: which are built and in what order, what each is
obliged to state about its own validity, whether a lab may be retired once its
question is answered, and how large a workload slot may be before a customized
lab measures something other than what it claims (§6.37).

### 7.30 Schema versioning across contributed databases

D6 makes the record a SQLite database and §XIV makes it shareable, which turns
its schema into a public interface. §7.13's comparability problem becomes
somebody else's problem: a contribution written by MCF 0.4 must be readable —
and honestly interpretable — by whatever reads it later, and a measurement whose
method changed between versions is not comparable with one taken after (§3.4).

Open: whether the schema is versioned independently of MCF, whether old
contributions are migrated or merely marked, and what a reader is obliged to do
with a contribution it cannot fully interpret. §3.1 forbids the silent option.

### 7.31 What contribution costs the contributor

§3.20 makes contribution opt-in and deliberate, which settles consent but not
economics. Open: whether contributing costs the user anything they would notice
(bandwidth, a moment's attention, a decision they must keep making), whether a
one-time choice may stand for future contributions or each must be asked, and
whether MCF may ever prompt for a contribution rather than waiting to be asked.

The last of those is the dangerous one. A tool that asks often enough becomes a
tool that is answered reflexively, and a reflexive yes is not the informed
consent §3.20 requires.

### 7.32 Distribution, and the update policy — **blocking D7**

D7 makes MCF other people's software and §XVI makes it self-contained; neither
says how it reaches them or how it changes underneath them. A candidate
direction exists — a website counterpart with a server component shipped as a
container image, alongside the self-contained local binary — but it is a
direction, not a decision, and the local tool must not come to depend on it
(§5, "not the website").

The update half is the dangerous half. §3.12 forbids silent auto-upgrade, and
§7.13 makes a version change a potential invalidation of measurement history.
Never updating strands users on versions whose results cannot be compared with
anyone else's; updating silently violates the reproducibility rule outright. The
shape of the answer is probably *offered, explained, never automatic, and
explicit about what it invalidates* — but that is reasoning, not a decision.

Also open: whether the container image and the local binary are the same
artifact in different clothing or two products with two test surfaces, since the
second answer doubles §VIII's obligations.

### 7.33 Whether the record keeps raw samples or summaries — **structural**

Nothing states whether a measurement stores its individual trials or only their
summary. The choice is unrecoverable in one direction: summaries cannot be
re-analysed, and DEC-023 concedes that the right statistic for agentic runs is
not yet known. Every measurement taken before this is answered is taken at the
mercy of the answer.

The candidate answer is *keep the raw samples* — they are small relative to
weights, §II is built on re-analysis, and §3.4's uncertainty requirement is
weaker than it sounds if the underlying distribution is discarded. Recorded here
rather than assumed, because it decides the schema (D6) and therefore must be
settled before the first row is written.

### 7.34 The identity of a measured configuration — **structural**

The whole project compares things, and nothing defines what makes two runs
comparable *as a key*. Weights revision, quantization, context length, runtime
build, sampling parameters, hardware, MCF version, instrumentation profile — the
subset that constitutes identity determines what can be grouped, what
invalidates history, and what §XV's identifier serializes.

§7.28 asks what an identifier looks like on the outside; this asks what it names
on the inside, and it is needed at the first write rather than at §XV.

### 7.35 Host platform scope, and the containment mechanism — **structural**

§7.8 bounds accelerators and §6.11 bounds client devices. Nothing bounds the
machines MCF itself runs on, and that omission hides a large architectural
decision: A14 requires a sandbox *by construction*, and OS namespaces, a
hypervisor, and a portable abstraction over both are three different daemons.
§XVII's privileged helper multiplies it, since elevation mechanisms are
per-platform.

Until this is drawn, "runs on this machine" is as unfalsifiable as §VII was
before §7.16.

### 7.36 Whether model licences constrain publishing measurements

§III makes licences legible for *use*. §XIV publishes results *about* an
artifact, and some model licences carry terms about benchmarking, comparison or
naming. Whether a contribution is a licensed act, and whether MCF must therefore
carry a per-artifact publication flag alongside its per-artifact use flag, is
unresearched. It constrains what a contribution can ever contain, so it is asked
before M9 is built rather than after.

### 7.37 Who writes to the record, and how concurrency is arbitrated

D6 chose an embedded single-file database before §7.9 and §7.12 decided who
writes to it. A serving path, a laboratory, a supervisor recording failures and
several attached clients are potential writers, and the failure modes of an
embedded store under concurrent writers are sharp and specific.

D8 removes much of the contention by making laboratories exclusive, which leaves
the ordinary case: a serving daemon and its clients. Whether writes funnel
through one owner, and what happens to a record write that loses, is unstated —
and §3.1 forbids the silent answer.

### 7.38 What happens when a pinned artifact decays

MCF pins revisions, which is right. Nothing says what happens when the pin goes
bad underneath it: a revision withdrawn, a repository gated after acquisition, a
licence changed, a tag repointed, a file replaced. The hub is mutable (§3.7) and
the record depends on it (§3.6).

Open: whether MCF ever checks, when, what it does on discovery, and whether a
withdrawn upstream invalidates measurements taken from the local copy — it
should not, since the local weights are unchanged, but the provenance chain now
points at something that no longer exists and that must be recorded rather than
quietly tolerated.

### 7.39 Which operations actually require elevation, on which platforms

§6.32 requires the privileged surface be an enumerable, auditable, short list.
The list does not exist. Reading power and thermal counters, setting a
performance governor, requesting exclusive accelerator access, locking pages,
pinning cores and raising scheduling priority each require different rights on
different platforms, and some require none at all.

Riding on it: whether a helper that turns out to need broad rights for a narrow
job should be granted them or the capability abandoned. §6.32's answer is
reconsider rather than grant, and that answer costs measurements MCF would
otherwise take.

### 7.40 What makes two machines alike

§6.38 permits statements of the form *"no working reports on hardware like
yours"*, and D14 leans on a corpus prior when local history is absent. Neither
defines the class. Two machines with the same accelerator may differ in driver,
host memory, thermal solution, power limit and host CPU, and each of those can
decide whether a configuration works.

Open: which attributes constitute similarity, whether similarity is one relation
or several — a memory-fitment class and a throughput class are not the same
grouping — how a machine outside every known class is treated, and whether
similarity is computed locally from the corpus or asserted by whatever
aggregates it.

This bounds how useful the corpus can be. Too coarse and the guidance is wrong;
too fine and every machine is its own class and there is no guidance at all.

### 7.41 The environment-control surface

§6.39 permits MCF to ask the machine to stand aside, and §3.25 bounds it to what
can be restored. Neither says what the surface is: which system knobs are in
scope, how a user grants a scope (per run, per session, a standing list), what
happens to a suspended process if MCF is killed rather than stopped, and how any
of it is expressed on platforms whose mechanisms differ entirely.

The last is the sharpest. Suspend-and-restore is straightforward on one family
of operating systems and awkward or unavailable on others, so §6.39's ladder may
have a different maximum height per platform — and §7.35's platform scope
decides where.

### Retired voids

Answered, and their substance moved to §2.1 per §8. The numbers stay citable.

| Void | Question | Answered by | Substance lives in |
|---|---|---|---|
| §7.1 | What "deployed" means | §VI | **D1** — MCF is a daemon |
| §7.3 | What quality is measured against | §IX | **D2** — agentic task success |
| §7.12 | The user surface | §XI | **D3** — both surfaces, headless primary |
| §7.19 | Implementation substrate | §3.16 | **D4** — Rust |
| §7.15 | Success beyond the author | §XIV, §XV | **D7** — MCF is for other people |

§7 shrinks over time. If it does not, we are building on undeclared assumptions.

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
  §7 in its light. §V–§VII are the worked example: three sentences added six
  conflicts and closed two blocking voids.

---

## 9. Changelog

The only historical record in this document. Every clause above states the
present position; this section states how it came to be held, because §8
requires that the *reasoning* behind each change survive it.

### Version 8 — self-containment, privilege, and what the laboratories are allowed to do

Two intents and four decisions, most of them answering questions §XIII opened
and left underspecified.

§XVI makes MCF self-contained: the user obtains one thing and runs it. This is
an ergonomic statement with a scientific consequence, which is why it is an
intent rather than a packaging note — *a dependency the user installs is a
dependency MCF did not pin*, and it becomes an unrecorded variable in every
measurement taken on that machine. §6.31 resolves what it does to §7.4's
delegation and to §VII's budget: MCF may drive an engine it did not write but
may not send the user to fetch it, and §VII's budget governs behaviour rather
than download size — a static binary that idles at nothing honours §VII at 12
MiB or 400.

§XVII permits full use of the machine, including elevated privilege where it
buys measurement validity, because a process that cannot read thermal state or
stop another process from stealing the accelerator takes measurements with
unknown conditions. §6.32 draws the boundary severely, because a privileged
daemon that fetches and executes remote code and may listen on a network is the
most dangerous object this document has described: the daemon holds no ambient
privilege, elevation belongs to a small auditable helper performing named
operations, nothing untrusted ever runs privileged, and MCF works without
privilege and says what it lost.

D8 settles what laboratories may do, and answers most of §7.9: a lab owns the
machine, no user traffic is served beside it, no second lab runs, and within its
run it may be as greedy as accuracy requires. Exclusivity removes the largest
confound available and removes it structurally — there is nothing to subtract
because there was nothing else running. §6.33 pays the cost §VI is owed:
suspension is a declared state, announced, bounded and interruptible, never an
outage.

D9 standardizes time — monotonic for durations, UTC for records, simulated in
the lab, anomalies invalidate loudly. D10 separates the two disciplines that
share the word "test": MCF's code is tested exhaustively and in tiers, including
mutation testing as the test of the tests, while models are *measured* in
laboratories that have no pass condition; §6.34 keeps the heavy tiers from
destroying the fast one. D11 makes energy and thermal state first-class, with
§6.35 confining power sampling to laboratory runs and forbidding a modelled
figure from being presented as a reading.

Eight voids added (§7.32–§7.39), five of them found by auditing what the earlier
versions had never asked: whether the record keeps raw samples or only
summaries, what constitutes the identity of a measured configuration, which host
platforms MCF runs on and how it contains untrusted code there, whether model
licences constrain publishing measurements, who writes to the record, and what
happens when a pinned artifact decays upstream. The first three are marked
structural because deciding them late destroys data or forces a rewrite.

### Version 7 — analysis, the shared record, and reproduction by identifier

Three intents added, and the largest enlargement of scope since the founding
four. §XIII makes MCF a bench rather than only a host: a range of purpose-built
diagnostic laboratories, each instrumented as deeply as its own question
requires. §XIV makes the record a portable artifact the user may choose to
contribute, so that many machines' evidence aggregates into something no single
machine can produce. §XV makes a configuration nameable, so that a user can hand
MCF an identifier found elsewhere and receive exactly that configuration here.

§1 grows from four verbs to six — *Analyse* and *Share* join Acquire, Serve,
Judge and Show — and the point of the project moves from Judge alone to Analyse
and Judge together.

**These intents collide with more of this document than any addition so far, and
three collisions were serious.**

*Deep telemetry versus the decision to refuse it.* §XIII asks for exactly what
D5 declined. §6.24 resolves it on two absolute distinctions: lab telemetry runs
only during an experiment the user started, and it watches the model rather than
MCF. The idle daemon still does nothing whether MCF carries three labs or
thirty, and §5's observability anti-goal is amended to name the subject — MCF is
not the specimen. §3.22 states the scoping rule, and §6.25 handles the validity
cost: timing-class results run under a reduced profile, behaviour-class results
may instrument freely, and every result records the profile it ran under.

*Contribution versus privacy.* §6.27 finds that the content/system split §6.8
already required is what makes contribution safe — the contributable database
holds no user content because content was never written to it, which a
misconfigured export filter cannot undo. §3.20 adds the principle the document
lacked: publication is the one irreversible act, so it is opt-in, per share,
never a side effect, and shows the user the rows rather than a description of
them. What remains identifiable after that is §7.27, and it is recorded as
genuinely unresolved rather than assumed away.

*Crowd-sourcing versus "not a leaderboard".* §6.28 reads the anti-goal precisely
rather than repealing it: MCF contributes outward and decides inward, no foreign
number ever chooses a local configuration, and MCF neither computes nor displays
a ranking. The suspicion is sharpened — MCF is not suspicious of foreign *data*,
it is suspicious of foreign *conclusions* — and what MCF contributes is the
thing public leaderboards lack, which is the conditions.

Also: §6.26 keeps a range of labs from becoming an ecosystem of them (in-tree,
one at a time, each justified by a claim it enables). §6.29 subjects an inbound
identifier to every rule that governs untrusted input, resolving it as a
*request to reproduce* followed by local verification, with §3.21 stating that
an imported configuration is declared until this machine verifies it. §6.30
handles the contamination that publishing results predictably causes.

Adds D6 (the record is a SQLite database) and D7 (MCF is meant to be used by
people other than its author, which answers §7.15 and makes documentation,
installation and interface stability goals rather than incidents). Adds two
standing tensions, four anti-goal amendments including two new ones — not the
website, not a data broker — and voids §7.27–§7.31.

The website that aggregates contributions is named as long-term context and is
explicitly not part of this project.

### Version 6 — the reference model

Intent XII added: `unsloth/Qwen3.8-27B-GGUF` as the reference model for initial
work. The first intent naming a specific artifact rather than a property of the
system, and it collides with more of this document than its size suggests —
§3.5's hermetic suite, §3.4's anti-overfitting rule, and §III's no-special-casing
commitment. Resolved in §6.22 (the reference model is a fixture for the
instrument, never a dependency of the suite or a case in the code) and §6.23
(one model builds an instrument and never supports a generality claim). Opens
§7.26. Narrows §7.8 and §7.16, since whatever runs a 27B model is now the first
characterized machine.

*Also in this version, and structural rather than substantive:* the document is
restated in the present tense with all history collected here; the four resolved
voids are migrated out of §7 into §2.1 as D1–D4, which §8 has required since
version 1 and which had not been done; and §6.9's resolution moves to D5.

### Version 5 — agentic evaluation, capability discovery, surface parity

Intents IX–XI added. §IX is the most consequential addition since the founding
four: agentic workflow benchmarking substantially answers §7.3 — what quality is
measured against — which this document called its single hardest unanswered
question from version 1 onward. It answers it well, because agentic task success
is verifiable without a ground-truth corpus and resembles the work the models
will actually do. It also brings expense, stochasticity and a sandbox
requirement (§6.17–§6.20), most usefully the finding that the agentic
environment and the §VIII laboratory are the same apparatus — which makes the
rigor requirement and the safety requirement one implementation.

§X (capability discovery) collides directly with §3.6's prohibition on inferring
metadata, resolved in §6.19 by turning detection into measurement. §XI ratifies
what §3.14 and §6.11 already held and closes the rest of §7.12.

### Version 4 — Rust, and confidence by laboratory

Two decisions. The implementation substrate is settled: **Rust** (§7.19
resolved, on §3.16 grounds rather than performance ones).

And the project's confidence strategy is deliberately rebalanced: **away from
ambient telemetry, toward exhaustive testing and simulated laboratory
verification.** That second decision is larger than it sounds. It cuts §6.9,
this document's sharpest and least confident conflict, by choosing a side; it
forces §3.3 to be split into two ideas that version 2 had wrongly fused; and it
adds Intent VIII, because a project that declines to watch itself in production
must be able to reproduce itself in a lab. §6.15 and §6.16 record what that
trade costs.

### Version 3 — the Ollama framing corrected

Intent VI cited Ollama as an *example of a friction level*, not as an
architectural or implementational model, and the document had begun to treat it
as the latter. §VI now says so explicitly, §5 gains an anti-goal against
clone-thinking, and every comparison in the document names the property rather
than the product. Adds §3.16 (prefer substrates a machine can hold to the
principles) and records §7.19.

### Version 2 — interface, hosting, lightness

Intents V–VII added. Two of them answer questions recorded as blocking voids:
what "deploy" means (§7.1), and how a human touches the system (§7.12). The
third — *be the fastest, lightest tool possible* — is the most disruptive
statement made about this project. It does not merely add a goal; it applies
downward pressure to every other intent, because rigor, observability and
universality all have weight. §6.9–§6.14 and §3.13–§3.15 exist to keep that
pressure from silently eroding the rest of the document.

### Version 1 — the founding four

Intents I–IV consolidated: reliability, science, custody, optimization. The
repository was empty; all conflict resolutions were arbitrated on coherence
rather than by implementation, and they remain so.
