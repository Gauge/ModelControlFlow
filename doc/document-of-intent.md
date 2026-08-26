# Document of Intent

| | |
|---|---|
| **Type** | Intent — the spirit of the rules |
| **Version** | 36 |
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
to the engine question D32 settles and to §VII's budget, and the short form is
that
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

**Conditional on §7.4, and the condition is met.** This reasoning assumed MCF
wraps inference engines rather than implementing kernels. D32 settles that it
does, on measured ground ([findings.md](findings.md) F8), so the substrate
question stands rather than resting on an assumption. It would reopen only with
that decision.

**Validated, not assumed.** §3.13 requires optimizing what is measured rather
than what is imagined, so this decision is confirmed by a small adversarial
prototype: probe an accelerator, supervise a child runtime deliberately made to
die badly, record both under §3.1, and measure the result against §7.16. If that
goes badly, this entry is amended rather than defended.

**The prototype ran, and this entry stands.** F1 in [findings.md](findings.md)
records it. Four ways a supervised runtime dies badly each reach a distinct
taxonomy category with the manager unaffected and the child's partial output
kept; the accelerator answers five of five questions over the C ABI against
three of five from the files its driver publishes; and MCF's own artifact,
resident memory and cold start sit under D24's ceilings with room. Two findings
travel with the confirmation rather than against it: the fields §3.8 actually
needs — live memory and thermal state — are reachable only over the C ABI, which
DEC-008 has to decide knowing; and D24 does not say which statistic its figures
name, which §7.50 now records.

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

### D8 — Exclusivity is bought by measurement class, never claimed by rank

A laboratory takes the machine only when **the validity of its measurement
requires it**, and for no longer. The requirement follows §6.25's class
distinction, and the consequence is the opposite of what it first appears:

| Class | Needs a quiet machine? | Typical duration |
|---|---|---|
| **Timing** — throughput, latency, energy, memory scaling | **Yes.** A timing taken under contention measures the contention (§3.8) | tens of minutes |
| **Behaviour** — tool calls, agentic tasks, extraction, retrieval, code | **No.** Whether a call parsed or a loop terminated is unperturbed by the user opening a browser | hours to days |

**The labs that need the machine are the short ones.** Exclusivity is inversely
correlated with duration, which means the practical cost of rigour here is a
coffee break rather than a lost day. The twenty-hour evaluation run — the one
that would make a machine unusable — is precisely the one that does not need
exclusivity at all.

**Exclusive windows.** A timing-class run opens a window: announced before it
starts, bounded by a declared maximum, interruptible, and closed automatically.
Inside it, MCF may be greedy — all of the accelerator, locked memory, pinned
cores, raised priority — and §6.39's ladder is available with the user's
agreement. Outside it, MCF holds nothing.

**Yielding runs.** A behaviour-class run is a background citizen: low priority,
yielding to the foreground, always behind user traffic on the serving path,
pausing on request. Contention is *recorded as a condition* (§3.4) rather than
prevented, because the outcome it measures does not depend on it. Two things
this makes obligatory, and they are the honest caveats:

- **Deadlines are token budgets, not wall clocks.** A task that fails because a
  busy machine made it slow is a measurement of the machine, and §3.8 forbids
  attributing that to the model.
- **An environment failure is not a model failure.** An out-of-memory caused by
  competition for host memory is classified as a condition of the run, never as
  the model giving up — the failure taxonomy (§7.10) must distinguish them or
  every yielding run is quietly contaminated.

**What this settles in §7.9.** The question *may MCF refuse to benchmark while
serving, or must it* is answered by class rather than by policy: it **must**
refuse for timing-class work, and **must not** for behaviour-class work. Only
the exclusive window is a state machine with one occupant; the rest is
scheduling, and scheduling is what a good guest does (§3.26).

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

### D15 — Resource boxes: a declared allocation a model runs inside

A model may be confined to a **box**: a declared allocation of cores, host
memory, accelerator share and I/O that it runs inside and cannot exceed. The box
is stated, reproducible, and is part of every measurement taken in it (§3.4).

**What a box buys.**

- **Reproducibility across sessions** (§7.6). A measurement taken on "the
  machine" depends on whatever else the machine was doing. A measurement taken
  in a 6-core, 16 GiB box is the same measurement next Tuesday.
- **Isolation without exclusivity** (§3.4, §3.26). A box removes contention as a
  *confound* without requiring the user to surrender the machine — the third
  answer alongside D8's exclusive window and its yielding mode.
- **Comparability between models.** Two models in identical boxes differ by one
  variable. Two models measured on a whole machine at different times differ by
  however busy it was.
- **A real question answered honestly:** *what do I get while I am working?*
  A boxed run under stated load is not a degraded timing; it is a different
  measurement, and often the one a user actually cares about.

**What a box cannot buy, and this is the limit that must never be papered
over.** A box bounds **what a process may take**; it does not bound **what a
process may be denied.** Memory bandwidth, cache, accelerator time-slicing, PCIe
and thermal headroom are shared on consumer hardware and largely unpartitionable
there. A model boxed to six cores still contends for memory bandwidth with
everything else running, and no allocation prevents it.

Therefore:

- **Every boxed result names what its box did not bound.** The unpartitionable
  dimensions are listed, not implied, so nobody reads a box as a guarantee of
  isolation it cannot provide.
- **Boxed and unboxed results are never compared** (A8), and neither are results
  from different boxes.
- **A box is not a substitute for the exclusive window** when the goal is a
  best-case timing (§6.41).
- **Thermal state remains global and uncontrollable** and is recorded rather
  than claimed away (D11, L21).
- **Box capability varies by platform and is stated, never assumed.** Where a
  platform offers no mechanism, boxing is `unknown` and unavailable (A7), not
  approximated. Where it requires privilege, §XVII and A26 govern.

### D16 — The record keeps raw trials *(answers §7.33)*

**Per-trial data is retained, always.** Every trial of every run is a row: its
value, its arm, its position in the interleaving, its session. Summaries are
*derived* at query time and never written in place of the trials that produced
them.

**Why it is not really a choice.** Three decisions already made depend on it.
§6.17 holds that agentic outcomes are often bimodal, and a shape cannot be
recovered from a mean. §3.27 requires paired differences, which is arithmetic
over trial pairs a summary has already destroyed. And §7.7's acceptance criteria
do not exist yet, so a result frozen under a statistic nobody has chosen is a
result that can never be re-asked.

**Intra-trial detail is per laboratory, and off by default.** Per-token arrival
times, per-turn outcomes and similar interior detail are declared by the lab
that needs them, justified the way a lab justifies its own existence (B32). A
throughput lab needs them, because the inter-token *distribution* is its
subject — a model averaging 40 tokens per second with periodic stalls feels
worse than one steady at 35, and a summary cannot tell them apart. An extraction
lab does not.

**Size is not the constraint, and the numbers say so.** A trial row is on the
order of a hundred bytes: a year of heavy daily use is tens of megabytes against
model weights measured in gigabytes. The real cost of interior detail is
cardinality — query and migration behaviour differ between thousands of rows and
millions — which is why it is declared rather than global.

**Where interior detail does grow, it is downsampled rather than dropped.** A
long-generation lab producing a million rows per configuration keeps full
resolution up to a declared cap and thins beyond it, **with the thinning
recorded as a condition** (§3.4). A downsampled series that does not say it was
downsampled is a silent alteration of evidence (§3.1).

### D17 — Identity is the runnable configuration; hardware is a condition *(answers §7.34 in principle)*

**A configuration is what the hosting system needs in order to run a model.**
Weights and revision, quantization, context length, runtime, sampling
parameters — the complete description of the thing that runs. That set is the
identity: two runs share an identity when they share that description.

**Hardware is not part of identity.** It is a *condition* of the measurement
(§3.4). The same configuration measured on two machines is the same thing
observed twice, not two things — which is precisely what makes §XIV's corpus
possible: measurements group by configuration and hardware becomes the axis they
are analysed *along*, rather than a key that fragments them into a separate
universe per machine.

**Grouping is a view, not the key.** Configurations are grouped and generalized
at query time — by model family, by quantization class, by anything a question
needs. This is the same principle as D16 one level up: **keep the fine-grained
thing and derive the coarse one**, because grouping up is always possible and
splitting down never is.

*The residual question is which parameters are identity and which are
conditions at the edges — placement, engine build, sampling — and it stays open
in §7.34.*

### D18 — Sampling is identity; the recommendation is a declaration to be verified *(answers §7.46)*

**Sampling parameters are part of a configuration's identity.** Temperature,
top-p, top-k, penalties and maximum tokens are what the hosting system needs in
order to run a model (D17), they change behaviour profoundly, §6.6 lists them
among the things MCF tunes, and §XV cannot reproduce behaviour without them. Two
configurations differing only in temperature are two configurations, and
grouping-as-a-view collapses them whenever a question does not care.

**The model's own recommendation is the default, and it is a *declaration*.**
Calibration (D13) adopts what the artifact recommends rather than imposing a
house style, because a global constant would measure every model under settings
some were never designed for. The value is marked *declared, unverified* (A21)
until MCF has tested it — which is §3.18 applied one level out: **recommendations
are measured, not believed.**

**Sweeps verify the recommendation, and divergence is a finding.** A sampling
laboratory sweeps around the declared values and reports whether they are in
fact the best available here. That the publisher's recommendation is *not*
optimal on this hardware, or not optimal for this workload, is exactly the kind
of thing this project exists to discover — the same shape as the declared-versus-
verified context divergence, arriving through a different parameter.

**"Better" is per workflow, never global** (D2, §3.23). The temperature that
maximizes tool-call reliability need not be the one that maximizes extraction
accuracy, and MCF reports a sweep per laboratory rather than crowning one value.

**A lab may still pin its own sampling** as declared method, where its question
requires it — a determinism laboratory cannot run at the user's temperature and
mean anything — provided it says so and its results are not compared with those
of a lab that inherits.

**The hazard, and it is not optional to handle.** A sweep is hyperparameter
optimization, and picking the best of eight arms inflates the apparent gain
whether or not any real difference exists — the garden of forking paths. So a
swept value is **selected on one split and its improvement reported from
another** (A10, B-125). The number MCF publishes is the validated one, never the
winning one, and a sweep whose winner does not survive validation reports *no
improvement found*, which is a §3.4 null result and a useful one.

### D19 — The seed set is a declared condition, not part of identity *(closes §7.34)*

**A seed is not good or bad.** It selects one trajectory through the sampling
distribution, and the mapping from seed to outcome is chaotic and
task-specific — a seed that happens to serve one prompt well has no tendency to
serve the next one well. There is no systematically underperforming seed, only
seed-by-task interactions, which look like bad luck and are.

**One fixed seed is therefore worse than it appears.** Thirty trials at a fixed
seed with identical inputs produce thirty identical outputs: `n=1` wearing the
costume of `n=30`, which destroys §3.4's uncertainty requirement at the exact
point it matters. Fixing a single seed does not reduce variance; it conceals it.

**What MCF uses instead: a declared seed *set*.** A published list — trial *i*
uses seed *i* from the set — identical on every machine. That satisfies both
things at once, which is why it is preferred to either alternative:

- **Reproducible across systems**, because everyone draws the same seeds.
- **Genuinely varied within a run**, because the seeds differ trial to trial, so
  the spread §3.4 requires is real rather than manufactured.
- **Not exposed to an unlucky draw**, because the result rests on the whole set
  rather than on one trajectory.

The set's size is the trial count and is therefore the same decision as §7.23's
statistics, not a separate one.

**The seed set is a condition, not identity** (D17, D18). Sampling parameters
are identity because they change the *distribution*; a seed only draws from it.
That also honours §6.17: stochasticity is reported rather than engineered away.
Comparisons require matching seed sets the way they require matching hardware —
recorded, checked, and refused when they differ (A8).

**The honest caveat: identical seeds do not guarantee identical output.**
Floating-point reduction order, kernel scheduling and batch composition make
inference non-deterministic on accelerators even at a fixed seed. So a seed set
buys comparable *inputs*, not identical *outputs*, and how much determinism a
machine actually delivers is itself a measurement — L19 exists for it, and its
answer feeds §7.6's reproducibility tolerance.

