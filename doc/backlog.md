# Backlog

| | |
|---|---|
| **Type** | Register — every outstanding decision and build item |
| **Version** | 4 |
| **Status** | Living |
| **Authority** | Derived from [document-of-intent.md](document-of-intent.md) v8, governed by [rules.md](rules.md), sequenced by [roadmap.md](roadmap.md) |

**204 items: 42 decisions (39 open, 1 drafted, 1 narrowed, 1 resolved) and 162 build items.** Every item cites the clause that
justifies it; an item that cannot cite is a finding, not a task, and the
response is to record a void in §7 rather than invent intent here (A23).

## How to read an item

| Field | Meaning |
|---|---|
| **ID** | Stable for life (C5). `DEC-*` is a decision, `B-*` is build or verification work. |
| **Cites** | The clause of intent that justifies the work. |
| **Done when** | The falsifiable condition. §7.14 has no answer yet, so these are deliberately narrow local criteria. |
| **M** | The owning milestone in [roadmap.md](roadmap.md). |
| **Status** | `open` · `in progress` · `blocked (by ID)` · `done` · `dropped (reason)` |

Nothing is deleted (C6): items that die are marked `dropped` with the reasoning,
so they are not re-proposed later as oversights.

**Decisions are tracked here, stated in §7.** §8 of the intent document requires
that a void be resolved *in place* and its substance migrated into §3, §6 or
§2.1 — so §7 holds the question and its reasoning, and this file holds only its
status and its owner. Restating them here would have produced two versions of
each question, and the second reader would not know which is current.

---

## 0. Decisions

Ordered by how much downstream design each blocks, which is §7's own ordering.
A **bold** entry in *Blocks* is a hard gate: the named intent cannot be
implemented, only gestured at, until the decision is made.

| ID | Question | Void | Blocks | M | Status |
|---|---|---|---|---|---|
| DEC-016 | The performance budget numbers, and the oldest client "any device" commits to | §7.16 | **§VII** | M0 | open |
| DEC-010 | The failure classification scheme, designed once | §7.10 | **§3.1** | M0 | open |
| DEC-019 | Whether the adversarial prototype confirms or amends D4 | §7.19 | **D4** | M0 | open |
| DEC-004 | Engine ownership: perform inference, or delegate it | §7.4 | **§VI, §VII** | M0 | open |
| DEC-021 | What the laboratory must simulate, what it declines to, and whether simulated time is structural | §7.21 | **§VIII** | M0 | open |
| DEC-022 | Where the end-to-end boundary falls for a daemon | §7.22 | **§3.5** | M0 | open |
| DEC-008 | Which hardware is characterized versus attempted-and-uncharacterized | §7.8 | §IV | M0 | open |
| DEC-011 | How much works offline, and the difference between no internet and no local network | §7.11 | §3.2 | M1 | open |
| DEC-001 | API surface, the supervision contract on runtime death, simultaneous residency | §7.1 | §VI | M2 | open |
| DEC-009 | Arbitration outside a laboratory: disk exhaustion mid-download, several clients, two resident models | §7.9 | §3.8 | M2 | **narrowed** — D8 answers the lab/serving half: a lab owns the machine |
| DEC-018 | Whether a served model stays resident when nobody is looking | §7.18 | §VI | M2 | open |
| DEC-024 | Which capabilities are probed, when, at what cost, and what *inconclusive* licenses | §7.24 | **§X** | M3 | open |
| DEC-025 | Whether automatic configuration may change under a user | §7.25 | §X | M3 | open |
| DEC-017 | Proportionate protection for an exposed control plane | §7.17 | §V | M4 | open |
| DEC-012 | Behaviour when several clients attach at once | §7.12 | §XI | M4 | open |
| DEC-007 | What makes a result publishable: sample count, variance, warm-up, thermal state | §7.7 | **§II** | M5 | open |
| DEC-006 | Reproducible to what tolerance, across which changes | §7.6 | §3.12 | M5 | open |
| DEC-023 | The agentic suite: tasks, trial counts, difference tests, contamination, cost | §7.23 | **§IX** | M6 | open |
| DEC-003 | Whether non-agentic quality is measured, declined, or judged with biases declared | §7.3 | §IX | M6 | open |
| DEC-002 | The objective function, and the default opinion §VI requires MCF to ship | §7.2 | **§IV** | M7 | open |
| DEC-026 | How much breadth a generality claim requires, and along which axes | §7.26 | **§IV** | M7 | open |
| DEC-020 | How much real hardware validates the lab, how often, and what divergence is fatal | §7.20 | **§VIII** | M8 | open |
| DEC-013 | Whether measurements survive MCF's own upgrades | §7.13 | §3.4 | M8 | open |
| DEC-005 | How long the record is kept, who may purge it, and what happens when its budget is exhausted | §7.5 | §3.10 | M8 | open |
| DEC-014 | What state lets us say MCF works | §7.14 | §3.5 | M8 | open |
| DEC-033 | Whether the record keeps raw per-trial samples or only summaries — unrecoverable in one direction, and DEC-023 concedes the right statistic is not yet known | §7.33 | **§II, D6** | M0 | open |
| DEC-034 | What constitutes the identity of a measured configuration, and therefore what §XV's identifier serializes | §7.34 | **§IV, D6** | M0 | open |
| DEC-035 | Which host platforms MCF runs on, and the containment mechanism A14 requires there | §7.35 | **§I, A14** | M0 | open |
| DEC-039 | Which operations actually require elevation, on which platforms | §7.39 | §XVII | M0 | open |
| DEC-037 | Who writes to the record, and what happens to a write that loses | §7.37 | §3.1, D6 | M2 | open |
| DEC-038 | What happens when a pinned artifact decays upstream — withdrawn, gated, relicensed, repointed | §7.38 | §III, §3.6 | M1 | open |
| DEC-032 | Distribution and update policy; whether the container image and the local binary are one artifact or two | §7.32 | **D7** | M8 | open |
| DEC-036 | Whether model licences constrain publishing measurements about the model | §7.36 | §XIV | M9 | open |
| DEC-040 | What makes two machines alike — which attributes constitute similarity, whether it is one relation or several, and how a machine outside every class is treated | §7.40 | §XIV, D14 | M9 | open |
| DEC-042 | What yielding guarantees, and how it is achieved per platform — process and accelerator priority, memory reservation, foreground detection without ambient polling, automatic versus offered pausing | §7.42 | **§3.26, D8** | M6 | open |
| DEC-041 | The environment-control surface: which knobs, how scope is granted, what happens to a suspended process if MCF is killed, and how the ladder's maximum height differs by platform | §7.41 | §6.39 | M6 | open |
| DEC-029 | Which laboratories exist, in what order, and what a lab must state about its own validity | §7.29 | **§XIII** | M6 | **drafted** — [labs.md](labs.md) proposes 20 in 4 families, first three named; ordering and slot contents unratified |
| DEC-028 | What an identifier is: content-addressed or looked up, what it binds, whether it resolves offline | §7.28 | **§XV** | M9 | open |
| DEC-027 | What a contribution contains, and whether a machine can be de-identified without being made useless | §7.27 | **§XIV** | M9 | open |
| DEC-030 | Schema versioning across contributed databases, and what a reader does with one it cannot fully interpret | §7.30 | §XIV | M9 | open |
| DEC-031 | What contribution costs the contributor, and whether MCF may ever prompt for one | §7.31 | §XIV | M9 | open |
| DEC-015 | Whether MCF is meant to be usable by anyone but its author | §7.15 | §V | M8 | **resolved** — D7: yes. Documentation, installation and interface stability become goals |

