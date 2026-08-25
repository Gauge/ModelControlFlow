# Rules

| | |
|---|---|
| **Type** | Rules — enforceable, checkable |
| **Version** | 10 |
| **Status** | Living |
| **Authority** | Derived from [document-of-intent.md](document-of-intent.md) v15, which wins on any disagreement |
| **Scope** | Every rule in the project. Rules live here and nowhere else. |

**93 rules in three tiers, each carrying a citation and a check.** Cite them by
ID. Where a rule and the intent document disagree, the intent document wins and
the rule is corrected.

## Contents

| § | Section | Holds |
|---|---|---|
| — | [Precedence](#precedence) | P1–P5, the order when rules genuinely conflict |
| A | [Absolute](#a--absolute) | 27 rules that admit no exception |
| B | [Conditional](#b--conditional) | 58 rules that permit something provided a condition holds |
| C | [Low value](#c--low-value) | 8 rules that are decided last and may be dropped |
| — | [Not adopted](#not-adopted-as-rules) | Statements deliberately not made rules |
| — | [Amending](#amending-this-file) | How a rule changes |
| — | [Coverage](#coverage) | Every source clause, and the rule that absorbed it |
| — | [Changelog](#changelog) | |

## The tiers

The labels are otherwise readings-in-the-eye-of-the-beholder, so they are
defined:

- **Absolute (A).** Admits no exception. No cost — performance, ergonomics,
  schedule, elegance — justifies violating one. A violation is a defect, not a
  trade-off, and it is fixed rather than argued. An absolute rule that genuinely
  cannot be held is an amendment to the intent document, not a local decision.
- **Conditional (B).** Holds under a stated condition, or *permits* something
  provided a condition is met. The condition is the rule; the permission is what
  makes it worth stating. Many conditional rules are the operational form of an
  absolute — B2 is how A6 is honoured while optimizing — and where they meet,
  the absolute governs.
- **Low value (C).** Real rules, kept, and the least load-bearing we have. They
  are decided **last** when rules compete, and may be dropped without amending
  the intent document. *Low value does not mean optional to follow.* It means
  the cost of enforcing them is closest to the benefit, so they are the first
  candidates for deletion if this file grows unwieldy.

The low-value tier is deliberately small — eight rules. That is itself a
finding. A document of intent whose statements are 88 % load-bearing is doing
its job; if this tier ever grows large, the correct response is to delete from
it rather than to organize it.

## Checks

§3.16 holds that a rule depending on human discipline is eventually violated by
someone tired at the end of a long change. A rule with no check is a wish, so
each names one:

| Check | Meaning |
|---|---|
| `compiler` | The violation does not compile. Strongest available. |
| `CI` | An automated test fails the build. |
| `lab` | A laboratory scenario demonstrates the rule holds. |
| `review` | A human check. Weakest; each instance is a candidate for promotion. |
| `blocked` | Not yet checkable. Names the backlog item or decision that makes it so. |

**76 rules carry at least one machine check, 15 rest on review alone, and 2 are
not yet checkable at all.** That middle figure is the number to drive down
(B16): it is the amount of this document that depends on somebody remembering
it.

---

## Precedence

Rules conflict. When two genuinely do, this is the order, and it is the same
order the intent document argues for rather than a new invention.

| | Ordering | Source |
|---|---|---|
| **P1** | **Honesty outranks continuity.** It is always correct to report that a run failed, a number is untrustworthy or a comparison is invalid. It is never correct to substitute, smooth, retry-until-pretty or omit in order to preserve an appearance of success. | §6.1 |
| **P2** | **Science outranks speed.** MCF may be fast in *how* it validates, records and repeats. It may not become fast *by* validating, recording or repeating less than the science requires. | §6.10 |
| **P3** | **Reproducibility outranks convenience.** Where a convenience would make a result harder to reproduce, reproducibility wins. | §3.12 |
| **P4** | **Explanation outranks autonomy.** Where MCF could either decide for the user or explain to them, it explains. | §4 |
| **P5** | **Lightness outranks features.** Refusing a feature on weight is a normal outcome, not a failure. | §3.13, §4 |

Two constraints on using this list. A conditional or low-value rule never
overrides an absolute one, whatever the precedence order suggests. And
precedence resolves *conflicts*, never inconvenience: reaching for P5 to avoid
work that A6 requires is a misuse of the list, and P2 exists to say so.

---

## A — Absolute

Twenty-seven rules. Each admits no exception.

### A1 — Never lose information
"Never fail" means MCF never becomes unable to tell you what happened. It is
always correct to report failure, untrustworthiness or invalidity; it is never
correct to substitute a value, smooth a curve, retry until the output looks
good, or omit an inconvenient outcome.
- **Absorbs:** §6.1, §I, §3.1
- **Check:** `review` — the load-bearing rule of the project, and the one least
  reducible to a mechanism. Every other absolute rule below is a specific,
  checkable instance of it, which is how it gets enforced in practice.
- **Violation looks like:** a retried request presented as the original; a
  failed trial dropped from an average; an error rendered as a zero.

### A2 — No silent failure
Every failure is caught, classified against the taxonomy, attributed to a
subsystem, and persisted with enough context to reconstruct it without a rerun.
A swallowed exception, a bare catch-and-continue, or a default substituted for a
missing value without a record is worse than a crash, because a crash is honest.
- **Absorbs:** §3.1
- **Check:** `compiler` + `CI` — no `unwrap`, `expect`, `panic`, `todo`,
  `unimplemented` or discarded `Result` in non-test code (B-003).
- **Violation looks like:** `let _ = write_record(…);`

### A3 — The manager survives the managed
No failure of a managed thing — a download, a conversion, a benchmark run, a
served runtime — may take down MCF itself.
- **Absorbs:** §3.1, §I, §7.1
- **Check:** `lab` — child death is injected at every lifecycle stage and the
  daemon returns coherent and queryable (B-033).
- **Violation looks like:** a segfaulting inference engine taking the daemon
  with it.

### A4 — Partial outcomes are outcomes
Partial success is representable and preserved. Nine of ten benchmark tasks
completing is a result with nine data points. Eleven tokens before a runtime
died are eleven tokens, marked truncated.
- **Absorbs:** §3.1, §6.17
- **Check:** `CI` — partial results round-trip through the record with their
  per-unit outcomes intact (B-087).
- **Violation looks like:** an all-or-nothing return type on anything that can
  partially succeed.

### A5 — Degradation is marked
Reduced capability is always accompanied by an explicit statement of what was
lost, and every result produced under it carries that mark. An unmarked degraded
result is a corrupted result.
- **Absorbs:** §3.2
- **Check:** `compiler` — a degraded result is a distinct type that cannot be
  rendered or exported as an undegraded one (B-008).
- **Violation looks like:** a CPU-derived timing displayed beside
  accelerator-derived ones with no distinction.

### A6 — No number without its conditions, its sample count and its spread
The unit of scientific output is not a number; it is a number bound to the
conditions that produced it. Single-shot timings are anecdotes. Any surface that
displays a result and drops its conditions is doing damage.
- **Absorbs:** §3.4, §3.3 (the floor), §II
- **Check:** `compiler` — `Measurement<T>` cannot be constructed without
  conditions, `n` and spread (B-005); `CI` asserts no surface renders a bare
  value (B-073).
- **Violation looks like:** `throughput: 41.2` anywhere in a record or a view.

### A7 — Unknown is recorded as unknown
What is not known is never filled with a plausible value. This governs
provenance, licensing, hardware attributes, model metadata and capability
verdicts alike. Inferred data that looks like recorded data is a form of the
silent failure A2 forbids.
- **Absorbs:** §3.6, §3.18
- **Check:** `compiler` — `Unknown` is a variant of the type, not a sentinel
  value, so the absence has to be handled (B-006).
- **Violation looks like:** defaulting a missing context length to 4096.

### A8 — Confounded comparisons are refused
A comparison is only meaningful when one thing differs. When more than one did,
the honest output is "these are not comparable", not a delta. A confound the
operator declares is science; a confound nobody declared is an error.
- **Absorbs:** §3.4
- **Check:** `CI` — an intentionally confounded comparison is refused by the
  tooling (B-085).
- **Violation looks like:** subtracting two numbers taken at different thermal
  states and reporting the difference.

### A9 — Null and negative results are results
"This model does not fit on this hardware" and "quantization gave no measurable
speedup here" are valuable outputs. They are stored and surfaced as results,
never discarded as failures.
- **Absorbs:** §3.4, §6.3
- **Check:** `CI` — both appear in the record and in the interface as outcomes
  (B-086).
- **Violation looks like:** an empty result set where a null result belongs.

### A10 — Never train on the test
Any suite used for selection is protected from the contamination and
overfitting that would make its numbers meaningless — including MCF's own
tendency to tune toward whatever it measures. Selection and validation suites
share no task, no template and no generator seed.
- **Absorbs:** §3.4, §7.3
- **Check:** `CI` — the two suites are structurally separate and the separation
  is audited (B-125).
- **Violation looks like:** tuning a default until the selection suite improves.

### A11 — No performance number originates in simulation
The laboratory tests *behaviour*, never *speed*. A simulated timing is fiction,
and publishing one violates P1 outright.
- **Absorbs:** §6.16
- **Check:** `compiler` — a measurement taken under the simulated clock cannot
  be constructed as a performance result (B-082).
- **Violation looks like:** a throughput figure produced by a lab scenario.

### A12 — Reality outranks the laboratory
When simulation and real hardware disagree, the world is right and the simulator
is defective. Divergence is a recorded finding about the simulator, and that
direction is never reversed to preserve a green suite.
- **Absorbs:** §6.16, §3.17
- **Check:** `lab` + `blocked (DEC-020)` — real-hardware validation, at a cadence
  and coverage not yet decided (B-140).
- **Violation looks like:** adjusting a real-hardware expectation to match what
  the simulator predicts.

### A13 — Every failure MCF claims to handle has a simulation that produces it
The failure taxonomy and the laboratory's fault catalogue are the same list. A
category with no scenario is an untested claim, and an untested claim is not
made.
- **Absorbs:** §3.17, §7.21
- **Check:** `CI` — the two lists are cross-checked and divergence fails the
  build (B-010).
- **Violation looks like:** a taxonomy entry added without its scenario.

### A14 — The benchmark environment is a sandbox by construction
Tools available to a model under test operate on constructed, disposable state.
No benchmark tool reaches the filesystem, the network, MCF's records or the
serving path — not by policy or configuration, but because those capabilities
are absent from the environment. The result must be identical for a hostile
model and a merely incompetent one.
- **Absorbs:** §6.20
- **Check:** `lab` — an adversarial model attempts escape across every vector
  and reaches nothing real (B-101).
- **Violation looks like:** a capability present and disabled by a check.

### A15 — Untrusted code never runs implicitly
Executing code from a model repository is always the user's explicit, per
artifact decision, made with the risk stated and recorded in provenance. MCF's
own integrity never depends on that code behaving.
- **Absorbs:** §3.7, §6.4
- **Check:** `lab` — deliberately hostile repository code runs and MCF's records
  and state are provably intact (B-025).
- **Violation looks like:** a `trust_remote_code` default of true, anywhere,
  under any convenience argument.

### A16 — Five categories are always gated
These are asked every time, no matter how much friction it adds: executing
untrusted code, consuming large irrecoverable resources, exposing the system to
a network, destroying existing artifacts, and **publishing anything off this
machine**. The line is drawn at *category*, never at *frequency*. Everything
else flows (B1).
- **Absorbs:** §6.14, §3.11, §6.12, §3.20
- **Check:** `CI` — the five categories are enumerable in code and each has a
  test asserting the gate (B-039).
- **Violation looks like:** a `--yes` flag that covers all five.

### A17 — Nothing leaves the machine unchosen
Anything that leaves does so because the user chose it, knowing what it
contains. Local inference is often chosen precisely for privacy, and a tool that
manages it while leaking its contents has betrayed the reason it was installed.
- **Absorbs:** §3.10, §6.8
- **Check:** `CI` + `lab` — no egress path exists that is not user-initiated;
  asserted at the network layer, not by inspection (B-145, B-146).
- **Violation looks like:** anonymous usage statistics, however aggregated.

### A18 — Tests and benchmarks are never conflated
Tests gate correctness: fast, deterministic, hermetic, green. Benchmarks produce
measurements: slow, stochastic, hardware-bound, and with **no pass condition**.
A benchmark that "fails" has usually just told you something true. A regression
detector built on benchmark results is a third thing, and its thresholds are
statistical judgments, not assertions.
- **Absorbs:** §6.7
- **Check:** `CI` — benchmarks cannot fail the build; correctness tests cannot
  emit measurements (B-080).
- **Violation looks like:** a throughput assertion in the test suite, which is
  how suites become flaky and then ignored.

### A19 — Anything reported is tested against an independently known value
Nobody should believe published numbers from software that cannot demonstrate it
computes what it claims. Failure paths are tested as rigorously as success
paths, because A2 makes them a feature; untested error handling is decorative.
- **Absorbs:** §3.5, §II
- **Check:** `CI` — every reported quantity has a test with an independently
  known answer.
- **Violation looks like:** a statistic whose only validation is that it looks
  about right.

### A20 — Estimates are labelled and never promoted
Fast, clearly-labelled estimates are permitted. An estimate can never be
mistaken for a measurement, never be promoted into one, and never be compared
with one. It can only be *replaced* by one.
- **Absorbs:** §4
- **Check:** `compiler` — estimates and measurements are different types with no
  conversion between them.
- **Violation looks like:** an interactive session's tok/s appearing in a
  comparison table.

### A21 — Declared, verified, unknown — never a fourth state
What an artifact claims, what MCF observed, and what nobody knows are three
distinct things and are never confused. Divergence between declared and verified
is a finding, recorded and surfaced; it is often the most useful thing MCF can
say about a model.
- **Absorbs:** §3.18, §6.19, §X
- **Check:** `compiler` — the type prevents a declared capability being read as
  a verified one (B-050); `CI` surfaces divergence (B-058).
- **Violation looks like:** a boolean `supports_tools` field.

### A22 — The headless path can do everything
Every action is available with no display attached. The interface may not be the
only way to do anything. This is self-enforcing: a capability reachable only
through the interface is one the laboratory cannot test, which A19 already
forbids.
- **Absorbs:** §6.21, §XI, §3.14
- **Check:** `CI` — interface actions and control operations are enumerated and
  matched; any orphan fails the build (B-072).
- **Violation looks like:** a confirmation dialog with logic behind it.

### A23 — Work cites intent
Every rule, specification, test and design decision cites the clause it serves.
Work that cannot cite is a finding: record a void in §7 of the intent document
rather than inventing intent silently. An unrecorded void is how a project
acquires intent nobody chose.
- **Absorbs:** §8, doc §"How to use it"
- **Check:** `review` — enforced at review; every rule in this file and every
  item in the backlog carries a citation.
- **Violation looks like:** a rule that seemed obviously right to somebody once.

### A24 — Publication is irreversible, therefore deliberate and itemized
Nothing leaves this machine except by an explicit act, taken per share, that
shows the user **the rows that leave** rather than a description of them, and
that states plainly that the act cannot be undone. There is no default-on
contribution, no "help us improve", and no publication that is a side effect of
enabling something else.
- **Absorbs:** §3.20, §XIV, §3.10, §6.27
- **Check:** `CI` — no egress path exists that is not user-initiated per share;
  the confirmation renders the payload (B-160).
- **Violation looks like:** a settings checkbox that starts sending, or a
  summary shown in place of the data.

### A25 — A contributable record contains no user content, by construction
Prompt and completion content lives in a different store from the system record
and is never written to the contributable one. The guarantee is structural: a
filter can be misconfigured, a store that never held the data cannot leak it.
- **Absorbs:** §6.27, §6.8, D6
- **Check:** `compiler` — the content store and the record store are distinct
  types with no path between them (B-146, B-161).
- **Violation looks like:** one database with an `is_user_content` column and an
  export query that excludes it.

### A26 — Privilege is per-operation, and never reaches untrusted code
The long-lived daemon holds no ambient privilege. Elevation belongs to a small,
auditable helper that performs one named operation from a short enumerable list
and exits. Model repository code and models under test run unprivileged in the
sandbox, always, with no configuration that relaxes it. Elevation is a gated
category (A16) and MCF runs without it, marking every measurement it could not
take.
- **Absorbs:** §XVII, §6.32, §3.7, §6.20, §3.10
- **Check:** `lab` — the privileged surface is enumerated and each entry tested;
  a scenario asserts untrusted code cannot reach an elevated path (B-180).
- **Violation looks like:** running the daemon as root because one reading
  needed it, which converts every other rule in this file into a formality.

### A27 — MCF changes nothing it cannot restore, and restores everything it changes
What MCF owns it changes freely. What it can restore it may change with
permission, records as a measurement condition, and reverses when the work ends
— including after a crash. What it cannot restore it does not touch: installing,
upgrading a driver, editing another program's data, terminating a process
holding unsaved work, modifying weights. The test has one right answer: *if this
run were interrupted at the worst possible moment, could the machine be returned
to how it was found?*
- **Absorbs:** §3.25, §XVII, §3.10, §3.11, §6.39
- **Check:** `lab` — a scenario kills MCF mid-run at every stage and asserts the
  environment is restored: governors, priorities, exclusive modes, suspended
  processes (B-220).
- **Violation looks like:** a benchmark that leaves the performance governor
  pinned, which is a permanent change made to produce a temporary number.

---

## B — Conditional

Fifty-eight rules. Each holds under a stated condition, or permits something
provided a condition is met.

### B1 — Defaults flow, provided they are recorded, attributed, explained and overridable
MCF chooses a sensible default for everything reversible and benign —
quantization, context length, runtime, placement — so nothing blocks the first
token. The condition: every such choice is visible, attributed, explained on
demand with the reasoning and measurements behind it, and overridable. The user
is never stopped to be *informed*; only to *authorize* (A16).
- **Absorbs:** §3.15, §6.14, §VI
- **Check:** `CI` — every configured value resolves to a source and an
  explanation (B-038, B-059).
- **Violation looks like:** a default nobody can ask about, which is also a
  measurement condition nobody can reproduce.

### B2 — Optimization may not introduce undeclared state
Caching, process reuse and adaptation are permitted and expected. The condition:
anything that could change a result is visible in that result's conditions. A
measurement taken with a warm cache is a different measurement from one taken
cold, and MCF must know which it produced.
- **Absorbs:** §6.13, §3.12
- **Check:** `CI` — warm and cold are distinguishable in the record (B-081).
- **Violation looks like:** "we already validated this once, so skip it."

### B3 — The measurement path does not adapt, and its instrumentation is reduced and characterized
What MCF does to be fast for the user, it does not do while measuring. Adaptive
behaviour that improves the serving experience destroys the isolation A8
requires. Instrumentation during measurement is reduced to a declared profile,
and MCF's own overhead is measured and travels as a condition — an
uncharacterized instrument is not a scientific one.
- **Absorbs:** §6.13, §6.2, §3.8
- **Check:** `CI` — pre-flight disables adaptive paths and records the profile;
  overhead is measured and attached (B-081, B-012).
- **Violation looks like:** a batch size that responds to load during a
  benchmark.

### B4 — Record at events, not on a timer
Something happening is a reason to write; time passing is not. Ambient
telemetry — continuous sampling, always-on tracing, metric streams — is refused
by default. **The condition is a decision, not a discovery:** this holds until
§6.9 is amended, and reducing telemetry may never be allowed to erode the record
itself, which is not on the dial.
The refusal is about *ambient* observation of MCF. Instrumentation inside a
laboratory, for the duration of an experiment the user started, is not ambient
and is governed by B30 instead.
- **Absorbs:** §3.3, §6.9, §3.13, §5, D5
- **Check:** `CI` — an idle daemon performs zero timer wakeups and writes zero
  records over the observation window, and the figure does not change with the
  number of labs compiled in (B-004, B-031).
- **Violation looks like:** a background sampler that runs whether or not
  anything is occurring.

### B5 — Untrusted execution: per artifact, authorized, contained, recorded
Where A15 permits the user to choose execution, it happens under containment
strong enough that hostile or broken model code cannot corrupt MCF's records or
state, and the choice is recorded in provenance permanently.
- **Absorbs:** §6.4, §3.6
- **Check:** `lab` + `blocked (§7.17 degree)` — containment strength is a real
  open question; the current answer is asserted by scenario (B-025).
- **Violation looks like:** containment that protects the host but not the
  record.

### B6 — Optimize against a declared objective
There is no universally best configuration. MCF maps the frontier and lets a
declared objective choose the point. Absent a declared objective it uses an
explicit, visible, overridable default — never a hidden or emergent one. Every
recommendation carries its objective, its reasoning and its runners-up.
- **Absorbs:** §6.5, §3.9, §IV
- **Check:** `blocked (DEC-002)` — unfalsifiable until the objective function
  exists (B-120).
- **Violation looks like:** a recommendation with no stated objective, which is
  an oracle.

### B7 — Coverage governs attempt and diagnosis, not success
"Any model" commits MCF to accepting any reference without special-casing,
reaching a defined actionable outcome for every one, and never being damaged by
a hostile or malformed one. It does not commit MCF to running any of them. The
commitment is **no unhandled outcomes**, not **no unsuccessful outcomes**.
- **Absorbs:** §6.3, §III, §3.7
- **Check:** `lab` — hostile-hub fixtures all reach a classified outcome with no
  state damage (B-020, B-022).
- **Violation looks like:** a hang, which is the one outcome that is neither
  success nor diagnosis.

### B8 — Local by default; exposure is deliberate and revocable
MCF binds locally. Reachability from another device is a capability the user
turns on deliberately, knowing what becomes reachable, and can revoke. It is
never the out-of-box state and never a side effect of enabling something else.
Access to the control plane is access to a service that downloads and runs code
from the internet, and is treated with that seriousness rather than the
informality typical of localhost developer tools.
- **Absorbs:** §6.12, §3.10, §V
- **Check:** `CI` — the default configuration is unreachable from another host
  (B-036); protection once exposed is `blocked (DEC-017)`.
- **Violation looks like:** binding `0.0.0.0` because it was convenient for
  testing.

### B9 — Record the system exhaustively, the content minimally, and never in the same store
Metrics, timings, resource states, configurations and error conditions are
recorded in full. Prompt and completion content is a distinct category with its
own explicit retention. Benchmark suite content is fixture data, not user data,
and may be kept in full — which is exactly why the two must be structurally
separated rather than separated by a flag.
- **Absorbs:** §6.8, §3.10
- **Check:** `CI` — separate stores; the separation cannot be defeated by
  configuration (B-146). Defaults are `blocked (DEC-005)`.
- **Violation looks like:** one table with a `is_user_content` column.

### B10 — Configure only from an observation or a marked declaration
MCF may configure anything automatically provided it can say, for every setting,
whether the value came from a declaration, from an observation, or from a
default — and which probe established it. Where a capability cannot be
established either way it is unknown (A7, A21), and MCF does not quietly pick a
value that makes the model appear to work.
- **Absorbs:** §6.19, §3.18, §X
- **Check:** `CI` — every auto-set parameter resolves to a probe reference or a
  marked default (B-059, B-060).
- **Violation looks like:** an inconclusive probe coerced into a working
  default, which corrupts every measurement taken under it.

### B11 — Automatic configuration does not change under the user silently
If MCF learns something better, it either asks, or it marks prior results as not
comparable, or both. It never silently reconfigures a model the user has been
using, because yesterday's benchmark and today's would then not be comparable
and A6 would be violated invisibly.
- **Absorbs:** §7.25, §6.13, §3.11
- **Check:** `blocked (DEC-025)` — the branch has not been chosen (B-061).
- **Violation looks like:** a probe methodology change applied on upgrade
  without a word.

### B12 — Agentic results are distributions, and everything except the model is held still
Success rate over `n` trials with its spread and its **shape** — bimodal
outcomes make a mean a poor summary and it is labelled as such. The task, the
tool implementations, the environment's starting state, the seeds, the harness
version and the sampling parameters are all pinned and recorded. Irreducible
stochasticity is reported, not engineered away: temperature zero is not a fix,
it is a different and less representative experiment.
- **Absorbs:** §6.17, §IX, §3.4
- **Check:** `CI` — no surface can render an agentic result as a single number
  (B-104); trial counts and difference tests are `blocked (DEC-023)`.
- **Violation looks like:** a leaderboard row.

### B13 — Evaluation resembles the work and grades by verification
Evaluations look like use: multi-turn, tool-calling, instruction-bound,
format-constrained, error-prone, scored on whether the task got done. Outcomes
are *checked* — the call parsed, the file was written, the value matched, the
loop terminated — never graded by another model. A verifiable task needs no
ground-truth corpus and is far harder to have memorised. *How* a trial failed is
more informative than the pass rate.
- **Absorbs:** §3.19, §IX, §6.17, §7.3
- **Check:** `CI` — every task grades by verification (B-102); every failed
  trial is classified (B-105).
- **Violation looks like:** a model-as-judge introduced quietly for convenience.

### B14 — Destruction is previewed, and the record outlives the artifact
Eviction, overwrite and cleanup are real operations with real cost: previewed,
logged, reversible where reasonable, and never automatic to reclaim space.
Removing an artifact never erases the fact that it was here.
- **Absorbs:** §3.11, §3.6
- **Check:** `CI` — no code path deletes without a recorded authorization
  (B-027).
- **Violation looks like:** a cache eviction policy.

### B15 — Weight is admitted only against a stated cost
The overhead of a feature is part of the feature's cost, and features are
refused on that basis; refusal is a normal outcome. Each dependency is admitted
for a stated reason and justifies itself against the alternative of not
existing. An instrument grows for **validity**, never for capability (B23).
- **Absorbs:** §3.13, §4, §5, §6.18
- **Check:** `review` + `CI` — budgets fail the build (B20); the reasoning is a
  review artifact.
- **Violation looks like:** a dependency added because it was familiar.

### B16 — Prefer the machine-checked form of every rule
Where a choice of language, structure or tooling determines whether a rule is
checked by a compiler or merely hoped for, choose the one that checks. Make
illegal states unrepresentable rather than validating against them. Every
`review` check in this file is a standing candidate for promotion to `CI` or
`compiler`.
- **Absorbs:** §3.16, §7.19
- **Check:** `review` — measured by the review-check count reported at the top of
  this file, which this rule exists to drive down.
- **Violation looks like:** a code review comment where a type would have done.

### B17 — Coverage is whole-system
A suite that proves every function correct in isolation and never exercises the
daemon end to end has tested the parts and not the thing. Supervision, recovery,
contention and degradation exist only *between* components, and those are
precisely the behaviours §I is about.
- **Absorbs:** §3.5, §7.22
- **Check:** `CI` — whole-system tests cross the process boundary and exercise
  restart with persisted state; the exact boundary is `blocked (DEC-022)`.
- **Violation looks like:** 100 % unit coverage and no end-to-end test.

### B18 — A bug becomes a fixture before it becomes a fix
An escaped bug is reproduced in the laboratory as a permanent scenario first.
Diagnosis happens by reproduction, not by having logged enough in advance — that
is the telemetry strategy B4 declines.
- **Absorbs:** §3.5, §3.17, §6.15
- **Check:** `review` — checkable from the commit record (B-143).
- **Violation looks like:** a fix with no test, justified by the bug being
  obvious.

### B19 — The suite runs on a laptop, offline, with no accelerator
Tests must not require a GPU, a network or a large model. Expensive paths are
tested through seams, and the seams are part of the design rather than an
afterthought. Simulated hardware, synthetic artifacts, fake hubs and replayed
conditions are legitimate and expected: a lab has calibration rigs, and so do
we.
- **Absorbs:** §3.5, §3.17
- **Check:** `CI` — the full suite runs offline on a machine with no accelerator
  (B-015).
- **Violation looks like:** a test skipped in CI because the runner has no GPU.

### B20 — Budgets are asserted, and performance changes carry before-and-after
Idle CPU, resident memory, disk footprint, cold start and the latency MCF
interposes between request and first token are budgeted, measured and
regression-tested. A performance change without a before-and-after under stated
conditions is not a performance change; it is a guess that also increased
complexity. Optimize what is measured, not what is imagined.
- **Absorbs:** §3.13, §3.5, §VII
- **Check:** `CI` — budgets fail the build on regression (B-011); the numbers
  themselves are `blocked (DEC-016)`.
- **Violation looks like:** "fastest and lightest possible" invoked with no
  number attached, which is unfalsifiable and therefore indefensible.

### B21 — The failure record is sufficient exactly when the lab can rebuild the failure from it
This is a measurable requirement, not an aspiration. When the laboratory cannot
reconstruct a failure from its record alone, that is a defect **in the record**,
and the fix is more context at the failure site — never a return to ambient
streaming.
- **Absorbs:** §6.15, §3.1, §3.17
- **Check:** `CI` — a sample of real failures is reconstructed from records
  alone and the rate is tracked against a target (B-142).
- **Violation looks like:** a failure record that reads well and reproduces
  nothing.

### B22 — The interface is a client, not a place
The interface makes state legible and triggers actions. It holds no logic, no
authority, and nothing hides in it. It consumes the same API a script uses.
Universality is bought by demanding almost nothing of the client — not by
bundling a runtime, shipping a framework or building per-platform applications —
and its idle cost, when nobody is looking, is indistinguishable from zero.
- **Absorbs:** §3.14, §6.11, §V, §XI
- **Check:** `CI` — client weight and cold render meet budget; daemon CPU with
  an idle tab open is zero (B-070, B-071).
- **Violation looks like:** a UI that polls a busy machine every second in order
  to look responsive, spending the user's inference budget on decoration.

### B23 — The instrument grows for validity, never for capability
Applied ruthlessly and directionally to the agentic harness, the laboratory and
every measuring device MCF builds: *does this make the measurement more valid,
or does it make the harness more capable?* Only the first justifies weight. It
costs nothing when no benchmark is running.
- **Absorbs:** §6.18, §5, §3.13
- **Check:** `CI` + `review` — zero idle cost asserted (B-108); refusals
  recorded rather than forgotten (B-109).
- **Violation looks like:** a tool-authoring API, which is how a harness becomes
  a framework.

### B24 — Unattributable is a verdict
Hardware is a time-varying condition, not a static fact read once at install.
MCF knows the difference between "this model is slow" and "this machine was
busy". When it cannot tell, it says so rather than attributing the result — and
contended trials are excluded from a claim rather than averaged into one.
- **Absorbs:** §3.8, §7.9
- **Check:** `lab` — a deliberately contended run is marked unattributable
  (B-088).
- **Violation looks like:** a number that is really a claim about an unrelated
  process.

### B25 — Scope refusals
A feature is refused, without further argument, when it exists only to make MCF:
a training or fine-tuning platform; a ranking of models across machines; a fleet
orchestrator; a model-quality authority; a wrapper that makes the runtime
unreachable; opinionated about which model you should want; a chat product; a
platform with a plugin ecosystem or extension API — laboratories included (B32);
an agent framework; an observability platform *for MCF itself*, as distinct from
instrumentation of the model under test (B30); the aggregating website of §XV;
or a broker of anyone's data. "Because that is how the other tools do it" is not
an argument, and matching a competitor's feature is never in itself a reason to
carry its weight.
- **Absorbs:** §5 (all thirteen anti-goals)
- **Check:** `review` — a refusal cites this rule and the specific anti-goal.
- **Violation looks like:** a feature whose only justification is a comparison
  table.

### B26 — Managed tensions are not relitigated
These are permanent conditions of the problem, named so that they are managed
rather than re-argued: rigor costs time; coverage costs sharpness; automation
costs agency; good evaluation is expensive; simulated confidence is not real
confidence; ease costs transparency; lightness costs features; and reduced
observation costs retrospective diagnosis. Reopening one requires new
information, not renewed discomfort.
- **Absorbs:** §4, §6.15
- **Check:** `review` — a proposal to resolve one of these cites what is new.
- **Violation looks like:** the third redesign this year of the same trade-off.

### B27 — The laboratory is production code and does not grade itself
The lab is held to every rule in this file, because everything else is believed
on its authority — a sloppy simulator produces confident wrong results.
Determinism is a feature of the lab, not of the world: simulated time, injected
faults and replayable scenarios exist so a failure found once reproduces
exactly, forever. Its fidelity is measured against reality (A12) and its
boundary is stated, so confidence is claimed only where it was earned.
- **Absorbs:** §3.17, §6.16, §VIII
- **Check:** `CI` — scenarios are deterministic across repeated runs (B-009);
  the stated fidelity boundary is `blocked (DEC-021)` (B-141).
- **Violation looks like:** a test helper nobody reviews, which is where
  confident wrong results come from.

### B28 — The reference model is a fixture, never a case in the code
`unsloth/Qwen3.8-27B-GGUF` is the subject of early benchmarking, probing,
evaluation and real-hardware validation. The condition: no code path behaves
differently because an artifact is the reference model, and the test suite never
depends on it — B19's hermetic suite is unchanged by §XII. The check is
mechanical: substituting a different model must change what is measured and
nothing about how MCF behaves.
- **Absorbs:** §XII, §6.22, §III, §3.5
- **Check:** `CI` — the suite runs offline with no real weights (B-015, B19);
  a grep-level check that no identifier names the reference model outside
  fixtures and documentation (B-018).
- **Violation looks like:** a special case for GGUF-from-unsloth that makes the
  reference model work and quietly breaks the next artifact.

### B29 — One model builds an instrument; it never supports a generality claim
Results measured on the reference model characterize *the instrument* — that it
records conditions, classifies failures and reproduces runs. They characterize
models in general not at all. No §IV recommendation is made from a field of one:
a frontier with one point is not a frontier, and MCF refuses rather than ranks.
A10 applies to MCF itself here — tuning a default until the reference model
looks better is training on the test, however reasonable each adjustment seemed.
- **Absorbs:** §6.23, §3.4, §3.9, §XII
- **Check:** `CI` — the recommender refuses a single-candidate field (B-127);
  the breadth required before a generality claim is `blocked (DEC-026)`.
- **Violation looks like:** "measured on the reference model" quietly becoming
  "measured", which is how one artifact turns into a claim about the world.

### B30 — Lab instrumentation is scoped to its experiment and recorded as a condition
A laboratory instruments as deeply as its question requires, provided the
instrumentation exists only while the experiment runs, watches the model under
test rather than MCF, never touches the serving path a user's application is
talking to, and appears in the result's conditions. Deep telemetry inside a lab
is not the ambient telemetry B4 refuses; it is the experiment.
- **Absorbs:** §XIII, §3.22, §6.24, §6.2
- **Check:** `CI` — idle cost is identical with three labs and thirty (B-162);
  every lab result carries its instrumentation profile (B-163).
- **Violation looks like:** a lab that keeps a sampler running after its run
  finishes, which is how a bench becomes a monitoring system.

### B31 — Timing-class measurements run under a reduced profile
A lab states whether its outputs are timing-class or behaviour-class. Timing
results are taken under reduced instrumentation with the residual overhead
characterized; behaviour results — did the call parse, did the loop terminate,
did the model recover — may instrument freely, because watching them does not
perturb them. A measurement that is both is timing-class.
- **Absorbs:** §6.25, §6.2, §3.4
- **Check:** `compiler` — a timing result cannot be constructed from a
  deep-profile run (B-164).
- **Violation looks like:** a throughput figure from the lab that watches every
  token, compared against one from the lab that does not.

### B32 — Laboratories are in-tree and admitted one at a time
There is no lab API, no third-party lab, no discovery mechanism and no
configuration language for labs. Each is code in this repository, held to every
rule that governs the rest of it, and each is admitted by answering one
question: *what claim can MCF make once this lab exists that it cannot make
now?* A lab that makes MCF better at running experiments in general, rather than
able to make a specific new claim, is refused and the refusal is recorded.
- **Absorbs:** §6.26, §5, §6.18, §3.13
- **Check:** `review` — the admitting question is answered in the lab's own
  documentation, and refusals are recorded (B-165).
- **Violation looks like:** a lab base class with extension points.

### B33 — An imported configuration is declared until this machine verifies it
A configuration arriving by identifier is reproduced exactly — weights,
quantization, context, runtime, sampling parameters — and every parameter it
sets is attributed to that identifier. Numbers that travelled with it are
somebody else's measurement until MCF takes its own. Resolution passes the same
gates any acquisition passes (A15, A16). Failure to reproduce here is a
first-class outcome, and different numbers here are a *finding* about how far
results travel, not a failure of the import.
- **Absorbs:** §XV, §3.21, §6.29, §3.18, §3.7
- **Check:** `CI` — an imported configuration reads as `declared` until a local
  probe or benchmark verifies it (B-166).
- **Violation looks like:** an imported benchmark figure displayed as though MCF
  measured it.

### B34 — Foreign numbers never decide a local configuration
MCF contributes outward and decides inward. §IV's recommendations come from
measurements taken on this machine under §3.4's conditions. An aggregate from
elsewhere may tell a user what to *try*; only a local measurement tells them
what to *run*. MCF neither computes nor displays a ranking, and a contributed
row without its full conditions is not contributable at all.
- **Absorbs:** §6.28, §XIV, §5, §3.4
- **Check:** `CI` — the recommender's inputs are locally-originated
  measurements, enforced at the type level (B-167).
- **Violation looks like:** "most users run Q4_K_M" appearing anywhere near a
  recommendation.

### B35 — Timing-class work opens an exclusive window; nothing else takes the machine
A timing-class run — throughput, latency, energy, memory scaling — requires a
quiet machine, because a timing taken under contention measures the contention.
It opens a window that is announced before it starts, bounded by a declared
maximum, schedulable, interruptible, and closed automatically. Inside it MCF may
be greedy; outside it MCF holds nothing. No behaviour-class run and no serving
request may proceed inside the window, and no window is opened without an
explicit decision.
- **Absorbs:** D8, §6.33, §6.25, §3.8, §7.9
- **Check:** `CI` — a timing result cannot be constructed from a run that
  overlapped serving or another lab; opening a window drains the endpoint by
  explicit decision and refuses requests within one round trip (B-181, B-182).
- **Violation looks like:** a throughput number taken beside a served model,
  which measures the pair and reports it as the model.

### B49 — Behaviour-class work yields, and records the contention it ran under
A behaviour-class run — tool calls, agentic tasks, extraction, retrieval, code —
does not take the machine, because its outcomes do not depend on having it. It
runs at low priority, behind user traffic on the serving path, pausable and
resumable, with contention recorded as a condition (§3.4) rather than prevented.
Two consequences are obligatory, not optional: its deadlines are **token budgets
rather than wall clocks**, since a task that failed because the machine was busy
is a measurement of the machine; and an **environment failure is classified
apart from a model failure**, so an out-of-memory caused by competition is a
condition of the run rather than the model giving up.
- **Absorbs:** D8, §6.40, §3.26, §3.8, §7.10
- **Check:** `compiler` — a behaviour lab cannot express a wall-clock deadline
  (B-230); `lab` — a run under injected contention produces the same outcomes as
  one without, with different conditions (B-231).
- **Violation looks like:** an agentic task marked failed because a game was
  running, which attributes the machine's state to the model.

### B50 — Hosting yields to the user, and nothing yields to hosting
A served endpoint is a service somebody is using. No measurement, laboratory or
background task may make it slow or unavailable, except inside B35's declared
window. MCF defaults to the polite mode — low priority, yielding, interruptible,
pausable — and the user escalates; MCF never assumes the escalation. Nothing is
taken without being announced first.
- **Absorbs:** §3.26, §VI, §I, §6.40
- **Check:** `CI` — serving latency under a concurrent behaviour-class run stays
  within its budget (B-232).
- **Violation looks like:** a user discovering MCF took the machine by noticing
  their machine is gone.

### B36 — MCF ships what it needs; a missing prerequisite is never the user's errand
The user obtains MCF and runs it: no runtime, interpreter, toolchain, framework
or separately-fetched engine. Anything on the common path is vendored or
reimplemented; where that is impossible the feature is refused rather than
converted into an instruction. Absent *platform* capabilities — a vendor driver,
hardware that is not present — are stated, marked unavailable, and continued
past (§3.2), never turned into a request.
- **Absorbs:** §XVI, §6.31, §3.12, §3.2
- **Check:** `CI` — a from-scratch container with no toolchain runs the binary
  and reaches a first token (B-183).
- **Violation looks like:** an error message containing installation
  instructions, which is an unpinned dependency wearing a helpful face.

### B37 — Durations are monotonic, records are UTC, lab time is simulated
A duration is never computed from wall-clock readings. Records timestamp in UTC
with the local offset stored alongside. The laboratory's clock is simulated and
a result knows which clock produced it, so a simulated duration can never be a
performance number (A11). A clock anomaly during a measurement invalidates that
measurement loudly rather than being smoothed.
- **Absorbs:** D9, §3.4, §3.17, §6.1
- **Check:** `compiler` — duration and timestamp are distinct types with no
  arithmetic between them (B-184); `lab` — clock-jump scenarios invalidate
  rather than corrupt.
- **Violation looks like:** `end_wall - start_wall`, which silently reports an
  NTP correction as latency.

### B38 — The suite is tiered, and every tier reports its age
The fast hermetic tier gates every change and stays hermetic and fast (B19).
Load, soak, mutation and the full fault matrix run on a schedule and before
every release. A heavy tier that has not run recently is reported as **stale**,
never assumed green — an unstated staleness is A2's silent failure aimed at the
suite. Mutation score is budgeted like any other property (B20) and may not
regress silently.
- **Absorbs:** D10, §6.34, §3.5, §3.13
- **Check:** `CI` — tier ages are published with every result set and a stale
  tier fails a release (B-185); mutation score has a floor (B-186).
- **Violation looks like:** a green badge that means "the fast tests passed" and
  is read as "the software works".

### B39 — Energy is measured, estimated, or unknown — and sampled only in a lab
Power and thermal counters are read during a laboratory run, at a declared
sampling rate recorded as a condition, never by an ambient sampler. A vendor's
modelled figure is recorded as an estimate (A20); a platform with no interface
yields `unknown` (A7) and never a number derived from utilization. Energy
figures taken under different sampling profiles are not comparable (A8).
- **Absorbs:** D11, §6.35, §3.9, D5
- **Check:** `CI` — idle MCF reads no counters (B-187); every energy value
  carries its provenance and sampling rate (B-188).
- **Violation looks like:** watts inferred from GPU utilization, presented in
  the same column as watts that were measured.

### B40 — A laboratory runs only where the capability is verified; not applicable is not zero
A lab runs against a model only where the capability it depends on has been
verified present (§X), and reports **not applicable** where it is verified
absent. Four outcomes exist and are never collapsed: *measured*, *not
applicable*, *unknown*, *failed*. A model with no tool-calling that scores 4 %
on an agentic suite has not been measured badly — it has not been measured.
- **Absorbs:** §3.23, §X, D2, §3.4
- **Check:** `compiler` — a lab result is a sum type whose *not applicable* and
  *unknown* variants carry no score and cannot be averaged (B-200).
- **Violation looks like:** a coverage table where absent capabilities render as
  low numbers, which turns the wrong instrument into a verdict.

### B41 — MCF publishes a profile, never a score, and ranks only against a declared workflow
There is no weighted average across laboratories, no overall rating and no
general ranking. Within a *declared* workflow (§6.5) MCF answers plainly, with
reasoning and runners-up; across workflows it refuses, because "which model is
better" has no referent once quality is plural. Coverage travels with every
answer: which laboratories informed it, and which the candidates were
inapplicable to.
- **Absorbs:** D2, §6.36, §3.9, §5
- **Check:** `compiler` — no type combines results from two laboratories into a
  scalar (B-201); `CI` — every recommendation renders its coverage (B-202).
- **Violation looks like:** an "overall quality" column, which is the
  leaderboard §5 refuses wearing local clothes.

### B42 — A customized workload yields local results: not comparable, not contributed
A laboratory ships a default workload and accepts a replacement. Results from
the default are comparable and contributable; results from a replacement are
marked non-comparable at the point of *production*, not at the point of export,
and never enter a contribution — they are the user's content (A25) and are
uninterpretable to anyone who cannot see the workload. The customization surface
is a workload slot — data, schemas, labels, constraints, documents, tasks — never
code, so B32's prohibition on a lab API is unaffected.
- **Absorbs:** §6.37, §XIII, §3.19, A25, B32
- **Check:** `compiler` — a custom-workload result cannot be constructed into a
  contribution (B-203); `CI` — a lab declares what its slot accepts and refuses
  what it cannot grade (B-204).
- **Violation looks like:** a "share my results" button on a lab nobody else can
  reproduce.

### B43 — The corpus narrows the search; local measurement decides
Contributed data may order candidates, prune a search space, warn that a
configuration has no working reports on hardware like this, and supply a first
duration estimate where local history is absent. It may never be the source of a
number MCF reports about this machine (B34). Every corpus-derived statement is
labelled, carries its sample count, is visibly distinguishable from a local
measurement, and is overridable. Arithmetic may refuse; the corpus may only
advise.
- **Absorbs:** §6.38, §6.28, §XIV, D14
- **Check:** `compiler` — a corpus-sourced value and a locally-measured value are
  distinct types, and only the second can back a recommendation (B-221).
- **Violation looks like:** a sorted candidate list whose ordering nobody
  explains, which is a foreign conclusion wearing a local interface.

### B44 — Unreported is not unsupported
A corpus-derived negative states what the corpus contains, never what reality
permits. It carries its sample count, so a claim resting on two reports reads
differently from one resting on four hundred, and a claim resting on nothing
says so. No option is ever hidden on corpus grounds: it is ranked lower,
annotated, and still reachable.
- **Absorbs:** §3.24, §3.1, §3.4
- **Check:** `CI` — every corpus statement renders its `n`; no filter removes a
  candidate from a listing (B-222).
- **Violation looks like:** "not supported on your hardware", which converts
  nobody-tried-it into a fact.

### B45 — Calibration precedes measurement
A laboratory in the characterization or evaluation tier runs only against a
calibrated configuration (D13). Calibration adjusts as well as detects, and each
adjustment carries the provenance of the probe that forced it (B10). Running a
long evaluation on an uncalibrated configuration is not merely wasteful — §X
makes it a measurement of the misconfiguration.
- **Absorbs:** D13, §X, §3.18, §3.8
- **Check:** `compiler` — an evaluation run cannot be constructed from an
  uncalibrated configuration handle (B-223).
- **Violation looks like:** a twenty-hour agentic run against a model whose chat
  template was never verified.

### B46 — Durations are estimated from measured work, banded, and scored
A laboratory declares its work in units it can count — trials, sweep points,
tokens, documents — never in minutes. The machine supplies the rate from the
characterization tier. The product is a band, marked an estimate (A20), absent
where no local history exists unless a labelled corpus prior fills it (B43).
Every estimate is scored against what happened, and persistent error is a
finding about the laboratory or the approximator.
- **Absorbs:** D14, §3.4, A20, §6.16
- **Check:** `CI` — no lab declares a duration directly; estimate error is
  tracked and reported (B-224, B-225).
- **Violation looks like:** a progress bar that lies, which is the smallest
  possible version of a confident wrong number.

### B47 — A budget produces a proposal, and results are anytime
Given a time budget MCF proposes what it will run **and states what it is
leaving out and why**; it never silently truncates, because a selection that
quietly drops work reads as coverage it never had. Every laboratory reports as
it goes, so a run stopped early keeps what it produced, marked incomplete (A4).
- **Absorbs:** D14, §3.1, A4, §VI
- **Check:** `CI` — a budgeted selection renders its exclusions; interrupting a
  lab preserves and marks the partial result (B-226, B-227).
- **Violation looks like:** "ran 6 of 20 laboratories" with no list of the
  fourteen.

### B48 — Environment control climbs a ladder, and only as far as agreed
Report what is competing, always and freely. Wait for quiet, with a stated
timeout, as the default for a laboratory that needs it. Suspend and restore only
with per-run approval of a named list. Never terminate a process and never touch
what was not approved. Everything MCF changes about the environment is a
measurement condition (§3.4) and is restored (A27).
- **Absorbs:** §6.39, §3.25, §3.8, §3.11
- **Check:** `lab` — a scenario asserts nothing outside the approved list is
  touched and every suspension is resumed, including when MCF is killed (B-228).
- **Violation looks like:** a "focus mode" that kills processes, which may
  destroy unsaved work MCF cannot see.

### B51 — A box is a measurement condition, and names what it did not bound
A declared resource allocation travels with every result taken inside it, and so
does the list of dimensions it could not enforce — memory bandwidth, cache,
accelerator time-slicing, PCIe, thermal. Boxed and unboxed results are never
compared, and neither are results from different boxes (A8). Where a platform
cannot enforce a dimension, that dimension is `unknown` and the box does not
claim it (A7).
- **Absorbs:** D15, §6.41, §3.4, §7.43
- **Check:** `compiler` — a result carries its box or the explicit absence of
  one, and the two cannot be compared (B-240).
- **Violation looks like:** a box that appears to isolate and does not, which
  attaches a reproducible-looking number to an unreproducible quantity.

### B52 — Politeness is measured, not asserted
Yielding has two costs and they belong to different disciplines (D10). What it
costs the **user** is an application test: a background run must not degrade an
interactive workload beyond a stated budget. What it costs the **model** is a
laboratory: L24's dose-response and L25's contention sweep. A claim that MCF is
a good guest is worth exactly what those two measurements say.
- **Absorbs:** §3.26, D10, §6.40, §3.13
- **Check:** `CI` — interactive degradation under a background run is asserted
  against a budget (B-241); `lab` — degradation curves are produced rather than
  assumed (B-242).
- **Violation looks like:** "runs at low priority" offered as evidence that it
  is unobtrusive.

### B53 — Comparisons are paired, interleaved and randomized within one session
Arms of a comparison are interleaved — A, B, A, B — rather than run in blocks,
so drift in thermal state, contention or clock affects both equally, and order
is randomized so going first is not an advantage. The reported quantity is the
**paired difference and its distribution**, not the difference of two means. A
comparison assembled from separate sessions is a weaker claim and is labelled as
one.
- **Absorbs:** §3.27, §3.4, §3.8, §6.41
- **Check:** `compiler` — a comparison result can only be constructed from
  paired trials carrying a common session id (B-250).
- **Violation looks like:** thirty runs of A, then thirty of B, subtracted —
  which reports the afternoon's drift as a difference between configurations.

### B54 — What travels is the comparison; what stays is the absolute
A contribution carries comparisons — *this configuration beat that one by
roughly this much, on hardware like this* — in preference to absolute figures,
because a tokens-per-second number from a stranger's machine is nearly
uninterpretable while a ratio survives the messiness it came from. Absolute
figures remain in the local record, fully conditioned, and remain the basis of
every local decision (B34).
- **Absorbs:** §3.27, §XIV, §6.28, §6.38
- **Check:** `CI` — a contribution's comparison rows carry both arms and the
  pairing; absolute rows carry the full §3.4 condition set or are not
  contributable (B-251).
- **Violation looks like:** a corpus of bare throughput numbers, which is a
  leaderboard with extra steps.

### B55 — Recommendations generalize; the record does not
A recommendation speaks in terms the evidence supports — *meaningfully faster*,
*within noise*, *about a third less memory* — and carries its effect size and
confidence. Generalization happens in the **rendering only**: the record keeps
the precise measurement, and any generalized statement expands on demand into
the measurements, conditions and spread behind it (§3.15). Where the evidence
supports nothing general, MCF says nothing general.
- **Absorbs:** §3.28, §3.9, §3.15, §3.14
- **Check:** `CI` — every generalized statement resolves to the measurements
  behind it, and no summary is written in place of its evidence (B-252).
- **Violation looks like:** "41.2 vs 38.4 tok/s" offered as a recommendation,
  which is false precision inviting action on a difference inside the noise.

### B56 — Trials are kept; summaries are derived
Every trial is a row carrying its value, its arm, its position in the
interleaving and its session. A summary is computed at query time and never
written in place of the trials that produced it. Interior detail — per-token
timings, per-turn outcomes — is declared by the laboratory that needs it and off
by default, and where it must be thinned the thinning is recorded as a condition
(§3.4).
- **Absorbs:** D16, §6.17, §3.27, §7.7
- **Check:** `compiler` — a summary type cannot be persisted, only projected
  from trials (B-270); `CI` — a downsampled series carries its thinning factor
  (B-271).
- **Violation looks like:** a stored mean, which is a question nobody can ask
  again.

### B57 — Identity is the runnable configuration; hardware is a condition
Two runs share an identity when they share the description the hosting system
needs to run the model. Hardware never enters the key — it is a condition, so
the same configuration measured on two machines is one thing observed twice.
Grouping is a view applied at query time, never the key itself, and the rule for
the boundary is to err toward more in the identity: a group can be widened and
never narrowed.
- **Absorbs:** D17, §3.4, §XIV, §XV
- **Check:** `compiler` — the identity type excludes hardware fields by
  construction (B-272).
- **Violation looks like:** a per-machine configuration table, which gives every
  user their own universe and makes the corpus unaggregatable.

### B58 — A laboratory sets up, tears down, and leaves nothing behind
Each lab owns its own setup and teardown and may use whatever tooling its
question requires — labs are not forced through one harness shape. The
obligation that comes with that freedom is A27's: whatever a lab changed, it
restores, including after a crash and including state inside the sandbox. A lab
that leaves residue has contaminated the next one.
- **Absorbs:** §XIII, A27, §3.17, §6.26
- **Check:** `lab` — a scenario runs two labs back to back and asserts the second
  sees no trace of the first, including after the first is killed mid-run
  (B-273).
- **Violation looks like:** a warm cache left behind, which silently makes the
  next lab's first trial different from its rest.

---

## C — Low value

Eight rules. Real, kept, and the least load-bearing we have. They are decided
last when rules compete and may be dropped without amending the intent document.

### C1 — Prose logs are a rendering of the record, never the record itself
- **Absorbs:** §3.3 · **Check:** `review`
- **Why low value:** A6 and B4 already force the record to be structured and
  complete. This rule only prevents someone treating a formatted string as the
  source of truth, which the type system makes awkward anyway.

### C2 — Verbosity above the floor is a dial, defaulting low
The §3.4 floor is not on the dial; everything above it is, and the default is
quiet.
- **Absorbs:** §3.3 · **Check:** `review`
- **Why low value:** a preference about defaults. Nothing downstream breaks if
  the default is wrong, and the floor — which does matter — is protected by A6.

### C3 — Minimal chrome, never minimal truth
A minimalist interface shows less decoration, not less information.
- **Absorbs:** §3.14 · **Check:** `review`
- **Why low value:** the substantive half is A6, which already forbids stripping
  conditions from a measurement on any surface. What remains is taste.

### C4 — Repeatability before precision
A number reproducible to ±10 % beats one quotable to three decimals once.
- **Absorbs:** §3.4 · **Check:** `review`
- **Why low value:** a tie-breaking heuristic for methodology arguments. A6 and
  the acceptance criteria (DEC-007) do the actual work.

### C5 — Identifiers are stable for life
Rule IDs, backlog IDs, void numbers and record IDs are never reused and never
renumbered, so a citation made once remains valid.
- **Absorbs:** §7 preamble, backlog convention · **Check:** `review`
- **Why low value:** a convention. Cheap to hold, mildly annoying to violate,
  nothing scientific rests on it.

### C6 — Nothing is deleted; dropped work keeps its reasoning
Backlog items, rules and resolutions that die are marked dropped with the
reasoning rather than removed, so they are not re-proposed later as oversights.
- **Absorbs:** §8, backlog convention · **Check:** `review`
- **Why low value:** it preserves institutional memory, which is valuable but
  recoverable from version control if the rule lapses.

### C7 — Mockups are illustrative and never citable
Every figure in `mockup/` is invented. No figure there may be cited as a
measurement, and where a mockup and the intent document disagree, the document
wins.
- **Absorbs:** §5, mockup convention · **Check:** `review`
- **Why low value:** A20 and B25 already forbid treating a non-local, non-
  measured number as evidence. This states it where the temptation is highest.

### C8 — Documentation conventions
In documents: `$` is the operator's shell and output is verbatim; records are
shown as JSON for legibility regardless of on-disk form; placeholders are
written `<like-this>`; a surface that must show something unknown shows
`unknown` (A7).
- **Absorbs:** mockup conventions · **Check:** `review`
- **Why low value:** presentation. The one substantive clause is a restatement
  of A7.

---

## Not adopted as rules

Recorded so their absence is deliberate rather than an oversight, per C6.

| Statement | Why it is not a rule here |
|---|---|
| "The fastest, lightest tool it can possibly be" (§VII) | Unfalsifiable as written. It becomes B20 the moment DEC-016 supplies numbers; until then it is a mood, and a rule nothing can be checked against is worse than none. |
| "The highest scientific standards" (§II) | Not operationalized. It is the *source* of A6, A8, A9, A10, A19, B3 and B12, and DEC-007 will make its acceptance criteria checkable. As a standalone rule it would be cited to justify anything. |
| "Runs on any system or device" (§V) | Unbounded as stated (§6.11 says so). B22 carries the checkable half; the client bound is DEC-016. |
| "Reproducible" (§3.12) | Present as precedence P3 rather than a rule, because reproducible *to what tolerance across what changes* is DEC-006. A rule would have to guess the answer. |
| Reliability as uptime | Deliberately not a rule. §6.1 redefined "never fail" as "never lose information", which is A1. A rule about uptime would compete with A1 and lose. |

---

## Amending this file

1. **The intent document is upstream.** A rule changes because a principle,
   resolution or void changed — not the reverse. Code that diverges from a rule
   is either a bug or an argument, and an argument is raised in the intent
   document first (§8).
2. **A new rule must cite, and must carry a check.** A rule with no citation is
   invented intent (A23). A rule with no check is a wish (B16); if the only
   available check is `review`, say so and record what would make it stronger.
3. **A new rule must earn its place against consolidation.** The first question
   is whether an existing rule already covers it. This file holds 93 rules
   refined from about 150 scattered statements, and it is worth less the moment
   it starts growing back. Integrating a whole new intent should cost one or two
   rules, not a section.
4. **Tier changes are decisions, not edits.** Promoting a rule to absolute means
   asserting that no cost justifies violating it. Demoting one means the
   opposite. Both are recorded with reasoning.
5. **Deleting from tier C is normal.** That is what the tier is for.

---

## Coverage

Every normative clause of the source documents, and the rule that absorbed it.
This table is the audit that the refinement lost nothing; a source clause with
no rule is a defect in this file.

| Source | Absorbed by |
|---|---|
| §I Reliability | A1, A2, A3, P1 |
| §II Science | A6, A19, P2, "Not adopted" |
| §III Custody | B7, A7, A15 |
| §IV Optimization | B6, A6 |
| §V Interface | B22, B8, "Not adopted" |
| §VI Hosting | B1, A16, B22 |
| §VII Lightness | B20, B15, B4, P5, "Not adopted" |
| §VIII Verification | B27, A13, A12, B17 |
| §IX Workflow evaluation | B12, B13, A14, B40 |
| §X Capability discovery | A21, B10, B11 |
| §XI Both surfaces | A22, B22 |
| §XII The reference model | B28, B29, B19 |
| §XIII Analysis laboratories | B30, B31, B32 |
| §XIV The shared record | A24, A25, B34 |
| §XV Reproduce by identifier | B33, A15, A16, D12 |
| D12 Import is convenience | B33 |
| D13 Four tiers | B45 |
| D14 Budgets and estimates | B46, B47 |
| §XVI Self-contained | B36, B15 |
| §XVII Full utilization | A26, B35 |
| D8 Exclusivity by measurement class | B35, B49 |
| §3.26 MCF is a guest | B50, B49, B52 |
| D15 Resource boxes | B51 |
| §6.41 Boxes vs the window | B51, A8 |
| §3.27 The comparison is durable | B53, B54 |
| §3.28 Gather precisely, recommend generally | B55, B41 |
| D16 Raw trials are kept | B56 |
| D17 Identity is the configuration | B57 |
| §6.40 Long runs on a used machine | B49, B47 |
| D9 The time model | B37 |
| D10 Test the app, measure the model | B38, A18, B19 |
| D11 Energy is first-class | B39 |
| D6 The record is SQLite | A25, B9 |
| D7 MCF is for other people | B15, A23 |
| §3.1 Failure is first-class | A2, A3, A4, A1 |
| §3.2 Degrade, don't die | A5 |
| §3.3 The record | A6, B4, C1, C2 |
| §3.4 Measurements carry conditions | A6, A8, A9, A10, C4 |
| §3.5 Tests establish credibility | A19, B17, B18, B19, B20 |
| §3.6 Provenance never inferred | A7, B14, B5 |
| §3.7 The hub is untrusted | A15, B7, A16 |
| §3.8 The machine is the apparatus | B24, B3, B20 |
| §3.9 "Optimal" needs an objective | B6, A9 |
| §3.10 The user's data is theirs | A17, B8, B9 |
| §3.11 Nothing destroyed casually | B14, A16 |
| §3.12 Reproducibility over convenience | P3, B2 |
| §3.13 Lightness is a budget | B20, B15, B4, B23, P5 |
| §3.14 The interface is a window | B22, A22, C3 |
| §3.15 Ease means fewer decisions | B1, A16 |
| §3.16 Machine-enforced principles | B16, and the `Check` field on every rule |
| §3.17 The laboratory | B27, A13, B18, B19, B21 |
| §3.18 Capabilities are measured | A21, A7, B10 |
| §3.19 The benchmark resembles the work | B13, A10, B42 |
| §3.23 Measured only where capable | B40 |
| §3.24 Absence is not absence of possibility | B44 |
| §3.25 Reversibility bounds autonomy | A27, B48 |
| §3.20 Publication is irreversible | A24, A16 |
| §3.21 An imported configuration is a claim | B33 |
| §3.22 Instrumentation is scoped | B30, B31 |
| §4 Standing tensions | B26, A20, B15 |
| §5 Anti-goals (11) | B25, B4, B23 |
| §6.1 Honesty wins | P1, A1 |
| §6.2 Separable measurement path | B3 |
| §6.3 "Any" governs attempt | B7, A9 |
| §6.4 Untrusted execution never implicit | A15, B5 |
| §6.5 Declared objective | B6 |
| §6.6 "Build" means compose | B25 (training anti-goal), B6 |
| §6.7 Tests vs benchmarks | A18 |
| §6.8 System vs content | B9, A17 |
| §6.9 Lightness vs telemetry (resolved) | B4 |
| §6.10 Science outranks speed | P2 |
| §6.11 Thin client, not portable runtime | B22 |
| §6.12 Local by default | B8, A16 |
| §6.13 No undeclared state | B2, B3, B11 |
| §6.14 Gate by category | A16, B1 |
| §6.15 Reproduce, don't observe | B21, B18, B26 |
| §6.16 Simulated confidence | A11, A12, B27 |
| §6.17 Distributions, not scores | B12, B13, A8 |
| §6.18 Instrument, not platform | B23, B15 |
| §6.19 Detection is measurement | B10, A21 |
| §6.20 Sandbox by construction | A14 |
| §6.21 Parity, headless primary | A22 |
| §6.22 Reference model is a fixture | B28, B19 |
| §6.23 One model does not generalize | B29, A10 |
| §6.24 Lab telemetry is not ambient | B30, B4 |
| §6.25 Instrumentation vs validity | B31, A8 |
| §6.26 A range of labs is not a platform | B32, B15 |
| §6.27 Contribution vs privacy | A25, A24, B9 |
| §6.28 Contribute outward, decide inward | B34, A6 |
| §6.29 An identifier is a request to reproduce | B33, A15 |
| §6.30 Publishing vs contamination | B13, A10 |
| §6.31 Self-contained vs delegation | B36, B20 |
| §6.32 Privilege boundary | A26, A16 |
| §6.33 Exclusive labs vs the endpoint | B35 |
| §6.34 Heavy tiers vs a fast suite | B38, B19 |
| §6.35 Power vs the observer effect | B39, B30 |
| §6.36 Plural quality vs an answer | B41, B6 |
| §6.37 Customizable labs | B42, B32 |
| §6.38 Corpus narrows, local decides | B43, B34 |
| §6.39 The environment ladder | B48, A27 |
| §7 Voids — the process | A23 |
| §8 Amending | A23, C6, §"Amending this file" |
| Roadmap standing rules (8) | A2, A5, P1, P2, B15, A18, B18, A23 — now removed from the roadmap and cited from there |
| Backlog conventions | C5, C6, A23 |
| Mockup conventions | C7, C8, A7 |

---

## Changelog

### Version 10 — trials, identity, teardown

B56 makes the summary unpersistable: it can only be projected from trials, so
the question stays askable. B57 excludes hardware from the identity type by
construction, which is what stops the corpus fragmenting into one universe per
machine. B58 gives a laboratory freedom over its own tooling and binds it to
A27's restore obligation — a lab that leaves a warm cache behind silently
changes the next lab's first trial.

### Version 9 — pairing, and the two directions

B53 is a technique rule and the most immediately useful in this file: interleave
the arms of a comparison within one session and report the paired difference.
Thirty runs of A followed by thirty of B, subtracted, reports the afternoon's
drift as a difference between configurations — and on the machines MCF actually
runs on, that drift is not small.

B54 follows §3.27 into §XIV: what travels between machines is the comparison,
because a bare throughput number from a stranger is nearly uninterpretable while
a ratio survives the mess it came from.

B55 encodes §3.28's direction — precise inward, general outward — with the
guardrail that makes it safe: generalization happens in the rendering only, and
never in the record.

### Version 8 — boxes, and measured politeness

B51 makes a resource box a measurement condition that names its own limits. The
rule exists because the attractive error is treating a box as isolation: it
bounds what a process may take, not what it may be denied, and the dimensions it
cannot bound on consumer hardware — memory bandwidth, cache, accelerator
time-slicing, thermal — are the ones that determine tokens per second.

B52 turns §3.26's politeness from an intention into two measurements on
different sides of D10's split: an application test for what yielding costs the
user, and a laboratory for what it costs the model. "Runs at low priority" is
not evidence of anything.

### Version 7 — exclusivity narrowed to timings

B35 held that a laboratory owns the machine. It now holds that a *timing* does.
The distinction was already in §6.25 and had been generalized past its evidence:
a timing under contention measures the contention, while whether a tool call
parsed is unperturbed by the user opening a browser. The practical consequence
is favourable — the runs needing a quiet machine are the short ones.

B49 governs everything else: yield, run behind user traffic, record the
contention rather than prevent it. Its two caveats are what keep it honest and
are written as obligations rather than advice — token-budget deadlines, because
a task that failed on a busy machine is a measurement of the machine, and
environment failures classified apart from model failures, because an
out-of-memory from competition is not a model giving up.

B50 states the ordering §3.26 requires: hosting yields to the user, and nothing
yields to hosting.

### Version 6 — six rules for tiers, budgets and the environment

A27 states the boundary the widest reading of "improve" needed: MCF changes
nothing it cannot restore and restores everything it changes. Absolute, because
the alternative is a tool that makes permanent changes to produce temporary
numbers.

B43 and B44 give the corpus a job without breaking B34: it narrows the search,
local measurement decides, and a corpus negative says *unreported* rather than
*unsupported* and never hides an option. B45 makes D13's tier ordering a type
property, since an evaluation run against an uncalibrated configuration measures
the misconfiguration. B46 and B47 encode D14 — work declared in countable units,
rate supplied by the machine, estimates banded and scored, budgets producing
proposals rather than silent truncations, results anytime. B48 is the
environment ladder, whose bottom rung costs nothing and probably satisfies most
of the need.

### Version 5 — three rules for plural quality

B40 encodes §3.23's joining rule between capability discovery and the
laboratories: four outcomes, never three, with *not applicable* and *unknown*
carrying no score and refusing to be averaged. Written as a type rather than a
convention because the failure it prevents — an absent capability rendering as a
low number — looks like data and reads like a verdict.

B41 forbids the aggregate. There is no overall quality column, because "which
model is better" has no referent once quality is plural; MCF ranks within a
declared workflow and refuses to rank across them, with coverage travelling
alongside every answer.

B42 admits customized workloads and confines them: results marked
non-comparable at the point of production rather than at export, never
contributed, and a slot that accepts data rather than code so B32 stands.

### Version 4 — six rules for privilege, exclusivity, self-containment, time, tiers and energy

A26 is the rule that makes §XVII survivable: the daemon holds no ambient
privilege, elevation is a short enumerable list performed by a helper that
exits, and nothing untrusted ever runs privileged. Written as absolute because
the alternative — running the daemon as root because one reading needed it —
converts every other rule in this file into a formality.

B35 states what D8 decided: a lab owns the machine, may be greedy while running,
and costs nothing when it is not. B36 makes §XVI checkable by the only test that
matters — a from-scratch container with no toolchain runs the binary and reaches
a first token. B37 makes D9's time model a compiler property rather than a
convention, since `end_wall - start_wall` is the bug that silently reports an
NTP correction as latency. B38 tiers the suite and requires every tier report
its age, because an unstated staleness is A2's silent failure aimed at the
suite. B39 keeps energy honest: measured, estimated or unknown, sampled only
inside a lab.

### Version 3 — seven rules for the analysis and exchange intents

A16 gains a fifth gated category: publication. The four it had were all
recoverable — a re-fetch, a copy, a replacement — and §3.20 identifies
publication as the one act MCF cannot undo, which makes it the category most
deserving of a gate rather than the one that was missing by oversight.

Seven rules added for §XIII–§XV. A24 and A25 govern contribution: nothing leaves
except by an itemized per-share act, and the contributable record holds no user
content by construction rather than by filter. B30 and B31 govern lab
instrumentation, drawing the line B4 needed once §XIII asked for deep telemetry:
scoped to the experiment, watching the model rather than MCF, and reduced
whenever the question is a timing. B32 keeps a range of labs from becoming an
ecosystem of them. B33 and B34 govern the exchange: an imported configuration is
declared until verified here, and no foreign number decides a local
configuration.

B4 and B25 amended rather than replaced — B4 to name what its refusal is *about*
(ambient observation of MCF, not instrumentation inside a lab), B25 to carry the
two new anti-goals and the carve-outs the old ones acquired.

### Version 2 — standardized, and two rules for the reference model

B28 and B29 added, absorbing §XII and its resolutions §6.22 and §6.23: the
reference model is a fixture rather than a case in the code, and one model
builds an instrument without ever supporting a generality claim. Integrating an
entire new intent cost two rules, which is the right order of magnitude.

Restated to the format contract in [README.md](../README.md): front matter,
contents, present tense, changelog last. The check counts move out of the lead
and into their own section, where they are a metric rather than a preamble.

### Version 1 — the rules are consolidated

Rules were scattered across four documents: §3, §4, §5 and §6 of the intent
document, the roadmap's standing rules, the backlog's conventions, and the
mockup conventions. About 120 normative statements, many restating one another
in different words, none carrying a stated means of enforcement.

Refined to 58 rules in three defined tiers. Two things were added that the
sources implied but never stated: a precedence order for genuine conflicts,
taken from the orderings §6.1, §6.10, §3.12 and §4 already argue for; and a
check on every rule, because §3.16 holds that an unchecked rule is a wish.

Nothing was discarded — the coverage table maps every source clause to the rule
that absorbed it, so the refinement is auditable rather than trusted — and the
five statements deliberately not made rules are recorded rather than dropped
silently, chiefly the unfalsifiable ones that cannot be checked against anything
until DEC-016 and DEC-007 supply numbers.