**The set is validated, not assumed.** Periodically, a larger random set is run
and its distribution compared with the fixed set's. Divergence means the
standard set is unrepresentative and is replaced, with the replacement recorded
as a break in comparability (§7.13). This is §6.16's discipline — the instrument
does not get to grade itself — applied to the seed set.

**Timing laboratories ignore seeds and pin generation length instead.** A seed
changes which tokens are produced and therefore possibly how many, and a timing
that varies because one run stopped earlier is measuring the stop, not the
speed.

### D20 — The record is a rebuildable index over an append-only journal *(answers §7.49)*

Trials are appended to a journal as they complete; the queryable database is
**derived** from it and can be discarded and rebuilt. This is D16's principle
applied to durability — keep the fine-grained thing, derive the coarse one — and
it converts corruption from fatal to recoverable, because an append-only file
survives a crash far better than a mutable index and can be replayed.

- **Crash-safety is configuration, not a feature.** Write-ahead logging and
  synchronous commit supply what a local tool needs and cost nothing to enable.
- **Export is one command producing one portable file.** Nearly free: §XIV's
  contribution path and PR2's repro bundle both need the serialization already, so
  one mechanism serves three purposes.
- **No replication and nothing automatic that leaves the machine.** §3.13 refuses
  the weight and A17 forbids the egress. What a user does with an exported file
  is theirs.
- **Loss is reported with its extent.** Where a replay cannot complete, MCF says
  what was lost and how much, rather than opening quietly with a shorter history
  — §3.1's prohibition applied to the record itself.

The scale this is proportionate to: most installations host a model and
occasionally measure one. A tool that shipped replication for that would be
spending the user's weight on a scenario they do not have.

### D21 — Contributions are dedicated, stated up front, and cannot be withdrawn *(answers §7.48)*

A contribution is offered under a **public-domain dedication**, stated plainly at
the moment of sharing, with **no withdrawal right**.

**Why no withdrawal.** §3.20 already establishes that publication cannot be
undone: once rows leave, they may be copied, indexed and retained by people who
cannot be reached. Offering to withdraw would be a promise MCF cannot keep, and
an unkeepable promise is worse than an honest refusal.

**Why dedication rather than attribution.** Rows from thousands of machines merge
into one corpus, and an attribution requirement that must travel with every row
makes the aggregate nearly unusable. Dedication is the ordinary choice for this
shape of crowd-sourced technical data.

**The one hard requirement: the terms are stated before anything is collected.**
Data gathered under unstated terms cannot be given terms retroactively, and A24
already requires the share screen show what leaves — the terms belong on the
same screen, in one sentence.

*Still open beneath it:* whether a contributor is identified, pseudonymous or
anonymous (§7.27, from the privacy side), and what terms the aggregate itself
carries.

### D22 — MCF is copyleft *(narrows §7.47)*

MCF is distributed under the GPL-3.0 family. Spin-offs and derivative work are
welcome and must stay open; selling remains permitted, but the source travels
with the binary, which extinguishes the close-it-and-sell-it case the author was
guarding against without adopting a non-commercial licence that would have
narrowed what §XVI may vendor.

**The remaining choice is GPL-3.0 versus AGPL-3.0**, and it is genuinely
consequential for this project specifically:

- **AGPL's trigger is network interaction, and MCF's core feature is serving over
  a network.** §6.12 makes LAN exposure a supported capability, so AGPL's
  network clause reaches further here than it would for a desktop application.
- **What AGPL would protect is narrower than it appears.** §5 already declares
  the aggregating website out of scope, so a competing hosted service would be
  its own code consuming contributions rather than a fork of MCF — which AGPL
  would not reach.
- **AGPL carries an adoption cost.** Some organizations refuse AGPL software by
  policy, and those organizations are much of the audience D7 imagines.

On balance GPL-3.0 fits MCF's actual shape — a locally-installed tool — and AGPL
is the right answer only if hosted forks later prove to be a real threat rather
than a hypothetical one.

**An open consequence that the licence creates, recorded because it was not
obvious.** Copyleft interacts awkwardly with proprietary accelerator runtimes.
Vendoring permissively-licensed components is unproblematic — the engines,
tokenizers and format libraries MCF wants are MIT or Apache-2.0, and both are
compatible with GPL-3.0. But vendor inference runtimes are frequently closed, and
§XVI's instruction to *ship* rather than *link against* what MCF needs weakens
the usual system-library argument that lets copyleft software use them. So §XVI
and D22 may collide precisely where MCF wants to support an accelerator, and the
resolution — treat a vendor runtime as a platform capability that is detected and
reported absent (§3.2) rather than shipped — needs stating rather than assuming.
Recorded in §7.47.

### D23 — Three tiers of engine support: vendored, platform-provided, declined *(resolves the sharp half of §7.47)*

Not everything MCF could drive is something MCF may ship. The question is not
whether a component is free to *use* — most accelerator runtimes are — but
whether its terms permit **redistribution inside a copyleft binary** (D22), and
those are different questions that get conflated.

**Tier 1 — Vendored.** Permissively licensed, shipped inside the artifact,
version pinned by MCF. This is the preferred tier and the one §XVI is about. The
engines and libraries MCF principally wants are MIT or Apache-2.0 and sit here
without difficulty.

**Tier 2 — Platform-provided.** Present on the user's machine already, by their
own action or because it arrived with a driver. MCF would detect it, use it, and
not redistribute it — which sidesteps licence compatibility entirely, because
nothing is distributed.

**Tier 2 is deferred, not adopted.** MCF ships a stack it controls end to end,
and anything requiring a component it cannot vendor is **avoided for now and
recorded as a candidate for later** — the same treatment as tier 3, arrived at by
a different route. The reasoning is stated below, because deferring it costs
something real and the cost should not be discovered later.

**Tier 3 — Declined.** Anything requiring a negotiated licence, a payment, or
redistribution terms MCF cannot meet is **not supported**, and that is a normal
outcome rather than a failure (§3.13, B15). It is recorded as declined with the
reason, and revisited if the project ever becomes mature enough that negotiating
is worth someone's time.

### Why a fully-vendored stack, and what it costs

**The scientific argument is the strong one, and it points the same way.** A
stack MCF ships entirely is a stack MCF has pinned entirely: engine, kernels,
math libraries, every version. Every measurement is then taken against
conditions MCF controls rather than conditions it merely records. §3.12 puts
reproducibility above convenience, and this is that principle applied to the
largest available dependency — a tier-2 runtime is an unpinned variable in every
result taken through it, forever.

**It also keeps §XVI honest without a caveat.** One artifact, one behaviour, on
every machine. No user gets a fast path because of what they happened to install,
and no user is quietly slower for lacking it.

**The cost, stated plainly.** The accelerator paths MCF can vendor are the open
ones. On hardware whose vendor-optimized runtime is closed, a vendored path is
frequently slower — sometimes substantially. So:

- **MCF's numbers describe the stack MCF ships, not the hardware's ceiling.**
  This is a §3.4 condition and must travel with every result, because a user
  comparing MCF's throughput against a figure from a vendor-optimized tool will
  otherwise conclude their hardware is slow when what they are seeing is a
  different engine.
- **The comparison MCF is *for* survives intact.** §3.27 already holds that the
  durable output is the comparison rather than the absolute number, and a ratio
  between two configurations measured on one pinned stack is unaffected by that
  stack being slower than some other. What degrades is the absolute figure, which
  was the local, least-transferable quantity anyway.
- **Revisiting is a decision, not a drift.** If the performance gap proves large
  enough to change which model a user should run — which is the only thing that
  would make it matter to §IV — tier 2 is reconsidered on that evidence, with its
  reproducibility cost understood in advance rather than absorbed silently.

**The constraint that keeps §XVI intact: tier 1 is never empty, and at this
revision it is the whole of what ships.** A vendored engine works with no
external dependency, so the common path requires nothing of the user. Should
tier 2 ever be adopted, it is an *accelerated path on top of* that baseline and
never the baseline itself, and its absence is stated and continued past (§3.2)
rather than turned into an errand.

**If tier 2 is ever adopted, its cost is recorded rather than absorbed.** A
runtime MCF did not ship is a runtime MCF did not pin. Its exact version becomes
a *condition* of every measurement taken through it, and since engine identity
already includes the build (§7.34), the surrounding runtime stack belongs there
too: measurements taken against two vendor runtime versions are not the same
configuration.

**On "support all models" (§III).** Declining a tier-3 engine does not weaken
§6.3, which governs *attempt and diagnosis* rather than success. "This model runs
only under an engine MCF cannot distribute, so it cannot run here" is a defined,
actionable outcome and a complete discharge of §III — the same shape as "this
needs 48 GiB and you have 24."

### D33 — Offline is the ordinary case, and MCF reports what it observed rather than which layer is missing *(answers §7.11)*

**Everything except acquisition works with no network at all.** `--version`,
`licence`, `doctor` — the hardware profile, the self-cost measurement, the whole
laboratory — `list`, `rm` and `export` need nothing but this machine. That is
not an aspiration: B-183's from-scratch check runs them in a container with no
network interface, no libc, no shell and no `/etc`, every time it runs. §3.2
suggested *most of it, loudly labelled*; this states it, and the only command
that needs a network is the one that fetches.

**What MCF says when a network is needed and missing is what it saw.**
[findings.md] F10 measured what a platform actually tells a program, and the
deciding row is small: a name that will not resolve produces
`ErrorKind::Uncategorized` — no error kind at all, no errno — whether this
machine has no network, has no resolver, or asked for a name that does not
exist. One observation, three causes, and nothing in the report distinguishes
them.

So MCF reports the observation and names the question it is not answering:
*this machine could not turn that name into an address*, and — explicitly —
*this does not say whether there is no network, no resolver, or no such name*.
A7 forbids the plausible substitute, and the plausible substitute here is
"you're offline", which is wrong exactly when somebody has a mirror they could
have used.

**The three failures that *are* distinguishable are kept apart**, because they
are three different things to act on: a refusal means something answered and
said no, so a path exists; no route means this machine cannot get there at all;
and silence means MCF's own deadline ended the wait rather than the far end.
Two of the three are asserted in the gating tier without a network, because
`.invalid` never resolves and a closed loopback port is always refused.

**§V's wrinkle — no internet is not no local network — is answered by not
asserting it.** MCF could tell the two apart only by making requests nobody
asked for: a second resolver, a known-good address, a ping. §3.2 refuses
unrequested network traffic and §3.13's idle discipline refuses it again, and a
tool that quietly probes to improve its error messages has become a tool that
talks to the network when nobody asked. The distinction is one an operator
draws with `--from`: pointing at a mirror is a request MCF was *told* to make,
and its outcome — reached, refused, or unresolvable — is the same three
observations reported the same way.

**What would reopen it.** A platform that does distinguish the causes of a
failed lookup, in which case MCF should say which it was rather than declining
to. Or a decision that MCF may make an unrequested request — which would be a
change to §3.2 rather than to this entry.

[findings.md]: findings.md

### D32 — MCF delegates inference and owns the wrapper *(answers §7.4)*

**MCF does not implement inference kernels. It drives engines that do, and its
own performance mandate applies to what it adds.** §VII's "fastest possible"
means: the lightest possible wrapper, adding the least possible latency between
a request and a token, over the fastest engine *available here* — and knowing
empirically which engine that is on this hardware, which is what §IV is for.

**The reading §7.4 anticipated is now measured.** [findings.md] F8 multiplies
one matrix by another four ways, each of them safe portable Rust MCF could
maintain, and then through a tuned BLAS on the same machine. MCF's best
single-threaded attempt is **twenty-five to fifty times** slower than one core
of a *generic* specialist kernel; using all thirty-two threads it is still about
ten times slower than that one core. The gap is SIMD microkernels per
instruction set, operand packing and prefetch scheduling — none of it reachable
without per-architecture `unsafe`, all of it moving with every new processor.

F8 records something the reading did not anticipate and which sharpens it: the
*careful* step — cache blocking — came out slower than the one-line loop
reorder. From the bottom of that slope the sign of an optimization is not
obvious, so owning kernels would mean owning a measurement programme for every
kernel on every machine, for ever. That is the treadmill, priced.