---

## 1. Build items

Grouped by the milestone that owns them. Within a group, ordered by dependency
first and importance second.

### M0 — The instrument (foundations of belief)

| ID | Title | Cites | Done when | Status |
|---|---|---|---|---|
| B-001 | Rust workspace: crate split (`mcf-core`, `mcf-record`, `mcf-lab`, `mcf-hub`, `mcf-serve`, `mcf-bench`, `mcf-cli`), pinned toolchain, reproducible build | §7.19, §3.12 | `cargo build --locked` reproduces byte-identically from a clean checkout on a pinned toolchain | open |
| B-002 | Adversarial substrate prototype: probe an accelerator, supervise a child process made to die badly, record both, measure the result against the budgets | §7.19 | Both scenarios produce a well-typed record and a measured footprint; §7.19 is amended or confirmed in writing | blocked (DEC-016, DEC-010) |
| B-003 | Failure type: every fallible boundary returns a classified, attributed, context-carrying error; no `unwrap`, no `panic`, no discarded `Result` in non-test code | §3.1, §3.16 | CI denies `unwrap_used`, `expect_used`, `panic`, `todo`, `unimplemented`, `let _ =` on `Result` outside tests | blocked (DEC-010) |
| B-004 | Record store: append-only, structured-first, machine-readable, written at events and never on a timer | §3.3, §6.9 | A running idle daemon writes zero records and performs zero timer wakeups over 60 s | open |
| B-005 | `Measurement<T>` type that cannot be constructed without its conditions, sample count and spread — illegal states unrepresentable rather than validated against | §3.4, §3.16 | No code path can produce a measurement value without conditions attached; enforced by the type, not a check | open |
| B-006 | `Provenance` type that travels with an artifact by construction: repository, revision, checksum, license, retrieval time, and every transformation since | §3.6, §3.16 | An artifact handle cannot exist without provenance; unknown fields are the `Unknown` variant, never a plausible default | open |
| B-007 | Condition capture at measurement time: hardware state, thermal, driver and runtime versions, quantization, context length, batch shape, MCF version and configuration | §3.3, §3.4 | The §3.4 floor is captured from a live machine and round-trips through the record store losslessly | open |
| B-008 | Degradation marking: a result produced under reduced capability is typed as degraded and cannot be rendered without its mark | §3.2 | A CPU-derived result cannot be displayed or exported as though it were accelerator-derived | open |
| B-009 | Laboratory skeleton: deterministic harness, simulated clock, injectable faults, replayable scenarios; held to production code standards | §3.17, §VIII | A scenario reproduces a given failure identically across 100 runs and on a machine with no accelerator | blocked (DEC-021) |
| B-010 | Fault catalogue cross-check: the lab's catalogue and the failure taxonomy are the same list, and a taxonomy entry with no simulation fails the check | §3.17, §7.21 | An automated check fails CI when a taxonomy category has no producing scenario | blocked (DEC-010, DEC-021) |
| B-011 | Performance budget test suite: idle CPU, resident memory, cold start, added latency and installed footprint asserted against numbers, not eyeballed | §3.13, §3.5, §VII | Budgets are asserted in CI; a regression fails the build with a before/after under stated conditions | blocked (DEC-016) |
| B-012 | Overhead self-characterization: MCF measures and reports the cost of its own observation, because an uncharacterized instrument is not a scientific one | §6.2, §3.8 | The measured delta between instrumented and reduced-instrumentation paths is reported as part of a result's conditions | open |
| B-013 | Hardware profiler: accelerators, memory, thermal and power state, driver and runtime versions; unrecognized hardware degrades and is labelled, never guessed | §3.8, §7.8 | Profiles a machine with and without an accelerator; unknown vendors produce an `Unattributed` profile rather than an inference | blocked (DEC-008) |
| B-014 | `mcf doctor`: the M0 product — reports what the machine is, what MCF costs on it, and what it can and cannot promise here | §I, §3.8, §VII | Runs on a machine with no models, no network and no accelerator, and produces a complete, honest report | open |
| B-015 | Test seams for expensive paths: no test requires a GPU, a network or a large model | §3.5 | The full suite runs green on a laptop, offline, in under the time budget set by DEC-016 | open |
| B-016 | `rules.md`: the enforceable rules derived from the Document of Intent, each citing the principle it serves | §II, doc §"How to use it", §3.16 | Every rule cites; every rule is checkable by a machine or names the human check it replaces | **done** — [rules.md](rules.md): 58 rules in three tiers; 42 carry a machine check, 14 rest on review alone (tracked as the number to reduce, B16), 2 await a decision |
| B-041 | Documentation conformance check: front matter, changelog, present tense outside changelogs, no dangling `B-*`/`DEC-*`/`§` citation, no broken relative link | [README.md](../README.md) format contract, C5, B16 | A single command fails when any document in `doc/` violates the contract; run in CI beside the code checks | open |
| B-018 | Reference-model neutrality: no code path behaves differently because an artifact is the reference model, and the suite never depends on it | §6.22, §XII, §3.5 | Substituting a different model changes what is measured and nothing about how MCF behaves; a CI check fails if the reference model is named outside fixtures and documentation | open |
| B-184 | Duration and timestamp are distinct types with no arithmetic between them; the lab clock is simulated and travels with the result | B37, D9 | `end_wall - start_wall` does not compile; a clock-jump scenario invalidates rather than corrupts | open |
| B-191 | Test tiers: unit, property, functional, whole-system, fault-injection, load, soak, fuzz, performance, mutation — with the fast hermetic tier gating every change | D10, §6.34, §3.5 | Each tier runs; the gating tier stays offline and fast on a laptop | open |
| B-185 | Every tier publishes its age; a stale heavy tier fails a release rather than being assumed green | B38, §3.1 | A release with a stale mutation or soak tier is refused with the age stated | open |
| B-186 | Mutation score is measured and floored, budgeted like any other property | B38, B20, §3.5 | The score is asserted in CI and may not regress silently | open |
| B-192 | Self-contained build: the inference engine and every common-path tool are vendored or reimplemented, statically linked, no runtime and no toolchain required | §XVI, B36 | The artifact has no dynamic dependency a stock machine lacks | open |
| B-183 | From-scratch conformance: a container with no toolchain, no runtime and no package manager runs the binary and reaches a first token | B36, §XVI | Asserted in CI on every platform in DEC-035's scope | blocked (DEC-035) |
| B-190 | Privileged helper: a separate, auditable executable performing one named operation from a short list and exiting; the daemon holds no ambient privilege | A26, §6.32, §XVII | The daemon runs unprivileged in every scenario; the helper's surface is enumerated | blocked (DEC-039) |
| B-180 | Untrusted code cannot reach an elevated path, asserted by scenario rather than by policy | A26, §6.20, §6.4 | An adversarial model and hostile repository code both fail to touch a privileged operation | open |
| B-220 | Environment restoration: a scenario kills MCF mid-run at every stage and asserts governors, priorities, exclusive modes and suspended processes are all restored | A27, §3.25, §6.39 | The machine is returned to how it was found from every interruption point | open |
| B-221 | Corpus-sourced values and locally-measured values are distinct types; only the second can back a recommendation | B43, B34, §6.38 | A foreign number cannot reach a recommendation, enforced by the compiler | open |
| B-042 | Record store is a single SQLite database, schema-versioned from the first write, corruption-resistant and recoverable | D6, §3.3, §3.1 | The schema carries a version; a truncated write is a classified failure and the database reopens; the file is portable between machines | open |
| B-161 | Content store and record store are distinct types with no path between them, so no export can carry content that was never written | A25, §6.8, §6.27 | The type system prevents writing prompt or completion content to the record store | open |
| B-017 | Decision record (ADR) format and index, so §7 resolutions and their reasoning survive the code that implements them | §8 | A resolved void points at an ADR and the ADR points back at §7 | open |

