# ModelControlFlow — Backlog

**Status:** Living document. Derived from [document-of-intent.md](document-of-intent.md)
Revision 6. Companion to [roadmap.md](roadmap.md). Governed by [rules.md](rules.md).

**What this is.** The single register of outstanding work. Every item here traces
to a citation in the Document of Intent — an intent (§I–§XI), a principle (§3.x),
a conflict resolution (§6.x), or a void (§7.x). An item with no citation is a
finding, not a task: it means intent is missing, and the correct response is to
record a void in §7 rather than invent the intent here.

**Relationship to §7 (Voids).** The Document of Intent's §8 requires that voids
be recorded and resolved *in place*, in §7, and migrated into §3 or §6 when
answered. This backlog therefore **tracks** those voids rather than relocating
them: §7 remains the authoritative statement of each open question, and the
`DEC-*` items below are the scheduled acts of deciding them. Deleting §7 to
populate this file would have destroyed the reasoning the document exists to
preserve. Every other item — `B-*` — is implementation work that did not
previously exist in written form anywhere.

**How to read an item.**

- **ID** — stable for life. Never reused, never renumbered.
- **Kind** — `DEC` a decision that must be made and recorded in §7; `B` a build
  or verification task.
- **Cites** — the clause of intent that justifies the work. Per §8, work that
  cannot cite is work nobody chose.
- **Done when** — the falsifiable condition. §7.14 (Definition of Done) is
  itself unanswered, so these are local acceptance criteria, deliberately
  narrow, until it is.
- **M** — the milestone in [roadmap.md](roadmap.md) that owns it.

**Status vocabulary:** `open`, `in progress`, `blocked (by ID)`, `done`,
`dropped (reason)`. Per **C5** and **C6** in [rules.md](rules.md), IDs are stable
for life and nothing is deleted from this file: items that die are marked
`dropped` with the reasoning, so they are not re-proposed later as oversights.

---

## 0. Blocking decisions

These are §7 voids that block a named intent outright. Until each is answered,
the intent it blocks can only be gestured at, and any code written under it is
building on an undeclared assumption. They are ordered by how much downstream
design they block, which is §7's own ordering.

| ID | Kind | Title | Cites | M | Status |
|---|---|---|---|---|---|
| DEC-016 | DEC | The performance budget itself — real numbers for idle CPU, resident memory, cold start, added request→first-token latency, installed footprint; plus the oldest client "any device" commits to | §7.16, §VII, §3.13, §6.11 | M0 | open |
| DEC-010 | DEC | Failure taxonomy — the classification scheme every failure is filed under, designed once because it surfaces in records, tests, the lab's fault catalogue, the recommender, and the UI | §7.10, §3.1, §3.17 | M0 | open |
| DEC-019 | DEC | Validate the Rust decision adversarially rather than defend it — probe a GPU, supervise a child runtime made to die badly, record both under §3.1, measure against DEC-016 | §7.19, §3.13 | M0 | open |
| DEC-004 | DEC | Engine ownership: does MCF perform inference or delegate it? §7.4 states the likely answer (lightest possible wrapper over the fastest available engine) and deliberately declines to resolve it | §7.4, §VI, §VII, §III | M0 | open |
| DEC-021 | DEC | What the laboratory is obliged to simulate — the fault catalogue, and whether simulated time is structural (§3.17 implies yes); plus what the lab explicitly declines to model | §7.21, §3.17, §6.16 | M0 | open |
| DEC-022 | DEC | What "full system" means for a daemon — does an end-to-end test drive real HTTP, cross a process boundary, start a real or simulated engine, exercise restart with persisted state | §7.22, §3.5 | M0 | open |
| DEC-020 | DEC | How much reality validates the lab — what fraction of the suite needs a real-hardware counterpart, on which hardware, and what divergence declares the simulator defective | §7.20, §6.16, §VIII | M8 | open |
| DEC-002 | DEC | The objective function — how quality, latency, throughput, memory, power and disk trade, and how a user states their weighting; §VI additionally requires a *default* opinion that is visible as a default | §7.2, §IV, §6.5 | M7 | open |
| DEC-026 | DEC | The reference set — how many models beyond `unsloth/Qwen3.8-27B-GGUF`, chosen along which axes (family, format, size, quantization lineage, tuning style), and when models two and three join; §6.23 forbids a generality claim from a field of one but does not say how large "exists" is | §7.26, §6.23, §XII | M7 | open |
| DEC-023 | DEC | The agentic suite: contents, statistics, and honesty over time — task provenance (published / authored / generated / user-derived), trial counts, what counts as a difference, difficulty calibration, contamination strategy, cost per run | §7.23, §IX, §6.17 | M6 | open |
| DEC-024 | DEC | Scope and cost of capability probing — which capabilities, when probes run, what they cost, whether results cache across MCF versions, and how to act on *inconclusive* | §7.24, §X, §3.18 | M3 | open |
| DEC-007 | DEC | Scientific acceptance criteria — minimum sample count, maximum variance, required warm-up, thermal steady state, what retroactively invalidates a run | §7.7, §II, §3.4 | M5 | open |

## 1. Open decisions, non-blocking

Answerable later without stalling a milestone, but each is load-bearing for the
milestone named.