**What MCF owns instead, and is held to.** The wrapper is where §VII bites, and
D24 already numbers it: cold start, added request-to-first-token latency, idle
cost, footprint. F8 bounds the other side of that trade — a single 512³ multiply
on a specialist's kernel is over a millisecond, and a real forward pass is
thousands of those, so MCF's few hundred microseconds of cold start and few
microseconds of record write are not where the time goes. A wrapper that is
careless would be visible; MCF's is measured (B-011) and budgeted (D24).

**What this decision is conditional on, and what it is not.**

- **It is not a decision about *which* engine.** D23 settles the terms — MCF
  vendors a stack it controls end to end and defers what it cannot vendor, with
  the reason recorded — and D28 settles the licence that constrains the choice.
  Which engine is admitted first is [vendored.md]'s matrix and B-320's work.
- **It does not make MCF an engine's client at arm's length.** D1's daemon
  supervises engines as child processes (A3), the engine's build is part of a
  configuration's identity (D17), no absolute figure renders without naming it
  (B64), and an engine that dies mid-token is a classified failure with the
  partial output kept (A4). Delegating the arithmetic is not delegating the
  responsibility.
- **It does not contradict D31.** MCF writes a stand-in engine, and B65 forbids
  it from ever reporting a speed. That is the same decision from the other side:
  MCF writes inference for *validity and coverage*, never for speed, so there is
  no first step on the slope this entry declines.
- **What would reopen it.** Evidence that MCF's own wrapper, rather than the
  kernels, is where a user's tokens go; or a platform on which no vendorable
  engine exists at all, which D23 already answers with a deferral and D31 with
  the stand-in.

**What this closes.** D4's substrate reasoning was recorded as *conditional on
§7.4* — Rust was chosen on the assumption that MCF wraps engines rather than
implementing kernels. That condition is now met rather than assumed, and D4
stands on measured ground.

[findings.md]: findings.md
[vendored.md]: vendored.md

### D31 — MCF writes a stand-in engine, and a stand-in cannot produce a timing *(accepts PR8)*

**MCF ships a second implementation of inference: its own, deliberately slow,
written to be read.** A model no vendored engine will run still runs on it,
marked; and where both can run an artifact, each is something the other can be
checked against.

**The second half is the reason, and it is worth stating in that order.** §II's
A19 forbids believing published numbers from software that cannot demonstrate it
computes what it claims — and for inference, the only available demonstration is
a second implementation that agrees. Coverage is what the second implementation
also buys; it is not what justifies it. §III's *any model* is honoured further as
a consequence rather than as the goal, which is the right way round, because
B23 refuses weight admitted for capability and admits it for validity.

**A stand-in cannot produce a timing, and this is a condition rather than a
caveat.** A throughput figure from a naive kernel measures the naive kernel: it
says nothing about the model and nothing about the machine, and publishing one
would be worse than publishing nothing (P1). So a stand-in serves
**behaviour-class** laboratories only (B31) — did the call parse, did the loop
terminate, did the format hold, did the model recover — and the prohibition is
enforced by type, in the way A11 and B37 keep a simulated duration from becoming
a performance number.

That constraint does a second job. It removes the reason this work would drift
into the own-engine question: there is no point optimizing something that can
never report a speed, so the slope from *stand-in* to *our own engine* has no
first step. D32 settles that question the other way and this entry is untouched
by it — MCF's performance mandate applies to its own overhead, not to the
inference kernels — because a stand-in makes no performance claim at all.

**What it does not need is what makes an engine hard.** No SIMD, no fusion, no
threading, no accelerator path, no memory-layout work. Its maintenance is
proportional to *architectures* rather than to hardware, so a new accelerator
costs it nothing — which is exactly the treadmill §7.4 was right to refuse.

**Comparability needs no new machinery.** Intent v16 makes the engine build part
of a configuration's identity (D17), so a stand-in result and a vendored-engine
result are two configurations rather than two readings of one, and A8 keeps them
apart without being asked.

**An artifact is therefore in one of three states**, which is the shape D25 gives
a device and D29 gives a platform: it runs on the vendored engine; it runs on the
stand-in and every result is marked (A5); or it does not run, and MCF says which
component was missing (`engine.unavailable`).

**The cost, stated.** A naive implementation may be two orders of magnitude
slower, so a large model on the stand-in is a matter of hours. B49 already makes
that tolerable rather than fatal: a behaviour-class run's deadline is a token
budget rather than a wall clock, because a task that failed for want of time is
a measurement of the machine. A slow stand-in makes a run long; it does not make
it wrong.

### D30 — Attributability is a property of a reading, not of the machine *(answers §7.51)*

§7.51 recorded a deadlock: D27 refuses to count an unattributable run as a pass,
B38 refuses a release on a stale tier, and F2 found a machine — the one MCF is
written on — where every event-class reading was unattributable all day. Both
rules are right. What was wrong was the question they were being asked.

**The question is not "is this machine busy". It is "was this reading
affected".** B24 already says so — *MCF knows the difference between "this model
is slow" and "this machine was busy"* — and a machine-wide average cannot answer
a question about one measurement. The kernel accounts, per thread, how long it
was *runnable but not running*. Read across a measurement, that is the
contamination, measured directly, on the measurement's own time scale.

**The evidence, and it is not close.** F3 in [findings.md](findings.md) records
it. A hundred cold starts quiet and the same hundred under thirty-two spinning
processes:

| | quiet | under load |
|---|---|---|
| p99 | 360 µs | 4 822 µs |
| one-minute load average | 0.29 | **0.29** |
| scheduling delay, as a fraction of the measurement | 0.019 % | **11.1 %** |

The load average is not a coarse signal. It is a signal on the wrong time
scale: a one-minute average cannot say anything about a 140-millisecond
measurement, and it read identically in both. It is still recorded as a
condition (§3.4) because it says something true about the machine, and it
decides nothing.

**The threshold is one part in a hundred**, stated rather than hidden. Below it,
at most a hundredth of a measured interval was queuing, which cannot move a p99
by the factors F1, F2 and F3 observed; above it, the contention is in the
reading and the reading is about the contention. Three orders of magnitude
separate the two states F3 measured, so the threshold is not a fine judgement.

**What this dissolves.** The deadlock. A budget is asserted whenever the reading
itself was clean, which on an ordinary workstation is most of the time even
while its owner is working — the machine being busy elsewhere does not
contaminate a measurement that was not queuing behind it. MCF does not need to
ask for the machine, and B35's exclusive window stays what it is for: measuring
*models*, where the contention competes for the accelerator rather than for a
scheduler slot.

**What it costs.** The reading is per-platform. A platform that does not account
for it cannot assert a timing budget and says so, which is D29's *attempted,
uncharacterized* applied to a capability rather than to a device.

### D29 — Every platform is in scope; Linux is first, and the rest say what they cannot do *(answers §7.35)*

§7.35 said that until this is drawn, *"runs on this machine" is as unfalsifiable
as §VII was before §7.16*. The author has drawn it: **all platforms, with Linux
taking priority.**

**What "all platforms" commits MCF to, and it is not what it sounds like.** §III
already faced the same shape and B7 settled it: *"any model" commits MCF to
accepting any reference without special-casing, reaching a defined actionable
outcome for every one, and never being damaged by a hostile or malformed one. It
does not commit MCF to running any of them.* The same reading holds here. MCF
runs everywhere, reaches a defined outcome everywhere, and states what it cannot
do on each — it does not promise every capability on every platform.

So a platform is in one of three states, and they are D25's states one level up
because the reasoning is identical:

- **Characterized.** MCF can read what §3.4's floor asks of a machine, contain a
  benchmark by construction (A14), and elevate through a helper for the
  operations that need it (A26). Results are comparable and contributable.
- **Attempted, uncharacterized.** MCF runs, and one or more of those is
  unavailable. It says which, marks every result taken there as degraded (A5),
  and those results are not comparable with characterized ones (A8) and are not
  contributable (B54).
- **Unsupported.** MCF does not run at all, and says so: `platform.unsupported`.
  This state is for a platform MCF cannot start on, not for one where it can
  only do less.

**Linux is first, and "first" is a schedule rather than a tier.** Every
capability is built and proven on Linux before it is attempted elsewhere,
because that is where the author works and because a capability that has never
worked anywhere is not a portability problem. A platform reaching *characterized*
later is the normal path, not an exception.

**Per-platform artifacts are the expected shape, not a compromise.** The target
triple is already a §3.4 condition — it is in every record MCF writes, through
`BuildIdentity` — so a result taken on one platform already declares which. That
means separate artifacts cost nothing in comparability: a run is *already*
qualified by its operating system, and shipping one binary per platform simply
matches the artifact to the qualification that was going to be recorded anyway.
§XVI is unaffected: each artifact is self-contained on its own platform, which
is what §XVI asks, rather than one artifact being self-contained on all of them.

**The mechanism §7.35 warned about is where the cost actually sits.** A14
requires the benchmark sandbox be a sandbox *by construction*, and the
construction differs: namespaces and cgroups on Linux, sandbox profiles and
`seatbelt` on macOS, job objects and AppContainer on Windows. §XVII's privileged
helper multiplies it, since elevation is per-platform. Three consequences follow
and are stated rather than discovered:

- **The sandbox is an interface with per-platform implementations, and the
  interface is the narrow part.** A14's test is that a capability be *absent*
  rather than present-and-disabled, so the interface describes what the
  environment *lacks*, and a platform that cannot remove a capability cannot
  offer that environment — it is uncharacterized for the laboratories that need
  it, and says so.
- **A platform without a containment mechanism does not run untrusted code at
  all.** A15 and A14 are absolute, and *degrade and say so* (§3.2) is not
  available for a rule that admits no exception. The capability is refused on
  that platform, which is a stated absence rather than a weaker sandbox.
- **DEC-039's list is per-platform** and gets a column per platform rather than
  one answer, and §6.32's *reconsider rather than grant* applies per column: a
  helper that would need broad rights for a narrow job on one platform does not
  get them there, and the measurement is marked untaken on that platform alone.

### D28 — MCF is GPL-3.0-only *(closes the licence half of §7.47)*

**GPL-3.0-only.** D22 narrowed §7.47 to the GPL family and reasoned its way to a
lean; the author has taken it. `LICENSE` holds the verbatim text and every
manifest declares `GPL-3.0-only`.

**Why not AGPL, restated so the choice survives the decision.** AGPL's trigger
is network interaction and MCF's core feature is serving over a network, so its
clause reaches unusually far here — while what it would protect is narrower than
it looks, since §5 already puts the aggregating website out of scope and a
competing hosted service would be its own code rather than a fork of MCF. Set
against that, AGPL carries an adoption cost with organizations that refuse it by
policy, and those are much of the audience D7 imagines. The clause would cost
reach and buy little.

**What this settles for §XVI.** Every component MCF vendors must be
GPL-3.0-compatible. Permissive terms — MIT, Apache-2.0, BSD — are, which is
where the engines and libraries §XVI wants already sit (D22). A component whose
terms are not is one MCF cannot ship whatever its merits, and D23's third tier
is where it goes, recorded with the reason (B-321).

**What it does not settle.** The per-engine compatibility matrix is still owed:
each vendored component is checked against this licence *before* it is admitted,
and the finding is recorded rather than assumed (B-330). §7.47's remaining half
is that matrix, not the licence.

### D27 — A budget names a statistic, a window and a quiet machine *(answers §7.50)*

D24 gives sixteen figures and names a statistic for one of them. F1 in
[findings.md](findings.md) showed what the omission costs: twenty cold-start
trials on a quiet machine gave a median of 3.9 ms and a p95 of 9.6 ms, and the
same twenty on a machine that was compiling gave a passing median and a p95 over
the ceiling by a factor of two. One run, two verdicts, from a document that
states one number.

**Every figure is one of three kinds, and the kind decides the statistic.**

