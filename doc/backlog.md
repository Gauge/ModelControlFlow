# Backlog

| | |
|---|---|
| **Type** | Register — every outstanding decision and build item |
| **Version** | 44 |
| **Status** | Living |
| **Authority** | Derived from [document-of-intent.md](document-of-intent.md) v24, governed by [rules.md](rules.md), sequenced by [roadmap.md](roadmap.md) |

**244 items: 50 decisions (35 open, 1 drafted, 2 narrowed, 12 resolved) and 194
build items (21 done, 3 in progress, 56 blocked on a decision, 114 open).** Every item cites
the clause that justifies it; an item that cannot cite is a finding, not a task, and the
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
| DEC-016 | The performance budget numbers | §7.16 | §VII | M0 | **resolved** — D24: sixteen figures, ceilings not targets; footprint provisional pending the prototype |
| DEC-010 | The failure classification scheme | §7.10 | §3.1 | M0 | **resolved** — [taxonomy.md](taxonomy.md): three axes, sixteen domains, 110 codes |
| DEC-019 | Whether the adversarial prototype confirms or amends D4 | §7.19 | **D4** | M0 | **resolved** — confirms. [findings.md](findings.md) F1: four death modes classified with the manager unaffected, five of five accelerator questions answered over the C ABI, every measurable D24 figure under its ceiling |
| DEC-004 | Engine ownership: perform inference, or delegate it | §7.4 | **§VI, §VII** | M0 | open |
| DEC-021 | What the laboratory must simulate, what it declines to, and whether simulated time is structural | §7.21 | **§VIII** | M0 | **resolved** — D26: the taxonomy, bound to what MCF's code claims rather than to the whole table; observed rather than caused; the clock is structural |
| DEC-022 | Where the end-to-end boundary falls for a daemon | §7.22 | **§3.5** | M0 | open |
| DEC-008 | Which hardware is characterized versus attempted-and-uncharacterized | §7.8 | §IV | M0 | **resolved** — D25: characterized means MCF can read the device's live state, per run; the boundary is a capability of the observer, never a vendor list |
| DEC-050 | Which statistic each of D24's sixteen budget figures names | §7.50 | **B-011**, B20 | M0 | **resolved** — D27: three kinds of figure; p99 over ≥100 trials for events; an unattributable run is neither a pass nor a failure |
| DEC-051 | How an event-class budget is ever asserted on a machine somebody is using, and what a scheduled tier does on a CI runner that is never quiet | §7.51 | **B-011**, B38 | M0 | open — [findings.md](findings.md) F2: the development machine's load never drops below the threshold, so every event-class figure is correctly unattributable and none can be asserted |
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
| DEC-033 | Whether the record keeps raw per-trial samples or only summaries | §7.33 | §II, D6 | M0 | **resolved** — D16: raw trials always; interior detail per lab, off by default |
| DEC-034 | The identity of a measured configuration | §7.34 | §IV, D6 | M0 | **resolved** — D17 principle, D18 sampling, D19 seed; placement declared-vs-realized, engine build is identity |
| DEC-046 | Sampling and seeding | §7.34 | §IV, §7.6 | M0 | **resolved** — D18 sampling is identity; D19 the seed set is a condition |
| DEC-047 | GPL-3.0 or AGPL-3.0, and the compatibility matrix of every candidate engine | §7.47 | §XVI, §7.4 | M0 | **narrowed** — D22 copyleft; D23 settles the runtime question |
| DEC-049 | What protects the record from loss | §7.49 | §II, D6 | M0 | **resolved** — D20: rebuildable index over an append-only journal |
| DEC-048 | What rights a contribution carries | §7.48 | §XIV | M9 | **resolved** — D21: dedicated, stated up front, no withdrawal |
| DEC-035 | Which host platforms MCF runs on, and the containment mechanism A14 requires there | §7.35 | **§I, A14** | M0 | open |
| DEC-039 | Which operations actually require elevation, on which platforms | §7.39 | §XVII | M0 | open |
| DEC-037 | Who writes to the record, and what happens to a write that loses | §7.37 | §3.1, D6 | M2 | open |
| DEC-038 | What happens when a pinned artifact decays upstream — withdrawn, gated, relicensed, repointed | §7.38 | §III, §3.6 | M1 | open |
| DEC-032 | Distribution and update policy; whether the container image and the local binary are one artifact or two | §7.32 | **D7** | M8 | open |
| DEC-036 | Whether model licences constrain publishing measurements about the model | §7.36 | §XIV | M9 | open |
| DEC-044 | How a user declares a workflow — a named list, a weighting across laboratories, inference from their own traffic, or from an imported configuration | §7.44 | **§6.36, B41** | M7 | open |
| DEC-045 | Which changes to the machine invalidate comparability, whether MCF detects them, and what it does on discovery | §7.45 | §3.4, §3.8 | M5 | open |
| DEC-040 | What makes two machines alike — which attributes constitute similarity, whether it is one relation or several, and how a machine outside every class is treated | §7.40 | §XIV, D14 | M9 | open |
| DEC-043 | Which box dimensions are enforceable on which platforms, and whether a partially-enforceable box is offered or refused as misleading | §7.43 | **D15** | M6 | open |
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
| B-001 | Rust workspace: crate split (`mcf-core`, `mcf-record`, `mcf-lab`, `mcf-hub`, `mcf-serve`, `mcf-bench`, `mcf-cli`), pinned toolchain, reproducible build | §7.19, §3.12 | `cargo build --locked` reproduces byte-identically from a clean checkout on a pinned toolchain | **done** — [build.md](build.md); toolchain pinned to 1.98.0, layering asserted by test, `scripts/check-reproducible-build.sh` compares two checkouts byte for byte |
| B-002 | Adversarial substrate prototype: probe an accelerator, supervise a child process made to die badly, record both, measure the result against D24's budgets — the run that confirms or amends D4 | §7.19, DEC-019, D24 | Both scenarios produce a well-typed record and a measured footprint; §7.19 is amended or confirmed in writing | **done** — `prototypes/adversarial`; written up as [findings.md](findings.md) F1 and confirmed in D4. Superseded in place by B-013, B-033 and B-011, and dropped when those land |
| B-003 | Failure type: every fallible boundary returns an error carrying a [taxonomy.md](taxonomy.md) category, an attribution and a disposition; no `unwrap`, no `panic`, no discarded `Result` in non-test code | §3.1, §3.16, A2 | CI denies the panicking constructs, and `internal.unclassified` is counted against a target of zero | **done** — `mcf_core::failure`: three axes as types, all 110 codes, cross-checked against [taxonomy.md](taxonomy.md) in both directions; ten constructs denied and demonstrated to bite with a negative control; nothing constructs `internal.unclassified` |
| B-004 | Record store: append-only, structured-first, machine-readable, written at events and never on a timer | §3.3, §6.9 | A running idle daemon writes zero records and performs zero timer wakeups over 60 s | **in progress** — `mcf_record::journal`: append-only, one JSON line per entry, `sync_data` per append, schema-versioned from the first write, and a replay that reports the line, offset and bytes of anything it could not read. The stated condition needs a daemon and is B-031's assertion at M2 |
| B-005 | `Measurement<T>` type that cannot be constructed without its conditions, sample count and spread — illegal states unrepresentable rather than validated against | §3.4, §3.16 | No code path can produce a measurement value without conditions attached; enforced by the type, not a check | **done** — `mcf_core::measurement`: two samples are positional arguments so n≥2 is a property of the type; `Quantity: Ord` rules out floating point, so no NaN can enter; the §3.3 floor is a struct literal with eight `Attested` fields and no `Default` |
| B-350 | `Estimate<T>` and `Measurement<T>` are distinct types with no conversion between them; an estimate can be *replaced* by a measurement and never promoted into one | A20, §4 | The compiler refuses to compare, average or substitute one for the other; a source check refuses a conversion added later | **done** — `mcf_core::measurement::Estimate`: unrelated types, no `From` in either direction, a band rather than a point (B46), and a basis that carries its sample count when it is the corpus (B44) |
| B-006 | `Provenance` type that travels with an artifact by construction: repository, revision, checksum, license, retrieval time, and every transformation since | §3.6, §3.16 | An artifact handle cannot exist without provenance; unknown fields are the `Unknown` variant, never a plausible default | **done** — `mcf_core::provenance`: one constructor, a private field and no setter; every readable field is `Attested`; the upstream artifact's provenance is kept whole, so §XII's requantization chain traverses to its source or stops at a stated unknown |
| B-007 | Condition capture at measurement time: hardware state, thermal, driver and runtime versions, quantization, context length, batch shape, MCF version and configuration | §3.3, §3.4 | The §3.4 floor is captured from a live machine and round-trips through the record store losslessly | **done** — `mcf_core::capture` fills five of the nine from the live machine and leaves the four that describe a running model unknown; `mcf_record::decode` reads them back, and the round trip is asserted on this machine and on a no-accelerator one produced through the seam |
| B-008 | Degradation marking: a result produced under reduced capability is typed as degraded and cannot be rendered without its mark | §3.2 | A CPU-derived result cannot be displayed or exported as though it were accelerator-derived | **done** — `mcf_core::degradation`: `Degraded<T>` is a distinct type with no `Deref`, no `into_inner` and no combinator returning an unmarked value; a degradation is a `Failure` with disposition `degraded`, so the taxonomy travels with the mark |
| B-009 | Laboratory skeleton: deterministic harness, simulated clock, injectable faults, replayable scenarios; held to production code standards | §3.17, §VIII | A scenario reproduces a given failure identically across 100 runs and on a machine with no accelerator | **done** — `mcf-lab`: a constant catalogue (B32), a supplied simulated clock, a per-run world that clears itself on the way in as well as out (B58), and 100-run determinism asserted for every scenario |
| B-010 | Fault catalogue cross-check: the lab's catalogue and the failure taxonomy are the same list, and a taxonomy entry with no simulation fails the check | §3.17, §7.21 | An automated check fails CI when a category MCF's own code constructs has no producing scenario (D26) | **done** — `checks/tests/fault_catalogue.rs`, both directions: a claimed category with no scenario fails, and a scenario for a category nothing constructs fails too |
| B-011 | Performance budget suite: every D24 figure asserted, with zero idle wakeups and zero external requests enforced as prohibitions rather than thresholds | §3.13, §3.5, D24 | A regression fails the build with a before/after under stated conditions | **in progress** — `crates/mcf-cli/tests/budget.rs` behind `scripts/ci.sh --with-budget`: the three figures measurable without a daemon, read by D27's statistics, asserted only in release and only on an attributable machine. The before-and-after needs a stored baseline and tier ages (B-185) |
| B-012 | Overhead self-characterization: MCF measures and reports the cost of its own observation, because an uncharacterized instrument is not a scientific one | §6.2, §3.8 | The measured delta between instrumented and reduced-instrumentation paths is reported as part of a result's conditions | open |
| B-013 | Hardware profiler: accelerators, memory, thermal and power state, driver and runtime versions; unrecognized hardware degrades and is labelled, never guessed | §3.8, §7.8 | Profiles a machine with and without an accelerator; unknown vendors produce an `Unattributed` profile rather than an inference | **done** — `mcf_core::hardware`: two routes, each declaring its coverage, merged without overwriting and with disagreements reported (A8); D25's verdict computed per read and naming which of the four readings is missing |
| B-014 | `mcf doctor`: the M0 product — reports what the machine is, what MCF costs on it, and what it can and cannot promise here | §I, §3.8, §VII | Runs on a machine with no models, no network and no accelerator, and produces a complete, honest report | **done** — `mcf doctor [--no-record] [--json]`: the machine, MCF's cost against D24's ceilings, what it cannot measure here and why, and the promises it cannot make listed beside the ones it can. Writes to the journal; a record it could not write is a stated degradation, not a lost report |
| B-015 | Test seams for expensive paths: no test requires a GPU, a network or a large model | §3.5 | The full suite runs green on a laptop, offline, in under the time budget set by DEC-016 | **done** — `Machine::read_through(routes)` is the seam: passing none produces the profile of a machine with no accelerator *on a machine that has one*, which is how B19's condition is checked rather than assumed. `scripts/ci.sh` passes `--offline`. **The stated time budget does not exist**: D24 closed DEC-016 with sixteen figures and none of them is a suite time. The gating tier is measured and reported in [build.md](build.md) §4 instead |
| B-016 | `rules.md`: the enforceable rules derived from the Document of Intent, each citing the principle it serves | §II, doc §"How to use it", §3.16 | Every rule cites; every rule is checkable by a machine or names the human check it replaces | **done** — [rules.md](rules.md): 99 rules in three tiers; 82 carry a machine check, 15 rest on review alone (tracked as the number to reduce, B16), 2 await a decision |
| B-041 | Documentation conformance check: front matter, changelog, present tense outside changelogs, no dangling `B-*`/`DEC-*`/`§` citation, no broken relative link | [README.md](../README.md) format contract, C5, B16 | A single command fails when any document in `doc/` violates the contract; run in CI beside the code checks | **done** — `checks/tests/documents_conform.rs`, twelve checks in the gating tier. The tense clause is checked only for the constructions the contract names outright; the rest stays a `review` obligation rather than a claim |
| B-353 | The letter `P` names two things — the five precedence rules in [rules.md](rules.md) and the seven proposals in [proposals.md](proposals.md) — and C5 forbids renumbering either | C5, [README.md](../README.md) citation style | A `P` citation resolves unambiguously, by deprecating one namespace in favour of a named successor or by a stated convention the conformance check enforces | open |
| B-018 | Reference-model neutrality: no code path behaves differently because an artifact is the reference model, and the suite never depends on it | §6.22, §XII, §3.5 | Substituting a different model changes what is measured and nothing about how MCF behaves; a CI check fails if the reference model is named outside fixtures and documentation | **done** — `checks/tests/reference_model_neutrality.rs`: no shipped source and no test names the publisher or the family, documentation excepted; a third check fails if the documents stop naming it, so the first two cannot pass by the reference model quietly ceasing to exist |
| B-184 | Duration and timestamp are distinct types with no arithmetic between them; the lab clock is simulated and travels with the result | B37, D9 | `end_wall - start_wall` does not compile; a clock-jump scenario invalidates rather than corrupts | **in progress** — `mcf_core::time`: `Timestamp` has no arithmetic and no interval method, intervals come from `Instant`, and the clock is a type parameter so `Duration<Simulated>` and `Duration<Monotonic>` never meet. The clock-jump scenario waits on B-009 (DEC-021) |
| B-352 | Read the machine's local UTC offset, or record that this platform offers no way to | D9, A7, §3.4 | A record carries a known offset where the platform supplies one, and `unknown` where it does not — never `+00:00` as a stand-in | open |
| B-191 | Test tiers: unit, property, functional, whole-system, fault-injection, load, soak, fuzz, performance, mutation — with the fast hermetic tier gating every change, and the end-to-end boundary drawn by DEC-022 | D10, §6.34, §3.5, DEC-022 | Each tier runs; the gating tier stays offline and fast on a laptop | open |
| B-185 | Every tier publishes its age; a stale heavy tier fails a release rather than being assumed green | B38, §3.1 | A release with a stale mutation or soak tier is refused with the age stated | open |
| B-186 | Mutation score is measured and floored, budgeted like any other property | B38, B20, §3.5 | The score is asserted in CI and may not regress silently | open |
| B-320 | Fully-vendored stack: engines, kernels and math libraries shipped and pinned; every result renders the shipped stack's versions among its conditions; an engine MCF cannot vendor yields a classified outcome naming the reason | B64, D23, §3.12 | The from-scratch conformance run reaches a first token with no vendor runtime installed, and no figure renders without its engine | open |
| B-321 | Deferred-engine register: engines and runtimes avoided because they cannot be vendored, recorded with the reason and revisited on evidence that the performance gap changes which model a user should run | D23, §3.13, C6 | The list exists and is maintained rather than the omissions being silent | open |
| B-192 | Self-contained build: the inference engine and every common-path tool are vendored or reimplemented, statically linked, no runtime and no toolchain required | §XVI, B36 | The artifact has no dynamic dependency a stock machine lacks | open |
| B-183 | From-scratch conformance: a container with no toolchain, no runtime and no package manager runs the binary and reaches a first token | B36, §XVI | Asserted in CI on every platform in DEC-035's scope | blocked (DEC-035) |
| B-190 | Privileged helper: a separate, auditable executable performing one named operation from a short list and exiting; the daemon holds no ambient privilege | A26, §6.32, §XVII | The daemon runs unprivileged in every scenario; the helper's surface is enumerated | blocked (DEC-039) |
| B-180 | Untrusted code cannot reach an elevated path, asserted by scenario rather than by policy | A26, §6.20, §6.4 | An adversarial model and hostile repository code both fail to touch a privileged operation | open |
| B-220 | Environment restoration: a scenario kills MCF mid-run at every stage and asserts governors, priorities, exclusive modes and suspended processes are all restored | A27, §3.25, §6.39 | The machine is returned to how it was found from every interruption point | open |
| B-221 | Corpus-sourced values and locally-measured values are distinct types; only the second can back a recommendation | B43, B34, §6.38 | A foreign number cannot reach a recommendation, enforced by the compiler | **done** — `mcf_core::origin`: `LocallyMeasured<T>` and `FromCorpus<T>` are unrelated types with no conversion either way; a corpus value cannot be built without the sample count B44 requires, and neither reads like the other on a surface |
| B-270 | Summaries cannot be persisted, only projected from trials; every trial carries its arm, interleave position and session | B56, D16, §3.27 | A stored mean does not compile; paired analysis is possible from the record alone | **done** — `mcf_core::trial`: a `Trial` cannot be built without its arm, position and session; there is no mean anywhere in MCF to store, and a check keeps it that way; a pairing is reconstructed from a journal round trip in `mcf-record`'s own suite |
| B-271 | Interior detail is declared per laboratory and off by default; thinning is recorded as a condition | B56, D16, §3.4 | A downsampled series carries its thinning factor and cannot be read as full resolution | **done** — `mcf_core::trial::Series`: no constructor omits the thinning, no accessor returns the points without it, and factors compose so a re-thinned series cannot claim the resolution of its last step. Per-laboratory declaration arrives with the laboratories (M6) |
| B-272 | The identity type excludes hardware by construction; grouping is a query-time view | B57, D17, §XIV | The same configuration on two machines is one identity with two condition sets | **done** — `mcf_core::configuration`: six fields, none of which can hold a machine, checked by a vocabulary sweep as well as by the compiler; sampling in thousandths so identity is an exact equality; realized placement moved to the condition floor, which grows to nine |
| B-300 | Journal-and-index: trials append to a journal, the database is derived and rebuildable, crash-safe write settings enabled, and a failed replay reports the exact extent of the loss | B62, D20, §3.1 | A scenario corrupts the database at every lifecycle stage and the record rebuilds or states what it could not recover | open |
| B-302 | Export: one command, one portable file, sharing the serialization §XIV and P2 need | D20, §XIV, [P2](proposals.md#p2--the-repro-bundle) | One mechanism serves export, contribution and repro bundles | open |
| B-301 | Re-verify artifact checksums before a long measurement run, not only at acquisition | §7.49, §3.6, §3.8 | Silent disk corruption is caught before it produces a garbage result rather than after | open |
| B-042 | Record store is a single SQLite database, schema-versioned from the first write, corruption-resistant and recoverable | D6, §3.3, §3.1 | The schema carries a version; a truncated write is a classified failure and the database reopens; the file is portable between machines | open |
| B-161 | Content store and record store are distinct types with no path between them, so no export can carry content that was never written | A25, §6.8, §6.27 | The type system prevents writing prompt or completion content to the record store | **done** — `mcf_record::content`: two stores in two places, neither module naming the other's types, no conversion either way, and a `Debug` that reports a length rather than a body |
| B-330 | `LICENSE` in the repository, and the per-engine compatibility matrix every vendored component is checked against before it is admitted | DEC-047, D22, D23 | No component ships without a recorded compatibility finding; the licence is stated in the artifact and surfaced to a redistributor | blocked (DEC-047) |
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
| B-331 | Upstream decay: detect that a pinned artifact has been withdrawn, gated, relicensed or repointed, and record it against the provenance without invalidating the local copy | DEC-038, §7.38, §3.6 | A decayed pin is a recorded finding; measurements from the local weights stay valid and the broken chain is visible | blocked (DEC-038) |
| B-029 | `mcf pull` / `mcf list` / `mcf rm`: the M1 product — models enter, live on and leave this machine with provenance intact | §III | A model is acquired, listed with full provenance, and removed deliberately, offline against the fake hub and online against the real one | open |

### M2 — Serve

| ID | Title | Cites | Done when | Status |
|---|---|---|---|---|
| B-030 | Daemon: long-lived, restartable, recovers its state across restarts, survives indefinitely | §7.1, §I | The lab kills the daemon at every lifecycle stage and it recovers to a coherent, queryable state each time | open |
| B-031 | Idle discipline: no polling loops, no background timers, no always-on watchers; idle cost indistinguishable from zero | §3.13, §6.9 | Measured idle CPU and wakeups meet DEC-016's budget, asserted in CI | open |
| B-032 | Engine adapter layer: inference engines are supervised subprocesses, and which engine is in use is a recorded condition | §7.4, §6.2 | At least one engine is driven end to end; swapping engines changes a recorded condition, not a code path | blocked (DEC-004) |
| B-033 | Supervision contract: a runtime that dies mid-token is a classified, attributed failure that does not take the manager down | §3.1, §7.1 | The lab kills a runtime at every stage — pre-load, mid-load, mid-token, post-token — and the daemon stays coherent | blocked (DEC-001) |
| B-034 | Serving API: models addressed by name, stable surface, first token without the user knowing about runtimes, formats or flags | §VI, §3.15 | A first token is obtained from a named model in one command, on a machine that has never served before | blocked (DEC-001) |
| B-035 | Added-latency budget: the overhead MCF interposes between an inbound request and the engine's first token is measured and asserted | §VII, §3.13 | The interposed latency is measured under stated conditions and defended in CI | open |
| B-036 | Local-only by default: the control plane binds locally; network exposure is an explicit, informed, revocable act, never a side effect | §6.12, §3.10 | Default configuration is unreachable from another host; exposure requires an explicit authorization that is recorded | open |
| B-037 | Model residency policy: what stays loaded when nobody is looking, recorded as a measurement condition | §7.18, §3.4 | Residency state is part of every serving latency result | blocked (DEC-018) |
| B-038 | Visible defaults: quantization, context length, runtime and placement are chosen without prompting, and every choice is attributed, explained on demand and overridable | §3.15, §6.14 | `mcf explain <model>` returns the actual reasoning and the measurements behind each default | open |
| B-039 | Authorization gates by category, not frequency: untrusted execution, large irrecoverable resource use, network exposure and destruction are asked every time; everything else flows | §6.14 | The four gated categories are enumerable in code and each has a test asserting it prompts | open |
| B-210 | `mcf stop`: refuse new work, interrupt a lab preserving its partial result, drain and terminate runtimes on a stated deadline, release every held resource including privileged state, record what was stopped, and report what could not be released | [P6](proposals.md#p6--the-stop-control), §3.1, A26, A22 | A held accelerator, locked pages and a changed governor are all released; anything that could not be is named rather than claimed | open |
| B-332 | Record write ownership: a single writer, a defined outcome for a write that loses, and no silent drop | DEC-037, §7.37, §3.1 | Concurrent writers are exercised by the lab; a losing write is classified, never discarded | blocked (DEC-037) |
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
| B-070 | Thin client: no framework, no bundled runtime, no build step, no per-platform application; the device already has what is needed to display it | §V, §6.11 | Total transferred weight and cold render time meet DEC-016's client budget on the oldest committed client | open |
| B-071 | Zero idle cost when nobody is looking: no polling to appear responsive | §6.11, §3.13 | With a browser tab open and idle, daemon CPU is indistinguishable from closed | open |
| B-072 | Parity enforcement: the interface is a client of the same API a script uses, and introduces no action reachable only there | §XI, §6.21 | An automated check fails when an interface action has no headless equivalent | open |
| B-073 | Conditions travel to the surface: no view renders a measurement without its conditions, sample count and spread | §3.4, §3.14 | Rendering a bare number is impossible by construction, not by review | open |
| B-074 | Failure legibility: classified failures, their context and their configuration are inspectable from the window | §3.1, §3.2 | Every taxonomy category has a rendering that names the subsystem and the reconstruction context | open |
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
| B-250 | Comparisons are constructed only from paired, interleaved, order-randomized trials sharing a session id; the reported quantity is the paired difference distribution | B53, §3.27, §3.4 | Block-then-subtract does not compile; a cross-session comparison is constructible but labelled weaker | open |
| B-252 | Every generalized statement resolves on demand to the measurements, conditions and spread behind it; no summary is written in place of its evidence | B55, §3.28, §3.15 | A recommendation expands to its evidence without leaving the interface; the record retains full precision | open |
| B-085 | Isolation check: a comparison in which more than one variable differed reports "these are not comparable" rather than a delta | §3.4 | An intentionally confounded comparison is refused by the tooling | open |
| B-086 | Null and negative results are stored and surfaced as results — "does not fit here", "no measurable speedup" | §3.4 | Both appear in the record and in the window as outcomes, not failures | open |
| B-087 | Partial success representation: nine of ten tasks completing is nine data points | §3.1 | Partial runs are queryable as partial, with their per-unit outcomes intact | open |
| B-088 | Contention governance: MCF knows the difference between a slow model and a busy machine, and says so when it cannot tell | §3.8, §7.9 | A deliberately contended run is marked unattributable rather than reported | blocked (DEC-009) |
| B-089 | Environment pinning to the tolerance DEC-006 sets | §3.12, §7.6 | Every result carries enough environment to be reproduced to the stated tolerance | blocked (DEC-006) |
| B-280 | A reported improvement traces to a workload split that was not used to select it; a winner failing validation reports *no improvement found* | B59, D18, A10 | Sweep results cannot be published from the selection split | open |
| B-290 | A trial cannot be constructed without its seed, and a run cannot declare one seed for every trial; the seed set is published and recorded as a condition | B61, D19, §3.4 | Identical-output runs are unrepresentable; comparisons refuse mismatched seed sets | open |
| B-291 | Seed-set validation: periodically compare the fixed set's distribution against a larger random set; divergence replaces the set and records a break in comparability | D19, §6.16, §7.13 | The standard set is shown to be representative rather than assumed | open |
| B-281 | Recommended sampling renders as *declared* until a sweep promotes it; a lab that pins its own sampling declares it and its results stay apart | B60, D18, A21 | No global default sampling exists; divergence between recommended and best-measured is surfaced | open |
| B-091 | Quantization frontier on the reference model: one model, one machine, the full GGUF quantization range — the cleanest available §3.4 comparison, a single variable across many points | §XII, §3.4, §IV | A frontier is produced across quantizations with one variable differing, and results state they characterize the instrument, not models in general | open |
| B-211 | Repro bundle: one file carrying a claim, its method, its full §3.4 conditions, its raw samples, the artifact's provenance chain, the §XV identifier, and a verification manifest | [P2](proposals.md#p2--the-repro-bundle), §II, A6 | A bundle is emitted for any published measurement and contains everything needed to re-run it | open |
| B-212 | `mcf verify <bundle>`: reproduce the configuration, re-run the method, report agreement or divergence with conditions compared side by side | [P2](proposals.md#p2--the-repro-bundle), §II, A8 | A bundle from another machine either agrees, or names which conditions differ and refuses to attribute the gap | open |
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
| B-105 | Model failure taxonomy: wrong tool, malformed call, loop, early stop, gave up — more informative than the pass rate | §6.17, §7.10 | Every failed trial is classified, and the classification is the primary output | blocked (DEC-023) |
| B-106 | Statistical test for "is this a real difference": §3.9's "within noise, pick either" said honestly | §3.9, §6.17 | Two indistinguishable models produce a refusal to rank, not a ranking | blocked (DEC-023) |
| B-107 | Contamination strategy: private, rotated or procedurally generated tasks, per DEC-023 | §7.3, §3.4 | The strategy is stated, implemented and re-checkable as the suite ages | blocked (DEC-023) |
| B-108 | Zero cost when idle: the benchmark subsystem consumes nothing during ordinary serving | §6.18, §3.13 | Measured serving footprint is identical with the harness compiled in and no benchmark running | open |
| B-109 | Harness minimality guard: each addition must make the measurement more valid, not the harness more capable | §6.18, §5 | Additions cite validity; capability-only additions are refused and the refusal is recorded | open |
| B-181 | Exclusive window for timing-class work: announced, bounded by a declared maximum, schedulable, interruptible, closed automatically; drains the serving path by explicit decision | B35, D8, §6.33 | A timing result cannot be constructed from a run that overlapped serving or another lab | open |
| B-230 | Behaviour-class labs cannot express a wall-clock deadline; deadlines are token budgets | B49, D8, §3.8 | A wall-clock timeout in a behaviour lab does not compile | open |
| B-231 | A behaviour-class run under injected contention produces the same outcomes as one without, differing only in recorded conditions | B49, §6.40, §3.4 | The lab scenario asserts outcome equivalence and condition divergence | open |
| B-232 | Serving latency under a concurrent behaviour-class run stays within its budget; hosting yields to the user and nothing yields to hosting | B50, §3.26, §VI | Asserted in CI against the interposed-latency budget | open |
| B-233 | Environment failures are a distinct taxonomy branch from model failures: an out-of-memory from competition is a condition of the run, never the model giving up | B49, §7.10, §3.1 | Every yielding run's failures classify to one branch or the other, never ambiguously | open |
| B-234 | Yielding mechanism: low priority, foreground-aware, pausable and resumable, per platform | B49, §3.26, §7.42 | A background run does not stutter an interactive application, measured rather than asserted | blocked (DEC-042) |
| B-236 | Resource boxes: a declared allocation of cores, host memory, accelerator share and I/O that a model runs inside, enforced where the platform allows and reported as unavailable where it does not | D15, §XVII, A7 | A model runs inside a stated box; unenforceable dimensions are named rather than silently unbounded | blocked (DEC-043) |
| B-240 | A result carries its box or the explicit absence of one, names the dimensions the box could not bound, and cannot be compared with a result from a different box | B51, D15, A8 | Boxed and unboxed results are not comparable by construction | open |
| B-241 | Interactive degradation under a background run is asserted against a budget | B52, §3.26, D10 | A background behaviour run does not degrade an interactive workload beyond the stated budget, measured in CI | open |
| B-242 | Degradation curves are produced rather than assumed: L24's dose-response sweep and L25's contention sweep | B52, §3.26, §6.41 | Both curves exist for the reference model before yielding is claimed to be unobtrusive | blocked (DEC-029) |
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
| B-228 | Environment ladder: report, wait for quiet, suspend-and-restore only with per-run approval of a named list, never terminate; scope granted per DEC-041 | B48, §6.39, A27, DEC-041 | A scenario asserts nothing outside the approved list is touched and every suspension resumes, including when MCF is killed | open |
| B-222 | Every corpus statement renders its sample count; no filter removes a candidate from a listing | B44, §3.24 | An unreported option is ranked lower and annotated, never hidden | open |
| B-273 | Lab setup and teardown: each lab owns both, may use its own tooling, and leaves nothing behind — asserted by running two labs back to back, including after the first is killed mid-run | B58, A27, §XIII | The second lab sees no trace of the first, warm caches included | open |
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
| B-333 | Workflow declaration: the surface by which a user states what they do, feeding §6.36's ranking and §6.5's visible default | DEC-044, §6.36, B41 | An undeclared workflow produces a visible default, never a hidden one; the declaration selects which laboratories inform a recommendation | blocked (DEC-044) |
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
| B-260 | Longitudinal regression detection: compare like with like, detect against historical spread rather than a threshold, correlate with the diff of everything that changed, report as a labelled hypothesis and never as a cause | [P7](proposals.md#p7--longitudinal-regression-detection), §6.7, A18 | A drop exceeding historical spread is surfaced with what changed alongside it; improvements are reported the same way | blocked (DEC-045, DEC-007) |
| B-261 | Machine-change detection: a profile diff against the last known state, with affected history marked rather than silently carried forward | §7.45, DEC-045, §3.4 | A driver update marks prior results non-comparable rather than leaving them to be misread | blocked (DEC-045) |
| B-334 | Distribution and update: how MCF reaches a user and changes under them, with no silent upgrade and an explicit statement of what an upgrade invalidates | DEC-032, §7.32, §3.12 | An upgrade is offered, explained and never automatic; what it invalidates is stated before it is applied | blocked (DEC-032) |
| B-149 | Answer §7.14 with evidence: the state that lets us say MCF works | §7.14 | The definition is written, and the suite is measured against it | blocked (DEC-014) |

### M9 — The exchange

| ID | Title | Cites | Done when | Status |
|---|---|---|---|---|
| B-310 | The share confirmation renders the contribution terms; no code path offers a retraction | B63, D21, §3.20 | Terms appear before anything is sent; no retraction affordance exists | open |
| B-160 | Share flow: per-share, opt-in, renders the rows that leave rather than a description of them, states the terms (D21) and that publication cannot be undone; whether MCF may ever prompt is DEC-031 | A24, §3.20, §6.27, DEC-031 | No egress path exists that is not user-initiated per share; the confirmation shows the payload | open |
| B-168 | De-identification: fields coarsened, withheld or sent per DEC-027, with MCF stating plainly what a contribution does and does not protect | §7.27, §6.27, §3.10 | A contribution's identifying content is enumerated and the honest claim about anonymity is displayed at the moment of sharing | blocked (DEC-027) |
| B-169 | Identifier: emit one for a configuration MCF holds, and resolve one it is given | §XV, §7.28 | Round-trip on this machine: emit, wipe, resolve, and reproduce the identical configuration | blocked (DEC-028) |
| B-166 | An imported configuration reads as `declared` until a local probe or benchmark verifies it; numbers that travelled with it are attributed elsewhere | B33, §3.21, §6.29 | No imported figure renders as though MCF measured it; verification promotes it and records the divergence | open |
| B-172 | Failure to reproduce an identifier is a first-class outcome, and divergence between imported and local numbers is a recorded finding about how far results travel | §6.29, §6.3, §3.4 | "This identifier needs 48 GiB and you have 24" is a complete answer; a numeric divergence is stored as evidence, not an error | open |
| B-170 | Contribution schema versioning: a contribution declares the schema and MCF version that wrote it, and a reader that cannot fully interpret one says so | §7.30, §3.1, §3.4 | An older contribution is read, marked, or refused — never silently misinterpreted | blocked (DEC-030) |
| B-251 | A contribution carries comparisons in preference to absolutes: both arms, the pairing, and the effect size; absolute rows carry the full §3.4 condition set or are not contributable | B54, §3.27, §XIV | The corpus accumulates ratios that survive travel rather than bare numbers that do not | open |
| B-171 | Contribution carries outcomes, never artifacts: scores, classifications, conditions and distributions leave; tasks, tools, fixtures and model outputs do not | §6.30, §3.19 | An audit of a contribution finds no task content; contamination exposure is recorded per task | open |
| B-335 | Publication constraints from model licences: a per-artifact publication flag alongside the per-artifact use flag, where terms require one | DEC-036, §7.36, §III | A contribution excludes rows whose artifact's terms forbid publishing measurements, and says so | blocked (DEC-036) |
| B-336 | Machine similarity classes: the relation by which "hardware like yours" is computed, and what happens to a machine outside every class | DEC-040, §7.40, §6.38 | A corpus statement names the class it rests on and its sample; an unclassifiable machine gets no corpus guidance rather than wrong guidance | blocked (DEC-040) |
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

### Version 44 — the conditions are captured, and the unknowns survive as unknowns

B-007 is done. The half worth recording is what "losslessly" turned out to
require.

A floor whose unknowns came back from the record as the *word* `unknown` would
compare equal to a floor that had read something — and every comparison
downstream would be quietly wrong. That is A7's substitution arriving through
the back door of a decoder, and it is why the round trip is a test of its own
rather than a property assumed of the encoder. It is asserted twice: once on
this machine, and once on a machine with no accelerator, produced through
B-015's seam rather than waited for.

Two refusals in the decoder follow §7.30: a floor missing a question this
version asks is not decoded at all, because it was written by a version that
asks different ones; and a condition written in a shape this version does not
use is not coerced into text, because a decoder that coerced would make a record
say something nobody wrote.

Four of the nine conditions stay unknown and will until M2: quantization,
context length, batch shape and realized placement describe a model being run,
and nothing runs one. `mcf doctor` says so on every line rather than omitting
them.

### Version 43 — the seam, the neutrality check, and a budget that was never written

B-018 and B-015 are done, and B-015 turned up a defect in its own condition.

**The seam.** `Machine::read_through(routes)` is what B19 means by testing an
expensive path *through* a seam: passing no routes produces the profile of a
machine with no accelerator, on a machine that has one, so the claim *the suite
runs on a laptop with no accelerator* is checked rather than assumed. It is a
parameter and not an environment variable, because an ambient switch that turned
off hardware detection would be undeclared state that changes a result (B2) —
and somebody would eventually set it in production.

**The defect.** B-015's condition says the suite must run *in under the time
budget set by DEC-016*. DEC-016 is closed by D24, D24 gives sixteen figures, and
none of them is a suite time. The clause has been unsatisfiable since the day
D24 was written. It is corrected rather than quietly dropped: the gating tier's
time is measured and reported in [build.md](build.md) §4, and asserting it would
need a figure nobody has stated. Registering a new void for it would be
inventing a requirement — B38 already requires the gating tier be fast and
states that qualitatively.

**The neutrality check** has three parts, and the third is the one that makes
the first two mean anything: it fails if the *documents* stop naming the
reference model. Without it, deleting §XII would turn both other checks green.

### Version 42 — a foreign number cannot decide a local question

B-221 is done. B34 states the direction — *MCF contributes outward and decides
inward* — and B43's violation is a sorted candidate list whose ordering nobody
explains, which is a foreign conclusion wearing a local interface.

The shape that prevents it is two unrelated types rather than one type with a
flag. A flag is a filter and a filter can be misconfigured; a function that
takes `LocallyMeasured<T>` cannot be handed a `FromCorpus<T>` however tired the
author is.

There is no conversion in either direction, and the reason is A20's: a corpus
value does not *become* local by being confirmed, it is **replaced** by the
local measurement. Replacement needs no mechanism, so the absence of one is the
enforcement.

The corpus type cannot be constructed without its sample count, because B44
requires every corpus statement to render one — and the rendering says so, since
a label that lives only in the type is a label nobody sees.

### Version 41 — the trials are the record, and there is no mean to store

B-270 and B-271 are done. The first has a condition — *a stored mean does not
compile* — that turned out to have a stronger reading than the obvious one.

The obvious reading is a rule about the write path: something that refuses to
serialize a summary. The stronger one is that **MCF has no mean to write**.
`Quantity` requires only `Ord`, `Measurement` reports order statistics,
and nothing in the shipped crates averages anything — so a mean is not
forbidden, it is unrepresentable. A check now keeps it that way, along with the
absence of floating point that goes with it, because the failure mode is
somebody adding one for a single call site and a NaN arriving a division later.

The second condition — *paired analysis is possible from the record alone* — is
demonstrated rather than argued: a session's trials are written to a journal,
dropped from memory, replayed, and §3.27's question asked of what came out.

B-271's design constraint is that a thinned series looks exactly like a full one
— an ordered list of values — so nothing about the *data* can distinguish them.
Only the type can, which is why `Series` has no constructor that omits the
thinning and no accessor that returns the points without it. Factors compose, so
a series thinned twice reports six rather than two.

### Version 40 — content is in a different place, not behind a flag

B-161 is done. A25's guarantee has to be structural — *a filter can be
misconfigured, a store that never held the data cannot leak it* — so there are
two stores, in two directories, and neither module names the other's types. An
export that walks the record cannot reach content because there is nothing to
follow.

Two smaller decisions inside it are worth keeping. `Content`'s `Debug` reports a
length and never a body, because a debug rendering is a surface and A17's *nothing
leaves the machine unchosen* includes a terminal somebody is sharing. And the
accessor is called `disclose` rather than `text`, because that is what calling it
does: the structure cannot help against a caller who has decided to look, so the
call site says so.

Length is deliberately not content. It is a measurement *about* content, it
belongs in the system record, and it is what lets a record say "a 4 096-byte
prompt" without holding one.

### Version 39 — the budget tier measures, and says when it is not judging

B-011 is in progress rather than done, and the remainder is named rather than
glossed: its condition is *a regression fails the build with a before/after
under stated conditions*, and a regression needs a stored baseline to be
measured against. That is B-185's tier ages and a history the record does not
yet keep. What exists is everything up to that point — the figures, D27's
statistics, and the conditions a comparison would need.

Two refusals are the design. The tier **asserts only in release**, because
D24's ceilings are about the artifact MCF ships and a debug binary is a
different one. And it **asserts only on a quiet machine**: a run under
contention is unattributable rather than failing, so a budget cannot be violated
by somebody else's compile.

Both are stated in the tier's own output. A tier that printed figures without
saying when it was not judging them would read as judging them, which is A2's
silent failure aimed at the suite.

Four of D24's sixteen figures still cannot be measured at all: idle CPU, timer
wakeups, memory growth over thirty simulated days and added latency are about a
daemon, and there is none until M2.

### Version 38 — the budget statistic closes, and B-011 unblocks

DEC-050 is resolved by D27, three versions after F1 raised it. What made it
answerable was that D24 had already named a statistic for one figure and given
the reason — *the tail is what a user feels* — so the work was to notice that
the reason generalizes and to say what it costs: an event-class figure needs at
least a hundred trials, because a p99 of twenty is the maximum wearing a
percentile's name.

The half that changes B-011's shape is the other one. A budget is asserted only
on a run the machine was quiet enough to attribute, so a busy machine makes a
run unattributable rather than failing it — and an unattributable run is not a
pass either, which is what keeps a real regression from hiding behind a
permanently busy machine.

### Version 37 — the laboratory exists, and `doctor` runs it

B-009 and B-010 are done, and `mcf doctor` now runs the catalogue rather than
reporting that one exists. The difference is §VIII's whole content: *there is a
laboratory* is a fact about the repository, and *every failure MCF claims to
handle was reproduced on this machine a moment ago* is a fact about the
operator's machine.

**A design defect the laboratory found in itself.** The first world gave each
scenario a directory named for the scenario and the process, so that repeated
runs would produce byte-identical failures. Two threads running the same
scenario then cleared each other's directory. The fix is the more honest
design: each run gets its own directory, and the path is elided from the
outcome before outcomes are compared — because *where* a run happened is
environment and *what happened* is the failure, and only the second is what
§3.17 asks to reproduce.

**B-010 checks both directions.** A claimed category with no scenario fails, as
the item asks. So does a scenario for a category nothing constructs any more —
which is worse than an absent scenario, because it passes for ever while
testing nothing and looks like coverage.

### Version 36 — the laboratory unblocks, and B-010's condition sharpens

DEC-021 is resolved by D26, and two M0 items unblock with it.

B-010's stated condition changes, and the change is the substance of the
decision. It read *an automated check fails CI when a taxonomy category has no
producing scenario*, which would have made the check fail on 110 categories the
day it was written and stayed failing for years. A13's actual rule is that an
untested **claim** is not made, and MCF claims a category when its own code can
produce one — so the check binds to what the code constructs. Coverage of what
is claimed is complete from the first day and cannot regress, because a new
failure site cannot land without its scenario.

The other half of D26 is a constraint on code that is not test code: anything
that waits, times out or measures an interval takes its clock rather than
reaching for one. That is why the decision could not wait — retrofitting it is a
rewrite of every deadline in the system.

### Version 35 — M0 has its product

B-014 is done. `mcf doctor` reports what this machine is, what MCF costs on it
against D24's ceilings, and what MCF will and will not promise here.

Three things it refuses to do are the design rather than omissions from it.

**It does not report an unmeasured budget as passing.** *Not measured* is its
own verdict, beside *within* and *over*, because a figure with no reading that
rendered as a tick would be a reassurance rather than a report (A7).

**It names what it cannot measure.** Idle CPU, timer wakeups, memory growth
over thirty simulated days and added latency are D24 figures about a daemon, and
there is no daemon until M2 — so the report says that, in a list, rather than
showing three figures where seven belong.

**It lists the promises MCF cannot make.** A report showing only the ticks
would read as complete. The two it cannot make at M0 — nothing about model
quality, and no laboratory has demonstrated any failure MCF claims to handle —
appear beside the ones it can.

The cold-start line shows both the median and the p95 and says that which one
D24's ceiling names is §7.50 and open. On the machine this was developed on the
two disagree by more than an order of magnitude under load, so choosing one
quietly would have been choosing the answer.

### Version 34 — the journal is the record

B-004 is in progress rather than done, and the reason is a defect in its own
condition rather than in the work. *A running idle daemon writes zero records
and performs zero timer wakeups over 60 s* cannot be asserted at M0: there is
no daemon until M2, and B-031 states the same assertion where the daemon exists.
What the item is actually about — an append-only, structured-first store written
at events and never on a timer — is built.

The property is structural rather than measured. There is no flush thread, no
batching timer and no background writer in the module, so nothing in it *can* be
scheduled; an idle MCF writes nothing because there is no code that would.

Two design notes worth keeping. **One line per entry**, so a crash mid-append
leaves a torn *line* — a failure mode replay can find, bound and report — rather
than a file whose record boundaries are unrecoverable. And **the format is
versioned from the first write**, because §7.30 makes a schema a public
interface the moment it is shared, and a journal from a version this build
cannot read is refused rather than appended to: appending would make the file
unreadable to both.

The line format is written rather than depended on, and [build.md](build.md) §6
records the reasoning. The data model is closed, the format is one §7.30 makes
MCF's to keep stable for ever, and correctness is a testable property of about
three hundred lines — so it is bought with tests (A19) rather than with a
dependency.

### Version 33 — the profiler reads the machine, and says what it could not

B-013 is done. Its condition — *unknown vendors produce an `Unattributed`
profile rather than an inference* — turned out to be the easy half; the harder
one is D25's, and the design that satisfies both is routes rather than branches.

A route declares which of D25's four readings it can supply and returns what it
found. Two are compiled in, and neither knows about the other. Readings merge
**without overwriting**: a route that ran second cannot replace a reading the
first took, because that would resolve a disagreement by preferring whichever
ran last, and A8 makes a disagreement a finding rather than a tie to break.
Where two routes both claim a field and differ, the difference is reported
alongside the device.

The characterization verdict is computed on every read and names *which*
readings are missing rather than only that something is. "Something is unknown"
is the report A2 calls worse than a crash, scaled down to a field.

### Version 32 — hardware scope closes, and B-013 unblocks

DEC-008 is resolved by D25, on the evidence F1 produced rather than on an
argument. The boundary between *characterized* and *attempted,
uncharacterized* is what MCF can read about a device at measurement time, not
which vendor made it — because §3.8's requirement is about telling a slow model
from a busy machine, and a vendor list answers nothing about that.

B-013 unblocks. Its own condition already anticipated the answer — *unknown
vendors produce an `Unattributed` profile rather than an inference* — and D25
supplies what the other side of that line means.

### Version 31 — the prototype ran, and it moved two decisions

B-002 is done and DEC-019 is resolved: the prototype confirms D4 rather than
amending it. [findings.md](findings.md) F1 holds the evidence and the
conditions it was taken under, and D4 gains a paragraph citing it.

It did not only confirm. Two things it found change what other decisions have to
weigh, and both are registered rather than left in a report nobody re-reads.

**DEC-008 now has a fact to decide against.** The accelerator's *identity* is in
the files a driver publishes; its *live state* — device memory, temperature — is
not, and is reachable only over the C ABI. §3.8 requires MCF know the difference
between a slow model and a busy machine, and that difference is made of exactly
the two fields the file route cannot supply. Deciding what "characterized" means
without knowing that would have been deciding it blind.

**DEC-050 is new.** D24 names a statistic for one of its sixteen figures and for
none of the rest. On a quiet machine that costs nothing; on a machine that was
compiling, twenty cold-start trials gave a passing median and a p95 over the
ceiling by a factor of two. B-011 cannot assert a budget without knowing which
statistic it asserts, so this gates that item rather than merely informing it.

### Version 30 — identity is exact, and the floor grows by one

B-272 is done. Two consequences worth recording.

**Sampling parameters are carried as thousandths, not as floating point.** D18
puts them in the identity, and identity is an equality question — `0.7` is not
a value a binary float holds exactly, so two configurations that should be the
same would depend on how each was parsed, and `Eq` and `Hash` are not available
on `f64` at all. Thousandths are exact, orderable and hashable, at a resolution
finer than any publisher states.

**The condition floor grows from eight questions to nine.** Intent v16 splits
placement: the declared intent is identity and the realized layout is a
condition, because the realized layout names hardware and B57 keeps hardware out
of identity. §3.3 says the floor never shrinks; it does not say it never grows,
and "everything that varies and could change a result" plainly reaches which
devices held which layers. The floor is a struct literal with no `Default`, so
the ninth field broke every construction site — which is the friction the design
intends.

### Version 29 — the estimate is a band, and it does not meet a measurement

B-350 is done. Two things it forced are worth recording.

**"Replaced" needs no mechanism, and that is the enforcement.** A20 says an
estimate can only be replaced by a measurement, and the temptation is to build
the replacement — a `promote`, an `into_measurement`, a comparison helper. None
exists: a caller that has taken a measurement uses the measurement, so there is
no line of code that could be read as a promotion, and a source check refuses
one added later.

**An estimate is a band, not a point.** B46 requires it, and the reason is that
a duration predicted from a rate is a range; rendering it as a single number is
false precision, which B46 calls the smallest possible version of a confident
wrong number. A zero-width band is still an estimate — the width says how
uncertain the guess is, not whether it is one.

### Version 28 — the documents are checked, and four of them were wrong

B-041 is done, and what it found on its first run is the argument for it
existing.

**The intent document's changelog had stopped at version 8 while its front
matter claimed version 24.** Sixteen versions of change had their reasoning
recorded only in the commits that made them, which §8 does not permit: it
requires the reasoning outlive the change, and a reader of the document is not
a reader of a version-control history. The entries were reconstructed from
those commits and the document is now at version 25.

Three smaller defects: a citation to B-125 written without its hyphen, so that it read as a rule identifier that does not exist; this
register's own front matter two versions behind its changelog, and its header
counts three revisions stale — the last two corrected in version 22 before the
check existed to find them.

B-353 registers a defect the check surfaced rather than fixed. The letter `P`
names two things — the five precedence rules and the seven proposals — and C5
forbids renumbering either, so the check accepts a `P` citation that resolves in
either namespace and the ambiguity is recorded rather than papered over.

The tense clause of the contract is the one that resists a machine. It is
checked only for the constructions the README names outright, and the remainder
is left as a stated `review` obligation. B16 counts a review check as a cost;
claiming a machine check that is really a keyword search would have been worse
than counting it.

### Version 27 — degradation is contagious, not merely recorded

B-008 is done. The item asks that a degraded result be *typed* rather than
flagged, and the consequence worth recording is what that forces: there is no
operation on `Degraded<T>` that returns a plain `T`, so anything computed from
a degraded input is degraded. A flag would have made degradation something a
later stage could forget; a type makes forgetting it a line somebody has to
write, and `into_parts` hands the mark back alongside the value so that line is
visible.

A degradation is a `Failure` whose disposition is `degraded`, not a parallel
vocabulary. §3.2 and §3.1 are the same requirement seen from two sides — a
degradation is exactly a failure MCF continued past — so the taxonomy, the
attribution and the context come along, and the M0 mockup's `⚠ DEGRADED` block
renders from the same fields as any other failure.

### Version 26 — provenance closes, and the chain is the reason

B-006 is done. The design decision worth recording is that provenance is a
*linked* structure rather than a flat record, and §XII is why: the reference
model is a third-party requantization, so the artifact MCF actually handles is
several transformations away from the weights it describes. A flat record can
say *this file came from that repository*; it cannot say *these bytes are a
requantization, by that tool, of weights from a different repository under a
different licence*. The upstream provenance is therefore kept whole rather than
summarized — A1 — and a chain that stops does so at a stated unknown, which A9
makes a result rather than a failure.

The licence needed three states, not two. An identifier MCF matched, terms that
are present and unmatched, and nothing there at all are genuinely different
answers; collapsing the middle into the last would let MCF proceed past terms
nobody read.

### Version 25 — the time model, and the half of it that waits

B-184 is in progress rather than done, and the distinction is the point. Its
first condition holds: `Timestamp` has no arithmetic and no interval method, so
subtracting two wall-clock readings has no spelling, and the clock is a type
parameter — `Duration<Simulated>` and `Duration<Monotonic>` are different types
that cannot be compared, so A11 is a compiler check rather than a review
comment. Its second condition, that a clock-jump scenario invalidates rather
than corrupts, needs the laboratory to produce the jump; that is B-009, blocked
on DEC-021. Marking the item done would have claimed a demonstration nobody has
run.

B-352 is registered. `Timestamp::now` records the local offset as `unknown`,
because reading it needs a platform call the standard library does not offer
and B15 admits weight only against a stated cost. Writing `+00:00` instead
would have been wrong for most of the world, which is exactly the plausible
substitute A7 forbids — so the gap is registered rather than filled.

### Version 24 — the measurement type closes, and A20 gains an item

B-005 is done. Two design consequences are worth recording because neither was
obvious from the item's text.

**Uncertainty became a property of the signature.** §3.4 calls a single-shot
timing an anecdote, so the constructor takes two samples as positional
arguments and any others after them. Nothing refuses `n < 2` at runtime because
nothing can express it — which also means the type needs no failure category
for a case that cannot arise.

**Ordering, not arithmetic.** Every summary MCF reports from a measurement is
an order statistic, so `Quantity` requires `Ord` rather than `PartialOrd`. The
consequence is that no floating-point sample type exists and therefore **no NaN
can enter a measurement** — a NaN being a number that has lost the information
about what went wrong, which is what A1 forbids.

B-350 is registered. A20 is an absolute rule whose check is `compiler` —
estimates and measurements are different types with no conversion — and it had
no build item. It is cheapest to satisfy while the measurement type is new and
nothing produces estimates yet.

### Version 23 — the failure type closes

B-003 is done. The taxonomy is now types the compiler holds, and the agreement
between `crates/mcf-core/src/failure/` and [taxonomy.md](taxonomy.md) is
checked in both directions rather than maintained by hand.

What is worth recording is what the item's condition turned out to require. "CI
denies the panicking constructs" is a claim about a lint table, and a lint table
is exactly the kind of claim that reads as true while being false — deleted,
reordered by a `priority`, or renamed by a compiler release. It is now checked
by writing each construct into a copy of the workspace and requiring the build
to refuse it, with a negative control that must *not* fire, because ten
refusals and a probe that never compiled look identical from outside.

### Version 22 — the first build item closes, and the counts are recounted

B-001 is done: the workspace exists, the toolchain is pinned to an exact
version, and the reproducibility claim is checked by comparing two checkouts
byte for byte rather than asserted. [build.md](build.md) holds what the build
asserts about itself, so this register keeps only the status.

Two stale figures corrected, both of the kind that make a register unusable for
deciding what is left. The front matter had said version 4 while the changelog
had reached 21, and the header's decision counts were three versions behind the
table beneath them — they are now counted from the rows. B-016's status still
reported the 58 rules of the version that closed it, against 99 in
[rules.md](rules.md) v16.

### Version 21 — milestone coverage audited

Two findings, both mechanical rather than judgemental.

**Eighteen stale blocks.** Items still marked blocked on decisions since
resolved — chiefly DEC-016 and DEC-010, closed by D24 and
[taxonomy.md](taxonomy.md). All cleared; items blocked only on those are now
open. DEC-029 is *drafted* rather than resolved, so blocks on it stand:
[labs.md](labs.md) is explicitly unratified.

**Seven decisions with no work registered against them.** A decision nobody has
scheduled work for is a decision that lands and changes nothing. B-330 through
B-336 fill them: the licence file and per-engine compatibility matrix (M0),
upstream artifact decay (M1), record write ownership (M2), workflow declaration
(M7), distribution and update (M8), publication constraints from model licences
and machine similarity classes (M9). Four more decisions gained citations on
existing items rather than new ones.

One decision is deliberately left with no item: **DEC-003**, whether non-agentic
quality is measured at all, is a scope question whose answer decides whether
there is work rather than what the work is.

Verified in the same pass: every one of the roadmap's 106 delivers-bullets cites
a registered item, and no backlog item sits outside a milestone.

### Version 20 — two M0 gates close

DEC-016 resolved by D24 and DEC-010 by [taxonomy.md](taxonomy.md). B-011 and
B-003 unblock, and with them the four other items that were waiting on numbers
or on a classification scheme.

### Version 19 — vendored only

Tier 2 deferred. B-320 becomes the fully-vendored stack and B-321 registers the
deferred-engine list, so the omissions are maintained rather than silent.

### Version 18 — engine tiers

B-320 registers D23's tiering. DEC-047 narrowed again: the proprietary-runtime
question is settled, leaving the GPL/AGPL choice and the per-engine compatibility
matrix.

### Version 17 — durability, terms and licence

DEC-049 and DEC-048 resolved by D20 and D21; DEC-047 narrowed by D22 to two
sub-questions, one of which is sharp: whether a proprietary accelerator runtime
may be shipped under copyleft or must be treated as a detected platform
capability. That one can exclude an engine §7.4 would otherwise prefer.

### Version 16 — three critical gaps

DEC-047 (MCF's licence and what vendoring inherits), DEC-049 (record durability)
and DEC-048 (contribution rights) registered. The first two join M0's gates: an
engine MCF cannot legally ship is not a candidate for §7.4, and a record with no
durability story is months of unreconstructible science on one file.

### Version 15 — identity closed

DEC-034 and DEC-046 both resolved. Two of the three structural M0 gates are
answered; only DEC-016, the performance budget, remains.

### Version 14 — sampling settled

DEC-046 narrowed to the seed question alone; D18 settles the rest. B-280 and
B-281 registered — the split discipline for swept values, and recommended
settings rendering as declared until verified.

### Version 13 — sampling split out

DEC-046 registered for §7.46. Placement and engine build settled inside DEC-034;
sampling is a larger question than either and gets its own decision.

### Version 12 — two M0 gates close

DEC-033 resolved by D16 and DEC-034 narrowed by D17 — two of the three
structural decisions that were blocking M0. Build items for unpersistable
summaries, hardware-free identity, per-lab interior detail, and lab teardown.

### Version 11 — three gaps registered

DEC-044 (how a workflow is declared, blocking §6.36) and DEC-045 (what happens
when the machine changes underneath) registered. B-260 and B-261 for P7's
regression detection and the machine-change detection it depends on.

### Version 10 — pairing and generalization

B-250 registers paired interleaved comparison as a type property, B-251 makes
the corpus carry ratios in preference to absolutes, and B-252 keeps
generalization confined to the rendering.

DEC-043 gains a candidate answer in §7.43 rather than a resolution: §3.27
largely defuses the partial-box worry, since a box is no longer what holds
conditions still.

### Version 9 — boxes and measured politeness

B-236 registers resource boxes, gated on DEC-043 since a box that appears to
isolate and does not is worse than no box. B-240 makes the box a measurement
condition at the type level. B-241 and B-242 are the two halves of B52: an
application test for what yielding costs the user, a laboratory for what
constrained resources cost the model.

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