| ID | Kind | Title | Cites | M | Status |
|---|---|---|---|---|---|
| DEC-008 | DEC | Hardware scope — which accelerators, vendors and runtimes are supported-and-characterized versus attempted-and-uncharacterized, and what happens on hardware MCF does not recognize | §7.8, §3.2 | M0 | open |
| DEC-001 | DEC | Residual §7.1 sub-questions: which API surface is offered (OpenAI-compatible, native, both), the supervision contract when a served runtime dies, how many models may be resident at once | §7.1, §VI | M2 | open |
| DEC-009 | DEC | Resource arbitration and concurrency — who decides what is resident; what happens when a benchmark and a served model both want the accelerator, or a download would exhaust the disk mid-flight; whether MCF may (or must) refuse to benchmark while serving | §7.9, §3.8, §3.11 | M2 | open |
| DEC-018 | DEC | What happens to a served model when the user stops looking — indefinite residency versus unload-and-cold-start; whichever is chosen becomes a §3.4 measurement condition | §7.18, §3.13, §VI, §3.11 | M2 | open |
| DEC-017 | DEC | Authentication and the trust posture of the control plane — what proportionate protection looks like once exposed, and whether the serving API and the control API deserve different answers | §7.17, §6.12 | M4 | open |
| DEC-006 | DEC | Reproducibility guarantee level — reproducible to what tolerance across which changes (same machine same day / after a driver update / a different machine of the same model); determines how much environment is pinned | §7.6, §3.12, §6.13 | M5 | open |
| DEC-011 | DEC | Offline and degraded-network operation — how much of MCF works with no network; and the distinction §7.11 draws between "no internet" and "no local network", which the interface conflates | §7.11, §3.2, §V | M1 | open |
| DEC-013 | DEC | State, versioning and migration — whether historical measurements remain comparable across MCF versions, which implies MCF's own version is a measurement condition and some upgrades must invalidate history | §7.13, §3.4 | M8 | open |
| DEC-005 | DEC | Retention and residency of the record — how long the record must be kept, whether anything may leave the machine, whether the user can inspect and purge it, and what happens when its disk budget is exhausted (§6.9 forbids a silent drop) | §7.5, §3.10, §6.8 | M8 | open |
| DEC-025 | DEC | Whether automatic configuration may change under a user — silent improvement breaks §3.4 comparability; never changing lets configuration rot | §7.25, §X, §6.13 | M3 | open |
| DEC-003 | DEC | Residual §7.3: whether non-agentic quality (prose, summarization, translation, tone) is measured at all, declined, or admitted via model-as-judge with biases declared | §7.3, §5 | M6 | open |
| DEC-014 | DEC | Definition of done — what state lets us say MCF works; without it the test suites have no target to be complete against and the project cannot distinguish progress from motion | §7.14 | M8 | open |
| DEC-015 | DEC | Success beyond the author — whether MCF is meant to be usable by others, which decides whether documentation, installation and interface stability are goals or incidents | §7.15, §V | M8 | open |
| DEC-012 | DEC | Residual §7.12: behaviour when several clients attach at once (a concurrency question under DEC-009, not a surface question) | §7.12, §7.9 | M4 | open |

---

## 2. Build items

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
| B-018 | Reference-model neutrality: no code path behaves differently because an artifact is the reference model, and the suite never depends on it | §6.22, §XII, §3.5 | Substituting a different model changes what is measured and nothing about how MCF behaves; a CI check fails if the reference model is named outside fixtures and documentation | open |
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
| B-062 | `mcf probe` / `mcf explain`: the M3 product — a model is configured the way it was designed to run, and can prove it | §X | A model whose defaults were previously wrong measurably improves, and the improvement is attributable to a named probe | open |

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
| B-127 | The recommender refuses a field of one: a frontier with a single point is not a frontier, and a single-model recommendation is a claim MCF has no basis for | §6.23, §3.9 | A single-candidate field produces a refusal with its reasoning, never a ranking | open |
| B-128 | Expand the reference set to the breadth DEC-026 requires before any §IV recommendation is published | §7.26, §6.23 | No generality claim is made until the set exists; results before that say so on every surface | blocked (DEC-026) |
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

---

## 3. Dropped and superseded

Recorded rather than deleted, per §8.

| ID | Title | Reason |
|---|---|---|
| — | Ambient telemetry subsystem: continuous sampling, always-on tracing, metric streams, dashboards, trace backends | Never opened as work. §6.9 resolved against it by decision in Revision 4, and §5 makes "not an observability platform" an anti-goal. Recorded here so it is not re-proposed as an oversight. The confidence it would have bought is supplied by §VIII (M0's laboratory) and its cost is accepted in §6.15. |
| — | Fine-tuning, distillation, merging and pruning | Out of scope at this revision per §6.6 and §5. §6.6 flags itself as the resolution most likely to be revisited; if it is, it gives way by amendment to §6.6, not by an item appearing here. |
| — | Cross-machine leaderboard and result publication | §5 anti-goal. MCF measures *this* machine and is actively suspicious of numbers that did not originate locally. |
| — | Plugin ecosystem, extension API, general configurability | §5 anti-goal. Every generalization is weight (§3.13). |
| — | Chat product surface beyond an instrument for exercising a model and capturing evidence | §5 anti-goal. |