| Kind | Statistic | Which of D24's figures | Why |
|---|---|---|---|
| **Prohibition** | The maximum, which must be exactly zero | Timer wakeups while idle; external requests from the interface | D24 already calls these prohibitions rather than thresholds. One occurrence is a violation, and a percentile would be a way of tolerating some |
| **Ceiling on state** | The maximum over the window | Resident memory (both figures); installed footprint (both); memory growth over 30 days; daemon CPU with an idle tab open | These bound a quantity that *is*, not a quantity that *happens*. Memory that exceeded the ceiling once exceeded it |
| **Ceiling on an event** | The 99th percentile over at least 100 trials, with the median reported beside it | Cold start; added request-to-first-token latency; record write per event; cold render | D24 says it for one of them and the reason generalizes: *the tail is what a user feels*. A median hides exactly the behaviour a budget exists to prevent |
| **Rate over a window** | The mean over the stated window | Idle CPU over 60 s | D24 states the window in the figure itself, and a rate has no meaningful percentile without one |

**Why one percentile rather than several.** Choosing p95 for some figures and
p99 for others would be inventing variety D24 does not have. p99 is the one it
names, and the cost of adopting it everywhere is a sample count: a p99 of twenty
trials is the maximum wearing a percentile's name, so an event-class figure is
asserted over at least a hundred. At the magnitudes involved — milliseconds —
that is affordable, and where it is not the honest answer is to report the
figure as not asserted rather than to assert it from too few trials (§3.4).

**A budget is asserted only on an attributable run.** B35 holds that a timing
taken under contention measures the contention, and B24 makes *unattributable* a
verdict rather than a gap. So the suite reads the machine's load alongside the
measurement, records it as a condition (§3.4), and **marks a run unattributable
rather than failing it** when the machine was not quiet. A budget cannot be
violated by somebody else's compile.

That creates one hazard and it is closed rather than accepted: a real regression
could hide behind a permanently busy machine. So an unattributable run does not
count as a pass. The budget tier reports its most recent *attributable* run, and
B38's staleness discipline applies — a tier whose last attributable run is old
is **stale**, never green, and a release on a stale tier is refused with the age
stated.

**What this does not settle.** The numbers themselves stay D24's, and they stay
ceilings rather than targets: a budget that is merely met has not been
optimized. This says which reading is compared against them.

### D26 — The laboratory simulates what MCF observes, and its clock is structural *(answers §7.21)*

§7.21 asked three things: what the laboratory must simulate, what it declines
to, and whether simulated time is structural. The first has been answered since
[taxonomy.md](taxonomy.md) existed, and the other two follow from it.

**The scope is the taxonomy, and it is enumerable rather than a judgement.**
A13 already holds that the fault catalogue and the failure taxonomy are the same
list; §7.21 was open because the list did not exist. It does now — sixteen
domains, 110 codes — so *what the laboratory must simulate* is not a scope to
argue about but a table to work through.

**A13 binds a claim, not a code.** *An untested claim is not made* is the rule,
and MCF claims a category when its code can produce one. So the obligation is:
**every category MCF's own code constructs has a scenario that produces it**, and
the check fails the build when one does not (B-010). A category in the taxonomy
that nothing yet constructs is a classification waiting for the code that will
use it, and the code and the scenario arrive together. This makes A13 a ratchet
rather than a cliff: the coverage that matters is complete from the first day,
and it stays complete because a new failure site cannot land without its
scenario.

**What the laboratory declines, stated because §6.16 requires the boundary be
stated.**

- **It simulates what MCF observes, never what causes it.** A thermal ceiling is
  injected by making a probe report a throttled device, not by heating one. A
  torn journal is a file written short, not a power failure. The laboratory's
  subject is MCF's *response*, and the cause is out of scope — which is exactly
  why A12 puts reality above it: when the simulator and real hardware disagree,
  the world is right and the divergence is a recorded finding about the
  simulator.
- **It produces no performance number** (A11). The laboratory tests behaviour.
  A duration from its clock is a different *type* from one from the monotonic
  clock (B37), so this is a compiler check rather than a convention.
- **It does not model a vendor stack's internals.** A driver that crashes, a
  kernel that miscomputes, a device that returns wrong numbers: MCF can simulate
  *being told* any of those and cannot simulate the thing itself.
- **It does not model the world's timing.** Simulated time makes a scenario
  deterministic; it does not make it representative. A scenario that says "the
  transfer stalled for thirty seconds" is a statement about what MCF then did,
  never about how often that happens.

**Simulated time is structural, and this is the part that had to be decided
early.** §3.17 wants a failure found once to reproduce exactly, for ever, and a
scenario whose outcome depends on how busy the machine was is not that. The
consequence is a constraint on ordinary code rather than on tests: **anything
that waits, times out or measures an interval takes its clock rather than
reaching for one**, so a scenario can supply the simulated one. Retrofitting
that is a rewrite of every deadline in the system, which is why §7.21 called it
structural rather than a testing convenience.

The same shape governs the other injection points. A fault is injected at a
**seam** — the clock, the accelerator route, the process supervisor, the
filesystem, the hub — and the seams are declared, not discovered: B19 already
requires expensive paths be reachable through them, and this makes the same list
the laboratory's injection surface. A subsystem with no seam is a subsystem the
laboratory cannot reach, and that is a design defect rather than a coverage gap.

### D25 — A device is characterized when MCF can read its live state *(answers §7.8)*

§7.8 asks where the boundary falls between *supported and characterized* and
*will attempt, uncharacterized*, and observes that §IV's meaning changes
completely depending on where it falls. **The boundary is a capability of the
observer, not a property of the vendor.**

**Why it cannot be a list of vendors.** §3.8 requires MCF know the difference
between "this model is slow" and "this machine was busy", and that difference is
made of readings: available device memory, thermal and throttle state, and what
else is competing. A vendor allowlist would say which devices MCF *approves of*
and answer nothing about whether it can tell those two apart on any of them. It
would also be a special case in code, which B28 refuses on principle and which
tends to make one artifact work and quietly break the next.

**The three states, and what each licenses.** A device is in exactly one of
them, for each run:

- **Characterized.** MCF can read, at measurement time: the device's identity,
  its driver and runtime versions, its total and available memory, and its
  thermal or throttle state. Results taken on it carry those among their
  conditions (§3.4), are comparable with other characterized results under A8,
  and are contributable (B54).
- **Attempted, uncharacterized.** The device is present and MCF can run on it,
  and at least one of those readings is unavailable. MCF uses it and marks every
  result [`Degraded`] with the reading it could not take (A5). Such results are
  not comparable with characterized ones and are not contributable, because
  B54 requires the full condition set and §3.4 has a hole in it.
- **Absent.** There is no accelerator. Results are processor-derived and marked
  as such (§3.2). This is a result, not a failure (A9).

**The state is per run, not per install.** §3.8 makes hardware a time-varying
condition rather than a static fact read once, and the consequence is
uncomfortable and correct: a device that was characterized this morning and
whose driver query fails this afternoon is *uncharacterized for this run*.
Anything else would let a stale reading stand in for a live one, which is A7's
plausible substitute wearing a cache.

**What this rests on.** The three states derive from §3.8, §3.4, A5, A7 and A9.
The empirical half comes from F1 in [findings.md](findings.md), and it is what
makes the definition non-trivial: on the machine tested, the files a driver
publishes give the device's *identity* and none of its *live state*, which is
reachable only over the C ABI. A profiler built on published files alone would
therefore report every device as uncharacterized under this definition — which
is the honest outcome, and is why the definition is worth having rather than
being satisfied by whatever MCF happens to implement first.

**What follows, and is not deferred.** Support for a vendor means one thing:
a probe route exists that declares which of the four readings it can supply.
Routes are enumerable, each states its own coverage, and a device is
characterized when some available route covers all four. Adding a vendor is
adding a route, never a branch in a measurement path.

[`Degraded`]: §3.2

### D24 — The performance budget *(answers §7.16)*

Numbers, so that §VII can be defended rather than invoked. Each is asserted by
the suite (B20) and fails the build on regression.

**MCF's own cost**

| Quantity | Budget | Reasoning |
|---|---|---|
| Idle CPU, 60 s average | **≤ 0.05 %** | B4 makes recording event-driven, so idle work should be indistinguishable from none |
| Timer wakeups while idle | **exactly 0** | The number that should embarrass us first (§3.13). Not a threshold — a prohibition |
| Resident memory, nothing loaded | **≤ 20 MiB** | A Rust daemon with an embedded store, no interpreter and no GC |
| Resident memory above the engine, model resident | **≤ 30 MiB** | MCF's overhead, not the model's footprint |
| Cold start to first command response | **≤ 100 ms** | Static binary, process spawn plus opening a file |
| Added latency, request to the engine's first token | **≤ 5 ms at p99** | An HTTP hop and a routing decision. Tail, not mean — the tail is what a user feels |
| Record write, per event | **≤ 2 ms** | An append to a journal (D20) |
| Memory growth over 30 simulated days | **≤ 1 MiB** | A daemon that must run for months |

**Installed footprint** is budgeted separately, because §6.31 holds that §VII
governs behaviour rather than download size and one number cannot honestly do
both:

| Quantity | Budget |
|---|---|
| Core binary, no engines | **≤ 40 MiB** |
| Complete artifact, engines vendored (D23, §XVI) | **≤ 250 MiB** |

**The interface** (§V, §6.11)

| Quantity | Budget |
|---|---|
| Document, gzipped | ≤ 40 KiB |
| JavaScript | 0 by default; any is a budgeted addition with a stated reason |
| External requests | exactly 0 |
| Cold render on the reference client | ≤ 400 ms |
| Daemon CPU with an idle tab open | ≤ 0.02 % |
| Daemon wakeups with an idle tab open | exactly 0 |

**The reference client**, which §6.11 requires be bounded honestly: a browser on
a mid-range phone roughly ten years old, over local Wi-Fi. A decade-old laptop is
in scope. A smart fridge is not a commitment this project makes.

**Two honest notes.** The footprint figures are the least certain here, because
§XVI's vendoring and D23's tiering decide them and neither has been built — D4's
adversarial prototype exists partly to check them, and this entry is amended if
it goes badly rather than defended. And every figure above is a **ceiling, not a
target**: a budget that is merely met has not been optimized, and §3.13's
accounting discipline is about the direction of travel, not about passing.

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

### 3.26 MCF is a guest on the user's machine

§1 already says MCF is a support structure around the thing the user actually
wants to run, and that a support structure consuming what it supports has
failed. The same sentence applies one level out: **a tool that makes the machine
unusable while it works has failed, however good its numbers are.** These
machines are daily drivers, not benchmark rigs, and the person operating them
has other things to do on them.

The spirit:

- **Hosting always yields to the user, and nothing yields to hosting.** A served
  endpoint is a service somebody is using; no measurement, no laboratory and no
  background work may make it slow or unavailable except inside a declared
  exclusive window (D8).
- **Take what the measurement requires, for as long as it requires, and not one
  minute more.** Greed is licensed by validity, never by importance.
- **Default to the polite mode and let the user escalate.** Low priority,
  yielding, interruptible, pausable. The user may grant more; MCF never assumes
  it.
- **Announce before, not after.** A user should never discover that MCF took the
  machine by noticing their machine is gone.
- **Every interruption is survivable.** A paused run resumes or reports what it
  had (A4), and stopping is always available and always restores (A27).

The test this principle is meant to survive: *can the user keep working while
MCF works?* Where the answer must be no, the window is short, announced and
chosen by them.

**And the answer is measured, not asserted.** Politeness is a claim like any
other, and it has two costs that are measured separately because they are
different disciplines (D10): what yielding costs the *user* is an application
test — does a background run stutter an interactive workload — and what
constrained resources cost the *model* is a laboratory (L24). Neither is
established by intending to be considerate.

### 3.27 The comparison is durable; the absolute number is local

An absolute measurement — 38.4 tokens per second — is bound to the machine, the
moment, the thermal state and whatever else was running. It is true, it is
conditioned, and it travels badly. A *comparison* taken under the same
conditions — this configuration was 1.31× that one — survives all of those,
because whatever perturbed one arm perturbed the other.

This is the strongest available answer to the problem that MCF runs on machines
nobody controls: **common-mode noise cancels in a ratio and accumulates in a
scalar.** It follows that the comparison, not the number, is MCF's durable
output.

The spirit:

