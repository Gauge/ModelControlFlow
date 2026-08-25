# Roadmap

| | |
|---|---|
| **Type** | Plan — ten milestones, each a vertical MVP slice |
| **Version** | 14 |
| **Status** | Living |
| **Authority** | Derived from [document-of-intent.md](document-of-intent.md) v24 and governed by [rules.md](rules.md) |
| **Registers to** | [backlog.md](backlog.md) · illustrated by [mockup/](mockup/) |

**Every milestone is a product, not a phase.** The MVP rule applies per
milestone rather than once to the project: each ends with something a competent
operator can install, run and get value from on its own, and each is a
*vertical* slice — acquisition, record, failure handling, lab coverage, budget
assertion and surface, at whatever depth that milestone requires. A milestone
that delivers a layer rather than a capability is mis-drawn; horizontal layering
is the specific failure mode this ordering prevents.

## How to read it

**Nothing is deferred that is ruinous to retrofit.** The record (§3.3, A6), the
laboratory (§3.17, A13) and the performance budget (§3.13, B20) are present from
M0. A milestone producing numbers without conditions produces nothing that
survives §3.4; a budget introduced late is a budget already blown.

**The ordering is dependency-truthful, then value-ordered.** §1 names the third
of the four verbs — *Judge* — as the point of the project, and acquisition and
serving as table stakes. That does not put Judge first: it makes Judge the thing
everything else exists to enable, and it cannot be honest before the instrument
is trustworthy (M0), before there is anything to judge (M1–M2), or before models
under test are configured the way they were designed to run (M3) — §X calls a
misconfigured model a measurement error. Earliest is therefore the work with the
most downstream design blocked behind it, which is §7's own ordering principle.

**Milestones are gated by decisions, not dates.** Each names the §7 voids it
closes first; closing one means amending the intent document in place (§8), not
settling it in code and hoping the document catches up. Dates are omitted
deliberately — §4 accepts that rigor costs time, and a schedule is the first
thing to erode it.

---

## Milestone map

| M | Product | The user can now… | Primary intents | Closes |
|---|---|---|---|---|
| **M0** | **The instrument** — `mcf doctor` | Learn what this machine is, what MCF costs on it, and what MCF will and will not promise here | §I, §II, §VII, §VIII | §7.16✓, §7.10✓, §7.19✓, §7.21, §7.22, §7.8, §7.50 |
| **M1** | **Custody** — `mcf pull` | Bring any Hugging Face model onto this machine with its provenance intact and its licence legible, or learn precisely why not | §III, §3.7 | §7.11 |
| **M2** | **The host** — `mcf serve` | Get a first token from a named model in one command, from a daemon that stays up | §VI, §I | §7.1 residuals, §7.9, §7.18 |
| **M3** | **Right by construction** — `mcf probe` | Run a model the way it was designed to run, and see where its claims and its behaviour diverge | §X, §3.18 | §7.24, §7.25 |
| **M4** | **The window** | See and drive all of the above from the machine or from a handheld device, with nothing installed | §V, §XI | §7.17, §7.12 residual |
| **M5** | **The measurement** — `mcf bench` | Obtain a defensible performance number taken *here*, with its conditions and its uncertainty | §II, §IV | §7.7, §7.6 |
| **M6** | **The bench** — `mcf lab` | Find out what a model is good at, across purpose-built laboratories gated by verified capability, as distributions rather than scores — and against your workload, not only the shipped one | §IX, §XIII, §3.19, §3.23 | §7.23, §7.29, §7.3 residuals |
| **M7** | **The loop** — `mcf recommend` | Be told which configuration to run, why, what came second, and when the difference is noise | §IV, §3.9 | §7.2, §7.26 |
| **M8** | **Endurance** | Trust all of it over time, across upgrades, offline, and on hardware the lab only simulated | §VIII, §I | §7.20, §7.13, §7.5, §7.14 |
| **M9** | **The exchange** — `mcf share`, `mcf import` | Contribute evidence deliberately, and reproduce a configuration found elsewhere — or learn precisely why this machine cannot | §XIV, §XV | §7.27, §7.28, §7.30, §7.31 |