### M1 — Acquire

| ID | Title | Cites | Done when | Status |
|---|---|---|---|---|
| B-020 | Hub reference resolution: accept any Hugging Face reference without special-casing, and reach a defined, actionable outcome for every one | §III, §6.3 | No input produces a hang, an unclassified crash or corrupted local state — asserted by the lab's hostile-hub scenarios | open |
| B-021 | Resumable, integrity-checked fetch: checksums verified, partial transfers resumed, mutation-under-us detected | §III, §3.7 | A transfer interrupted at 90% resumes and verifies; a file that changed mid-fetch is a classified failure, not a corrupt local artifact | open |
| B-022 | Untrusted-input validation of every fetched byte: malformed configs, deceptive metadata, enormous files, path traversal in archives | §3.7 | The lab's hostile-hub fixtures are all rejected with a classified outcome and no state damage | open |
| B-023 | License legibility: the license is surfaced before use, and a use it forbids is stated rather than discovered | §III, §3.7 | Every acquired artifact reports its license, or reports it as `Unknown` — never as a plausible default | open |
| B-024 | Gated and authenticated repositories: credentials are the user's, held deliberately, never a silent prerequisite | §III, §3.10 | A gated model produces an actionable outcome naming exactly what is missing | open |
| B-025 | Repository-code execution is possible but never implicit: per artifact, with the risk stated, the choice recorded in provenance, and contained so hostile code cannot corrupt MCF's records or state | §6.4, §3.7 | The lab runs deliberately hostile repository code and MCF's records and state are provably intact afterwards | open |
| B-026 | Disk arbitration on acquisition: a download that would exhaust the disk is a decision, not a surprise | §3.11, §7.9 | The disk-exhaustion scenario ends with a classified refusal and no partial garbage | blocked (DEC-009) |
| B-027 | Eviction and deletion: previewed, logged, reversible where reasonable, never automatic to reclaim space | §3.11 | No code path deletes an artifact without an explicit, recorded authorization | open |
| B-028 | Fake hub: a complete, deterministic simulated Hugging Face — well-formed, malformed, gated, hostile, truncated, mutating | §3.17, §7.21 | Every M1 test runs against it with no network | blocked (DEC-021) |
| B-019 | Acquire and pin the reference model as M1's first real artifact — the third-party requantization chain (`unsloth/Qwen3.8-27B-GGUF` → `Qwen/Qwen3.8-27B`) is the hard provenance case, not the easy one | §XII, §3.6 | The derivative traces to its source weights through the publisher's pipeline, with every field either recorded or `Unknown`; the revision is pinned at acquisition | open |
| B-213 | Pre-acquisition fitment across every variant a repository publishes: weights plus KV cache at the requested context against available memory, computed from metadata before a byte is fetched | [P3](proposals.md#p3--pre-acquisition-planning), §III, §6.3 | Twenty quantizations are classified fits / fits-without-context-headroom / does-not-fit without downloading any of them; the plan is re-checked against reality on acquisition and divergence is a finding | open |
| B-029 | `mcf pull` / `mcf list` / `mcf rm`: the M1 product — models enter, live on and leave this machine with provenance intact | §III | A model is acquired, listed with full provenance, and removed deliberately, offline against the fake hub and online against the real one | open |

### M2 — Serve

| ID | Title | Cites | Done when | Status |
|---|---|---|---|---|
| B-030 | Daemon: long-lived, restartable, recovers its state across restarts, survives indefinitely | §7.1, §I | The lab kills the daemon at every lifecycle stage and it recovers to a coherent, queryable state each time | open |
| B-031 | Idle discipline: no polling loops, no background timers, no always-on watchers; idle cost indistinguishable from zero | §3.13, §6.9 | Measured idle CPU and wakeups meet DEC-016's budget, asserted in CI | blocked (DEC-016) |
| B-032 | Engine adapter layer: inference engines are supervised subprocesses, and which engine is in use is a recorded condition | §7.4, §6.2 | At least one engine is driven end to end; swapping engines changes a recorded condition, not a code path | blocked (DEC-004) |
| B-033 | Supervision contract: a runtime that dies mid-token is a classified, attributed failure that does not take the manager down | §3.1, §7.1 | The lab kills a runtime at every stage — pre-load, mid-load, mid-token, post-token — and the daemon stays coherent | blocked (DEC-001) |
| B-034 | Serving API: models addressed by name, stable surface, first token without the user knowing about runtimes, formats or flags | §VI, §3.15 | A first token is obtained from a named model in one command, on a machine that has never served before | blocked (DEC-001) |
| B-035 | Added-latency budget: the overhead MCF interposes between an inbound request and the engine's first token is measured and asserted | §VII, §3.13 | The interposed latency is measured under stated conditions and defended in CI | blocked (DEC-016) |
| B-036 | Local-only by default: the control plane binds locally; network exposure is an explicit, informed, revocable act, never a side effect | §6.12, §3.10 | Default configuration is unreachable from another host; exposure requires an explicit authorization that is recorded | open |
| B-037 | Model residency policy: what stays loaded when nobody is looking, recorded as a measurement condition | §7.18, §3.4 | Residency state is part of every serving latency result | blocked (DEC-018) |
| B-038 | Visible defaults: quantization, context length, runtime and placement are chosen without prompting, and every choice is attributed, explained on demand and overridable | §3.15, §6.14 | `mcf explain <model>` returns the actual reasoning and the measurements behind each default | open |
| B-039 | Authorization gates by category, not frequency: untrusted execution, large irrecoverable resource use, network exposure and destruction are asked every time; everything else flows | §6.14 | The four gated categories are enumerable in code and each has a test asserting it prompts | open |
| B-210 | `mcf stop`: refuse new work, interrupt a lab preserving its partial result, drain and terminate runtimes on a stated deadline, release every held resource including privileged state, record what was stopped, and report what could not be released | [P6](proposals.md#p6--the-stop-control), §3.1, A26, A22 | A held accelerator, locked pages and a changed governor are all released; anything that could not be is named rather than claimed | open |
| B-040 | `mcf serve` / `mcf run`: the M2 product — having a model and using a model are one command apart | §VI | A cold machine reaches a first token in one command, and the daemon survives a deliberately hostile lab session unattended | open |

### M3 — Configure by measurement

| ID | Title | Cites | Done when | Status |
|---|---|---|---|---|
| B-050 | Three-state capability model: *declared*, *verified*, *unknown* — never confused, never defaulted | §3.18, §3.6 | The type system prevents a declared capability being read as a verified one | open |
| B-051 | Probe framework: capability probes are bounded experiments carrying a method, a result, conditions and a record | §3.18, §3.4 | A probe's output is a `Measurement`, not a boolean | blocked (DEC-024) |
| B-052 | Probe: chat template correctness | §X, §3.18 | A model with a wrong or missing template is detected by observation, not by reading a config field | blocked (DEC-024) |
| B-053 | Probe: tool-calling format and reliability | §X, §IX | A model that emits a well-formed tool call is distinguished from one whose metadata merely claims support | blocked (DEC-024) |
| B-054 | Probe: structured output conformance | §X | Verified by parsing what the model actually emits over repeated trials | blocked (DEC-024) |
| B-055 | Probe: context length usable versus claimed | §X, §3.18 | Divergence between claimed and usable is reported as a finding | blocked (DEC-024) |
| B-056 | Probe: stop-condition behaviour | §X | A model that will not stop is a recorded characteristic, not a hung request | blocked (DEC-024) |
| B-057 | Probes: vision, embeddings, reasoning modes, multilingual — scoped and prioritized by DEC-024 rather than assumed | §X, §7.24 | Each in-scope modality has a probe; each out-of-scope one is recorded as declined | blocked (DEC-024) |
| B-058 | Divergence reporting: declared-versus-verified disagreement is surfaced as a first-class finding, often the most useful thing MCF can say about a model | §3.18 | Divergences are listed per model and exportable | open |
| B-059 | Derived configuration carries the provenance of the capability that set it: which probe, when, under what conditions | §3.18, §6.19 | Every auto-set parameter answers "why this value" with a probe reference or a declared default | open |
| B-060 | Inconclusive handling: a probe that neither confirms nor denies leaves the capability unknown and says so | §3.18, §7.24 | No inconclusive probe result is ever coerced to a working default | blocked (DEC-024) |
| B-061 | Reconfiguration policy enforcement: whatever DEC-025 decides, comparability across a configuration change is preserved or explicitly invalidated | §7.25, §3.4 | A configuration change either preserves comparability or marks prior results non-comparable | blocked (DEC-025) |
| B-062 | `mcf probe` / `mcf explain`: the M3 product — a model runs the way it is designed to run, and can prove it | §X | A model whose defaults were previously wrong measurably improves, and the improvement is attributable to a named probe | open |

### M4 — The window

| ID | Title | Cites | Done when | Status |
|---|---|---|---|---|
| B-070 | Thin client: no framework, no bundled runtime, no build step, no per-platform application; the device already has what is needed to display it | §V, §6.11 | Total transferred weight and cold render time meet DEC-016's client budget on the oldest committed client | blocked (DEC-016) |
| B-071 | Zero idle cost when nobody is looking: no polling to appear responsive | §6.11, §3.13 | With a browser tab open and idle, daemon CPU is indistinguishable from closed | open |
| B-072 | Parity enforcement: the interface is a client of the same API a script uses, and introduces no action reachable only there | §XI, §6.21 | An automated check fails when an interface action has no headless equivalent | open |
| B-073 | Conditions travel to the surface: no view renders a measurement without its conditions, sample count and spread | §3.4, §3.14 | Rendering a bare number is impossible by construction, not by review | open |
| B-074 | Failure legibility: classified failures, their context and their configuration are inspectable from the window | §3.1, §3.2 | Every taxonomy category has a rendering that names the subsystem and the reconstruction context | blocked (DEC-010) |
| B-075 | Exposure flow: turning on reachability from another device is deliberate, informed, revocable and recorded | §6.12, §3.10 | Exposure cannot be enabled as a side effect of any other action | blocked (DEC-017) |
| B-076 | Multi-client behaviour: several attached clients is a defined condition, not an emergent one | §7.12, §7.9 | Concurrent clients are exercised by the lab | blocked (DEC-012) |
| B-077 | The window: the M4 product — the system's state is legible and actionable from the machine itself and from a handheld device on the same network | §V, §XI | The whole of M0–M3 is operable from the window with nothing installed on the client | open |

### M5 — Measure

| ID | Title | Cites | Done when | Status |
|---|---|---|---|---|
| B-080 | Benchmark runner distinct from the test suite: no pass condition, never a gate on correctness | §6.7 | Benchmarks cannot fail CI; correctness tests cannot produce measurements | open |
| B-081 | Non-adaptive measurement path: caching, reuse and adaptation are disabled while measuring, and warm-versus-cold is a recorded condition | §6.13, §6.2 | A measurement taken warm is distinguishable in the record from one taken cold | open |
| B-082 | Real-hardware only for performance numbers: no performance figure may originate in simulation | §6.16 | The type system or the record prevents a simulated timing being published | open |
| B-083 | Repeated trials with reported spread and sample count; single-shot timings are refused as anecdotes | §3.4 | No result is publishable with n=1 | blocked (DEC-007) |
| B-084 | Warm-up and thermal steady state per DEC-007's acceptance criteria | §3.4, §7.7 | Runs that fail the criteria are marked invalid, not silently included | blocked (DEC-007) |
| B-085 | Isolation check: a comparison in which more than one variable differed reports "these are not comparable" rather than a delta | §3.4 | An intentionally confounded comparison is refused by the tooling | open |
| B-086 | Null and negative results are stored and surfaced as results — "does not fit here", "no measurable speedup" | §3.4 | Both appear in the record and in the window as outcomes, not failures | open |
| B-087 | Partial success representation: nine of ten tasks completing is nine data points | §3.1 | Partial runs are queryable as partial, with their per-unit outcomes intact | open |
| B-088 | Contention governance: MCF knows the difference between a slow model and a busy machine, and says so when it cannot tell | §3.8, §7.9 | A deliberately contended run is marked unattributable rather than reported | blocked (DEC-009) |
| B-089 | Environment pinning to the tolerance DEC-006 sets | §3.12, §7.6 | Every result carries enough environment to be reproduced to the stated tolerance | blocked (DEC-006) |
| B-091 | Quantization frontier on the reference model: one model, one machine, the full GGUF quantization range — the cleanest available §3.4 comparison, a single variable across many points | §XII, §3.4, §IV | A frontier is produced across quantizations with one variable differing, and results state they characterize the instrument, not models in general | open |
| B-211 | Repro bundle: one file carrying a claim, its method, its full §3.4 conditions, its raw samples, the artifact's provenance chain, the §XV identifier, and a verification manifest | [P2](proposals.md#p2--the-repro-bundle), §II, A6 | A bundle is emitted for any published measurement and contains everything needed to re-run it | blocked (DEC-033) |
| B-212 | `mcf verify <bundle>`: reproduce the configuration, re-run the method, report agreement or divergence with conditions compared side by side | [P2](proposals.md#p2--the-repro-bundle), §II, A8 | A bundle from another machine either agrees, or names which conditions differ and refuses to attribute the gap | blocked (DEC-033) |
| B-214 | Expectation bands from local history: project throughput for unmeasured configurations from what this machine has measured, as a labelled estimate that can never sit beside a measurement unlabelled | [P3](proposals.md#p3--pre-acquisition-planning), A20, B34 | Projections are band-shaped, marked as estimates, derived from local history only, and absent where there is no history | open |
| B-215 | Every projection is scored against the measurement that eventually replaces it, and the score is reported | [P3](proposals.md#p3--pre-acquisition-planning), §6.16, §3.4 | Prediction error is tracked over time; a projection model whose error grows is a finding about the model | open |
| B-216 | Contention snapshot: on demand and on invalidation, sample per-process accelerator occupancy, memory pressure, thermal and clock state against baseline, and attach it to the invalidation record | [P5](proposals.md#p5--contention-diagnosis), §3.8, B24 | An unattributable run names what it was competing with; the snapshot persists with the record rather than on a screen | open |
| B-090 | `mcf bench`: the M5 product — a defensible performance number taken here, with its conditions and its uncertainty | §II, §IV | Two configurations of one model are compared on this machine with a stated method, spread and conclusion — including "within noise" | open |

### M6 — Judge

| ID | Title | Cites | Done when | Status |
|---|---|---|---|---|
| B-100 | Agentic harness built on the M0 laboratory rather than beside it — one apparatus, per §6.17 | §6.17, §6.18 | The harness shares the lab's determinism, fault injection and simulated clock; no second environment exists | open |
| B-101 | Sandbox by construction: benchmark tools cannot reach the filesystem, the network, MCF's records or the serving path, because those capabilities are absent from the environment | §6.20 | A deliberately hostile model under test cannot touch anything real; asserted by the lab, not by policy | open |
| B-102 | Task definitions: multi-turn, tool-calling, instruction-bound, format-constrained, with checkable outcomes | §IX, §3.19 | Every task grades by verification, never by another model's judgment | blocked (DEC-023) |
| B-103 | Everything except the model held still: task, tool implementations, starting state, seeds, harness version, sampling parameters — all recorded | §6.17 | A run is reconstructible from its record alone | open |
| B-104 | Results are distributions, never scores: success rate over n trials with its spread and shape | §6.17, §3.4 | No surface can render an agentic result as a single number | blocked (DEC-023) |
| B-105 | Model failure taxonomy: wrong tool, malformed call, loop, early stop, gave up — more informative than the pass rate | §6.17, §7.10 | Every failed trial is classified, and the classification is the primary output | blocked (DEC-010, DEC-023) |
| B-106 | Statistical test for "is this a real difference": §3.9's "within noise, pick either" said honestly | §3.9, §6.17 | Two indistinguishable models produce a refusal to rank, not a ranking | blocked (DEC-023) |
| B-107 | Contamination strategy: private, rotated or procedurally generated tasks, per DEC-023 | §7.3, §3.4 | The strategy is stated, implemented and re-checkable as the suite ages | blocked (DEC-023) |
| B-108 | Zero cost when idle: the benchmark subsystem consumes nothing during ordinary serving | §6.18, §3.13 | Measured serving footprint is identical with the harness compiled in and no benchmark running | open |
| B-109 | Harness minimality guard: each addition must make the measurement more valid, not the harness more capable | §6.18, §5 | Additions cite validity; capability-only additions are refused and the refusal is recorded | open |
| B-181 | Exclusive window for timing-class work: announced, bounded by a declared maximum, schedulable, interruptible, closed automatically; drains the serving path by explicit decision | B35, D8, §6.33 | A timing result cannot be constructed from a run that overlapped serving or another lab | open |
| B-230 | Behaviour-class labs cannot express a wall-clock deadline; deadlines are token budgets | B49, D8, §3.8 | A wall-clock timeout in a behaviour lab does not compile | open |
| B-231 | A behaviour-class run under injected contention produces the same outcomes as one without, differing only in recorded conditions | B49, §6.40, §3.4 | The lab scenario asserts outcome equivalence and condition divergence | open |
| B-232 | Serving latency under a concurrent behaviour-class run stays within its budget; hosting yields to the user and nothing yields to hosting | B50, §3.26, §VI | Asserted in CI against the interposed-latency budget | blocked (DEC-016) |
| B-233 | Environment failures are a distinct taxonomy branch from model failures: an out-of-memory from competition is a condition of the run, never the model giving up | B49, §7.10, §3.1 | Every yielding run's failures classify to one branch or the other, never ambiguously | blocked (DEC-010) |
| B-234 | Yielding mechanism: low priority, foreground-aware, pausable and resumable, per platform | B49, §3.26, §7.42 | A background run does not stutter an interactive application, measured rather than asserted | blocked (DEC-042) |
| B-235 | Scheduling: an exclusive window may be deferred to a stated time or to a stated period of machine idleness | B35, §6.40, §3.26 | A user can say "overnight" or "after ten minutes idle" and the window opens then | open |
| B-182 | Suspension is declared: a request during a lab receives an immediate refusal naming the lab and the expected remaining time, never a queue or a timeout; a lab is bounded and interruptible with its partial result preserved | B35, §6.33, §3.1 | Requests during a lab are refused within one round trip; interrupting preserves and marks the partial result | open |
| B-187 | Idle MCF reads no power or thermal counters | B39, D5, §3.13 | Counter reads are zero outside a lab run | open |
| B-188 | Every energy value carries its provenance — measured, estimated or unknown — and its sampling rate as a condition | B39, D11, A20, A7 | A modelled figure cannot render as a reading; a platform with no interface yields `unknown` | open |
| B-189 | Energy laboratory: energy per token, sustained power draw and thermal behaviour under load, with fidelity stated per platform | D11, §3.9, §XIII | The lab reports joules per token with its measurement provenance, or states that this platform cannot supply it | blocked (DEC-029) |
| B-223 | An evaluation run cannot be constructed from an uncalibrated configuration handle | B45, D13, §X | The tier ordering is a type property, not a convention | open |
| B-224 | A laboratory declares its work in countable units — trials, sweep points, tokens, documents — never in minutes | B46, D14 | No lab declares a duration; duration is derived from work × the machine's measured rate | open |
| B-225 | Duration estimates are banded, marked as estimates, scored against actuals, and their error is tracked and reported | B46, D14, A20 | A lab whose estimates are persistently wrong surfaces as a finding | open |
| B-226 | A time budget produces a proposal naming what will run and what is excluded and why; never a silent truncation | B47, D14, §3.1 | "Ran 6 of 20" is always accompanied by the fourteen | open |
| B-227 | Anytime results: every lab reports as it goes; a run stopped early keeps what it produced, marked incomplete | B47, A4, §3.1 | A multi-day lab interrupted at hour three yields three hours of marked data | open |
| B-228 | Environment ladder: report, wait for quiet, suspend-and-restore only with per-run approval of a named list, never terminate | B48, §6.39, A27 | A scenario asserts nothing outside the approved list is touched and every suspension resumes, including when MCF is killed | open |
| B-222 | Every corpus statement renders its sample count; no filter removes a candidate from a listing | B44, §3.24 | An unreported option is ranked lower and annotated, never hidden | open |
| B-111 | Lab framework: a lab is named, versioned, reproducible, declares its class (timing or behaviour), declares its capability gate and its workload slot, and states what it does and does not establish | §XIII, §3.17, §6.26, B40, B42 | A lab that cannot state its class, gate, slot or validity boundary fails to register | blocked (DEC-029) |
| B-200 | Lab results are a sum type: *measured*, *not applicable*, *unknown*, *failed* — the middle two carry no score and cannot be averaged | B40, §3.23 | An absent capability cannot render as a low number anywhere | open |
| B-201 | No type combines results from two laboratories into a scalar | B41, D2, §3.9 | An overall quality score is unrepresentable | open |
| B-203 | A custom-workload result cannot be constructed into a contribution, and is marked non-comparable at production | B42, §6.37, A25 | The marking exists before export, not at it | open |
| B-204 | Each lab declares what its workload slot accepts and refuses what it cannot grade | B42, [P1](proposals.md#p1--customizable-workloads) | An ungradable workload is refused at load, never run | open |
| B-205 | Workload slot format, loader and validator: a documented data format per slot kind — labels, documents, schemas, constraints, tasks, test suites — with authoring documentation aimed at someone who has never read the intent document | [P1](proposals.md#p1--customizable-workloads), B42, D7 | A user authors a workload for at least three labs from the documentation alone and gets a marked, non-comparable, local result | open |
| B-217 | Quiet-machine pre-flight: a laboratory refuses to begin on a contended machine rather than producing an invalid result, using B-216's snapshot | [P5](proposals.md#p5--contention-diagnosis), D8, B35, §3.8 | A lab started while another process holds the accelerator refuses with the contender named, and does not run | open |
| B-162 | Idle cost is invariant to the number of labs compiled in | B30, §3.22, §3.13 | Measured idle CPU, memory and wakeups are identical with three labs and thirty | open |
| B-163 | Every lab result carries the instrumentation profile it ran under | B30, §3.4, §6.25 | A result without its profile cannot be constructed | open |
| B-164 | Timing-class results cannot originate in a deep-instrumentation run; residual overhead is characterized | B31, §6.25, §6.2 | The type system refuses the construction; the overhead is reported as a condition | open |
| B-165 | Lab admission: each lab answers what claim it enables, and refusals are recorded rather than forgotten | B32, §6.26 | Every registered lab documents its admitting answer; the refusal list is maintained | open |
| B-110 | `mcf eval`: the M6 product — whether a model can do the work, measured here, as a distribution | §IX | Two models are evaluated on the suite unattended, and the output distinguishes them or honestly declines to | open |

### M7 — Recommend

| ID | Title | Cites | Done when | Status |
|---|---|---|---|---|
| B-120 | Objective expression: the user declares a weighting; MCF refuses to invent one | §6.5, §3.9 | An unstated objective invokes an explicit, visible, overridable default — never a hidden one | blocked (DEC-002) |
| B-121 | Pareto frontier mapping across quality, latency, throughput, memory, power and disk | §3.9, §IV | The frontier is computed from real measurements and is inspectable | blocked (DEC-002) |
| B-122 | Interrogable recommendation: reasoning, the measurements behind it, and the runners-up | §3.9 | Every recommendation answers "why not the other one" with data | open |
| B-123 | Refusal to manufacture a distinction: "the differences here are within noise, pick either" is a supported output | §3.9 | The recommender returns it when the statistics warrant | blocked (B-106) |
| B-124 | Construct the configuration, not merely name it: compose base model, quantization, format, context size, runtime and sampling parameters, then validate the result | §6.6, §IV | A recommended configuration is materialized and re-measured to confirm it performs as predicted | open |
| B-125 | Anti-overfitting guard: MCF must not tune toward whatever it measures | §3.4 | The selection suite and the validation suite are structurally separate | blocked (DEC-023) |
| B-202 | Every recommendation renders its coverage: which laboratories informed it, which the candidates were inapplicable to | B41, §6.36, §3.23 | A recommendation resting on two of eleven laboratories says so | open |
| B-127 | The recommender refuses a field of one: a frontier with a single point is not a frontier, and a single-model recommendation is a claim MCF has no basis for | §6.23, §3.9 | A single-candidate field produces a refusal with its reasoning, never a ranking | open |
| B-128 | Expand the reference set to the breadth DEC-026 requires before any §IV recommendation is published | §7.26, §6.23 | No generality claim is made until the set exists; results before that say so on every surface | blocked (DEC-026) |
| B-167 | Recommender inputs are locally-originated measurements, enforced at the type level; no foreign number reaches a recommendation | B34, §6.28, §5 | A contributed or imported measurement cannot be an input to a recommendation | open |
| B-126 | `mcf recommend`: the M7 product — the closed loop, measure→compare→select→re-measure | §1, §IV | A recommendation is produced, applied, re-measured, and the prediction is checked against the outcome | open |

### M8 — Endurance

| ID | Title | Cites | Done when | Status |
|---|---|---|---|---|
| B-140 | Real-hardware validation of the lab, at the cadence and coverage DEC-020 sets; divergence is a recorded finding about the simulator | §6.16, §VIII | Simulation predictions are checked against real hardware and the fidelity gap is quantified | blocked (DEC-020) |
| B-141 | Stated fidelity boundary: the lab declares what it does not model, so confidence is claimed only where earned | §6.16, §7.21 | The boundary is published alongside the suite's results | blocked (DEC-021) |
| B-142 | Failure-record sufficiency check: a failure record is sufficient when the lab can rebuild the failure from it; insufficiency is a defect in the record | §6.15, §3.17 | A sample of real failures is reconstructed in the lab from records alone, and the success rate is tracked | open |
| B-143 | Bug-to-fixture discipline: an escaped bug becomes a permanent lab scenario before it becomes a fix | §3.5, §3.17 | Enforced in review and checkable from the commit record | open |
| B-144 | State migration across MCF versions, with comparability of historical measurements handled per DEC-013 | §7.13 | An upgrade either preserves comparability or invalidates the affected history explicitly | blocked (DEC-013) |
| B-145 | Record retention, inspection and purge per DEC-005, including behaviour when the record's disk budget is exhausted | §7.5, §3.10 | The user can see and purge what MCF holds; budget exhaustion is a classified, loud outcome | blocked (DEC-005) |
| B-146 | Content-versus-system separation enforced structurally: suite fixture data and user traffic are different categories with different retention, not the same store with a flag | §6.8 | The separation is structural and cannot be defeated by configuration | open |
| B-147 | Offline operation: as much as possible works with no network, loudly labelled; "no internet" and "no local network" are distinct conditions | §7.11, §3.2 | The offline scenario runs the whole of M1–M7 to the extent possible and labels every degradation | blocked (DEC-011) |
| B-148 | Long-run endurance scenario: days of simulated operation with faults, restarts, thermal excursions and upgrades | §I, §3.17 | MCF remains coherent, queryable and restartable throughout, with no unclassified outcome | open |
| B-149 | Answer §7.14 with evidence: the state that lets us say MCF works | §7.14 | The definition is written, and the suite is measured against it | blocked (DEC-014) |

### M9 — The exchange

| ID | Title | Cites | Done when | Status |
|---|---|---|---|---|
| B-160 | Share flow: per-share, opt-in, renders the rows that leave rather than a description of them, and states that publication cannot be undone | A24, §3.20, §6.27 | No egress path exists that is not user-initiated per share; the confirmation shows the payload | open |
| B-168 | De-identification: fields coarsened, withheld or sent per DEC-027, with MCF stating plainly what a contribution does and does not protect | §7.27, §6.27, §3.10 | A contribution's identifying content is enumerated and the honest claim about anonymity is displayed at the moment of sharing | blocked (DEC-027) |
| B-169 | Identifier: emit one for a configuration MCF holds, and resolve one it is given | §XV, §7.28 | Round-trip on this machine: emit, wipe, resolve, and reproduce the identical configuration | blocked (DEC-028) |
| B-166 | An imported configuration reads as `declared` until a local probe or benchmark verifies it; numbers that travelled with it are attributed elsewhere | B33, §3.21, §6.29 | No imported figure renders as though MCF measured it; verification promotes it and records the divergence | open |
| B-172 | Failure to reproduce an identifier is a first-class outcome, and divergence between imported and local numbers is a recorded finding about how far results travel | §6.29, §6.3, §3.4 | "This identifier needs 48 GiB and you have 24" is a complete answer; a numeric divergence is stored as evidence, not an error | open |
| B-170 | Contribution schema versioning: a contribution declares the schema and MCF version that wrote it, and a reader that cannot fully interpret one says so | §7.30, §3.1, §3.4 | An older contribution is read, marked, or refused — never silently misinterpreted | blocked (DEC-030) |
| B-171 | Contribution carries outcomes, never artifacts: scores, classifications, conditions and distributions leave; tasks, tools, fixtures and model outputs do not | §6.30, §3.19 | An audit of a contribution finds no task content; contamination exposure is recorded per task | open |
| B-173 | `mcf share` / `mcf import`: the M9 product — evidence leaves deliberately, and a configuration found elsewhere reproduces here or explains why not | §XIV, §XV | A contribution is produced, inspected and sent; an identifier from that contribution reproduces the configuration on a second machine | open |

---

## 2. Dropped and superseded

Recorded rather than deleted, per §8.

| ID | Title | Reason |
|---|---|---|
| — | Ambient telemetry subsystem: continuous sampling, always-on tracing, metric streams, dashboards, trace backends | Never opened as work. §6.9 resolved against it by decision in Revision 4, and §5 makes "not an observability platform" an anti-goal. Recorded here so it is not re-proposed as an oversight. The confidence it would have bought is supplied by §VIII (M0's laboratory) and its cost is accepted in §6.15. |
| — | Fine-tuning, distillation, merging and pruning | Out of scope at this revision per §6.6 and §5. §6.6 flags itself as the resolution most likely to be revisited; if it is, it gives way by amendment to §6.6, not by an item appearing here. |
| — | Cross-machine leaderboard and result publication | §5 anti-goal. MCF measures *this* machine and is actively suspicious of numbers that did not originate locally. |
| — | Plugin ecosystem, extension API, general configurability | §5 anti-goal. Every generalization is weight (§3.13). |
| — | Chat product surface beyond an instrument for exercising a model and capturing evidence | §5 anti-goal. |

---

## Changelog

### Version 8 — exclusivity narrowed to timings

B-181 rewritten: the exclusive window belongs to timing-class work, not to
laboratories generally. New items for behaviour-class yielding — token-budget
deadlines as a compiler property, outcome-equivalence under injected contention,
serving latency held under concurrent lab load, and the taxonomy branch that
keeps an environment failure from being recorded as a model failure.

DEC-042 registered for §7.42, and marked a hard gate: yielding that still
stutters an interactive application is a broken promise, and §3.26 makes that a
correctness question rather than a comfort one.

### Version 7 — tiers, budgets, and the environment ladder

DEC-040 and DEC-041 registered for §7.40 and §7.41. Build items for environment
restoration and the type-level separation of corpus values from local ones (M0),
the tier-ordering type, work-unit declarations, banded and scored duration
estimates, budget proposals, anytime results, the environment ladder and corpus
sample counts (M6).

B-220 is the one to build early: it asserts the machine is returned to how it
was found from every interruption point, which is what makes §XVII's permissions
and §6.39's ladder safe to hold at all.

### Version 6 — four proposals accepted

P1, P2, P3 and P5 accepted from [proposals.md](proposals.md) and registered:
the workload slot format and its authoring documentation (M6), the repro bundle
and its verifier (M5, both blocked on DEC-033 since a bundle of summaries cannot
be re-analysed), pre-acquisition fitment (M1) with projection bands and their
scoring (M5), and the contention snapshot (M5) with the quiet-machine pre-flight
it enables (M6).

B-217 is the item worth noting: D8 made laboratories exclusive, which means a
lab must *begin* on a quiet machine or exclusivity is a claim rather than a
condition. That turns P5 from a diagnostic convenience into a precondition for
every lab in [labs.md](labs.md).

P4 remains proposed and its recommendation is revised downward — see the
proposal.

### Version 5 — plural quality, the lab catalogue, and the stop control

DEC-029 moves from open to drafted: [labs.md](labs.md) proposes twenty
laboratories in four families. Build items added for the four-outcome lab
result, the prohibition on an aggregate score, coverage rendering, and the
workload slot — all consequences of quality becoming plural in intent v9.

B-210 registers the stop control, the first proposal accepted from
[proposals.md](proposals.md).

### Version 4 — the structural decisions surface, and v8's work is registered

Eight decisions added. Three are marked structural and pulled into M0 because
deciding them late destroys data or forces a rewrite: whether the record keeps
raw samples (DEC-033), what constitutes the identity of a measured configuration
(DEC-034), and which host platforms MCF runs on together with the containment
mechanism A14 requires there (DEC-035). None of the three had been asked in any
earlier version, which is the finding.

DEC-009 is narrowed rather than closed: D8 answers the lab-versus-serving half,
leaving disk exhaustion mid-download, several clients and two resident models.

Build items added for the self-contained artifact and its from-scratch
conformance check, the privileged helper and the scenario proving untrusted code
cannot reach it, the time types, the test tiers with their published ages and a
mutation floor, lab exclusivity and its declared suspension, and the energy
laboratory.

### Version 3 — the analysis and exchange work is registered

Five decisions added for §XIII–§XV: what a contribution contains and whether a
machine can be de-identified without being made useless (DEC-027), what an
identifier actually is (DEC-028), which laboratories exist and in what order
(DEC-029), schema versioning across contributed databases (DEC-030), and what
contribution costs the contributor (DEC-031). DEC-015 is resolved by D7.

Build items added for the SQLite record and its structural separation from
content (M0), the lab framework and its instrumentation rules (M6), local-only
recommender inputs (M7), and a new M9 group for the exchange itself.

The three DEC entries marked as hard gates — DEC-027, DEC-028, DEC-029 — are the
ones where proceeding on an assumption would be unsafe rather than merely
premature: two of them decide what leaves a user's machine.

### Version 2 — standardized, and the decisions stop being restated

Restated to the format contract in [README.md](../README.md). The two decision
tables merge into one ordered by what each blocks, and each decision shrinks
from a paragraph to a question: §7 of the intent document holds the question and
its reasoning, and this file holds only its status and its owner. Restating them
here produced two versions of every question, which is the duplication defect
the contract exists to prevent.

Adds DEC-026 and the reference-model items B-018, B-019, B-091, B-127 and B-128
from §XII. Closes B-016.

### Version 1 — the work is registered

Created because every task reference in the project lived inside the intent
document, as §7 voids and as principles that name work without naming a task.
25 decisions and 80 build items were extracted; §7 was deliberately left intact,
since §8 requires voids be resolved in place.

Anti-goals and de-prioritized work were recorded as `dropped` with reasoning
rather than left absent, so they are not re-proposed later as oversights.