- **Compare by pairing, not by recall.** Arms are **interleaved** within one
  session — A, B, A, B — rather than run in blocks, so drift in temperature,
  contention or clock affects both equally. Order is randomized so that going
  first is not an advantage.
- **Report the paired difference and its distribution**, not the difference of
  two means. Thirty paired differences carry information that two summaries have
  already destroyed.
- **A comparison assembled from separate sessions is weaker and says so.** It
  may be all that exists, and it is not the same claim.
- **Absolute numbers stay, and stay conditioned.** They answer "will this fit in
  my latency budget", which a ratio cannot. They are simply not the thing that
  travels.
- **What is contributed is chiefly the comparison** (§XIV). A tokens-per-second
  figure from a stranger's machine is nearly uninterpretable; *"on hardware like
  this, this configuration beat that one by roughly this much"* is exactly what
  another user needs, and it is far more robust to the messiness of the machines
  it came from.

The corollary is a caution rather than an exception: a ratio measured at one
level of contention need not hold at another, because degradation is not
uniform — a configuration that spills to host memory falls off a cliff that a
resident one does not. Trends are durable *within* a regime and can invert
across one, which is precisely what L25 exists to map.

### 3.28 Gather precisely; recommend generally

Two halves of the same instrument, held to opposite standards:

**Inward, toward the record: as accurate as the machine allows.** Every
condition captured, arms interleaved (§3.27), raw samples kept, boxes and
windows used where they buy validity, uncertainty carried, contention recorded
rather than ignored. Precision is cheap to keep and impossible to recover, so
nothing is rounded away at the point of capture.

**Outward, toward the user: as general as the evidence supports.** A
recommendation speaks in the terms that survive — *meaningfully faster*, *within
noise*, *about a third less memory*, *fails on this workload* — rather than in
decimal places. False precision is not honesty; it is noise wearing the costume
of rigour, and it invites a user to act on a difference that does not exist.

The two are reconciled by direction, not by compromise:

- **Generalization happens in the rendering, never in the record.** MCF stores
  the precise thing and *renders* the general one. A summary that overwrote its
  own evidence has destroyed the ability to re-ask the question.
- **The precision is always one step away** (§3.15). Any generalized statement
  can be expanded into the measurements, conditions and spread behind it, on
  demand, without leaving the interface.
- **The generalization states its own strength.** "Meaningfully faster" carries
  the effect size and the confidence behind it, because a claim whose robustness
  is hidden is a claim the user cannot weigh.
- **Where the evidence supports nothing general, MCF says nothing general.**
  §3.9 already requires "these are within noise, pick either", and that is this
  principle's honest floor rather than a failure of it.

This is also why §3.4's floor and §3.14's "minimal chrome, never minimal truth"
are not in tension: the interface may show less, provided nothing was thrown
away to let it.

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

**Tension.** D32 settles that MCF wraps mature inference engines rather than
implementing kernels, and D4 accepted Python conversion tooling as supervised
subprocesses. §XVI forbids making the user fetch either. Meanwhile
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

### 6.33 Exclusive windows vs. a persistent, dependable endpoint

**Tension.** D8 gives a timing-class run exclusive use of the machine. §VI
promises a persistent local service and §I promises MCF is the calm component. A
user whose endpoint stops answering has been failed by both.

Note the scope this has since narrowed to: only timing-class work suspends
anything, and it is measured in tens of minutes. Behaviour-class runs — the long
ones — yield instead, and never suspend the endpoint at all.

**Resolution — suspension is a declared state, never an outage.**

- **The daemon stays up and keeps answering** — about itself. A request arriving
  during a lab receives an immediate, explicit refusal naming the lab, the
  reason, and the expected remaining time. It is never queued into a timeout,
  never silently slowed, and never dropped.
- **Opening an exclusive window while serving is a decision the operator
  makes**, with what will be suspended stated before it begins, and the option
  to schedule it for a time that suits them instead.
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

### 6.40 Long evaluations vs. a machine somebody is using

**Tension.** §XIII's catalogue contains runs measured in hours and days. §3.26
says MCF is a guest and §VI promises the endpoint stays up. On a dedicated rig
these do not conflict; on the average machine MCF is actually installed on, an
evaluation that owns the hardware for a day is a tool nobody runs twice.

**Resolution — the long runs do not need the machine, so they do not take it.**

- **Behaviour-class work yields** (D8): low priority, behind user traffic on the
  serving path, pausable, resumable, with contention recorded rather than
  prevented. Its validity does not depend on a quiet machine, so nothing is
  bought by taking one.
- **Timing-class work opens a short window** and is schedulable — overnight,
  during a break, or when the machine has been idle for a stated period. Tens of
  minutes, announced, bounded.
- **Progress survives interruption** (B47). A day-long run is a sequence of
  completed trials, each recorded as it finishes, so a user who needs their
  machine back loses nothing but the trial in flight.
- **The user can always look, and always stop** (A27, B-210).

**Why this is not a compromise.** It follows from §6.25 rather than softening
it: exclusivity was never a property of laboratories, only of timings, and the
earlier reading generalized a real requirement past its evidence. The practical
consequence is favourable — the runs that need quiet are short, and the runs
that are long do not need quiet.

**Confidence: high on the split, medium on the yielding mechanism.** Making a
background run genuinely unobtrusive on a contended machine is real engineering,
and a "low priority" that still stutters a game is a broken promise. §7.42
records what has to be decided.

### 6.41 Resource boxes vs. the exclusive window

**Tension.** D15 looks like it makes D8's exclusive window unnecessary: if a
model can be confined to a declared allocation, why suspend the machine to time
it? The reasoning is attractive and wrong, and getting it wrong would put a
reproducible number on an unreproducible quantity.

**Resolution — a box controls the *allocation*; only a quiet machine controls
the *conditions*, and the difference is exactly the dimensions that dominate
inference.** Memory bandwidth, cache pressure, accelerator time-slicing and
thermal headroom are what determine tokens per second, and they are the
dimensions a consumer box does not partition.

So both survive, answering different questions:

- **Best-case timing** — what this machine can do — requires the exclusive
  window. It is the number that belongs in a comparison between models or
  quantizations, because it is the one taken with the fewest uncontrolled
  variables.
- **In-practice timing** — what the user gets while working — is taken in a box
  under a *stated* load, and is a first-class result rather than a spoiled one.
  It answers the question most users actually have, and §3.19's
  resemble-the-work argument favours it.
- **The two are never compared,** and a surface that renders them in the same
  column has produced a confounded comparison (A8).

**What §3.27 does to this.** If the durable output is the paired comparison
rather than the absolute number, the exclusive window's job narrows considerably.
It is required for **small effects** that within-session noise would swamp, and
for **numbers intended to leave the machine** where reproducibility is the
point. It is *not* required for the everyday question — is A better than B here
— because interleaving already cancels what the window would have excluded.

That reordering is worth stating plainly: **most of what a user wants does not
need a quiet machine at all.**

**Confidence: high on the split, medium on how much a box helps at all.** On
hardware with real partitioning the box may approach the window; on a consumer
machine it may bound very little of what matters. That is measurable rather than
arguable, and L24 measures it.

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

### 7.42 What yielding actually means

D8 and §6.40 rest on a background run being genuinely unobtrusive, and neither
says how. Process priority, accelerator scheduling priority, memory reservation,
throttling on foreground activity, and pausing outright are different mechanisms
with different guarantees, and accelerator work is the hard part: a submitted
kernel does not yield mid-flight, so granularity is bounded by how small the
work can be cut.

Open: what "yielding" guarantees the user, how MCF detects that the user is
active without ambient polling (D5), whether pausing is automatic or offered,
and how any of it is expressed per platform (§7.35).

A "low priority" that still stutters a game is a broken promise, and §3.26 makes
this a correctness question rather than a comfort one.

### 7.43 What can actually be boxed, per platform

D15 declares boxes and §6.41 bounds what they buy. Neither says which dimensions
are enforceable where. Core pinning, memory ceilings, I/O limits, accelerator
share and priority have different mechanisms and different guarantees across
operating systems, and some have no mechanism at all — and where a mechanism
exists it may require privilege (§XVII), which changes the surface (§7.39).

Open: which dimensions MCF offers, what it reports where a dimension cannot be
enforced, whether a partially-enforceable box is offered at all or refused as
misleading, and how any of it interacts with §7.35's platform scope and §7.42's
yielding.

The trap to avoid is a box that appears to isolate and does not, which would
attach a reproducible-looking number to an unreproducible quantity — the §6.1
failure mode with a configuration screen in front of it.

**Candidate answer, stated here rather than resolved.** §3.27 substantially
defuses this. A partial box misleads mainly by licensing a comparison it cannot
support — chiefly across sessions, where a user assumes conditions were held
because a box was set. If comparisons are paired and interleaved within a
session (§3.27), the box is never what holds conditions still, and its
partiality stops mattering for the claim being made. That suggests: **offer
partial boxes, name exactly which dimensions they bound, and forbid a box from
being the sole basis of a cross-session comparison.** The remaining decision is
whether any dimension is so weakly enforced that offering it is worse than
refusing it.

### 7.44 How a user declares a workflow — **blocking §6.36**

§6.36 resolves that MCF ranks against a *declared workflow* and refuses to rank
in general, and B41 encodes it. Neither says how a workflow is declared, or what
the vocabulary is.

The options are not equivalent. A fixed list of named workflows is legible and
immediately usable, and wrong for everyone whose work sits between two entries.
A weighting across laboratories is exact and asks the user to have opinions
about instruments they have never run. Inferring it from the user's own traffic
is the most accurate and the most invasive (§3.10), and inferring it from an
imported configuration assumes the emitter's work resembles theirs.

Until this is answered, §6.36's promise — *MCF answers plainly within a declared
workflow* — has no surface, and §6.5's "explicit, visible, overridable default"
has nothing to default to. It is the practical half of §7.2's objective
function, and probably should be answered with it.

### 7.45 What happens when the machine changes underneath

§7.13 asks whether measurements survive MCF's own upgrades. Nobody has asked the
same question about the hardware, which changes far more often: a driver update,
a firmware revision, a second stick of memory, a swapped accelerator, a repasted
cooler, a different power profile.

§3.4 makes the machine a condition of every measurement, so a changed machine
means yesterday's results describe an apparatus that no longer exists. Open:
which changes invalidate comparability and which are cosmetic, whether MCF
detects them at all (a profile diff against the last known state is cheap), what
it does on discovery, and whether history is marked, partitioned, or left to the
user to interpret.

The failure mode is silent and slow: a user compares a result from before a
driver update with one from after and reads the driver's effect as the model's.
§3.1 forbids exactly that kind of quiet corruption.

### 7.47 MCF's own licence, and the licences it inherits by shipping — **structural**

Nowhere in this document does MCF state what licence it is distributed under,
which is an odd omission for a project D7 commits to putting in other people's
hands.

The larger half is inherited rather than chosen. §XVI requires MCF ship
everything it needs, and D32 has it driving inference engines it did not write. **Vendoring an engine means inheriting that engine's licence
obligations** — permissive for some, copyleft for others, and the difference
propagates into what MCF itself may be. The same applies to tokenizers,
quantization tools and anything else the self-contained artifact carries.

This is structural because it decides an architectural question: an engine whose
terms are incompatible with MCF's intended licence is an engine MCF cannot ship,
whatever its merits. D28 settled the licence and D32 settled the ownership
question afterwards, in that order and for that reason. It also decides whether per-accelerator builds (§6.31) are one
artifact or several under different terms.

**Stated intent, recorded ahead of the choice.** MCF is to be as open as
possible. Derivative work and spin-offs are welcome. There is a mild preference
against others selling it, held loosely and by the author's own account unlikely
to matter.

**The tension in that, which decides the answer.** "As open as possible" and
"nobody may sell it" are different goals, and every licence recognised as open
source permits commercial use. A non-commercial licence is available but costs
more than it appears to: it is incompatible with copyleft components, so it would
*narrow* what §XVI may vendor rather than widen it; "commercial" is famously
ill-defined in practice; and it would forbid the ordinary case of a company using
MCF internally to choose a model, which is much of the audience D7 imagines.