Each milestone's finished state is drawn in `mockup/M<n>-*.md`: sketches of
intent, not committed designs ([README.md](../README.md#mockups)).

---

## M0 — The instrument

> **Mockup:** [mockup/M0-instrument.md](mockup/M0-instrument.md)

**Why first.** MCF publishes numbers. Nothing it says is worth believing until
the thing saying it can demonstrate it computes what it claims (§3.5), can
classify its own failures (§3.1), and can prove it is not corrupting its own
readings by being heavy (§3.8). This milestone builds the *instrument's
instrument*: the record, the failure taxonomy, the laboratory, and the budgets —
plus the smallest honest product that exercises all four.

**The MVP.** `mcf doctor`. It profiles the machine, states MCF's own measured
footprint against the budget, names the accelerators it recognizes and refuses to
guess at the ones it does not, and writes the whole thing to the record. It ships
as a static binary that runs offline with no models present.

**Delivers**
- Rust workspace, pinned toolchain, reproducible build (B-001)
- Failure taxonomy ([taxonomy.md](taxonomy.md)) and a `Result` discipline the
  compiler enforces (B-003)
- `Measurement<T>` and `Provenance` types that cannot exist without their
  conditions and origins attached (B-005, B-006) — §3.16 applied at the smallest
  scale, where it is cheapest
- Append-only, event-driven record store; zero timer wakeups when idle (B-004)
- Laboratory skeleton: simulated clock, injectable faults, replayable scenarios
  (B-009), with a catalogue cross-checked against the taxonomy (B-010)
- The performance budget (D24), asserted in CI (B-011)
- Hardware profiler, degrading and labelling on unrecognized hardware (B-013)
- The adversarial Rust prototype §7.19 asks for as validation (B-002)
- `rules.md`, derived from the Document of Intent, every rule citing (B-016)
- The self-contained artifact (B-192, B-183): engine and common-path tooling
  vendored, statically linked, and a from-scratch container with no toolchain
  proving §XVI by the only test that matters
- The privileged helper (B-190, B-180): a separate auditable executable, the
  daemon holding no ambient privilege, and a scenario proving untrusted code
  cannot reach an elevated path — §6.32's boundary, built before anything needs
  it rather than after
- The time model as types (B-184) — `end_wall - start_wall` should not compile
- Test tiers with published ages and a mutation floor (B-191, B-185, B-186) —
  D10's discipline, including the tier that measures whether the suite would
  notice a deliberate break
- Environment restoration (B-220): a scenario kills MCF at every stage and
  asserts governors, priorities, exclusive modes and suspensions are restored —
  built before anything is permitted to change them
- Reference-model neutrality: no code path special-cases the reference model,
  and the suite never depends on it (B-018) — §6.22, built in at M0 because a
  special case is far cheaper to prevent than to find

**Gated on:** DEC-022, DEC-039. ~~DEC-004~~ closed by D32, on F8's measurement; ~~DEC-051~~ by D30. ~~DEC-008~~ closed by D25; ~~DEC-021~~ by D26; ~~DEC-050~~ by D27; ~~DEC-047's licence half~~ by D28; ~~DEC-035~~ by D29. ~~DEC-016~~ closed by D24; ~~DEC-010~~ closed by [taxonomy.md](taxonomy.md); ~~DEC-033/034/046~~ closed by D16–D19; ~~DEC-049~~ by D20.

**Note on the remainder:** all of them are questions a working prototype answers better than an argument does. D4's prototype has now run ([findings.md](findings.md) F1): it closed §7.19 and left DEC-008 and DEC-050 better informed rather than closed. ~~DEC-033~~ closed by D16; ~~DEC-034~~ and ~~DEC-046~~ closed by D17-D19.

**Explicitly not in M0:** any model, any inference, any network fetch, any UI.

**Exit criteria**
- The full suite runs green offline, on a laptop, with no accelerator (B-015)
- Idle CPU, resident memory, cold start and footprint are asserted against real
  numbers and fail the build on regression
- Every failure taxonomy category has a lab scenario that produces it
- A deliberately badly-dying child process is supervised, classified and recorded

---

## M1 — Custody

> **Mockup:** [mockup/M1-custody.md](mockup/M1-custody.md)

**Why second.** §III is the entry point of everything downstream, and §3.7 makes
it the project's largest untrusted surface. Building acquisition before serving
means the hostile-input work is done once, early, against a simulated hub, rather
than retrofitted onto a daemon that is already exposed.

**The MVP.** `mcf pull <ref>`, `mcf list`, `mcf rm`. Models enter, live on, and
leave this machine, each carrying repository, revision, checksum, licence,
retrieval time and every transformation since. Nothing is deleted without
deliberation.

**Delivers**
- Reference resolution reaching a defined, actionable outcome for *every* input
  (B-020) — §6.3's "no unhandled outcomes, not no unsuccessful outcomes"
- Resumable, integrity-checked transfer, including mutation-under-us (B-021)
- Validation of every fetched byte against hostile fixtures (B-022)
- Licence legibility and gated-repository handling (B-023, B-024)
- Repository-code execution: possible, never implicit, contained, recorded in
  provenance (B-025) — §6.4
- Deliberate eviction; disk exhaustion as a decision, not a surprise (B-026, B-027)
- The fake hub: well-formed, malformed, gated, hostile, truncated, mutating (B-028)
- Pre-acquisition fitment (B-213, [PR3](proposals.md#pr3--pre-acquisition-planning)):
  which of a repository's twenty quantizations fit here, computed from metadata
  before a byte is fetched
- The reference model acquired and pinned (B-019) — `unsloth/Qwen3.8-27B-GGUF`
  is a third-party requantization, so §XII's first real artifact is also §3.6's
  hard provenance case rather than its easy one

**Gated on:** DEC-011, DEC-009 (partial — the disk arbitration half).

**Exit criteria**
- Every hostile-hub fixture ends in a classified outcome with no state damage
- A transfer interrupted at 90% resumes and verifies
- Deliberately hostile repository code runs and leaves MCF's records provably intact
- No artifact exists without provenance; unknown fields read `Unknown`

---

## M2 — The host

> **Mockup:** [mockup/M2-host.md](mockup/M2-host.md)

**Why third.** §7.1 settled that MCF is a daemon, which makes the idle-cost rule
(§3.13) and the supervision contract (§3.1) central rather than incidental. §VI's
ergonomic bar — one command to a first token — is the first moment MCF is useful
to somebody who is not building it.

**The MVP.** `mcf serve` and `mcf run <model>`. A long-lived local service,
models addressed by name, a stable API, and a first token without the user
knowing what a runtime is.

**Delivers**
- The daemon: restartable, state-recovering, surviving indefinitely (B-030)
- Idle discipline: no polling, no timers, no watchers (B-031)
- Engine adapters as supervised subprocesses, with the engine as a recorded
  condition (B-032) — the concrete form of §7.4's answer
- Supervision contract for a runtime dying at any lifecycle stage (B-033)
- Serving API and name addressing (B-034); added-latency budget asserted (B-035)
- Local-only by default; exposure is deliberate and revocable (B-036) — §6.12
- Visible, attributed, overridable defaults, with `mcf explain` (B-038) — §3.15
- Authorization gated by category, not frequency (B-039) — §6.14
- `mcf stop` (B-210): the stop control, obligatory rather than optional once
  §XVII lets MCF take exclusive control of hardware — and a stop path that is
  reliable is a stop path the laboratory can test

**Gated on:** DEC-001, DEC-009, DEC-018. ~~DEC-004~~ closed by D32; ~~DEC-016~~ by D24.

**Exit criteria**
- A cold machine reaches a first token in one command
- The lab kills the daemon and its children at every lifecycle stage; the system
  returns coherent and queryable each time
- Idle cost meets budget with a model resident
- Nothing is reachable from another host in the default configuration

---

## M3 — Right by construction

> **Mockup:** [mockup/M3-capabilities.md](mockup/M3-capabilities.md)

**Why here, before any benchmarking.** §X states it directly: a misconfigured
model is a measurement error (§3.8). Benchmarking before this milestone would
produce numbers that measure MCF's ignorance of a chat template rather than the
model. Every measurement taken before M3 is provisional by construction.

**The MVP.** `mcf probe <model>` and an extended `mcf explain`. MCF establishes
what a model can actually do by *asking it to do the thing* (§3.18), configures
accordingly, and reports where the artifact's claims and its behaviour diverge.

**Delivers**
- The declared / verified / unknown model, never confused (B-050)
- Probe framework, where a probe's output is a `Measurement` and not a boolean
  (B-051) — §3.18's "capability probes are experiments"
- Probes: chat template, tool calling, structured output, usable context length,
  stop conditions (B-052–B-056); further modalities scoped by DEC-024 (B-057)
- Divergence reporting as a first-class finding (B-058)
- Configuration carrying the provenance of the probe that set it (B-059)
- Honest handling of *inconclusive* (B-060)

**Gated on:** DEC-024, DEC-025.

**Exit criteria**
- A model that the defaults configure wrongly measurably improves, and the
  improvement is attributable to a named probe
- No probe result is ever coerced into a working default
- Every auto-set parameter answers "why this value"

---

## M4 — The window

> **Mockup:** [mockup/M4-window.md](mockup/M4-window.md) · [mockup/M4-window.html](mockup/M4-window.html)

**Why here.** §XI makes headless primary and the interface a client of the same
API (§6.21), so the window is cheap only once the API it is a window onto exists.
Placed after M3, it costs almost nothing and renders four milestones of state;
placed earlier it would have grown logic of its own, which §3.14 forbids.

**The MVP.** A thin page served by the daemon: catalogue with provenance,
serving state, capability findings, and the failure record — visible from the
machine, and from a handheld device once the user deliberately exposes it.

**Delivers**
- No framework, no bundled runtime, no build step (B-070) — §6.11's answer to
  universality: demand almost nothing of the client
- Zero idle cost with a tab open (B-071)
- Parity enforcement: no action exists only here (B-072) — checkable, because
  §VIII can only test what is reachable headlessly
- Conditions travel to the surface; a bare number cannot be rendered (B-073)
- Failure legibility per taxonomy category (B-074)
- The deliberate, informed, revocable exposure flow (B-075)

**Gated on:** DEC-016 (client budget), DEC-017, DEC-012.

**Exit criteria**
- The whole of M0–M3 is operable from the window with nothing installed on the client
- Transferred weight and cold render meet the client budget on the oldest committed client
- The parity check passes; exposure cannot be enabled as a side effect

---

## M5 — The measurement

> **Mockup:** [mockup/M5-measurement.md](mockup/M5-measurement.md)

**Why here.** This is where MCF starts making scientific claims, and every
prerequisite for making them honestly now exists: the record (M0), real artifacts
(M1), a serving path to measure (M2), and correct configuration (M3).

**The MVP.** `mcf bench`. A performance number taken on this hardware, with a
stated method, a sample count, a spread, and the conditions in force — including
the honest outcomes: *within noise*, *not comparable*, *does not fit here*.

**Delivers**
- Benchmarks structurally separate from the test suite, with no pass condition
  and no gating role (B-080) — §6.7
- A non-adaptive measurement path; warm versus cold is a recorded condition
  (B-081) — §6.13's sharpest edge
- No performance number may originate in simulation (B-082) — §6.16
- Repeated trials, mandatory spread, refusal of n=1 (B-083)
- Warm-up and thermal steady state per acceptance criteria (B-084)
- Paired, interleaved, order-randomized comparison (B-250) — §3.27: the arms
  alternate within one session so drift hits both equally, and the reported
  quantity is the paired difference rather than two summaries subtracted
- Isolation checking, so a confounded comparison is refused (B-085)
- Null and negative results stored and surfaced as results (B-086)
- Partial success as a real outcome with its data intact (B-087)
- Contention marked unattributable rather than attributed (B-088) — §3.8, and
  the snapshot that names what it was competing with (B-216,
  [PR5](proposals.md#pr5--contention-diagnosis))
- The repro bundle and its verifier (B-211, B-212,
  [PR2](proposals.md#pr2--the-repro-bundle)) — the fourth obligation of §II, which
  nothing else on this roadmap delivers
- Projection bands from local history, scored against the measurements that
  replace them (B-214, B-215)
- The quantization frontier on the reference model (B-091): one model, one
  machine, the full GGUF range — a single variable across many points, which is
  the cleanest §3.4 comparison available before a second model exists

**Gated on:** DEC-007, DEC-006, DEC-009.

**Exit criteria**
- Two configurations of one model are compared here with a stated method and a
  stated conclusion, including the option of declining to distinguish them
- A deliberately confounded comparison is refused by the tooling
- No result is publishable with n=1 or without conditions

---

## M6 — The bench

> **Mockup:** [mockup/M6-judgment.md](mockup/M6-judgment.md)

**Why here.** §IX is what makes §IV's recommendations mean anything to somebody
choosing a model to actually use, and §7.3 called the question it answers the
hardest in the project. It is also the most expensive thing MCF does (§4), needs
the M0 laboratory as its environment (§6.17), and needs M3's configuration to be
measuring the model rather than the setup.

**The MVP.** `mcf eval` and `mcf lab`. Multi-turn, tool-calling,
instruction-bound tasks with checkable outcomes, run unattended in a sandbox and
reported as a distribution with its failure modes classified — delivered as the
*first laboratory* on a framework that admits more, rather than as a one-off
harness §XIII would have to be retrofitted onto.

**Delivers**
- The harness built *on* the M0 laboratory, not beside it (B-100) — §6.17's most
  useful finding: the agentic environment and the test lab are one apparatus
- Sandbox by construction: benchmark tools cannot reach anything real because
  those capabilities are absent from the environment (B-101) — §6.20, where the
  rigor requirement and the safety requirement have the same implementation
- Tasks graded by verification, never by another model (B-102) — §3.19
- Everything except the model held still and recorded (B-103)
- Distributions, never scores (B-104); model failure taxonomy as the primary
  output (B-105) — *how* it failed beats the pass rate
- A statistical test for "is this a real difference" (B-106)
- The exclusive window for timing-class work (B-181, B-182, B-235): announced,
  bounded, schedulable to overnight or to a stated period of idleness, and
  draining the endpoint only for the tens of minutes a timing needs
- Resource boxes (B-236, B-240): a model confined to a declared allocation so
  contention stops being a confound without the machine being surrendered — with
  every boxed result naming what its box could *not* bound (§6.41)
- The two politeness measurements (B-241, B-242): what yielding costs the user
  asserted in CI, what constrained resources cost the model produced as a curve
  — §3.26 requires both be measured rather than intended
- Yielding for behaviour-class work (B-230, B-231, B-232, B-233, B-234): the
  long runs stay out of the user's way — low priority, behind user traffic,
  token-budget deadlines rather than wall clocks, and environment failures
  classified apart from model failures so a contended run is not quietly
  contaminated
- The energy laboratory (B-189, B-187, B-188): joules per token with its
  measurement provenance, sampled only while a lab runs
- The four-outcome lab result (B-200) and the prohibition on an aggregate score
  (B-201) — §3.23 and D2: *not applicable* is not zero, and there is no overall
  quality column
- The four tiers as structure (B-223, B-224): calibration precedes measurement
  as a type property, and a lab declares its work in countable units rather than
  in minutes — D13 and D14
- Budgeted runs (B-226, B-227): a time budget yields a proposal naming its
  exclusions, and every lab reports as it goes so an interrupted run keeps what
  it produced
- Duration estimates, banded and scored against actuals (B-225)
- The environment ladder (B-228): report, wait, ask-suspend-restore, never
  terminate — §6.39
- The quiet-machine pre-flight (B-217): a lab refuses to begin on a contended
  machine, because D8's exclusivity is a claim rather than a condition until
  something checks
- The workload slot (B-204, B-205, [P1](proposals.md#p1--customizable-workloads)):
  every lab ships a default and accepts yours, built into the framework rather
  than retrofitted, since the retrofit is a rewrite of every lab
- The lab framework (B-111): a lab is named, versioned, reproducible, declares
  whether it is timing-class or behaviour-class, and states what it does and
  does not establish — §XIII's unit of work
- Instrumentation scoped to the experiment (B-162, B-163): idle cost identical
  with three labs and thirty, and every result carrying its profile — the line
  §6.24 draws so that §XIII does not reopen what D5 settled
- Timing-class results refused from deep-instrumentation runs (B-164) — §6.25
- Lab admission (B-165): each lab answers what claim it enables, and refusals
  are recorded so the same proposal does not return as an oversight (§6.26)
- A stated contamination strategy that survives the suite ageing (B-107)
- Zero cost during ordinary serving (B-108) and a minimality guard on the
  harness (B-109) — §6.18, because harnesses of this kind grow into frameworks

**Gated on:** DEC-023, DEC-029, DEC-042, DEC-043, DEC-003, DEC-010.

**Exit criteria**
- Two models are evaluated unattended and the output either distinguishes them
  or declines to, with the statistics shown
- A deliberately hostile model under test cannot touch anything real
- Every failed trial is classified

---

## M7 — The loop

> **Mockup:** [mockup/M7-loop.md](mockup/M7-loop.md)

**Why last of the capability milestones.** §1 says the closed loop is the reason
MCF is worth building — and it is the *composition* of everything before it. It
cannot precede its inputs.

**The MVP.** `mcf recommend`. A declared objective in, a configuration out, with
its reasoning, its measurements, its runners-up, and its re-measured confirmation
that the prediction held.

**Delivers**
- Objective expression, with a visible default rather than an invented one
  (B-120) — §6.5
- Pareto frontier across quality, latency, throughput, memory, power and disk
  (B-121) — §3.9's "map the frontier faithfully"
- Interrogable recommendations with runners-up (B-122): not an oracle
- Refusal to manufacture a distinction (B-123)
- Construction, not just selection: compose and validate the configuration
  (B-124) — §6.6's reading of "build"
- Structural separation of selection and validation suites (B-125) — §3.4's
  prohibition on training on the test, applied to MCF's own tuning instinct
- Refusal to recommend from a field of one (B-127), and the reference set
  expanded to the breadth DEC-026 requires before any generality claim (B-128) —
  §6.23: one model builds an instrument and never supports a claim about models

**Gated on:** DEC-002, DEC-026, B-106.

**Exit criteria**
- A recommendation is produced, applied, re-measured, and the prediction checked
  against the outcome
- Every recommendation answers "why not the other one" with data
- An unstated objective produces a *visible* default, never a hidden one

---

## M8 — Endurance

> **Mockup:** [mockup/M8-endurance.md](mockup/M8-endurance.md)

**Why a milestone and not a background activity.** §6.16 rates its own confidence
low and §7.20 is the question that decides "whether §VIII is rigor or theatre."
Answering it needs the whole system to exist first. Everything here is the
project grading its own instruments, which is not something that fits inside a
feature milestone without being quietly deprioritized.

**The MVP.** A published fidelity report: what the laboratory models, what it
declines to model, how far its predictions diverge from real hardware, and how
much of MCF's confidence is therefore earned.

**Delivers**
- Real-hardware validation of the lab, with divergence recorded as a finding
  about the simulator (B-140) — §6.16's "reality outranks the lab"
- A stated fidelity boundary (B-141)
- Failure-record sufficiency measured by whether the lab can rebuild the failure
  from the record alone (B-142) — §6.15's genuine requirement, not aspiration
- Bug-to-fixture discipline (B-143)
- State migration and historical comparability across MCF versions (B-144)
- Record retention, inspection and purge, including budget exhaustion (B-145)
- Structural separation of fixture data and user traffic (B-146) — §6.8
- Offline operation, loudly labelled (B-147)
- A long-run endurance scenario: days of faults, restarts, thermal excursions
  and upgrades (B-148)
- §7.14 answered with evidence (B-149)

**Gated on:** DEC-020, DEC-013, DEC-005, DEC-014, DEC-015, DEC-011, DEC-021.

**Exit criteria**
- The fidelity gap between simulation and real hardware is quantified and published
- A sample of real failures is reconstructed in the lab from records alone
- MCF survives the endurance scenario with no unclassified outcome

---

## M9 — The exchange

> **Mockup:** [mockup/M9-exchange.md](mockup/M9-exchange.md)

**Why last.** Everything here is about carrying evidence between machines, and
evidence has to exist and be trustworthy before it is worth carrying. M9 is also
where MCF's two most irreversible acts live — publishing data, and acting on a
stranger's configuration — so it is deliberately built on top of a system whose
record, gates and verification are already proven rather than beside one.

**The MVP.** `mcf share` and `mcf import <identifier>`. A contribution the user
inspects row by row before it leaves, and an identifier that reproduces a
configuration here exactly — or states precisely why this machine cannot.

**Delivers**
- The share flow (B-160): per-share, opt-in, showing the rows that leave rather
  than a description of them, and stating that publication cannot be undone —
  A16's fifth gated category, and §3.20's whole point
- Contribution carries outcomes, never artifacts (B-171): scores,
  classifications, conditions and distributions leave; tasks, tools, fixtures and
  model outputs do not, because a public corpus of results is a map of the tasks
  (§6.30)
- De-identification per DEC-027 (B-168), including the honest statement of what
  a contribution does *not* protect — §7.27 records that the useful fields are
  the identifying ones
- Identifier emit and resolve (B-169): round-trip on one machine, reproduce on
  another
- Imported configurations read as `declared` until verified here (B-166) — §3.21
- Reproduction failure and numeric divergence as first-class findings (B-172):
  "this needs 48 GiB and you have 24" is a complete answer, and different numbers
  here are evidence about how far results travel (§6.29)
- Contribution schema versioning (B-170): a reader that cannot fully interpret a
  contribution says so rather than misinterpreting it silently (§3.1)

**Gated on:** DEC-027, DEC-028, DEC-030, DEC-031.

**Explicitly not in M9:** the aggregating website, in any form. MCF emits and
resolves identifiers and produces contributions; it does not host, rank,
display or depend on the thing that consumes them (§5).

**Exit criteria**
- A contribution is produced, inspected row by row, and sent by an explicit act
- An identifier emitted on one machine reproduces the configuration on another,
  and any divergence in measured numbers is recorded as a finding
- No contributed or imported number can reach a recommendation (B-167, B34)
- An audit of a contribution finds no prompt or completion content, and the
  guarantee is structural rather than filtered (A25)

---

## Standing rules across every milestone

The rules live in one place: **[rules.md](rules.md)**. They are not restated
here, because a rule restated in two documents is a rule that will eventually
say two different things.

The five that a roadmap is most likely to trade away under schedule pressure,
by ID: **P1** honesty outranks continuity · **P2** science outranks speed ·
**A2** no silent failure · **A18** tests and benchmarks are never conflated ·
**B20** budgets are asserted, and a performance change carries a
before-and-after.

Every milestone above is subject to all 99. A milestone that can only be
delivered by breaking one is a milestone that has been mis-drawn, and the
correct response is to amend the intent document (§8) rather than to make a
local exception.

---

## Changelog

### Version 14 — the author closes two gates

D28 and D29 close the licence and the platform scope — the two M0 decisions
nobody but the author could make, one being a commitment about distribution and
the other a decision about where his time goes. M0 waits on the end-to-end
boundary, engine ownership, the elevation list, and how a budget is asserted on
a machine somebody is using.

### Version 13 — M0 is down to four gates

DEC-050 closes with D27, so M0 waits on four decisions: the end-to-end boundary
for a daemon, engine ownership, the host platforms, and which operations require
elevation. Every one of the three closed since M0 began was closed on evidence
or on what another document already implied, which is what §7's ordering
predicts: the decisions that block the most are the ones a working prototype
answers.

### Version 12 — two more M0 gates close

DEC-008 and DEC-021 are answered by D25 and D26, so M0's gate list is down to
four decisions and one new one. Both were closed on evidence and on what other
documents already implied rather than by fresh argument: the hardware boundary
followed from what F1 showed a file-only profiler cannot read, and the
laboratory's scope had been settled the day the taxonomy existed.

### Version 11 — M0's prototype has run

§7.19 is closed by the prototype D4 committed to, and the two gates it informed
rather than closed are now named where M0 lists what it waits on. DEC-050 joins
them: a budget figure with no statistic is a budget B-011 cannot assert.

The note on the remainder is corrected. It said all six of M0's open decisions
were questions a prototype answers better than an argument; one of them has now
been answered that way, and what it produced was two better-informed decisions
rather than two closed ones — which is what the note should have predicted.

### Version 10 — pairing lands in M5

M5 gains paired interleaved comparison, which is the technique that makes a
measurement on a messy machine trustworthy: drift affects both arms equally and
cancels in the difference. M7 gains the rendering-only generalization rule.

### Version 9 — boxes join M6

M6 gains resource boxes and the two measurements that turn politeness from a
claim into a quantity. DEC-043 becomes a gate: a box that appears to isolate and
does not attaches a reproducible-looking number to an unreproducible quantity.

### Version 8 — the long runs stop taking the machine

M6's exclusivity work splits: an announced, schedulable window for timing-class
runs, and yielding for everything else. DEC-042 becomes a gate, because a
background run that stutters an interactive application fails §3.26 rather than
merely disappointing.

### Version 7 — tiers and budgets shape M6

M6 gains the tier structure, budgeted runs with anytime results, scored duration
estimates and the environment ladder. M0 gains the environment-restoration
scenario, which is what makes §XVII's permissions safe to hold.

### Version 6 — four proposals land

M1 gains pre-acquisition fitment, M5 gains the repro bundle, projection bands
and the contention snapshot, M6 gains the workload slot format and the
quiet-machine pre-flight — the last of which exists because D8's exclusivity is
a claim rather than a condition until something checks the machine is quiet.

### Version 5 — the bench measures plural quality

M6 restated: laboratories gated by verified capability, reporting four outcomes
rather than three, with no aggregate score and a workload slot built into the
framework rather than retrofitted onto it. M2 gains the stop control, which
§XVII made obligatory rather than optional.

### Version 4 — v8's work lands, and three structural decisions move to M0

M0 gains the self-contained artifact, the privileged helper, the time model as
types, and the test tiers — and gains DEC-033, DEC-034 and DEC-035 as gates,
because a record whose sample retention and configuration identity are undecided
cannot be written, and a sandbox whose host platform is undecided cannot be
built.

M6 gains lab exclusivity and the energy laboratory. Exclusivity is milestone
work rather than a policy note: draining the serving path, refusing requests
within a round trip, and bounding and interrupting a run are all mechanisms.

### Version 3 — the bench, and the exchange

M6 becomes *the bench*: the agentic suite ships as the first laboratory on a
framework that admits more, rather than as a one-off harness §XIII would have to
be retrofitted onto. Retrofitting it would have been the expensive order, since
the instrumentation-scoping rules that keep labs from reopening D5 are
structural rather than additive.

M9 is new and deliberately last. Everything in it carries evidence between
machines, and evidence must exist and be trustworthy before it is worth
carrying; it also holds MCF's two most irreversible acts — publishing data, and
acting on a stranger's configuration — which are safest on top of a system whose
gates and verification are already proven.

### Version 2 — standardized

Restated to the format contract in [README.md](../README.md): front matter,
present tense, changelog last. The three-properties block collapses into one
paragraph citing A6, A13 and B20 rather than restating them — the roadmap is not
where rules live. M7 gains DEC-026 as a gate, and M0, M1, M5 and M7 gain the
reference-model work introduced by §XII.

### Version 1 — the work is sequenced

Nine milestones drawn from the intent document, each a vertical MVP slice with
its own product, gating decisions, explicit non-goals and exit criteria.

The ordering was the substantive decision. Judge is the point of the project and
is nonetheless seventh, because a judgment made by an untrustworthy instrument
on a misconfigured model is worse than no judgment. Everything ruinous to
retrofit — the record, the laboratory, the budget — was pulled into M0 instead.