The concern behind "keep people from selling it" is usually narrower than the
words — someone taking the work, closing it, and selling it without giving
back — and copyleft answers that while remaining open and commercially usable.
So the real choice is between two positions, both of which honour the stated
intent:

- **Permissive** (Apache-2.0). Maximum reach, an explicit patent grant, and
  anyone may build anything on it including proprietary forks. This is "as open
  as possible" read literally.
- **Copyleft** (GPL-3.0, or AGPL-3.0). Spin-offs are welcome and must stay open.
  Selling remains permitted, but the source travels with it, which extinguishes
  the close-and-sell scenario in practice. AGPL additionally covers the hosted
  case — relevant because §XV names a website — by requiring a service built on
  MCF to publish its source.

Either is compatible with vendoring the permissively-licensed components §XVI
needs. Neither is a legal opinion; the choice deserves a real review before
distribution, and the point of recording it here is that it must be *made* before
§7.4 picks an engine.

**Narrowed by D22 and closed by D28: GPL-3.0-only.** What remains open:

- ~~**GPL-3.0 or AGPL-3.0.**~~ **Resolved by D28:** GPL-3.0-only. `LICENSE`
  holds the text and every manifest declares it.
- ~~**Proprietary accelerator runtimes.**~~ **Resolved by D23:** vendored,
  platform-provided, or declined. A closed runtime is used where the user already
  has it and never redistributed, which sidesteps compatibility rather than
  arguing about it — and tier 1 is never empty, so the common path still requires
  nothing of the user.
- The compatibility matrix of every candidate engine, and how obligations are
  surfaced to a user who redistributes.

### Retired voids

Answered, and their substance moved to §2.1 per §8. The numbers stay citable.

| Void | Question | Answered by | Substance lives in |
|---|---|---|---|
| §7.1 | What "deployed" means | §VI | **D1** — MCF is a daemon |
| §7.3 | What quality is measured against | §IX | **D2** — quality is plural, measured per workflow |
| §7.12 | The user surface | §XI | **D3** — both surfaces, headless primary |
| §7.19 | Implementation substrate | §3.16 | **D4** — Rust |
| §7.33 | Raw samples or summaries | §6.17, §3.27 | **D16** — raw trials, always |
| §7.46 | Sampling: constant, identity, or lab-pinned | §3.18, D13 | **D18** — identity; recommendation verified by sweep |
| §7.34 | The identity of a measured configuration | §XIV, §XV | **D17**, **D18**, **D19** — the runnable configuration; hardware and seed are conditions |
| §7.48 | What rights a contribution carries | §3.20, §XIV | **D21** — dedicated, stated up front, no withdrawal |
| §7.49 | What protects the record from loss | §3.1, D6 | **D20** — a rebuildable index over an append-only journal |
| §7.16 | The performance budget | §VII, §3.13 | **D24** — the numbers, asserted in CI |
| §7.10 | Failure taxonomy | §3.1, §3.17 | [taxonomy.md](taxonomy.md) — three axes, sixteen domains |
| §7.15 | Success beyond the author | §XIV, §XV | **D7** — MCF is for other people |
| §7.8 | Hardware scope | §3.8, §3.4 | **D25** — characterized means MCF can read the device's live state |
| §7.21 | What the laboratory simulates | §3.17, §6.16, A13 | **D26** — the taxonomy, observed rather than caused; the clock is structural |
| §7.35 | Host platform scope | §I, A14, §XVI | **D29** — all platforms, Linux first; three states, per-platform artifacts |
| §7.51 | Asserting a budget on a used machine | B24, B35, D27 | **D30** — attributability is a property of a reading, measured as scheduling delay |
| §7.50 | Which statistic a budget names | §VII, D24, B20 | **D27** — three kinds of figure; p99 for events; an unattributable run is not a pass |
| §7.4 | Engine ownership | §VI, §VII, §IV | **D32** — delegate the kernels, own the wrapper; measured in [findings.md](findings.md) F8 |
| §7.11 | Offline and degraded-network operation | §3.2, §V | **D33** — offline is the ordinary case; MCF reports what it observed, never which layer is missing; measured in [findings.md](findings.md) F10 |

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

### Version 36 — offline is the ordinary case

D33 added, closing §7.11. Two halves, and the first needed no argument: the
from-scratch container has been running everything except acquisition with no
network at all since B-183, so *how much works offline* is a thing MCF already
demonstrates on every scheduled run rather than a thing to decide.

The second half turned on a measurement. [findings.md](findings.md) F10 asked
what a platform tells a program when a name will not resolve, and the answer is
*nothing useful*: no error kind, no errno, the same report whether the machine
has no network, no resolver, or asked for a name that does not exist. MCF
therefore says what it observed and names the question it is not answering — and
declines to tell the two conditions §V distinguishes apart, because the only way
to tell them apart is to make a request nobody asked for, which §3.2 forbids for
better reasons than a nicer error message.

§7.11 joins the answered table, where its number stays citable.

### Version 35 — the engine question is measured and answered

D32 answers §7.4, which this document called the most consequential unanswered
question in it. The answer is the one §7.4 named as likely — MCF delegates
inference and owns the wrapper — and it is recorded now because it stopped being
a reading and became a measurement.

[findings.md](findings.md) F8 is that measurement. One matrix multiply, four
ways in safe portable Rust, then through a tuned BLAS on the same machine: MCF's
best single-threaded attempt is twenty-five to fifty times slower than one core
of a *generic* specialist kernel, and all thirty-two of this machine's threads
are still about ten times slower than that one core. §7.4 asked not to be
settled on a paragraph, and it has not been.

The prototype also found what the paragraph could not: the careful optimization
step — cache blocking — came out *slower* than the one-line loop reorder. That is
the treadmill seen from the bottom, and it prices the alternative more honestly
than the headline ratio does.

Three things this does not decide, stated in the entry so they are not inherited
by accident: which engine is admitted first (D23's terms, D28's licence,
[vendored.md](vendored.md)'s matrix, B-320's work); that delegating the
arithmetic delegates any responsibility (it does not — supervision, identity and
failure classification are unchanged); and anything about D31's stand-in, which
is the same decision from the other side.

D4's substrate entry recorded itself as *conditional on §7.4*. That condition is
now met rather than assumed.

### Version 34 — MCF writes a second implementation

D31 accepts PR8. MCF ships a stand-in engine of its own — deliberately slow,
written to be read — so that a model no vendored engine will run still runs, and
so that the vendored engine has something to be checked against.

The order of those two reasons is the whole decision. Coverage sounds like
capability and would have run into B23, which admits weight for validity and
refuses it for capability. What actually justifies the work is A19: nobody
should believe published numbers from software that cannot demonstrate it
computes what it claims, and for inference the only available demonstration is a
second implementation that agrees. Coverage is what that also buys.

The condition of acceptance is a prohibition — **a stand-in cannot produce a
timing** — enforced by type in the way A11 and B37 keep a simulated duration from
becoming a performance number. A throughput figure from a naive kernel measures
the naive kernel. The prohibition also removes the reason this drifts into
§7.4's own-engine question: there is no point optimizing something that can
never report a speed.

### Version 33 — attributability is about the reading, not the machine

D30 answers §7.51, and dissolves it rather than trading it off. The deadlock was
real — D27 will not count an unattributable run as a pass, B38 will not release
on a stale tier, and F2 found a machine where every reading was unattributable
all day — but both rules were being asked the wrong question. B24 had already
said which question was right: MCF knows the difference between a slow model and
a busy machine, and *that* is a statement about a reading.

The evidence is in F3 and it is not close. Over the same work, the one-minute
load average read 0.29 quiet and 0.29 under thirty-two spinning processes, while
the scheduling delay read 0.019 % and 11.1 %. The load average is not a coarse
signal; it is a signal on the wrong time scale, and it now decides nothing while
remaining a recorded condition.

What this dissolves is the deadlock: a budget is asserted whenever the reading
itself was clean, which on a workstation is most of the time even while its owner
is working. MCF does not have to ask for the machine, and B35's exclusive window
stays what it was written for — measuring models, where the contention is for the
accelerator rather than for a scheduler slot.

### Version 32 — every platform, with Linux first

D29 answers §7.35, which said that until it was drawn, *"runs on this machine"*
was as unfalsifiable as §VII had been before its budget existed.

The answer is *all platforms, Linux first*, and the work was in reading what
"all" commits MCF to. §III faced the same shape and B7 settled it: coverage
governs **attempt and diagnosis, not success**. So a platform gets D25's three
states — characterized, attempted-uncharacterized, unsupported — for the same
reason a device does, and MCF states what it cannot do on each rather than
promising every capability everywhere.

Two things follow that are worth having written down. **Per-platform artifacts
cost nothing**, because the target triple is already a §3.4 condition in every
record MCF writes: a run is already qualified by its operating system, so
shipping one binary per platform matches the artifact to a qualification that
was going to be recorded anyway. And **a platform with no containment mechanism
does not run untrusted code at all** — A14 and A15 admit no exception, so
*degrade and say so* is not available, and the capability is refused there
rather than weakened.

### Version 31 — the licence is GPL-3.0

D28 closes the half of §7.47 that only the author could close. D22 had narrowed
it to the GPL family and stated the lean; taking it is a commitment about
distribution, and `LICENSE` now holds the verbatim text.

The consequence for §XVI is the part worth carrying forward: every component MCF
vendors must be GPL-3.0-compatible, which the permissive terms §XVI wants
already are, and one whose terms are not is one MCF cannot ship whatever its
merits — it goes to D23's third tier with its reason recorded. The remaining
half of §7.47 is the per-engine matrix, not the licence.

### Version 30 — a void the budget tier found by running

§7.51 recorded. D27 said an unattributable run is neither a pass nor a failure,
which is right; F2 then found a machine — the one this is written on — where
every event-class run is unattributable all day, so the rule that protects the
budget from somebody else's compile also prevents the budget from ever being
asserted.

It is recorded rather than patched. The mechanism is correct and the gap is
real: B38 refuses a release on a stale tier, D27 refuses to count an
unattributable run as refreshing one, and together those two correct rules can
deadlock. That is the part that has to be decided rather than discovered.

### Version 29 — a budget says which reading it is about

D27 answers §7.50, which existed because F1 found a single run of twenty
cold-start trials that both passed and failed D24's ceiling depending on which
statistic you read.

Figures divide into three kinds and the kind decides the statistic: a
prohibition is a maximum that must be zero, a ceiling on *state* is a maximum,
and a ceiling on an *event* is the 99th percentile — the one D24 already names,
for the reason it gives, which generalizes. Adopting one percentile rather than
inventing variety costs a sample count: a p99 of twenty trials is the maximum
wearing a percentile's name, so an event-class figure needs at least a hundred.

The second half matters more than the first. A budget is asserted only on a run
the machine was quiet enough to attribute, because B35 holds that a timing under
contention measures the contention — so a busy machine makes a run
*unattributable* rather than failing. The hazard that creates is closed rather
than accepted: an unattributable run is not a pass either, and B38's staleness
discipline applies, so a tier whose last attributable run is old is stale and a
release on it is refused.

### Version 28 — the laboratory's scope, and a clock that is not a convenience

D26 answers §7.21, which asked what the laboratory must simulate, what it
declines to, and whether simulated time is structural.

The first was answered the day [taxonomy.md](taxonomy.md) existed and nobody
noticed: A13 already made the fault catalogue and the taxonomy the same list,
and §7.21 was open only because the list did not exist. What needed deciding was
narrower and sharper — A13 binds a *claim*, not a code, and MCF claims a
category when its own code can produce one. So every category MCF constructs has
a scenario, and one nothing constructs is a classification waiting for the code
that will use it. That makes A13 a ratchet rather than a cliff: coverage of what
is claimed is complete from the first day and stays complete, because a new
failure site cannot land without its scenario.

What the laboratory declines is now stated, as §6.16 requires. The load-bearing
line is that **it simulates what MCF observes, never what causes it** — a
thermal ceiling is a probe reporting a throttled device, not a heated one — and
that is precisely why A12 puts reality above the simulator rather than beside
it.

Simulated time is structural, and that is the part that had to be settled now
rather than at M8. It is a constraint on ordinary code, not on tests: anything
that waits, times out or measures an interval takes its clock rather than
reaching for one. Retrofitting that is a rewrite of every deadline in the
system.

### Version 27 — hardware scope is a capability, not a list

D25 answers §7.8, which had been open since the first version and which §7.8
itself said changes what §IV means. The boundary between *characterized* and
*will attempt, uncharacterized* is a property of the observer: a device is
characterized when MCF can read its identity, its driver and runtime versions,
its available memory and its thermal state — the readings §3.8 needs in order to
tell a slow model from a busy machine.

A vendor list was the obvious alternative and answers the wrong question. It
would say which devices MCF approves of and nothing about whether MCF can tell
those two apart on any of them, and it would put a special case in a measurement
path, which B28 refuses on principle.

Two consequences are uncomfortable and follow directly. The state is **per run**,
because §3.8 makes hardware time-varying: a device whose driver query fails this
afternoon is uncharacterized this afternoon, whatever it was this morning.
And a profiler built only on the files a driver publishes reports every device as
uncharacterized, because F1 established that live state is reachable only over
the C ABI — which is the honest outcome and the reason the definition is worth
stating rather than being satisfied by whatever gets implemented first.

### Version 26 — the prototype ran, and D4 stands

§7.19's validation happened. D4 committed this decision to an adversarial
prototype — probe an accelerator, supervise a child made to die badly, record
both, measure against §7.16 — and said the entry would be amended rather than
defended if it went badly. It did not, and D4 gains a paragraph saying so and
citing the evidence rather than restating it.

Two findings travelled with the confirmation, and neither weakens it. The live
accelerator state §3.8 depends on — device memory, temperature — is reachable
only over the C ABI, not from the files a driver publishes, which is something
DEC-008 has to decide knowing rather than discover afterwards. And D24 names a
statistic for one of its sixteen figures and not for the other fifteen; on a
contended machine a cold-start median passes while its p95 fails by a factor of
two, from one run. §7.50 records that.

[findings.md](findings.md) is new, and exists because neither this document nor
the register was the right home for the evidence: the first states positions and
the second states status, while a run's conditions and readings are a third kind
of thing.

### Version 25 — the changelog catches up

Sixteen entries added, for versions 9 through 24. The document had carried
those changes since they were made and had recorded the *reasoning* for them
only in the commits that made them, which §8 does not allow: it requires that
the reasoning outlive the change, and a reader of this document is not a reader
of a version-control history.

Nothing above this section changed. The entries below were reconstructed from
the commit that made each change, so they state that change's own reasoning
rather than a later reading of it.

The gap was found mechanically, by the documentation conformance check B-041
builds — the front matter claimed version 24 and the newest entry was version
8. It is the fourth stale-figure defect that check has found and the largest,
and it is the argument for the check existing.

### Version 24 — the budget has numbers, and failures have a scheme

D24 answers §7.16. Eight figures for MCF's own cost, two for installed
footprint — separated because §6.31 holds that §VII governs behaviour rather
than download size and one number cannot honestly do both — and six for the
interface, with the reference client bounded at a mid-range phone about ten
years old. Two of the figures are prohibitions rather than thresholds: zero
timer wakeups while idle, zero external requests from the interface. Until this
existed §VII was unfalsifiable and six M0 items had nothing to assert against.

The footprint figures are the least certain, since §XVI's vendoring and D23's
tiering decide them and neither has been built. D4's prototype exists partly to
check them, and the entry is amended if it goes badly rather than defended.
Every figure is a ceiling rather than a target.

[taxonomy.md](taxonomy.md) answers §7.10, open since the first version of this
document. The design decision worth recording is the split into three axes —
category, attribution, disposition — rather than one tree, because a single tree
would have had to encode what failed, whose fault it was and what MCF did about
it, and the product of those is unmanageably large. Sixteen domains, 110 codes,
nothing deeper than three segments.

Two of its features are load-bearing elsewhere. `model.*` classifies the model
under test's behaviour and is a measurement rather than a failure of MCF, which
makes §6.17's *how it failed matters more than the pass rate* structural rather
than aspirational. And `internal.unclassified` is a tracked defect metric with a
target of zero rather than a bucket: every occurrence is a missing category, and
adding the category is the fix.

### Version 23 — a fully-vendored stack, for now

D23's second tier was deferred rather than adopted. MCF ships a stack it
controls end to end, and anything requiring a component it cannot vendor is
avoided and recorded as a candidate for later — the same treatment as the
declined tier, reached by a different route.

The scientific argument points the same way as the simplicity one, which is why
this is comfortable rather than a compromise. A stack MCF ships entirely is a
stack MCF has pinned entirely, so measurements are taken against conditions MCF
controls rather than conditions it merely records. §3.12 puts reproducibility
above convenience, and a platform-provided runtime would be an unpinned variable
in every result taken through it, permanently.

The cost is stated rather than discovered later. Vendorable accelerator paths
are the open ones, and on hardware whose vendor-optimized runtime is closed a
vendored path is frequently slower. So MCF's numbers describe *the stack MCF
ships, not the hardware's ceiling* — a §3.4 condition that travels with every
absolute figure, because a user comparing against a vendor-optimized tool would
otherwise conclude their hardware is slow.

What survives is what MCF is for: §3.27 already holds that the durable output is
the comparison, and a ratio between two configurations on one pinned stack is
unaffected by that stack being slower than another. What degrades is the
absolute figure, which is the least transferable quantity anyway.

B-321 registers the deferred-engine list, so the omissions are maintained rather
than silent.

### Version 22 — three tiers of engine support

D23 resolves the sharp half of §7.47 by separating two questions that had been
conflated: whether a component is free to use, and whether its terms permit
redistribution inside a copyleft binary. Most accelerator runtimes are the first
and not the second, and only the second is what D22 makes difficult.

*Vendored* — permissive, shipped, pinned — is the preferred tier and where the
engines MCF principally wants already sit. *Platform-provided* — present on the
user's machine already, detected and used but never redistributed — supports a
closed vendor runtime without shipping it and without negotiating anything,
because nothing is being distributed. *Declined* covers anything needing a
negotiated licence or payment, which is a normal outcome under §3.13 rather than
a failure, recorded with its reason and revisited if the project matures enough
for negotiating to be worth someone's time.

Two constraints keep this from eroding what it touches. Tier one is never empty:
a vendored engine always works with no external dependency, so tier two is an
accelerated path on top of a working baseline and §XVI's no-errand rule
survives. And tier two's cost is recorded rather than absorbed: a runtime MCF
did not ship is one it did not pin, so its version joins engine identity, and
measurements taken across two vendor runtime versions are not the same
configuration.

Declining a tier-three engine does not weaken §III, since §6.3 governs attempt
and diagnosis rather than success.

### Version 21 — copyleft, journal durability, dedicated contributions

D22 puts MCF under the GPL-3.0 family. Spin-offs are welcome and must stay open;
selling remains permitted, but the source travels with the binary, which
extinguishes the close-it-and-sell-it case without a non-commercial licence that
would have narrowed what §XVI may vendor.

The GPL-versus-AGPL choice stays open and leans GPL-3.0. AGPL's trigger is
network interaction and MCF's core feature is serving over a network, so its
clause reaches further here than it would for a desktop application — while what
it protects is narrower than it looks, since §5 already puts the aggregating
website out of scope and a competing hosted service would be its own code rather
than a fork.

One consequence surfaced only once the choice was made: copyleft interacts
awkwardly with proprietary accelerator runtimes. The engines and libraries MCF
wants are permissive and unproblematic, but vendor inference runtimes are
frequently closed, and §XVI's instruction to *ship* rather than *link against*
weakens the system-library argument copyleft software normally relies on. §XVI
and D22 may therefore collide exactly where MCF wants to support an accelerator.
The likely resolution — treat a vendor runtime as a detected platform capability
rather than something shipped — is recorded rather than assumed, and it may
exclude an engine §7.4 would otherwise prefer.

D20 answers §7.49: the database is a rebuildable index over an append-only
journal, crash safety is configuration, export is one command sharing the
serialization §XIV and PR2 need, nothing automatic leaves the machine, and a
failed replay reports the extent of the loss rather than opening with a shorter
history.

D21 answers §7.48: contributions are dedicated to the public domain, stated at
the moment of sharing, with no withdrawal right — because an unkeepable promise
is worse than an honest refusal.

### Version 20 — candidate answers for the licence, durability and rights gaps

None of the three resolved; all three narrowed to a choice small enough to make.

§7.47 records the stated intent — as open as possible, spin-offs welcome, a mild
and loosely held preference against others selling it — and the tension that
decides it. Every licence recognised as open source permits commercial use, and
a non-commercial licence would narrow what §XVI may vendor rather than widen it,
being incompatible with copyleft components. The concern behind keeping people
from selling it is usually narrower than the words, and copyleft answers that
while staying open. The choice is permissive versus copyleft, and both honour
the intent.

§7.49 gets a recommended architecture proportionate to what most installations
do: the database is a rebuildable index over an append-only journal, which is
D16's principle applied to durability and converts corruption from fatal to
recoverable at the cost of a write path rather than a subsystem. Crash safety is
configuration. Export is one command and one file, nearly free because §XIV and
P2 need the serialization anyway. No replication and no automatic off-machine
backup, since §3.13 refuses the weight and A17 forbids the egress.

§7.48 gets a minimal candidate so that the absence of an answer does not become
one: contributions offered under a public-domain dedication, stated at the
moment of sharing, with no withdrawal right — because §3.20 already establishes
that publication cannot be undone, and offering to undo it would be a promise
MCF cannot keep.

### Version 19 — three critical gaps recorded

§7.47, and it is structural: MCF never states its own licence, which is odd for
something D7 puts in other people's hands — and the larger half is inherited.
§XVI requires MCF ship everything it needs, so vendoring an inference engine
means inheriting that engine's obligations. An engine whose terms are
incompatible is one MCF cannot ship whatever its merits, which means §7.4's
engine question cannot be settled without knowing which candidates are eligible.

§7.48: what rights a contribution carries. §7.36 asks whether model licences
constrain publishing measurements about a model; nobody had asked what the
contributor grants or retains. It has to be answered before anything is
collected, because data gathered under unstated terms cannot be retroactively
given terms, and a withdrawal right may be a promise §3.20's irreversibility
makes unkeepable.

§7.49: what protects the record from loss. D6 notes that a single file is a
single point of corruption and calls it a §3.1 obligation, and nothing
discharged it. The record is the science — months of measurements whose
conditions are gone, so none of it is reconstructible by re-running. A lost
record is the loss of every claim MCF has made.

B-301 registered alongside: artifact checksums were verified at acquisition and
never again, so re-verifying before a long run catches silent disk corruption
before it produces a garbage result rather than after.

### Version 18 — the seed set closes the identity question

D19 settles the last edge of identity, against the intuitive answer. A single
fixed seed looks like the reproducible choice and is not: thirty trials at one
seed with identical inputs produce thirty identical outputs, which is `n=1`
wearing the costume of `n=30` and destroys §3.4's uncertainty requirement
precisely where it matters. Fixing one seed conceals variance rather than
reducing it.

The answer is a declared seed set — trial *i* uses seed *i*, the same list on
every machine. Reproducible across systems, genuinely varied within a run, and
not exposed to an unlucky draw, since the result rests on the whole set. Its
size is the trial count, so it is the same decision as §7.23's statistics.

A seed is neither good nor bad: it selects a trajectory, and the mapping from
seed to outcome is chaotic and task-specific, so there are seed-by-task
interactions but no systematically underperforming seed. The seed is a condition
rather than identity, since sampling parameters change the distribution and a
seed only draws from it — which also honours §6.17's refusal to engineer
stochasticity away.

Two caveats are recorded with it. Identical seeds do not guarantee identical
output, because floating-point reduction order and kernel scheduling make
accelerator inference non-deterministic anyway, so a seed set buys comparable
inputs rather than identical outputs. And the set is validated periodically
against a larger random one, because §6.16's rule that the instrument does not
grade itself applies here too.

§7.34 closed. Two of the three structural M0 gates answered; only the
performance budget remained.

### Version 17 — sampling is identity, and the recommendation is verified

D18 answers §7.46. Sampling parameters belong to a configuration's identity: a
model cannot run without them, §6.6 already lists them among what MCF tunes, and
§XV cannot reproduce behaviour without them.

The model's own recommendation becomes the default rather than a house style,
and is marked *declared* until MCF has tested it. This is §3.18 applied one
level out from capabilities — recommendations are measured, not believed. A
global default temperature would measure every model under settings some were
never designed for, which is §X's misconfiguration problem arriving through the
front door.

Sweeps do the verifying, and a publisher's recommendation turning out not to be
optimal here, or not optimal for this workload, is exactly the kind of finding
this project exists to produce. *Better* stays per workflow, so a sweep reports
per laboratory rather than crowning one value.

One hazard is written into the decision rather than left to be discovered. A
sweep is hyperparameter optimization, and picking the best of eight arms
inflates the apparent gain whether or not a real difference exists. A swept value
is selected on one split and its improvement reported from another, so the
published number is the validated one and never the winning one — and a winner
that fails validation reports *no improvement found*, which is a useful null
result.

L26 added for the sweep itself, with coordinate descent around the declared
values rather than a full grid, and the declared value always present as an arm
so the output is a comparison against the recommendation.

### Version 16 — placement and engine build settled; sampling split out

Placement became a declared intent in the configuration, with the realized
layout recorded as a condition. Divergence between them is a finding, and often
an informative one — it is how a configuration visibly fails to transfer.

Engine build became identity. An engine that changes silently colours every
measurement taken after it, so the fragmentation cost is paid deliberately:
grouping-as-a-view recovers what is needed, while discovering that a corpus
silently mixed two engines is unrecoverable.

Sampling parameters got their own void, §7.46, since they pull in three
directions rather than two and the scope needed narrowing: chat template and
stop conditions are not sampling preferences but correctness, established by
calibration and verified by probe. Getting those wrong makes a capable model look
mediocre; getting temperature wrong produces a different, still-valid experiment.

### Version 15 — raw trials kept, identity is the configuration

D16 answers §7.33. Raw trials always; summaries are derived at query time and
never written in place of what produced them. Barely a choice: §6.17 needs the
shape of a bimodal distribution, §3.27 needs the pairing, and §7.7 has not
decided what statistic matters, so a frozen summary is a question that can never
be re-asked. Interior detail is declared per laboratory and off by default,
because the constraint is cardinality rather than disk — a trial row is a hundred
bytes and a year of heavy use is tens of megabytes against weights measured in
gigabytes. Where it grows, it is downsampled with the thinning recorded, since a
thinned series that does not say so is a silent alteration of evidence.

D17 answers §7.34 in principle. Identity is the runnable configuration and
hardware is a condition, so the same configuration on two machines is one thing
observed twice. Excluding hardware is what makes the corpus possible at all:
measurements group by configuration, and hardware becomes the axis they are
analysed along rather than a key giving every machine its own universe. Grouping
is a query-time view, which is D16's principle one level up — keep the
fine-grained thing, derive the coarse one.

§7.34 narrowed to three boundary parameters — placement, engine build, sampling
— with candidate answers and the test recorded: err toward more in the identity,
because a group can be widened and never narrowed.

B58 gives a laboratory freedom over its own tooling and binds it to A27's
restore obligation, since a lab that leaves a warm cache behind silently changes
the next lab's first trial.

### Version 14 — three gaps found by auditing what is unanswered

§7.44: §6.36 and B41 both rest on a *declared workflow*, and nothing said how
one is declared or what the vocabulary is. A fixed list is legible and wrong for
anyone between two entries; a weighting is exact and asks users to have opinions
about instruments they have never run; inference from their own traffic is
accurate and invasive. It is the practical half of §7.2 and should probably be
answered with it.

§7.45: §7.13 asks whether measurements survive MCF's own upgrades, and nobody
had asked the same of the hardware, which changes far more often. A driver update
makes yesterday's results describe an apparatus that no longer exists, and the
failure is silent — a user reads the driver's effect as the model's.

PR7 registers longitudinal regression detection, the third thing §6.7 names in
passing and no milestone builds. The data for it is already kept: conditioned
measurements over months, MCF's version, the driver, the thermal baseline. For a
solo operator it may be the most valuable sentence MCF can produce, and it is
the one statement no corpus can make.

Also corrected: the retired-void index still described §7.3 as answered by
agentic task success, which version 9 replaced with *quality is plural*. A stale
pointer in an index is how a superseded answer survives its own correction.

### Version 13 — the comparison is durable, and the two directions

Two principles, both following from the observation that MCF runs on machines
nobody controls.

§3.27: common-mode noise cancels in a ratio and accumulates in a scalar, so the
comparison is the durable output and the absolute number is local. The
consequences are technique rather than philosophy — arms are interleaved within
one session rather than run in blocks, order is randomized, and the reported
quantity is the paired difference distribution rather than two summaries
subtracted. Thirty runs of A followed by thirty of B reports the afternoon's
drift as a difference between configurations, and on these machines that drift
is not small.

This changed what §XIV should accumulate. A tokens-per-second figure from a
stranger's machine is nearly uninterpretable; *on hardware like this, A beat B by
roughly this much* is what another user needs, and it survives the mess it came
from. The corpus should chiefly hold comparisons.

It narrows the exclusive window to two jobs — small effects, and numbers intended
to leave the machine — because interleaving already cancels what the window
would have excluded. Most of what a user wants does not need a quiet machine at
all. And it largely defuses §7.43: a partial box misleads mainly by licensing a
cross-session comparison, and if comparisons are paired within a session the box
is never what holds conditions still. A candidate answer was recorded there
rather than the void being resolved.

§3.28: gather precisely, recommend generally. Inward, as accurate as the machine
allows. Outward, as general as the evidence supports, because false precision is
noise wearing the costume of rigour. Reconciled by direction rather than
compromise — generalization happens in the rendering and never in the record, and
the precision is always one step away.

### Version 12 — resource boxes, and measuring the politeness

Two additions, both turning a comfort claim into a measured quantity.

Yielding is itself a measurement, on both sides. §3.26 promised MCF would be a
good guest, which was resting on intention. There are two costs and they belong
to different disciplines under D10: what yielding costs the *user* is an
application test — does a background run stutter an interactive workload — and
what constrained resources cost the *model* is a laboratory. L24 and L25 were
added for the second, B52 for both.

D15 adds resource boxes: a declared allocation a model runs inside. This is the
third answer to contention alongside the exclusive window and yielding, and the
most useful on a machine somebody is using, because it removes contention as a
confound without removing the machine from its owner, and makes a measurement
reproducible across sessions.

The limit is stated in the same breath, because building on it unstated would be
worse than not having it: a box bounds what a process may take, not what it may
be denied. Memory bandwidth, cache, accelerator time-slicing, PCIe and thermal
headroom are shared and largely unpartitionable on consumer hardware, and those
are the dimensions that determine tokens per second. Every boxed result names
what its box did not bound, boxed and unboxed results are never compared, and
§6.41 keeps the exclusive window rather than letting a box quietly replace it.

That split is a feature: a best-case timing on a quiet machine answers *what this
machine can do*; a boxed timing under stated load answers *what the user gets
while working*. Both are first-class, neither is comparable with the other, and
§3.19 arguably favours the second.

§7.43 and DEC-043 record what nobody has established — which box dimensions are
enforceable per platform, and whether a partially-enforceable box should be
offered at all or refused as misleading.

### Version 11 — MCF is a guest on the user's machine

D8 had held that a laboratory owns the machine for the duration of its run. On a
dedicated rig that is rigorous; on the average machine MCF is actually installed
on, it makes the heavy laboratories unusable, and a tool that takes the hardware
for a day is a tool nobody runs twice.

The correction was already latent in §6.25 and had been generalized past its
evidence. Exclusivity is a property of *timings*, never of laboratories. A timing
under contention measures the contention; whether a tool call parsed or a loop
terminated is unperturbed by the user opening a browser.

The consequence is favourable rather than a compromise: the runs that need a
quiet machine are the short ones. Timing work is tens of minutes and opens an
announced, bounded, schedulable window. Behaviour work is hours to days and
yields — low priority, behind user traffic, pausable, with contention recorded
rather than prevented.

Two caveats make that honest and are obligatory rather than advisory. A yielding
run's deadlines are token budgets rather than wall clocks, because a task that
failed on a busy machine is a measurement of the machine. And an environment
failure is classified apart from a model failure, because an out-of-memory
caused by competition is not the model giving up. Without both, every yielding
run is quietly contaminated.

§3.26 states what §1 already implied one level out: a support structure that
consumes what it supports has failed, and so has a tool that makes the machine
unusable while it works. Hosting yields to the user; nothing yields to hosting.

§7.42 records what this rests on — what yielding actually guarantees, given that
a submitted accelerator kernel does not yield mid-flight — and DEC-042 is marked
a hard gate, since a background run that stutters an interactive application
fails §3.26 rather than merely disappointing.

### Version 10 — tiers, budgets, and the boundary of autonomy

D12 settles §XV: import is convenience. Paste an identifier, host it, done —
nothing conditional on measuring anything. Where diagnostics happen to correct
it, the delta is recorded because it is unusually clean data and costs nothing,
but it is a byproduct that may never gate or slow an import. The earlier reading
had the tail wagging the dog.

D13 records four tiers — smoke, calibration, characterization, evaluation — and
the finding that their order is a correctness requirement rather than a
courtesy: §X calls a misconfigured model a measurement error, so evaluating an
uncalibrated configuration spends a day measuring the misconfiguration.

D14 answers the cost problem. The product of models, configurations,
laboratories and trials is unbounded, so selection is a first-class feature and
the interaction inverts: the user spends a budget rather than picking a list. A
lab declares its work in units it can count, the machine supplies the rate from
the characterization tier, and the product is a banded estimate scored against
what actually happens. Budgets produce proposals that state their exclusions,
never silent truncations, and results are anytime.

§6.38 gives the corpus a job without breaking §6.28: it narrows the search,
local measurement decides. §3.24 guards the failure mode a corpus makes
available — the confident negative drawn from silence — with *unreported* rather
than *unsupported*, a sample count on every claim, and no option hidden.

§3.25 draws the boundary the widest reading of *improve* needed: reversibility.
§6.39 applies it to environment control as a ladder — report, wait,
ask-suspend-restore, never terminate — with everything changed recorded as a
measurement condition and restored afterwards.

P4 dropped, superseded by §6.38: a pairwise comparison is a sample of two, and
the corpus is a sample of everyone.

### Version 9 — quality is plural, and a catalogue of laboratories

The correction: D2 had held that quality *is* agentic task success. That fails
on contact with the ecosystem, because many models have no tool-calling
capability at all, and scoring them near zero says nothing about them except
that the suite was the wrong instrument. A number that is precise, reproducible
and meaningless is §6.1's failure mode reached by a new route.

D2 rewritten. Quality is a set of separately measured, separately reported
qualities, each from a laboratory built for one workflow class, each run only
where the model is capable of it, each mattering in proportion to how much the
user's work resembles it. MCF publishes a profile and never a score. §IX amended
to match: agentic remains the first laboratory built, because it is the shape
this author uses, but it is one shape rather than the definition.

§3.23 is the joining rule between capability discovery and the laboratories, and
the reason the correction was needed: four outcomes, never three — measured, not
applicable, unknown, failed. Collapsing the middle two into the last is what made
the old D2 look reasonable. §6.36 keeps plural quality from becoming a research
project handed to the user; §6.37 admits customizable laboratories whose results
are local, non-comparable and never contributed.

B40, B41 and B42 encode those, all three as types rather than conventions,
because an absent capability rendering as a low number looks like data and reads
like a verdict.

[labs.md](labs.md) drafts twenty laboratories in four families, split by what
gates them rather than by subject, since the gate determines whether a lab can
make a claim about a given model at all. The first three are named: throughput,
structured output, agentic — one timing-class lab with no gate, one cheap gated
behaviour-class lab, one heavy multi-turn lab.

P1 rescoped from automatic session capture to a workload slot, with the capture
version recorded as refused rather than deleted. PR6 accepted as B-210.

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
