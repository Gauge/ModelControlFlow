# Backlog

| | |
|---|---|
| **Type** | Register — every outstanding decision and build item |
| **Version** | 191 |
| **Status** | Living |
| **Authority** | Derived from [document-of-intent.md](document-of-intent.md) v43, governed by [rules.md](rules.md), sequenced by [roadmap.md](roadmap.md) |

**273 items: 55 decisions (22 open, 1 drafted, 2 narrowed, 2 partly settled, 5
decided, 23 resolved) and 218 build items (109 done, 1 dropped, 13 in progress,
36 blocked on a decision, 59 open).** Every item cites
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
| DEC-010 | The failure classification scheme | §7.10 | §3.1 | M0 | **resolved** — [taxonomy.md](taxonomy.md): three axes, sixteen domains, 111 codes |
| DEC-019 | Whether the adversarial prototype confirms or amends D4 | §7.19 | **D4** | M0 | **resolved** — confirms. [findings.md](findings.md) F1: four death modes classified with the manager unaffected, five of five accelerator questions answered over the C ABI, every measurable D24 figure under its ceiling |
| DEC-004 | Engine ownership: perform inference, or delegate it | §7.4 | **§VI, §VII** | M0 | **resolved** — D32: delegate the kernels, own the wrapper. [findings.md](findings.md) F8 measured the slope: MCF's best safe portable Rust is 25–50× one core of a *generic* tuned BLAS on the same machine, and the careful tiling step came out slower than the one-line reorder |
| DEC-021 | What the laboratory must simulate, what it declines to, and whether simulated time is structural | §7.21 | **§VIII** | M0 | **resolved** — D26: the taxonomy, bound to what MCF's code claims rather than to the whole table; observed rather than caused; the clock is structural |
| DEC-022 | Where the end-to-end boundary falls for a daemon | §7.22 | **§3.5** | M0 | **decided** — D36: the shipped binary across every boundary MCF ships, and nothing on MCF's side mocked. All four of §7.22's questions are now answered by tests that exist rather than by a paragraph: a real protocol against a peer the laboratory can make hostile, the real engine MCF ships (the stand-in today, B-320's tomorrow), the shipped binary across every process boundary MCF has, and recovery from the disk wherever there is state. A simulated component appears only to produce a failure that is hard to cause on purpose (D26). The outer edge — no network beyond loopback, no accelerator, no large model — is what keeps the tier gating, and a claim that cannot be tested inside it belongs to a scheduled tier that says so |
| DEC-008 | Which hardware is characterized versus attempted-and-uncharacterized | §7.8 | §IV | M0 | **resolved** — D25: characterized means MCF can read the device's live state, per run; the boundary is a capability of the observer, never a vendor list |
| DEC-050 | Which statistic each of D24's sixteen budget figures names | §7.50 | **B-011**, B20 | M0 | **resolved** — D27: three kinds of figure; p99 over ≥100 trials for events; an unattributable run is neither a pass nor a failure |
| DEC-051 | How an event-class budget is ever asserted on a machine somebody is using, and what a scheduled tier does on a CI runner that is never quiet | §7.51 | **B-011**, B38 | M0 | **resolved** — D30: attributability is a property of a *reading*, measured as the scheduling delay across it. [findings.md](findings.md) F3 has the evidence and the defect the first implementation had |
| DEC-011 | How much works offline, and the difference between no internet and no local network | §7.11 | §3.2 | M1 | **resolved** — D33: offline is the ordinary case (everything but acquisition runs in a container with no network at all), and what MCF says when a network is needed and missing is what it *observed* rather than which layer is absent. [findings.md](findings.md) F10 measured why: a name that will not resolve reports no error kind at all, whether the cause is no network, no resolver or no such name. The distinction §V asks about is one an operator draws with `--from`, not one MCF probes for |
| DEC-001 | API surface, the supervision contract on runtime death, simultaneous residency | §7.1 | §VI | M2 | **decided** — [PR9](proposals.md#pr9--what-serving-looks-like), accepted by the operator. MCF's own line-delimited protocol over the socket it already has, one request and many answer lines so a first token is observable, and the conditions — engine build, seed, sampler, degradation mark — in the terminating line, because the shape everybody expects has nowhere to put them and a surface that cannot carry the mark strips it (A5, B65). A runtime that dies mid-token is a partial answer with the tokens already produced and a classified failure, never a silent retry (A4, B2). One model resident at a time, because two make every latency figure depend on what else was loaded (§3.4); DEC-018 stays separate. An OpenAI-compatible adapter is a later and explicitly lossy decision that must refuse to serve a degraded result rather than serve it stripped |
| DEC-009 | Arbitration outside a laboratory: disk exhaustion mid-download, several clients, two resident models | §7.9 | §3.8 | M2 | **narrowed** — D8 answers the lab/serving half, and the disk half is now built rather than decided: B-026 refuses a download that would not fit, with the arithmetic, and classifies a filesystem that fills anyway (F11). What remains is genuinely M2's: several clients at once, and two resident models |
| DEC-018 | Whether a served model stays resident when nobody is looking | §7.18 | §VI | M2 | **resolved** — D41, by measurement (F35): one model, held after its first request until another is asked for or the daemon stops, no timer (§6.9), stated in every account (`loaded: loaded` then `resident`, `resident_since`, dequantized size) and in `mcf status`. On MCF's own engine the load is 0.8 s against 6.8 s of forward passes for the shortest request, so residency is worth a twelfth today and the whole wait on a faster engine |
| DEC-024 | Which capabilities are probed, when, at what cost, and what *inconclusive* licenses | §7.24 | **§X** | M3 | **resolved** — D42, via [PR10](proposals.md#pr10--what-a-probe-is-and-when-configuration-may-change). A probe earns its place when a wrong answer to it would corrupt a measurement, which bounds §X's "full" by §3.8's reason for it and sorts the list into configuring probes (M3: chat template, stop conditions, usable context) and characterizing ones (framework in M3, probes beside §XIII). On demand, never at acquisition. A result is a `Measurement` with conditions and caches exactly as far as they hold. Inconclusive licenses nothing |
| DEC-025 | Whether automatic configuration may change under a user | §7.25 | §X | M3 | **resolved** — D43, via [PR10](proposals.md#pr10--what-a-probe-is-and-when-configuration-may-change). Never silently. A derived configuration carries the probe that set it; when MCF's answer would now differ that is a divergence it reports (B-058), and applying it is an act that is recorded and changes the conditions. Not the rotting branch: out-of-date is *visible* rather than stale |
| DEC-017 | Proportionate protection for an exposed control plane | §7.17 | §V | M4 | open |
| DEC-012 | Behaviour when several clients attach at once | §7.12 | §XI | M4 | open |
| DEC-007 | What makes a result publishable: sample count, variance, warm-up, thermal state | §7.7 | **§II** | M5 | **partly settled by the operator, 2026-08-27; the rest is measurement.** (1) *No result from a single run* — the floor is several repeats, never one. (2) *The numbers are not to be chosen, they are to be measured*: MCF measures its own noise floor — the same run repeated on a quiet machine and again under deliberate load — and the repeat count and acceptable spread are derived from what it finds and published beside it, the way every threshold here that worked was measured rather than assumed (F27, F32, F41). (3) **Quiet is relative, not absolute.** A user's machine may idle at thirty or forty percent and that is *its* normal; refusing to measure below an absolute quiet would deny most people a result while telling them nothing. The criterion is **stability against the machine's own baseline** — a machine steady throughout a run is measurable wherever its baseline sits; one that moves partway through is not — and the band that counts as steady comes from the noise measurement, which says how much variation actually moves the number. What remains open: warm-up and thermal steady state, and the band itself, both of which the noise measurement is expected to decide **First measurement taken (F51), and it answers more than it was asked.** On this machine, seven repeats detect a five-percent difference and two percent is not honestly reachable at any count a person will wait for. But the shape matters more than the count: sixteen burners moved the median sixty-six percent and widened the spread only from 4.2% to 9.1%, so **contention shifts the level coherently rather than adding noise** — an error that lands on every repeat in the same direction and that no repeat count removes. Interleaving does, which makes B-250 worth roughly fifteen times what the repeat count is worth. Still open: warm-up and thermal steady state, the same measurement through the *provisioned* engine (which is what benchmarks will use and whose noise is unmeasured), and whether pinning the frequency governor removes part of the 4.2% **Second measurement (F52) revises the count.** Seven repeats was MCF's own engine; the provisioned one — which is what benchmarks will use — is about three times noisier and needs **fifty** for a five-percent claim, ten for ten percent. The daemon path contributes nothing (3.6% either side of it), and pinning the server to one thread made it five times *worse* rather than better, so **thread count is a condition of any timing** and not an implementation detail. Still open: warm-up and thermal steady state, and whether pinning the frequency governor narrows any of it **Third measurement (F53) changes the shape of the answer, not the number.** Six clean readings of one command gave repeat counts from seven to over a hundred: **the noise floor is a property of the half-hour, not of the machine**, so *at least N repeats* cannot be the rule — N is not stable. The rule is a **stopping condition**: repeat until this run's own resampling separates the effect from its own noise, and report what that took. The governor piece is answered and needs no privilege — this machine is already at `performance` and offers only `powersave`, so there is nothing to pin; frequency still spans nine to one across cores but neither it nor load explains the noise, their correlations changing sign between rounds. **Thermal steady state is the one piece that stays unanswerable here**: the only sensor this machine exposes reads sixteen degrees, which is not a processor temperature |
| DEC-006 | Reproducible to what tolerance, across which changes | §7.6 | §3.12 | M5 | open |
| DEC-023 | The agentic suite: tasks, trial counts, difference tests, contamination, cost | §7.23 | **§IX** | M6 | open, and **bounded by the operator's answer of 2026-08-27** on how far MCF goes into qualities with no judgement-free measure: measure the honest proxies and name them as proxies — what a language costs, how much a model repeats itself, how wide a vocabulary it uses, whether a stated format is obeyed — never labelling any of them *creativity*; take the user's own material for the rest, which is the workload slot [PR1](proposals.md#pr1--customizable-workloads) already establishes, reporting only exactly-checkable outcomes; and say on the same screen that storytelling and persona are not measured. **A larger model grading outputs is refused**: it makes MCF's answer depend on believing another model's opinion, which is what this project declines everywhere else, and the grader's biases would become MCF's |
| DEC-003 | Whether non-agentic quality is measured, declined, or judged with biases declared | §7.3 | §IX | M6 | open |
| DEC-002 | The objective function, and the default opinion §VI requires MCF to ship | §7.2 | **§IV** | M7 | **partly settled by the operator, 2026-08-27.** The scope was clarified first, because the question read larger than it is: no model is bundled, no catalogue is consulted, and **no network is involved at any point** — the user fetches what they want themselves, and MCF ranks files already on their disk from measurements taken on their machine. The opinion is a weighting in the code, not a lookup. (1) **Plain-language profiles**, computed locally — *fastest that fits*, *most faithful that fits*, *cheapest to run* — so that somebody who is not technical reads one line rather than a four-column table, which is the person the interface must not frighten off. (2) **The measurements are shown beside the plain language, not behind it.** Not a summary with the numbers hidden: the intent is that an ordinary user is exposed to the meaningful values and *learns what to look for*, which a collapsed view prevents. (3) **An advanced view always exists** and holds everything. (4) **What is not measured is named.** Four axes are not the whole of what makes a model good, and a frontier that lists only what it measured implies otherwise — the same failure `mcf explain` already guards against by ending with what MCF cannot tell you. Creativity, storytelling, persona and range of language are qualities the operator names as wanted and unquantified; until a laboratory measures one, the frontier says so rather than omitting it. Still open: the exact weightings, and whether a profile may ever combine axes into a single figure — which A6 and the prohibition on scalars across laboratories both bear on (5) **Each measured value carries a touchstone: what a high or low reading *tends to* mean, in plain words** — a model that repeats itself is steadier for fixed phrasing and flatter for writing that should surprise; a vocabulary that spends three times as many tokens on a language costs three times as much in it. A bare number teaches nobody what to look for, which was the point of showing it. **The touchstone is *declared*, never verified**, and is written so it cannot be read as a result: MCF has not measured that repetition suits home automation, and says so beside the sentence. It is phrased so that a laboratory which later measures the relation *replaces* it with a finding rather than appearing to confirm it — the same three states this project already uses for a model's own claims, turned on MCF's (A21, §3.18) |
| DEC-026 | How much breadth a generality claim requires, and along which axes | §7.26 | **§IV** | M7 | open |
| DEC-020 | How much real hardware validates the lab, how often, and what divergence is fatal | §7.20 | **§VIII** | M8 | open |
| DEC-013 | Whether measurements survive MCF's own upgrades | §7.13 | §3.4 | M8 | open |
| DEC-005 | How long the record is kept, who may purge it, and what happens when its budget is exhausted | §7.5 | §3.10 | M8 | open |
| DEC-014 | What state lets us say MCF works | §7.14 | §3.5 | M8 | open |
| DEC-033 | Whether the record keeps raw per-trial samples or only summaries | §7.33 | §II, D6 | M0 | **resolved** — D16: raw trials always; interior detail per lab, off by default |
| DEC-034 | The identity of a measured configuration | §7.34 | §IV, D6 | M0 | **resolved** — D17 principle, D18 sampling, D19 seed; placement declared-vs-realized, engine build is identity |
| DEC-046 | Sampling and seeding | §7.34 | §IV, §7.6 | M0 | **resolved** — D18 sampling is identity; D19 the seed set is a condition |
| DEC-047 | GPL-3.0 or AGPL-3.0, and the compatibility matrix of every candidate engine | §7.47 | §XVI, §7.4 | M0 | **narrowed** — D22 copyleft; D23 the runtime question; **D28 the licence: GPL-3.0-only**. What remains is the per-engine compatibility matrix (B-330) |
| DEC-049 | What protects the record from loss | §7.49 | §II, D6 | M0 | **resolved** — D20: rebuildable index over an append-only journal |
| DEC-048 | What rights a contribution carries | §7.48 | §XIV | M9 | **resolved** — D21: dedicated, stated up front, no withdrawal |
| DEC-035 | Which host platforms MCF runs on, and the containment mechanism A14 requires there | §7.35 | **§I, A14** | M0 | **resolved** — D29: all platforms, Linux first; three states as D25 gives a device; per-platform artifacts, since the target triple is already a §3.4 condition |
| DEC-039 | Which operations actually require elevation, on which platforms | §7.39 | §XVII | M0 | **decided** — D35, from a machine rather than from documentation ([findings.md](findings.md) F15, `prototypes/elevation/measure.sh`, which changes nothing). Four things MCF wants need no privilege: pinning its own processes to cores, bounding its own memory through the cgroup the platform already delegates, reading temperatures, and reading per-process accelerator occupancy. Three are what a helper is for: the CPU frequency governor, an accelerator's exclusive compute mode, and the processor's energy counter — a *read* that needs elevation, which is the awkward case §7.39 anticipated and which D11 makes first-class. Five are declined outright — SMT, core offlining, IRQ affinity, dropping the page cache, real-time scheduling — because each changes the machine for everybody using it, and they become recorded conditions rather than knobs MCF turns. Per platform, because the answer is |
| DEC-037 | Who writes to the record, and what happens to a write that loses | §7.37 | §3.1, D6 | M2 | **decided** — D34: everybody writes, nobody arbitrates, and an identifier carries the writer that minted it. [findings.md](findings.md) F13 measured that concurrent appends of whole lines do not tear — eight processes, sixteen thousand lines, two filesystems, none torn — so the question was never coordination. It was naming: each writer counted its own appends from zero, so two programs recording the same kind of event in the same second produced one identifier for two events. A writer token — the process, the moment the writer was made, and a count of the writers made in that process — makes that impossible without any coordination at all. A write that fails is classified and returned, never retried silently and never dropped. The single-writer alternative is priced in D34: a running daemon for every command, to buy an ordering the record already has |
| DEC-038 | What happens when a pinned artifact decays upstream — withdrawn, gated, relicensed, repointed | §7.38 | §3.6, §3.7 | M1 | **decided** — D37, on [findings.md](findings.md) F17's measurement of what a hub will actually say. Checked when somebody asks and never on a timer (B4, §3.13); recorded against the provenance rather than over it, because provenance is what was true at acquisition (§3.6); and never retroactive — a model vanishing from a hub says nothing about the bytes on this disk, and a tool that greyed out its own measurements because somebody else deleted something would be destroying evidence. What is really lost is third-party reproducibility, which belongs in the repro bundle as a stated condition (PR2). What MCF cannot detect is *withdrawn* versus *private* versus *never existed*: the hub answers 401 to all three, so the refusal names the ambiguity instead of advising a credential that may not exist |
| DEC-052 | What a *controlled environment* is, concretely: a container MCF builds, a prefix MCF manages, or something else — and what containment it must give against an engine's own installer | §7.53 | D39, A27, §6.32 | M2 | **resolved** by measurement (F30) — a rootless container MCF drives: base image pinned by digest, source mounted read-only, a prefix the operator can point at any drive mounted for output, the exact package set recorded into the prefix beside what was built. The managed prefix survives as the shape of the output; the container is what makes the inputs enumerable. Decided against the host-prefix route by its own first use: the oracle was built by a cmake resolved from a pyenv shim, recorded nowhere and different from what the system says — D39's first condition failed silently on day one. Measured for the container: zero bytes of residue outside the prefix, and a binary that runs on the host and agrees with the host-built one exactly. Boundaries stated: glibc no newer than the host's, `podman` is now a condition a machine can lack, and rebuilds re-resolve packages unless a derived image is kept |
| DEC-053 | What B28 forbids, exactly: recognizing an *artifact* against reading a *field the file states* — and where a family may be named | §XII, B28, B29 | D26, A19 | M2 | **resolved** — they are different things and only the first is forbidden. GGUF states `general.architecture` and `tokenizer.ggml.pre`, and an engine that would not read them is an engine that runs one family; the reference model's publisher, repository and digest are identity and are read nowhere. A family may be named in `crates/mcf-standin/src/architecture.rs` and nowhere else, and that module is held to three things the neutrality check verifies: it names no publisher, it names no artifact, and no function in it takes a name and answers yes or no — the shape *is this the special one* is what B28 is actually about. Totality is deliberately **not** checked: a `match` on a `&str` does not compile without a catch-all, so a check for it would be one that cannot fail (F19) |
| DEC-054 | What the engine is developed against, given that the reference model is 16 GB and every iteration runs it | §XII, D38, §3.12 | D40, B-370 | M2 | **resolved** — the question the test asks picks the model, in both directions. §XII's three justifications are provenance, the quantization frontier, and residency pressure; none is about running a model, so the engine gets a conformance corpus of the smallest *trained* model per family, one distinct quantization each. But there is no size policy: reading a format needs the smallest artifact of that shape, refereeing behaviour needs one good enough to be right, and residency needs one big enough to press on the machine. F22 measured the middle floor — at 160M a correct engine and one with a swapped rotary pairing are indistinguishable because the correct engine answers three of four ordinary questions wrongly by itself; at 0.6B every prompt separates them, for 397 MB and sixteen seconds. F21's claim that the corpus *depends* on B-368's oracle is withdrawn: it holds for the corpus's smallest member and not one step up |
| DEC-055 | Embedding models are a different *kind* of model, not another family: what surface asks one for an answer | §XII, §VI, D38, §5 | B-371 | M0 | **resolved** — F29: the question these models answer is not `mcf run`'s question, so it is given its own verb. `mcf embed <model> --text <text>` returns one vector as a JSON line with the conditions after it, rather than a surface built for tokens returning something that is not one |
| DEC-032 | Distribution and update policy; whether the container image and the local binary are one artifact or two | §7.32 | **D7** | M8 | open |
| DEC-036 | Whether model licences constrain publishing measurements about the model | §7.36 | §XIV | M9 | open |
| DEC-044 | How a user declares a workflow — a named list, a weighting across laboratories, inference from their own traffic, or from an imported configuration | §7.44 | **§6.36, B41** | M7 | open |
| DEC-045 | Which changes to the machine invalidate comparability, whether MCF detects them, and what it does on discovery | §7.45 | §3.4, §3.8 | M5 | open |
| DEC-040 | What makes two machines alike — which attributes constitute similarity, whether it is one relation or several, and how a machine outside every class is treated | §7.40 | §XIV, D14 | M9 | open |
| DEC-043 | Which box dimensions are enforceable on which platforms, and whether a partially-enforceable box is offered or refused as misleading | §7.43 | **D15** | M6 | open |
| DEC-042 | What yielding guarantees, and how it is achieved per platform — process and accelerator priority, memory reservation, foreground detection without ambient polling, automatic versus offered pausing | §7.42 | **§3.26, D8** | M6 | open |
| DEC-041 | The environment-control surface: which knobs, how scope is granted, what happens to a suspended process if MCF is killed, and how the ladder's maximum height differs by platform | §7.41 | §6.39 | M6 | open |
| DEC-029 | Which laboratories exist, in what order, and what a lab must state about its own validity | §7.29 | **§XIII** | M6 | **drafted** — [labs.md](labs.md) proposes 26 in 4 families, first three named; ordering and slot contents unratified |
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
| B-004 | Record store: append-only, structured-first, machine-readable, written at events and never on a timer | §3.3, §6.9 | A running idle daemon writes zero records and performs zero timer wakeups over 60 s | **done** — `mcf_record::journal`: append-only, one JSON line per entry, `sync_data` per append, schema-versioned from the first write, and a replay that reports the line, offset and bytes of anything it could not read. The stated condition waited for a daemon to exist and is now measured: an idle daemon over sixty seconds wrote nothing and was scheduled zero times (B-031) |
| B-005 | `Measurement<T>` type that cannot be constructed without its conditions, sample count and spread — illegal states unrepresentable rather than validated against | §3.4, §3.16 | No code path can produce a measurement value without conditions attached; enforced by the type, not a check | **done** — `mcf_core::measurement`: two samples are positional arguments so n≥2 is a property of the type; `Quantity: Ord` rules out floating point, so no NaN can enter; the §3.3 floor is a struct literal with eight `Attested` fields and no `Default` |
| B-350 | `Estimate<T>` and `Measurement<T>` are distinct types with no conversion between them; an estimate can be *replaced* by a measurement and never promoted into one | A20, §4 | The compiler refuses to compare, average or substitute one for the other; a source check refuses a conversion added later | **done** — `mcf_core::measurement::Estimate`: unrelated types, no `From` in either direction, a band rather than a point (B46), and a basis that carries its sample count when it is the corpus (B44) |
| B-006 | `Provenance` type that travels with an artifact by construction: repository, revision, checksum, license, retrieval time, and every transformation since | §3.6, §3.16 | An artifact handle cannot exist without provenance; unknown fields are the `Unknown` variant, never a plausible default | **done** — `mcf_core::provenance`: one constructor, a private field and no setter; every readable field is `Attested`; the upstream artifact's provenance is kept whole, so §XII's requantization chain traverses to its source or stops at a stated unknown |
| B-007 | Condition capture at measurement time: hardware state, thermal, driver and runtime versions, quantization, context length, batch shape, MCF version and configuration | §3.3, §3.4 | The §3.4 floor is captured from a live machine and round-trips through the record store losslessly | **done** — `mcf_core::capture` fills five of the nine from the live machine and leaves the four that describe a running model unknown; `mcf_record::decode` reads them back, and the round trip is asserted on this machine and on a no-accelerator one produced through the seam |
| B-008 | Degradation marking: a result produced under reduced capability is typed as degraded and cannot be rendered without its mark | §3.2 | A CPU-derived result cannot be displayed or exported as though it were accelerator-derived | **done** — `mcf_core::degradation`: `Degraded<T>` is a distinct type with no `Deref`, no `into_inner` and no combinator returning an unmarked value; a degradation is a `Failure` with disposition `degraded`, so the taxonomy travels with the mark |
| B-009 | Laboratory skeleton: deterministic harness, simulated clock, injectable faults, replayable scenarios; held to production code standards | §3.17, §VIII | A scenario reproduces a given failure identically across 100 runs and on a machine with no accelerator | **done** — `mcf-lab`: a constant catalogue (B32), a supplied simulated clock, a per-run world that clears itself on the way in as well as out (B58), and 100-run determinism asserted for every scenario. A world now also sweeps the residue of worlds whose *process* is gone: clearing on the way in cleared only this world's directory, so a killed suite orphaned its scratch and no later run would ever name it — forty-five were found on the development machine after a morning of killing hung processes, which is A27's question answered wrongly. It sweeps only what it can prove is dead |
| B-010 | Fault catalogue cross-check: the lab's catalogue and the failure taxonomy are the same list, and a taxonomy entry with no simulation fails the check | §3.17, §7.21 | An automated check fails CI when a category MCF's own code constructs has no producing scenario (D26) | **done** — `checks/tests/fault_catalogue.rs`, both directions: a claimed category with no scenario fails, and a scenario for a category nothing constructs fails too |
| B-011 | Performance budget suite: every D24 figure asserted, with zero idle wakeups and zero external requests enforced as prohibitions rather than thresholds | §3.13, §3.5, D24 | A regression fails the build with a before/after under stated conditions | **done** — `crates/mcf-cli/tests/budget.rs` behind `scripts/ci.sh --with-budget`. Every figure measurable without a daemon is asserted rather than reported (D30) and refused under deliberate load ([findings.md](findings.md) F3); each is compared with the last reading recorded in `.mcf-tiers/baselines/`, and a figure that got worse by more than a stated tolerance fails with both readings and both condition sets (B20). Two figures are judged and one is only reported, because the cold start is dominated by a condition nothing records (B-193, F5). The figures that need a daemon — idle CPU, timer wakeups, added latency — arrive with it at M2. The core-binary footprint baseline was re-recorded on 2026-08-27 at 4,054,184 B (from 3,834,576, +5.7 %, over the 2 % tolerance): the growth is the daemon serving generations (B-034), provisioning (B-367), the embedding surface (B-371) and the bert family (B-371), each a feature the register names — a re-baseline with its reason, not a tolerance widened |
| B-012 | Overhead self-characterization: MCF measures and reports the cost of its own observation, because an uncharacterized instrument is not a scientific one | §6.2, §3.8 | The measured delta between instrumented and reduced-instrumentation paths is reported as part of a result's conditions | **done** — at M0 MCF's observation *is* the record write, and `mcf_record::overhead` measures it beside the real record over D27's hundred trials. The instrumentation profile joins the condition floor as its tenth question, so `mcf doctor` and `mcf doctor --no-record` produce results a reader can tell apart |
| B-013 | Hardware profiler: accelerators, memory, thermal and power state, driver and runtime versions; unrecognized hardware degrades and is labelled, never guessed | §3.8, §7.8 | Profiles a machine with and without an accelerator; unknown vendors produce an `Unattributed` profile rather than an inference | **done** — `mcf_core::hardware`: two routes, each declaring its coverage, merged without overwriting and with disagreements reported (A8); D25's verdict computed per read and naming which of the four readings is missing |
| B-014 | `mcf doctor`: the M0 product — reports what the machine is, what MCF costs on it, and what it can and cannot promise here | §I, §3.8, §VII | Runs on a machine with no models, no network and no accelerator, and produces a complete, honest report | **done** — `mcf doctor [--no-record] [--json]`: the machine, MCF's cost against D24's ceilings, what it cannot measure here and why, and the promises it cannot make listed beside the ones it can. Writes to the journal; a record it could not write is a stated degradation, not a lost report |
| B-015 | Test seams for expensive paths: no test requires a GPU, a network or a large model | §3.5 | The full suite runs green on a laptop, offline, in under the time budget set by DEC-016 | **done** — `Machine::read_through(routes)` is the seam: passing none produces the profile of a machine with no accelerator *on a machine that has one*, which is how B19's condition is checked rather than assumed. `scripts/ci.sh` passes `--offline`. **The stated time budget does not exist**: D24 closed DEC-016 with sixteen figures and none of them is a suite time. The gating tier is measured and reported in [build.md](build.md) §4 instead |
| B-016 | `rules.md`: the enforceable rules derived from the Document of Intent, each citing the principle it serves | §II, doc §"How to use it", §3.16 | Every rule cites; every rule is checkable by a machine or names the human check it replaces | **done** — [rules.md](rules.md): 99 rules in three tiers; 82 carry a machine check, 15 rest on review alone (tracked as the number to reduce, B16), 2 await a decision |
| B-041 | Documentation conformance check: front matter, changelog, present tense outside changelogs, no dangling `B-*`/`DEC-*`/`§` citation, no broken relative link | [README.md](../README.md) format contract, C5, B16 | A single command fails when any document in `doc/` violates the contract; run in CI beside the code checks | **done** — `checks/tests/documents_conform.rs`, twelve checks in the gating tier. The tense clause is checked only for the constructions the contract names outright; the rest stays a `review` obligation rather than a claim **And the register now counts itself** (`checks/tests/the_register_counts_itself.rs`): the backlog's headline sentence is totalled from its own rows rather than maintained by hand, and every row's status must begin with one of the states the register defines, because a status opening with prose is a row nobody can total. Its first run found the header thirty items behind the table — 204 build items of which 54 done, against 217 of which 84 — a decision row whose milestone and status had been overwritten by a pasted copy of a build row, and one status written as an essay |
| B-353 | The letter `P` names two things — the five precedence rules in [rules.md](rules.md) and the seven proposals in [proposals.md](proposals.md) — and C5 forbids renumbering either | C5, [README.md](../README.md) citation style | A `P` citation resolves unambiguously, by deprecating one namespace in favour of a named successor or by a stated convention the conformance check enforces | **done** — the proposals are `PR<n>`, `P<n>` there is deprecated in favour of the named successor digit for digit, and the conformance check now resolves `P` against the precedence rules alone |
| B-018 | Reference-model neutrality: no code path behaves differently because an artifact is the reference model, and the suite never depends on it | §6.22, §XII, §3.5 | Substituting a different model changes what is measured and nothing about how MCF behaves; a CI check fails if the reference model is named outside fixtures and documentation | **done** — `checks/tests/reference_model_neutrality.rs`: no shipped source and no test names the publisher or the family, documentation excepted; a third check fails if the documents stop naming it, so the first two cannot pass by the reference model quietly ceasing to exist **The role is split in two (operator, 2026-08-27)**, which changes what neutrality has to hold for but not that it holds. A *realism* reference — a large model whose cost resembles real work — and a *validation* reference, small, readable by MCF's own engine, cross-checkable, on which every method is proven before it is pointed at anything expensive. The named 27B is the first and cannot be the second: it declares an architecture MCF has not been taught and is refused by name, so it cannot be cross-checked and does not fit the fidelity method. **Both are named in the documents and neither may be named in code** — that is the whole of this row and the split does not soften it. |
| B-184 | Duration and timestamp are distinct types with no arithmetic between them; the lab clock is simulated and travels with the result | B37, D9 | `end_wall - start_wall` does not compile; a clock-jump scenario invalidates rather than corrupts | **done** — `mcf_core::time` keeps the two apart by type; `mcf_record::journal::anomaly` notices a moved calendar by holding *both* clocks across an append, and two scenarios produce a backward step and a forward jump with a disposition of `invalidated`. The entry is still written: what an anomaly invalidates is what was measured across it, not the event |
| B-352 | Read the machine's local UTC offset, or record that this platform offers no way to | D9, A7, §3.4 | A record carries a known offset where the platform supplies one, and `unknown` where it does not — never `+00:00` as a stand-in | **done** — `mcf_core::time::zone` reads the zone file in safe Rust, resolves the offset in force *at the moment* rather than now, and is unknown beyond what the file records rather than extrapolating. Checked against what the system itself reports |
| B-191 | Test tiers: unit, property, functional, whole-system, fault-injection, load, soak, fuzz, performance, mutation — with the fast hermetic tier gating every change, and the end-to-end boundary drawn by DEC-022 | D10, §6.34, §3.5, DEC-022 | Each tier runs; the gating tier stays offline and fast on a laptop | **done** — all ten exist, declared in `checks/src/tiers.rs` and compared against `scripts/ci.sh`, the tree and [build.md](build.md) in both directions. Five gate, in five seconds; five are scheduled behind flags. The whole-system tier covers the one of §7.22's four questions M0 has anything to answer and names the other three, so DEC-022 governs its extension rather than its existence |
| B-185 | Every tier publishes its age; a stale heavy tier fails a release rather than being assumed green | B38, §3.1 | A release with a stale mutation or soak tier is refused with the age stated | **done** — `scripts/check-tier-ages.sh [--release]`, and every `scripts/ci.sh` run reports the ages. Stale is *the source changed*, not *a clock advanced*: no clause states how old a soak result may be (A23), and what actually invalidates one is decidable — each tier stamps a digest of the manifests, the toolchain pin, the crates, the checks and the scripts. The stamps are machine-local, so a fresh checkout says it has run nothing rather than inheriting a result |
| B-186 | Mutation score is measured and floored, budgeted like any other property | B38, B20, §3.5 | The score is asserted in CI and may not regress silently | **done** — `MUTATION_FLOOR_PERCENT` in `scripts/lib-tiers.sh`, at 100 %: a claim about the twelve mutants in the catalogue, each of which breaks something a rule rests on, rather than about every conceivable mutation. A score below the floor *or* below the last run's stamp (B-185) exits non-zero, and both refusals were checked against a copy of the tree with a deliberate survivor in it. The twelfth mutant is there because it survived: nothing checked that a resident reading is in bytes, and now something does The catalogue is eighteen now: four were added for what the last stretch built, each one a mutation of something a *finding* paid for — counting every block rather than only those that cache (F16's fourfold cache), reporting a silent hub as a decay (F17), a helper writing the governor it was told rather than one the machine offers, and an index that describes a journal it does not cover. Each was checked to compile and to be killed before it was admitted And the catalogue's *placement* is now a gating check rather than something the scheduled tier discovers: B-300's rewrite of the journal replay moved a line one entry names, and the tier said so thirteen hours later by refusing to produce a score at all. `checks/tests/the_mutation_catalogue_still_fits.rs` reads the script's own arrays and asserts every `find` still matches exactly one line, in the tier that gates every change |
| B-193 | The storage an artifact is read from is a measurement condition; and a measurement whose cost is in another process is judged by that process's scheduling, not the measurer's | §3.4, D27, D30, B35 | The condition floor carries where the artifact was executed from, and a cold start on a slow filesystem is refused as unattributable rather than reported as over its ceiling | **done** — `artifact_storage` is the floor's eleventh question (`mcf_core::hardware::storage`), and `Attributability::Storage` is the second signal: a measurement whose work took a major page fault went to a device and is refused rather than asserted. `scripts/check-fault-signal.sh` runs in the gating tier and shows the signal moving — zero faults warm, thirty evicted, over the same thirty spawns ([findings.md](findings.md) F7) |
| B-320 | Fully-vendored stack: engines, kernels and math libraries shipped and pinned; every result renders the shipped stack's versions among its conditions; an engine MCF cannot vendor yields a classified outcome naming the reason | B64, D23, §3.12 | The from-scratch conformance run reaches a first token with no vendor runtime installed, and no figure renders without its engine | open — **reshaped by D39**: an engine no longer has to be shippable inside the artifact, it has to be one MCF can provision into an environment it controls, so the musl-toolchain wall F12 and F14 kept hitting is no longer what decides it. What ships is still only what is vendored, and MCF's own engine (D38) is the baseline that needs nothing. Measured, not chosen. [findings.md](findings.md) F12 has both halves now: llama.cpp is one project's C++ producing 12.9 MiB of static libraries against D24's 40 MiB ceiling; candle is 143 crates producing a 1.6 MiB binary that needs only the three libraries §3a allows. Both build here. **Neither builds for musl** without a C cross toolchain — the pure-Rust candidate through `tokenizers` → Oniguruma — so the test that chose the TLS provider (F9.5) does not separate these, and the from-scratch container either gains that prerequisite or ships without an engine, which is a difference in what two artifacts can *do* and therefore a §3.4 condition. Left open deliberately: this is the largest single thing MCF will ship [findings.md](findings.md) F18 removes one leg of that: the C toolchain targeting musl that F12, F14 and F15 each treated as absent is **packaged** for this machine — about nine megabytes — so the question is whether MCF is willing to require one at build time rather than whether it can be had. Nothing was installed to find that out |
| B-321 | Deferred-engine register: engines and runtimes avoided because they cannot be vendored, recorded with the reason and revisited on evidence that the performance gap changes which model a user should run | D23, §3.13, C6 | The list exists and is maintained rather than the omissions being silent | **done** — [vendored.md](vendored.md), which is also B-330's matrix: the same register seen from two sides, written before the first component is admitted so that it gates rather than describes |
| B-192 | Self-contained build: the inference engine and every common-path tool are vendored or reimplemented, statically linked, no runtime and no toolchain required | §XVI, B36 | The artifact has no dynamic dependency a stock machine lacks | **done** — `crates/mcf-cli/tests/artifact.rs` reads the binary's own `DT_NEEDED` entries rather than asking `ldd`, and refuses a stranger, a baked-in search path or an interpreter that is not the platform's own; the list and its reasoning are [vendored.md](vendored.md) §3a. The release artifact needs `libc` and `libgcc_s`. The condition holds as components arrive: an engine that dragged in a maths library the user must obtain fails here. Vendoring the engine is B-320, and the container that proves it from scratch is B-183 |
| B-183 | From-scratch conformance: a container with no toolchain, no runtime and no package manager runs the binary and reaches a first token | B36, §XVI, D29 | Asserted on every platform D29 calls characterized, and a platform that is only attempted says which capability it lacks rather than being skipped | **in progress** — `scripts/check-from-scratch.sh` behind `scripts/ci.sh --with-from-scratch`: a statically linked `x86_64-unknown-linux-musl` artifact in an image holding it and nothing else — no libc, no shell, no package manager, no `/etc`, no `/tmp` — running `--version`, `licence` and the whole of `doctor`, laboratory included. What it lacks it reports: the accelerator comes back *attempted, uncharacterized* because the vendor library is not there (D25, A5). **And it reaches a first token there**, which is the row's condition: the container has no toolchain to build a model with and no network to fetch one over, so the machine that has a toolchain writes out the laboratory's four-token fixture (`cargo run -p mcf-lab --example write-fixture`) and the image is handed the bytes. MCF's own stand-in reads it and produces text — the whole path with nothing installed. Both caveats travel with it: the fixture is not a model, and a stand-in's answer can never be a speed (B65). What a real artifact does here is B-019's, and the other platforms D29 names each need a machine to run this on |
| B-190 | Privileged helper: a separate, auditable executable performing one named operation from a short list and exiting; the daemon holds no ambient privilege | A26, §6.32, §XVII | The daemon runs unprivileged in every scenario; the helper's surface is enumerated | **done** — `crates/mcf-helper`, binary `mcf-helper`. Three operations, and they are D35's three: set every processor's governor and say what each one *was* (so a restoration is possible at all), take or release a device's exclusive compute mode, and read the processor's energy counter. It links `mcf-core` and nothing else, reads no environment — the arguments are the whole input — writes to no record, and refuses every name that is not on the list. The value written to a governor is chosen from the machine's own `scaling_available_governors` rather than passed through from an argument, because a helper that writes what it is told is a helper that writes anything. A failure part-way through is **partial** and names what was already changed, since a restoration that does not know what was altered cannot be made (A4, A27). `--under <path>` points every fixed path at a fixture, which is how three laboratory scenarios drive a privileged program without letting it near the machine (D26). `checks/tests/the_daemon_holds_no_privilege.rs` holds both halves of the row: nothing shipped reaches for elevation, nothing links the helper but the laboratory, and the surface is compared against D35 in both directions. **F50**: it also accepted `--under`, which rebased every path it touches, in the shipped binary and not only in tests — harmless while it held no privilege and a local escalation the moment it held any. The seam is a parameter now (`run_under`), reachable from the laboratory and this crate's tests and from nothing else, and the argument is refused rather than ignored |
| B-180 | Untrusted code cannot reach an elevated path, asserted by scenario rather than by policy | A26, §6.20, §6.4 | An adversarial model and hostile repository code both fail to touch a privileged operation | **done** — `crates/mcf-cli/tests/untrusted_cannot_elevate.rs`, and it is a scenario rather than a rule because a rule saying MCF does not do this is worth nothing: the question is whether a hostile input can *cause* it. A directory of sentinels goes first on `PATH` — `mcf-helper`, `sudo`, `pkexec`, `doas`, `su`, `nvidia-smi` — each recording its name and arguments to a witness and exiting. Then every surface that touches an artifact is run against a model whose metadata is a shell command, a path traversal, a format specifier and a governor name; a store whose *file names* are arguments to the privileged helper; and references that try to climb out of the store. Nothing is started, the hostile strings come back exactly as written, and a fourth test starts a sentinel deliberately so that *nothing happened* is evidence rather than a broken harness (A19). Today MCF starts none of those programs at all — a stronger claim than the row asks for, and the right one to hold while it is true; when the daemon starts the helper for a legitimate reason, this is where *never with anything an artifact could influence* is kept |
| B-220 | Environment restoration: a scenario kills MCF mid-run at every stage and asserts governors, priorities, exclusive modes and suspended processes are all restored | A27, §3.25, §6.39 | The machine is returned to how it was found from every interruption point | **in progress** — `mcf_record::restore`: a ledger written *before* the change and recovered on next open, so a killed process leaves a machine the next run puts back; two laboratory scenarios and seven tests, interrupting at each stage. The four things B-220 names — governors, priorities, exclusive modes, suspensions — do not exist to be interrupted yet (§6.39, DEC-041, DEC-042), so the item stays open until they do Load-bearing for the operator's rule of 2026-08-27 on B-181: a machine that must never be left out of the user's control is a machine whose governor cannot stay pinned because MCF was killed. |
| B-221 | Corpus-sourced values and locally-measured values are distinct types; only the second can back a recommendation | B43, B34, §6.38 | A foreign number cannot reach a recommendation, enforced by the compiler | **done** — `mcf_core::origin`: `LocallyMeasured<T>` and `FromCorpus<T>` are unrelated types with no conversion either way; a corpus value cannot be built without the sample count B44 requires, and neither reads like the other on a surface |
| B-270 | Summaries cannot be persisted, only projected from trials; every trial carries its arm, interleave position and session | B56, D16, §3.27 | A stored mean does not compile; paired analysis is possible from the record alone | **done** — `mcf_core::trial`: a `Trial` cannot be built without its arm, position and session; there is no mean anywhere in MCF to store, and a check keeps it that way; a pairing is reconstructed from a journal round trip in `mcf-record`'s own suite |
| B-271 | Interior detail is declared per laboratory and off by default; thinning is recorded as a condition | B56, D16, §3.4 | A downsampled series carries its thinning factor and cannot be read as full resolution | **done** — `mcf_core::trial::Series`: no constructor omits the thinning, no accessor returns the points without it, and factors compose so a re-thinned series cannot claim the resolution of its last step. Per-laboratory declaration arrives with the laboratories (M6) |
| B-272 | The identity type excludes hardware by construction; grouping is a query-time view | B57, D17, §XIV | The same configuration on two machines is one identity with two condition sets | **done** — `mcf_core::configuration`: six fields, none of which can hold a machine, checked by a vocabulary sweep as well as by the compiler; sampling in thousandths so identity is an exact equality; realized placement moved to the condition floor, which grows to nine |
| B-300 | Journal-and-index: trials append to a journal, the database is derived and rebuildable, crash-safe write settings enabled, and a failed replay reports the exact extent of the loss | B62, D20, §3.1 | A scenario corrupts the database at every lifecycle stage and the record rebuilds or states what it could not recover | **done** — `mcf_record::journal::index`, 32 bytes an entry: kind, moment, line, byte offset, byte length, and no body, because a copy of a body is a second place for a fact to live. It is append-only for the same reason the journal is — a torn tail is found by arithmetic and dropped — and it is never durable, deliberately: the journal pays for a barrier per entry because it *is* the record, and a rebuild reproduces the index exactly. Every way an index file can be wrong ends in a rebuild that says why, never in a failure and never in a wrong answer, and a journal shorter than the index describes throws the index away. It never covers past a loss, so B62's report is made again at every open rather than being indexed around. `mcf log` and the daemon's start read through it, and every entry a surface prints is read back out of the journal at the offset the index gave. The numbers that earned it are [findings.md](findings.md) F14: 7.89 s to replay a million entries against 72 ms to open the index and 196 µs to answer *the last twenty acquisitions*. `checks/tests/the_record_recovers_from_every_stage.rs` is the lifecycle half of the condition The property tier holds the invariant the whole item rests on: for any journal, the index agrees with a replay about what is in it — same entries, same order, and the offsets read back to the same bytes and the same identifiers. It also asserts the second open *loaded* what the first wrote, which is the assertion that makes the rest mean anything: an index that silently rebuilt itself every time would still be correct and would have thrown away the only reason it exists |
| B-302 | Export: one command, one portable file, sharing the serialization §XIV and PR2 need | D20, §XIV, [PR2](proposals.md#pr2--the-repro-bundle) | One mechanism serves export, contribution and repro bundles | **done** — `mcf export --to <path>` and `mcf_record::export`: one format, three kinds differing only in what is *selected*; entries carried verbatim so a digest does not depend on the version that wrote it; a damaged bundle is refused rather than read as a smaller one |
| B-301 | Re-verify artifact checksums before a long measurement run, not only at acquisition | §7.49, §3.6, §3.8 | Silent disk corruption is caught before it produces a garbage result rather than after | **done** — `mcf_core::integrity` streams a re-verification and names both digests when they differ; `mcf_core::digest` is SHA-256 written out and checked against the published vectors including the million-character one. Three laboratory scenarios: corrupted, missing, unreadable And it has a surface: `mcf check --here` re-reads every artifact on this machine against the digest recorded for it, with no network at all. Until then the capability existed and nothing called it, which is the shape A22 forbids — a thing MCF can do that an operator cannot ask for |
| B-042 | Record store is a single portable file, versioned from the first write, corruption-resistant and recoverable | D6, §3.3, §3.1 | The file carries a format version; a truncated write is a classified failure and the record reopens; the file is portable between machines | **done** — and not as it was written. The item said *a single SQLite database*; [findings.md](findings.md) F14 measured what that would buy and what it would cost, and D6 was amended: the record is one append-only file, and what queries it is derived from it (B-300). Every clause of the condition holds of what is there — `mcf_record::journal` writes a header naming `FORMAT_VERSION` before the first entry and refuses a file written by a version it does not read; a torn write is `record.replay.incomplete` with the line, the offset and the bytes unread, and the record reopens; and the file is line-delimited JSON, which is portable in the strongest sense available — a machine with no MCF on it can still read the evidence |
| B-161 | Content store and record store are distinct types with no path between them, so no export can carry content that was never written | A25, §6.8, §6.27 | The type system prevents writing prompt or completion content to the record store | **done** — `mcf_record::content`: two stores in two places, neither module naming the other's types, no conversion either way, and a `Debug` that reports a length rather than a body |
| B-330 | `LICENSE` in the repository, and the per-engine compatibility matrix every vendored component is checked against before it is admitted | DEC-047, D22, D23, D28 | No component ships without a recorded compatibility finding; the licence is stated in the artifact and surfaced to a redistributor | **done** — `LICENSE` is the verbatim GPL-3.0 text and a check asserts it stays so; [vendored.md](vendored.md) is the matrix and refuses a vendored component with no row. `mcf licence [--full]` is the second half: the whole text is compiled into the binary, because a redistributor has a binary rather than a repository and §4 obliges them to convey a copy. What it says about vendored components and what the register records are checked against each other The check that holds the two together was written when both lists were empty and only asserted they were empty or not-empty *together*; fourteen components later that is not the property B-330 wants. It now compares them name by name and revision by revision, because what `mcf licence` prints is what a redistributor is obliged to convey and the register is where each component's compatibility finding lives — a difference between them means one of the two is about a binary nobody is shipping |
| B-361 | A timing-class result cannot be constructed from a stand-in engine handle, in the way a simulated duration cannot become a performance number | B65, D31, A11, B31 | The compiler refuses it; a check refuses a conversion added later | **done** — `mcf_core::engine`: `timing` is defined on `Run<Vendored>` alone, `Timing` has no constructor of its own, and no conversion exists between the two runs. Four source checks and a `const` assertion; verified by giving the stand-in a timing and watching them fail |
| B-360 | MCF's own engine: readers for the formats MCF acquires, dequantization per scheme, the ordinary transformer operations written to be read, and sampling. Threads and the compiler's vectorizer are permitted and hand-written kernels are not; output is bit-identical whatever the thread count | D38, D31, §III, §3.2, B7 | **Every** model MCF can acquire reaches a first token on it, and every result taken on it is marked (A5) | **in progress** — `mcf-standin`, its own crate because B65's prohibition is a boundary as well as a type. A model runs: the GGUF reader, dequantization for five schemes, the transformer operations each written as its definition, the llama forward pass over a key/value cache, seeded sampling, and a generation loop whose result is a `Degraded<Behaviour<…>>` that cannot be unwrapped without its mark (A5). It is the fifth target in the fuzz tier and the three failures it constructs have laboratory scenarios (A13). The tokenizer is here too — the unigram vocabulary a llama-family file carries, with byte fallback, and a refusal by name for the byte-pair kind it does not implement. Text goes in and text comes out: `crates/mcf-standin/tests/a_first_token.rs`. What remains before the done-when holds is a *real* artifact rather than one a test constructs, which M1 acquires (B-019, B-020), and the cross-check against a vendored engine that would establish agreement rather than wiring (B-362) And it is exercised against weights MCF did not write: the online check acquires a 260-thousand-parameter model from a real publisher and runs it end to end, asserting text comes out with its mark on it. Everywhere else the stand-in reads a fixture the laboratory built, which is MCF checking its own arithmetic against its own file — A19 wants the other kind **It runs a real 7B model.** `Mistral-7B-Instruct-v0.3`, from the operator's own store, answers *The capital of France is* with *Paris, but the largest city is Marseille* — which took fixing the tokenizer's algorithm and three quantization decoders, neither of which any test could see wrong (F19) |
| B-362 | Cross-check laboratory: where both engines can run an artifact, compare them on a fixed input and report agreement or divergence | D31, A19, A12, §II | Disagreement between the two implementations is a recorded finding about one of them, with the tolerance stated (D19's shape) | **done** (F49) — `mcf cross-check <model>`: MCF's own engine reads what the provisioned one produced, position by position, so the comparison survives the point where two greedy generations part. What is asserted is the rank MCF gives the other engine's token, with the tolerance stated and measured rather than chosen (F41): the line is 8, four architectures and a Q2_K file sit at 1-3, and a deliberately swapped rotation reaches 3388. It refuses to say *which* engine is wrong — neither is the authority, and claiming it from two readings would be the manufactured certainty A19 forbids. Unbuildable before B-376, and it cannot reach a model MCF's own engine cannot read (B-372) |
| B-364 | Quantization coverage: every scheme the formats MCF acquires can carry, decoded against published vectors rather than against MCF's own reader | D38, §III, A19 | Every scheme a corpus file carries is decoded, and the oracle agrees through it; a scheme no file carries is refused by name | **done** (F32, F33) — Q2_K, Q3_K, Q4_0, Q4_1, Q4_K, Q5_0, Q5_1, Q5_K, Q6_K, Q8_0, IQ4_NL, IQ4_XS, IQ3_S each decoded by a path a real corpus file exercises, sixteen files compared. Q3_K was found wrong on the way and fixed. IQ2_*, IQ3_XXS, IQ1_* are refused by name. The Q2_K verdict rests on evidence rather than a verdict (F33) until B-373 |
| B-365 | Architecture coverage: the families the hub actually publishes, each as its own reading of the same operations rather than a special case bolted onto llama | D38, §III | A model of each covered family reaches a first token; one MCF does not cover is refused by name with what it declared | **done** — all six corpus families run, verified together: llama with either vocabulary (SmolLM2, Llama-160M), qwen3, gemma3 and a mixture of experts each reach a first token through MCF's own engine, and `bert` answers through `mcf embed` — its own verb, which is what DEC-055 decided a model answering a different question should have (F29). The division that makes it cheap: what the file states is read from the file (the mixture is found by `expert_count`, not by a family name) and only what a file cannot state lives in `architecture.rs` (F24, DEC-053). The other half of the criterion holds too — an uncovered family is refused by name with what it declared, what is implemented, and that the list grows by somebody reading an architecture rather than by MCF guessing one shaped like another will do |
| B-370 | The conformance corpus: smallest trained model per family, one quantization each, acquired by MCF with provenance, run by a scheduled tier | D40, DEC-054, §3.12, §XII | Every family MCF claims runs a corpus model; every family it does not claim is refused by name saying what it wanted; the tier fails when an entry stops being true in either direction | **done** — six acquired (1.4 GB against 16.5 GB), `scripts/check-corpus.sh` and `ci.sh --with-corpus`. Two run, four refuse and each names a different missing thing. All five failure modes were verified to fire: a runner declared to refuse, a refuser declared to run, a refusal that stops naming what it wanted, a runner that says something else, and no corpus at all. What remains is the oracle (B-368), which is exact where this is a judgement — F22 measured the judgement's floor at between 160M and 0.6B |
| B-371 | Embedding models: the bert family, its WordPiece vocabulary, and the surface that asks one for a vector | DEC-055, D38, §VI | An embedding model in the corpus produces a vector of the width its file declares; the tokenizer agrees with the reference exactly; the vector agrees at a measured floor and means what related-against-unrelated sentences say it means | **done** (F29) — tokenizer identifier-exact on five texts including `[UNK]` and accents; forward pass at cosine 0.9996–0.9998 against the reference where the floor is 0.999 and a single swapped normalization falls to 0.97; corpus six for six. Not covered, and said: nomic-bert's rotary positions and gated feed-forward, the first-position pooling no artifact exercises, and the floor is calibrated on this model at this size |
| B-372 | The reference model does not fit the stand-in's memory design, and the engine says so before trying: 27,320,697,856 parameters dequantized to f32 is 109.3 GB against what `/proc/meminfo` reports free | D38, §XII, A7, B7 | `mcf run` against an artifact whose dequantized weight exceeds free memory refuses up front, naming both numbers, rather than being killed by the kernel mid-load; an engine change that lifts the ceiling is measured against F29's oracle before it ships | **in progress** — the refusal half is done: `Model::fits_dequantized` is arithmetic on the directory against a number the surface observed, read from a bounded prefix (16 MiB, then 256, then the file) so the reference model's refusal fell from 72 s to 0.16 s, and the laboratory produces the failure (A13). The lifted ceiling is **argued in [PR12](proposals.md#pr12--lifting-the-stand-ins-ceiling-and-why-the-cheap-version-is-worse) and not accepted**: dequantizing per use lifts the memory ceiling by spending time the engine does not have — 0.9 s a forward pass on a 0.6B model is roughly forty on the reference model, so eighty minutes for the cross-check that is the point of lifting it. The memory ceiling and the speed are one problem; PR12 recommends B-366 first, then measuring how large a model MCF's own engine can usefully read |
| B-373 | The oracle compares logits, not texts: for the same model and prompt, MCF's next-token distribution against the reference's, so that a defect is a different vector and noise is the same vector to within arithmetic | B-368, A19, F33 | A generation divergence is judged by the distance between the two distributions at the parting step rather than by MCF's own margin; the threshold does not narrow as files are added | **done** (F34) — `llama-server` provisioned as the reference's one distribution-exposing tool; MCF's `margins --logprobs-of` prints its log-softmax for the reference's top twenty at the same step, compared only through identical token ids. KL over the top twenty, floor 0.20: the clean engine's maximum is 0.113 and a swapped rotation's median 0.32, and the tier judges every prompt of every file. Stated limit: a subtle defect is invisible at positions where the model does not care, on any statistic |
| B-366 | Threads without changing the answer: work split across processors, partitioned by index and reduced in a fixed order, so a result does not depend on how busy the machine was | D38, §3.12, D19 | The same input produces the same bytes at one thread and at many, asserted as a property over generated inputs | open — floating-point addition is not associative, which makes this the one place reproducibility can be lost without anybody noticing **F52 bears on this directly and cautions against reasoning about it:** thread count moved benchmark noise by a factor of five on the provisioned engine, in the direction opposite to the one predicted. Giving MCF's own engine threads will change its noise characteristics as well as its speed, and F52 is the evidence that the change must be measured rather than predicted |
| B-367 | Provisioned environments: MCF installs, builds and pins a component itself, records exactly what it got, can do it again, and can remove it without residue | D39, §3.4, A27, §6.32, DEC-052 | A provisioned engine is reproducible from its record; removing it leaves the machine as it was found; nothing outside the environment MCF made is touched | **done** (F31) — `mcf provision <component>`, `--list`, `--remove --because`. One component, the reference implementation: image by digest, source by commit, packages recorded exactly, self-contained build, `mcf-provenance.json` in the prefix and `component_provisioned` in the record; the oracle tier finds it on its own and agrees 57 of 57 through it. Four runs to get there, each finding something the manual run had not: podman's store must not follow MCF's data home, git's ownership guard inside a mount, a shared build's `RUNPATH` from the wrong side of the boundary, and that a success can still not run. Removal exercised once for real, reasoned and recorded. Not covered: a second component, GPU device access, and provisioning categories of its own — one category carried every failure with distinct details, which is fine for one component and worth revisiting at two |
| B-368 | A reference implementation as an oracle: where MCF's engine and a provisioned reference disagree about the same model, same input and same seed, the disagreement is the finding | D39, D38, A19, §3.12 | Two implementations are compared on the same artifact and the same text; a disagreement is reported with where it first appears and whether it is explainable | **done** for what it can reach — `scripts/check-oracle.sh` and `ci.sh --with-oracle`. Thirty tokenizer comparisons, exact; fifteen greedy generations, compared against a **measured** tolerance rather than an assumed one (F27): a divergence fails only where every step still had a margin over 0.50, which is three times the largest observed noise and two-thirds of the one observed defect. Both defects it found are recorded (F26, F27) and reintroducing either makes it fire. What remains is *breadth*: more prompts to narrow a threshold that four observations rest on. The sliding-window mask is now exercised past the boundary (F28), gated behind `MCF_ORACLE_LONG=1` because the stand-in pays a forward pass per prompt token |
| B-369 | Several model stores, chosen at acquisition time: where the large files go is the operator's choice, independent of where MCF is installed, and every surface looks in all of them | §3.15, §5, A7, A6 | Models land where the operator says; a listing covers every store; a name held in two is refused rather than resolved | **done** — `MCF_MODELS`, an ordered list of absolute paths in the platform's own idiom, because §5 refuses MCF a configuration language and a list in a variable is not one. The first is where a new acquisition goes and all of them are searched for what is held; `mcf pull --into <directory>` overrides for one acquisition and may name a store MCF was never told about, since naming a path *is* choosing one. A relative path is dropped and **named** rather than resolved against whatever directory MCF was started in. A name held in two stores is refused with both paths, because two files under one name are two artifacts with two provenances and a measurement against whichever MCF reached first is one nobody could reproduce. `mcf doctor` shows the stores in order, which one new models go to, and how much room each has — asking the filesystem that *would* hold a store that does not exist yet, since that is the number somebody deciding where to put sixteen gigabytes actually wants |
| B-017 | Decision record (ADR) format and index, so §7 resolutions and their reasoning survive the code that implements them | §8 | A resolved void points at an ADR and the ADR points back at §7 | **dropped** — the thing already exists under another name. §2.1 holds each resolution, the intent document's changelog holds the reasoning that produced it, and §7's retired-void index is the pointer back. An ADR set would be a second home for statements that have one, and duplication is a defect ([README.md](../README.md)); the item's own condition is already met by documents that exist |

### M1 — Acquire

| ID | Title | Cites | Done when | Status |
|---|---|---|---|---|
| B-020 | Hub reference resolution: accept any Hugging Face reference without special-casing, and reach a defined, actionable outcome for every one | §III, §6.3 | No input produces a hang, an unclassified crash or corrupted local state — asserted by the lab's hostile-hub scenarios | **done** — `mcf_hub::reference`: every way a reference is written — typed, pasted, from a browser's blob or resolve URL — and a named refusal for every string that is not one, including the traversals and schemes §3.7 exists to stop. Total, offline and deterministic; a fuzz target, and driven through the laboratory's simulated hub and its hub-on-a-socket by the scenarios A13 requires |
| B-021 | Resumable, integrity-checked fetch: checksums verified, partial transfers resumed, mutation-under-us detected | §III, §3.7 | A transfer interrupted at 90% resumes and verifies; a file that changed mid-fetch is a classified failure, not a corrupt local artifact | **done** — `mcf_hub::fetch` over `mcf_hub::client` and `mcf_hub::wire`. Bytes accumulate under a `.partial` name and the artifact's own name is given only to something verified, so a crash at any moment leaves a state MCF can see, resume and name. A transfer interrupted at ninety per cent continues from there; a source that cannot resume is restarted and *said*; a file that changed under the transfer is `artifact.corrupt` and the mixture is deleted. A source that answers a resumption from the wrong offset is refused *before* a byte is appended, and a scenario asserts the partial file is untouched afterwards. A hub declaring no digest leaves the artifact *held* rather than verified (A21). The transport reaches an http hub today and an encrypted one when B-322 admits a stack |
| B-022 | Untrusted-input validation of every fetched byte: malformed configs, deceptive metadata, enormous files, path traversal in archives | §3.7 | The lab's hostile-hub fixtures are all rejected with a classified outcome and no state damage | **in progress** — `mcf_hub::inspect`: what a repository *claims* against what is true. A card declaring an architecture the weights are not is caught by reading the weights (A21's divergence, and only possible because D31 gave MCF a second reader); a transfer shorter than its listing is partial and one longer is the repository lying about a number MCF plans with; a repository declaring no terms is a state to report rather than one to fill in. Three scenarios drive the whole path through the simulated hub (A13). Archives are not read yet, so path traversal in one is not: that arrives with the formats that need it |
| B-023 | License legibility: the license is surfaced before use, and a use it forbids is stated rather than discovered | §III, §3.7 | Every acquired artifact reports its license, or reports it as `Unknown` — never as a plausible default | **in progress** — `mcf_hub::licence`: the three states are distinct and none of them is a default. An identifier MCF recognizes, kept as the repository wrote it; terms that are present and unmatched, which is *not* a failure and *not* the same as absent; and nothing declared, which is `hub.metadata.absent`. `inspect::terms_are_legible` now returns the state rather than a string. Each recognized identifier carries the family its own name puts it in — permissive, copyleft, non-commercial, bespoke — and MCF says nothing further: whether a particular use is allowed is a legal judgement about a specific person, and a tool that guessed would be worse than one that stays quiet. The surfaces print it, and all of them the same way: `mcf pull` before a byte is fetched and after it arrives, `mcf list` beside every model this machine holds, and `mcf explain` beside what a model says about itself — one sentence written once in `mcf_hub::licence::describe`, because a surface that renders terms differently from another is two answers to one question (A6). An artifact nothing accounts for says its terms are **unknown** rather than leaving the line out: an absent line reads as *no restrictions*, which is the one thing MCF must not imply about somebody else's model. What remains is the *forbidden use* half of the row, which for the one case MCF could face — publishing measurements about a model — is DEC-036 and open |
| B-024 | Gated and authenticated repositories: credentials are the user's, held deliberately, never a silent prerequisite | §III, §3.10 | A gated model produces an actionable outcome naming exactly what is missing | **done** — `mcf_hub::credentials` and the surface that uses it. A secret redacts itself, an origin travels with it into the record, and an `Identity` says what MCF is to a hub — anonymous, offered, or an account it confirmed. The three refusals are written once so a real hub and a simulated one say the same sentence, each naming the repository, what MCF was, and the one thing to do next. *Held deliberately* is structural: nothing reads the environment, and `checks/tests/a_credential_is_never_picked_up.rs` holds that across the workspace. At the surface, `mcf pull --token-from <file>` and `--token-from-env <VARIABLE>` are the only ways in; a repository that needs one is told which, and MCF reports what it has *looked at and not used* rather than spending it. A credential is refused rather than downgraded over a connection that cannot keep it |
| B-025 | Repository-code execution is possible but never implicit: per artifact, with the risk stated, the choice recorded in provenance, and contained so hostile code cannot corrupt MCF's records or state | §6.4, §3.7 | The lab runs deliberately hostile repository code and MCF's records and state are provably intact afterwards | **in progress** — MCF today executes *nothing* it acquires, which is stronger than what this item will eventually claim, and `checks/tests/nothing_acquired_is_ever_run.rs` holds it while it is true: every place shipped code starts a process is declared with what it starts and why that is not an artifact — MCF starting itself to time its own start, the compiler at build time, and `dlopen` of the vendor's management library, whose two candidate names are a constant a check pins. The other half is what a model file's *contents* are: a test reads a model whose metadata is a shell command, a path traversal and a format specifier, and asserts each comes back exactly as written — neither run, nor resolved, nor interpolated. What remains is containment for an engine that *can* run a repository's own code, which arrives with the engine (B-320) |
| B-026 | Disk arbitration on acquisition: a download that would exhaust the disk is a decision, not a surprise | §3.11, §7.9 | The disk-exhaustion scenario ends with a classified refusal and no partial garbage | **done** — both halves, because one alone would be a promise. Before a byte moves, the hub's declared size is checked against what the kernel says is available and a file that will not fit is refused with the arithmetic — needs, available, short by, and which filesystem — rather than a verdict. Where MCF cannot read the room it proceeds, because `Unknown` is not `no` (A7). And a filesystem that fills anyway is classified rather than reported as a general write failure: [findings.md](findings.md) F11 found that a buffered write to a full disk *succeeds* and the flush is where it surfaces, so a fetcher that checked one and not the other would verify a digest over bytes that never landed. `hub/no-room-on-the-disk` holds it against `/dev/full`. Nothing whole-looking is ever left behind — the artifact's name is only ever given to something verified (B-021) |
| B-027 | Eviction and deletion: previewed, logged, reversible where reasonable, never automatic to reclaim space | §3.11 | No code path deletes an artifact without an explicit, recorded authorization | **done** — `mcf_hub::store`: four acts, each a type. `preview` says what would go, what it weighs and whether it could be undone — read from the device the kernel reports, not assumed. `Authorization::given` is somebody deciding, about that list of files at those sizes, for a stated reason. `remove` writes the record *first* and then **moves** the artifact to a shelf, deleting nothing. `purge` is the only function in MCF that destroys an artifact and it takes the authorization to do it. An authorization that no longer matches is refused with every difference named. `checks/tests/nothing_deletes_an_artifact.rs` holds the condition across the workspace: every deletion in shipped code is declared with what it destroys and why that is not an artifact. The surface an operator drives it from is `mcf rm`, which is B-029 |
| B-028 | Fake hub: a complete, deterministic simulated Hugging Face — well-formed, malformed, gated, hostile, truncated, mutating | §3.17, §7.21 | Every M1 test runs against it with no network | **done** — two of them, because MCF gained a socket. `mcf_lab::hub` answers the four questions `Source` asks, with a declared behaviour per repository — well-formed, needs credentials, refuses credentials, gated, throttled, truncating, stopping, never resuming, serving different bytes, deceptive, hostile. `mcf_lab::serving` is the same discipline one layer down: a hub on the loopback address answering from a script, because code that talks to an operating system cannot be tested against a value. Both simulate what MCF observes and never the cause (D26). Every M1 test runs against one of them with no network, including the acquisition path end to end |
| B-322 | The transport: a TLS stack vendored and pinned, and an HTTP/1.1 client MCF writes — redirects followed without carrying a credential across hosts, ranges resumed, the declared digest and revision read from the response | §III, §XVI, B15, B36, [findings.md](findings.md) F9 | `mcf pull` reaches the real hub over TLS; the artifact still demands nothing of a machine beyond `libc`, `libgcc_s` and the loader; the redirect and resume behaviours are driven by the laboratory's hub rather than by the network | **done** — the protocol is MCF's own (`mcf_hub::http`, `wire`, `client`) and the cryptography is vendored: `rustls` with the `graviola` provider, fourteen crates compiled, eighteen present-and-stubbed, 18 MiB. The provider was chosen by measurement rather than by default — the usual one is C and cannot build for the musl target B-183's container uses, which would have made an existing check runnable in fewer places (F9.5) — and it held a real TLS 1.3 session with the hub before it was admitted (F9.6). The from-scratch check still passes: the static binary runs with no libc, no shell and no `/etc`. `scripts/vendor.sh` builds the tree and checks both targets against it; `scripts/check-vendored-terms.sh` gates on what every crate declares; [vendored.md](vendored.md) §2 records what MCF verified, including the three crates that declare terms and ship no copy (A21) |
| B-019 | Acquire and pin the reference model as M1's first real artifact — the third-party requantization chain (`unsloth/Qwen3.8-27B-GGUF` → `Qwen/Qwen3.8-27B`) is the hard provenance case, not the easy one | §XII, §3.6 | The derivative traces to its source weights through the publisher's pipeline, with every field either recorded or `Unknown`; the revision is pinned at acquisition | **done** — the artifact is here. `Qwen3.8-27B-UD-Q4_K_M.gguf`, 16,464,440,224 bytes, acquired by MCF from the real hub onto the content drive, verified against the digest the hub declared, and pinned at revision `4ca7207`. Its provenance records §XII's hard case as the hard case: the chain traces to `Qwen/Qwen3.8-27B`, which MCF **has not fetched and says it cannot vouch for**, with the publisher's own word for what was done to it. The variant is the operator's choice; the planner's figures were what it was chosen against. The chain is built and traverses. A publisher names the weights they worked from in the hub's own tags, and `mcf pull` writes that down: the artifact, what the publisher says was done to it in their word, and the upstream repository as a link MCF has *not* fetched. Writing it found a defect in the type — `Provenance::retrieved_at` was a plain `Timestamp`, so an upstream link could only claim a moment MCF was never there for. It is `Attested` now, `known_of` is the constructor for a link nobody fetched, and the check that exempted the field no longer does. What remains is the reference model itself, which needs the real hub and therefore B-322 |
| B-213 | Pre-acquisition fitment across every variant a repository publishes: weights plus KV cache at the requested context against available memory, computed from metadata before a byte is fetched | [PR3](proposals.md#pr3--pre-acquisition-planning), §III, §6.3 | Twenty quantizations are classified fits / fits-without-context-headroom / does-not-fit without downloading any of them; the plan is re-checked against reality on acquisition and divergence is a finding | **done** — the plan is made from the model's own `config.json` and the hub's declared sizes, in three cheap questions and no weights: every variant classified fits / fits-at-a-shorter-context / does-not-fit, at a stated context, with what is left over or what is short. The shape is read whole or not at all, because the grouping factor is exactly what a guess gets wrong. And it is asked again once the model is here — the plan was made from what a hub declared and what is true now is a different question — with the answer beside the acquisition rather than left to be assumed Pointing it at the real reference repository for the first time found three defects in it ([findings.md](findings.md) F16): a configuration MCF's own JSON reader refused over `1e-06`, a multimodal configuration whose transformer fields are under `text_config`, and a hybrid attention scheme where counting every block rather than the sixteen that cache overstated the KV cache fourfold. All three are fixed with tests, and all 33 variants now classify without a byte being fetched |
| B-331 | Upstream decay: detect that a pinned artifact has been withdrawn, gated, relicensed or repointed, and record it against the provenance without invalidating the local copy | DEC-038, §7.38, §3.6 | A decayed pin is detected and recorded; the local artifact and its measurements are untouched | **done** — `mcf check [<model>] [--here]`, `mcf_hub::decay::look`, and `Provenance::observed`. It fetches no weights — a listing and a model card — and it runs when somebody runs it, because D37 forbids the timer a watcher would need. Each of §7.38's four decays has a finding and a test: a withdrawn revision, a gate that closed, a relicensing, a file replaced under its own name. A hub that will not answer is `unreachable` and is **not counted as a change**, and is not counted as *checked* either — a run of unanswered questions used to read as a clean bill of health. The two ways to get no answer are told apart where a person reads them: a hub that *refused* is F17's ambiguity, and a hub that could not be *reached* is a fact about a network, which does not get to borrow the sentence about privacy and withdrawal, because private, withdrawn and never-existed are one answer ([findings.md](findings.md) F17) and reporting an absence as an event is what A7 forbids. A check writes down what it found *whatever* it found, including that nothing was wrong: a verification that left no account could not answer *when was this last known to be fine*, and `--here` used to record nothing at all. The record entry carries both halves separately — the bytes as `matched`, `changed` or *no digest was recorded*, and the upstream as its finding or `null` when nobody asked — because *checked and fine* and *not checked* are different facts (A7). Findings are written twice and neither is a correction: appended to the provenance beside the artifact, where what was true at acquisition still stands, and into the record as `artifact_checked` — written whether or not anything changed, so *when was this last known to be fine* has an answer (A1). Nothing is invalidated by a finding, and the surface says so: the artifact is here and its digest is what it was (D37) |
| B-363 | The record is readable at the surface: what happened on this machine, read back out of the journal, with anything unreadable named rather than skipped | §3.3, A22, B62 | Every kind of entry has a rendering; a torn record is reported and what came before it is still read | **done** — `mcf log [--kind <kind>] [--last <n>] [--full]`. The record has been written to since M0 and the only way to read it was to open the file: §3.3 ranks machine-readability first and legibility *second*, which is not the same as omitted. Each kind gets its own sentence with the field a reader wants first — what was acquired, which category failed, why the daemon stopped — and `--full` gives the record's own JSON for anything that is not a person. A replay is not a `cat`: a torn last line is reported at the end, and everything before it is still shown (B62). An entry from a newer build is shown as what it is rather than hidden, because a log that skipped what it did not understand would lie by omission (§7.30) |
| B-029 | `mcf pull` / `mcf list` / `mcf rm`: the M1 product — models enter, live on and leave this machine with provenance intact | §III | A model is acquired, listed with full provenance, and removed deliberately, offline against the fake hub and online against the real one | **done** — both halves. Offline: a whole-system test acquires a model from a hub on the loopback address, lists it with where it came from, and removes it, each as a separate process against a real record. Online: `scripts/check-online.sh` does the same against Hugging Face over TLS — a real 1.2 MiB GGUF, verified against the LFS digest the hub declared with the listing, followed through the redirect to the CDN, listed with its provenance, purged, and both events read back out of the record. `pull` without a file offers what the repository publishes with the sizes and the terms and acquires nothing; with one it says plainly when a hub declared no digest to check against (A21) |

### M2 — Serve

| ID | Title | Cites | Done when | Status |
|---|---|---|---|---|
| B-030 | Daemon: long-lived, restartable, recovers its state across restarts, survives indefinitely | §7.1, §I | The lab kills the daemon at every lifecycle stage and it recovers to a coherent, queryable state each time | **done** — its condition is met at every stage the daemon has, mid-generation with a model resident included (F36); what follows is the same daemon growing, not this item. `mcf_serve::daemon`, with `mcf serve`, `mcf status` and `mcf stop` at the surface. It keeps no state a crash could lose: what it knows on start is what the record and the model store say, both read fresh, so *recovering across a restart* is a property of the disk rather than of a memory — and a test asserts that what one daemon leaves the next recovers. A damaged record is recovered **and said** (B62). Starting and stopping are recorded as events rather than on a timer, so *MCF was up between these two moments* is answerable (§3.4) without costing anything while idle (B-031). A second daemon on one socket is refused; a socket left by a dead process is taken over. Nothing a client says can stop it (A3). A daemon killed with `SIGKILL` at eight moments of its life — before the socket exists, during the append that says it started, idle in `accept`, and mid-answer — comes back every time: the next daemon takes over the socket the dead one left, the record opens whole or names a bounded loss inside the file, and `mcf status` answers with what it recovered. What remains is what a daemon *holds* — a runtime to drain, an accelerator to release — which arrives with B-032 and the engine. Serving added a stage that did not exist before: the daemon killed mid-generation with a model resident — the client keeps the tokens it received, says the account never came (A4, A7) and exits unserved; the next daemon takes the socket and answers; the record opens. The whole-system tier drives it |
| B-031 | Idle discipline: no polling loops, no background timers, no always-on watchers; idle cost indistinguishable from zero | §3.13, §6.9 | Measured idle CPU and wakeups meet DEC-016's budget, asserted in CI | **done** — measured rather than designed-for. The soak tier runs a real daemon for the minute D24 names, with nobody talking to it, and reads what the kernel keeps: **zero context switches and zero clock ticks of processor time**, with the record byte-for-byte unchanged. The shape is what makes it true — the daemon blocks in `accept` and has no tick, no poll and no watcher — and the tier is what makes it a claim rather than an intention |
| B-032 | Engine adapter layer: inference engines are supervised subprocesses, and which engine is in use is a recorded condition | §7.4, §6.2 | At least one engine is driven end to end; swapping engines changes a recorded condition, not a code path | **done** (F36) — the provisioned llama.cpp driven as a subprocess per generation through `mcf_serve::adapters::supervise`, chosen by a stated rule (asked for; else the one provisioned; else MCF's own), named in every account, unmarked because it is real. The reference model produced text through it. What remains is a further increment, not a condition: a long-lived server child that holds a model between requests |
| B-033 | Supervision contract: a runtime that dies mid-token is a classified, attributed failure that does not take the manager down | §3.1, §7.1 | The lab kills a runtime at every stage — pre-load, mid-load, mid-token, post-token — and the daemon stays coherent | **done** (F36) — the stages a process can die in each have a category and a scenario: not found, refused, dead before a byte, dead after part of an answer, killed. The whole-system tier runs an engine that dies mid-answer through the daemon: the partial answer is kept, the engine's own last words are attached, the client exits unserved, and `mcf status` answers afterwards |
| B-034 | Serving API: models addressed by name, stable surface, first token without the user knowing about runtimes, formats or flags | §VI, §3.15, DEC-001 | A first token is obtained from a named model in one command, on a machine that has never served before | **done** — PR9's shape on the socket the daemon already has: `{"ask":"generate",…}` in, one line per token out, a terminating `done` carrying the count, why it stopped, the conditions (model, bytes, engine and build, sampler, seed, limit, how it was loaded) and the mark. `mcf run` is a client of it when a daemon is listening and runs in-process when none is, and says which in its account. The daemon writes the terminating line to the record as `generated` (D20). A model that cannot be found or run is a `done` with a `failure`, never a hang; a stream that ends without its account says so and keeps the tokens it got (A4). Whole-system tier drives it by name on a machine that has never served |
| B-035 | Added-latency budget: the overhead MCF interposes between an inbound request and the engine's first token is measured and asserted | §VII, §3.13 | The interposed latency is measured under stated conditions and defended in CI | **done** — both halves of D24's figure, in the exclusive window, release profile, a hundred trials each: the control plane's round trip at **13.3 µs p99** (median 6.1 µs), and request to the engine's first token through the daemon at **120.8 µs p99** (median 46.1 µs) against the 5 ms ceiling. The second is measured on the laboratory's one-block fixture on purpose: there the engine's own share is microseconds and the figure is MCF's — accept, parse, resolve, load per request (DEC-018's price, included and named), tokenize, one forward pass, write back. On a real model the same path is the engine's. `mcf_serve::cost::to_first_token` names what it excludes and the tier prints it; the baseline records, the ceiling judges (B20). Not attributable on this platform, as the cold start is not |
| B-036 | Local-only by default: the control plane binds locally; network exposure is an explicit, informed, revocable act, never a side effect | §6.12, §3.10 | Default configuration is unreachable from another host; exposure requires an explicit authorization that is recorded | **in progress** — local by construction rather than by configuration: the control plane is a Unix socket under `$XDG_RUNTIME_DIR`, and there is no bind address, no port and no flag, so exposure is not something a mistake can do because it is not something MCF can do. What remains is the *deliberate* half — what exposing it would take, and what records it — which needs §XI's remote surface and DEC-017 |
| B-037 | Model residency policy: what stays loaded when nobody is looking, recorded as a measurement condition | §7.18, §3.4 | Residency state is part of every serving latency result | blocked (DEC-018) |
| B-038 | Visible defaults: quantization, context length, runtime and placement are chosen without prompting, and every choice is visible with its source | §3.15, §6.14 | Every default is enumerable with its origin; changing one is recorded | **in progress** — `mcf explain <model>` shows three columns and says which each line is: what the *file declares* (A21 — read, never believed), what *MCF read from the bytes* (the size, the digest, the tensor types, the provenance beside it), and what *MCF would choose* (engine, sampler, seed, budget, planning context, placement) with where each is written down so a reader can go and disagree. It ends with the questions MCF has no basis to answer — which quantization, how fast, what it is good at — each with the reason and the milestone that earns it, because a defaults screen listing only what MCF chose would imply a basis for choosing (§6.5, C7). What remains is the half that needs an engine: quantization and placement are not chosen because there is nothing to choose between, and *changing* a default is not yet a thing that can be done or recorded |
| B-039 | Authorization gates by category, not frequency: untrusted execution, large irrecoverable resource use, network exposure and destruction are asked every time; everything else flows | §6.14 | The four gated categories are enumerable in code and each has a test asserting it prompts | **done** — `mcf_core::authorization`: the four, enumerable, each saying where MCF asks or that no path exists to ask about yet. Two are commands — `mcf rm` will not destroy without a stated reason and deletes nothing without `--purge`; `mcf pull` acquires only what was named, and a repository asked for without a file is answered rather than fetched. Two are absences, and those are the ones worth checking: MCF runs nothing it acquires, and listens on nothing another machine could reach. `checks/tests/the_four_gates.rs` holds all four against the tree — including the absences, so a `TcpListener` added for a convenience fails there rather than in review, and every loopback listener the laboratory needs is declared with what it is for |
| B-210 | `mcf stop`: refuse new work, interrupt a lab preserving its partial result, drain and terminate runtimes on a stated deadline, release every held resource including privileged state, record what was stopped, and report what could not be released | [PR6](proposals.md#pr6--the-stop-control), §3.1, A26, A22 | A held accelerator, locked pages and a changed governor are all released; anything that could not be is named rather than claimed | **in progress** — the asking and the account. `mcf stop --because <why>` reaches the daemon, the daemon answers, and the reason goes into the record as `daemon_stopped` rather than being lost with the process (A26): a stop is now a thing that leaves a trace, which a signal never is. Everything the item is really about — draining work, releasing an accelerator, restoring a governor — waits for MCF to hold any of those, which is B-032 and the engine |
| B-332 | Record write ownership: a single writer, a defined outcome for a write that loses, and no silent drop | DEC-037, §7.37, §3.1 | Concurrent writers are exercised by the lab; a losing write is classified, never discarded | **done** — and *not* a single writer: D34 settled that nothing arbitrates, because F13 measured that nothing needs to. What the item was really protecting is met in full. `mcf_record::journal::Writer` gives every handle a token no other handle has, and only a journal can mint an identifier — an entry that has not been appended does not have one, which is the condition expressed as a type rather than as a rule. The load tier now asserts uniqueness across four concurrent writers and two thousand entries as well as the absence of a torn line. A write that loses is `record.unwritable` returned to its caller; nothing retries silently, and the one place that cannot propagate — the daemon recording its own start — reports the failure to the operator and carries on (A4). `let _ = journal.append(..)` does not compile under the workspace's lints, so *no silent drop* is a property of the build rather than of review |
| B-040 | `mcf serve` / `mcf run`: the M2 product — having a model and using a model are one command apart | §III | A model is acquired, listed with full provenance, and removed deliberately, offline against the fake hub and online against the real one | **in progress** — `mcf run <model> --prompt <text>` drives MCF's own engine end to end: model file, vocabulary, forward pass, sampler, tokens, text. It is the behaviour half and says so — the answer arrives with its mark, its sampler, its seed and a sentence saying it can never be a speed (B65, D31, A5). The conditions are printed beside the answer rather than under it (§3.4). What remains is the half that needs an engine: a *timed* answer, and `serve` handing a model to a client rather than a command loading one per run |

### M3 — Configure by measurement

| ID | Title | Cites | Done when | Status |
|---|---|---|---|---|
| B-050 | Three-state capability model: *declared*, *verified*, *unknown* — never confused, never defaulted | §3.18, §3.6 | The type system prevents a declared capability being read as a verified one | **done** — `mcf_core::capability::Capability<T>`: two separately optional halves, read as `declaration()` and `observation()`, with no operation between them and no `unwrap_or` to give an absence a value. The state is derived — unknown, declared, verified — and the fourth situation, *diverged*, is B-058's finding rather than a fourth kind of knowing. `is_established` is the whole of §3.18 in one method: only an observation may be acted on. `mcf_hub::inspect` now uses it instead of an enum of its own, which found a real loss — a card whose weights MCF could not read was being reported as *unknown*, discarding what the card said (A1). `checks/tests/a_declaration_is_not_an_observation.rs` holds the shape, because the failure mode is not somebody writing the wrong method but somebody adding a convenience that reads well and collapses the two |
| B-051 | Probe framework: capability probes are bounded experiments carrying a method, a result, conditions and a record | §3.18, §3.4 | A probe's output is a `Measurement`, not a boolean | **done** — `mcf_core::probe`: a `Method` that says what was asked and what it decides, an `Outcome` whose only reader is `observed() -> Option`, a cost in tokens, and conditions the result holds under. `Inconclusive` carries its reason and has no path to a value. `mcf probe` is the surface (F37) |
| B-052 | Probe: chat template correctness | §X, §3.18 | A model with a wrong or missing template is detected by observation, not by reading a config field | **done** (F38) — the probe asks a set of short questions through every addressing the model's own vocabulary can express and counts which ones it *answers under and then ends its turn*. It found four defects on the way, the worst of which inverted its own answer: a model refuses an unrecognised addressing by emitting its end-of-turn token immediately, which *did it stop* cannot tell from a finished reply. On SmolLM2 it now agrees with the file; on gemma-3-270m it reports a tie as inconclusive |
| B-053 | Probe: tool-calling format and reliability | §X, §IX | A model that emits a well-formed tool call is distinguished from one whose metadata merely claims support | blocked (DEC-024) | open — a *characterizing* probe under D42: it describes a model without changing how MCF addresses it, so it belongs beside §XIII rather than in M3, and uses B-051’s framework |
| B-054 | Probe: structured output conformance | §X | Verified by parsing what the model actually emits over repeated trials | blocked (DEC-024) | open — a characterizing probe under D42; framework in M3, probe beside §XIII |
| B-055 | Probe: context length usable versus claimed | §X, §3.18 | Divergence between claimed and usable is reported as a finding | **done** (F42) — `mcf probe` asks for a prompt of the declared length and, only if refused, halves to find the boundary; what is compared is integers, how many identifiers were sent against how many were read, so silent truncation is caught as well as an honest refusal. Both corpus models' claims hold (8192 and 32768). The divergence branch was exercised by giving the engine a 2048 context against a file declaring 8192: found at 2047 in fourteen trials. Reachable only through B-376. What it does *not* ask is whether an accepted context is an attended one |
| B-056 | Probe: stop-condition behaviour | §X | A model that will not stop is a recorded characteristic, not a hung request | **done** (F46) — the probe doubles the budget from 32 until the turn ends or a ceiling is reached, so *this model does not stop* is told apart from *the budget was too small*; reaching the ceiling is reported as **not within this many tokens**, with the number and the likelier cause. It is a configuring probe: MCF allowed 32 tokens and SmolLM2's turns run to 313, so every answer was being cut off by MCF rather than finished by the model (§3.8). The longest turn observed is what `--apply` writes — not an average, which truncates half the answers, and not a margin, which MCF would be inventing |
| B-057 | Probes: vision, embeddings, reasoning modes, multilingual — scoped and prioritized by DEC-024 rather than assumed | §X, §7.24 | Each in-scope modality has a probe; each out-of-scope one is recorded as declined | blocked (DEC-024) | open — characterizing probes under D42; each in-scope modality still records what it declines. **Multilingual is partly answerable now, and cheaply** (operator's answer, 2026-08-27): what a language *costs* this model's vocabulary needs no generation, no judgement and no rater — it is the tokenizer, in milliseconds. Measured on the corpus with the same sentence in six languages: all three models spend 16 tokens on the English, and on the Japanese SmolLM2 spends 52 against gemma3's 20. That is 2.6x the cost, 2.6x the context consumed and 2.6x the time, stated as the concrete thing it is rather than as *grasp of language*. `crates/mcf-standin/examples/lang.rs` is the instrument |
| B-058 | Divergence reporting: declared-versus-verified disagreement is surfaced as a first-class finding, often the most useful thing MCF can say about a model | §3.18 | Divergences are listed per model and exportable | **done** (F38, F42, F44) — three kinds of divergence are reported, each with the evidence behind it and the conditions it was seen under: what the file declares against what the model does (the chat template), what the file claims against what the engine takes (the usable context), and what was applied against what MCF would answer now (D43). The third is split in two on purpose — a moved *condition* is a fact needing no trials and is never reported as a disagreement, because F39 measured two engines agreeing on the same question; only a re-probe says the answer changed. What remains for *exportable* is a surface that lists them across models rather than one model at a time |
| B-374 | Addressing built from token identifiers: a chat turn is a sequence of tokens, and MCF's prompt surface can only send text — which its own refusal to parse control tokens from prompts (F26) makes unable to express one | B-052, D42, §3.18, F37 | A model can be addressed the way its template describes without a control token ever being parsed out of user text; the chat-template probe compares addressings rather than reporting that it cannot | **done** (F38) — a request carries either a prompt or identifiers; the markers come from the template in the model's own file, kept only where the vocabulary can express them; a built-in list of known shapes is a fallback and never the source. Nothing a user typed is ever among the identifiers |
| B-375 | A sharper question than *did the turn end*: on gemma-3-270m two addressings answer and end the turn equally often, and this observation cannot separate them | B-052, F38, F39, D42, §3.18 | Addressings that tie under the turn-ending question are separated by an observation that needs no judgement and does not change with the engine, or the tie is shown to be a real equivalence | **done** (F48) — there was no sharper question to find, because there was no ambiguity: the two candidates differed only in a role word, and gemma's template names the losing one exactly once in order to rename it. MCF read the template as a bag of words and manufactured a candidate the file explicitly rejects. It reads the *assignment* now — `set role = "literal"` — and falls back to mentioned names only where a template assigns nothing. A behavioural question was tried and rejected on evidence: stopping before the role word and letting the model supply it works on SmolLM2 and yields a newline on gemma, the one model it was for |
| B-377 | Why two engines give the same verdict and different turn lengths: the same model, the same identifiers, greedy from the same seed, and generations that diverge partway | B-373, B-376, F39, §3.12 | The divergence between MCF's own engine and the provisioned one on a full generation is explained, or MCF's engine is corrected | **done** (F40, F41) — near-tie amplification, not a defect. Teacher forcing compares the two after their greedy runs have parted: across 250 positions of SmolLM2 and 700 of gemma-3-270m the reference's token was never worse than MCF's rank 3, and agreement does not decay with position. gemma's sliding-window attention is correct past its own 512-token window when the cache is grown a token at a time — F28 had crossed the same boundary by a long prompt, which fills that cache in one pass rather than by generating. The comparison is now an oracle section at `MCF_ORACLE_FORCED=1`, shown to fail against a structural break (rank 618 against rank 3) and shown **not** to catch a boundary off by one |
| B-378 | A gate test whose result depends on whether a daemon happens to be running on the machine | §3.12, B-003 | Every test in the gating tier reports on what it was given, not on ambient machine state | **done** (F47) — `run_where` takes where the daemon is as an argument and `run` looks it up and passes it in; the tests call a helper named `without_a_daemon`, so what they assume is in the name rather than in the environment. Shown in the direction that matters: the tests now pass identically with a daemon running and with none, where before it was pass-without and fail-with. A table-driven check keeps the shape from coming back and was shown to fire; a second test asserts every alternative it names exists. The table watches one call in one file — the regression is mechanical, the class is not |
| B-379 | What a language costs this model: tokens spent per character of the same text, per language, from the vocabulary alone | §3.15, §X, B-057, B49 | `mcf explain` states what each of a stated set of languages costs on this model's vocabulary, as a ratio against its cheapest, with no generation and no judgement | open — measured already and worth surfacing: on one sentence, SmolLM2 spends 52 tokens on Japanese where gemma3 spends 20, which is 2.6x the money, 2.6x the context and 2.6x the time. It is the operator's *expose the meaningful values so people learn to read them* (DEC-002) in its cheapest form, and it is a fact about the vocabulary rather than a claim about the model's fluency — which is the distinction that must survive into the wording |
| B-380 | Touchstones: every measured value MCF shows a non-technical reader carries a plain-language note on what a high or low reading tends to mean, marked as guidance rather than as a result | DEC-002, A21, §3.15, §3.18 | No touchstone is phrased as a measurement; each names what MCF has *not* measured about the relation it describes; a laboratory that measures one replaces it and the replacement is visible as a change | open — the operator's addition to DEC-002. The hazard is the whole of the design: a rule of thumb printed beside a measured number in the same typeface becomes a measured number to a reader who is not looking for the difference, and the readers this is *for* are exactly those readers. The three states MCF already applies to a model's declarations are the shape — declared, verified, unknown — turned on MCF's own sentences **Where first, operator's answer 2026-08-27: the comparison view, and `mcf explain` second.** A number teaches by contrast — 0.77 tokens per character means nothing alone and everything beside 2.00 — so a reader placed in front of two values *sees* the difference the touchstone describes instead of being asked to believe it. `mcf explain` then serves the curious reader for a single model, having already learned what the column means. `mcf run`'s footer stays conditions only: it is the wrong moment to teach, and a footer that grows is the one people stop reading. |
| B-381 | Segmentation: the prompt shown as the tokens the model actually receives, fragment by fragment | PR11, §3.15, §X | A person can see where their text breaks into pieces, where a vocabulary has no word for what they wrote, and where a term survives whole; no generation and no judgement is involved | done `mcf segment <model> --prompt <text>`: every token, the text it contributed exactly, whether the vocabulary reached for a raw byte, and how many whitespace-separated words survived whole. No generation and no judgement (§3.15). Contributions are byte-differences of successive decodes rather than per-token decodes, which is the only correct method for a byte-level vocabulary (F19) — the naive one reported a whole Japanese phrase as one token's work, and that string is now a test. F78. |
| B-382 | Prompt cost: what a prompt spends, against what this model can actually take | PR11, B-055, §3.8 | The token count of a prompt is stated against the *usable* context measured for this model rather than the declared one, so that a guidance document too long for a model is known before it is sent | open — accepted from PR11. Arithmetic, plus the usable context B-055 already measures. The same document is a rounding error on one model's window and a tenth of another's, and the difference is the vocabulary rather than the word count |
| B-383 | Marker fidelity: which of the markers a person wrote are real control tokens for this model, and which are ordinary text | PR11, F26, F37, F38, §3.7 | A marker typed into a prompt is shown as what it becomes; a template whose markers do not survive is visible as such before it is relied on | open — accepted from PR11, and the one with a measured cost behind it. F37: `<|im_start|>` written into a prompt reaches the model as **eight ordinary tokens**, and the table that produced was the most decisive-looking wrong answer in this repository. A person tuning a prompt today has strictly less visibility than the probe that was fooled. The machinery exists — the marker check was built because F37 forced it |
| B-384 | How large a model MCF's own engine can usefully read: the size at which its answer stops arriving in a useful time, measured rather than assumed | PR12, B-366, B-372, D31, B49 | A stated size, measured across the models held, with the cost per token at each; the reference role's *validation* half is chosen by that number rather than by convenience | open — accepted from PR12 and sequenced after B-366, which moves the number. Nothing between 0.6 billion and 27 billion has been tried: the small end is 0.9 s a forward pass and the large end is refused outright, and the whole middle is unmeasured. Until it is measured, *how big a model can MCF check itself on* is answered by whatever happens to be on the disk |
| B-385 | A projection carries the conditions of the history it rests on: how many of its points were taken while the machine was not in its own steady state, and how far it moved | §3.4, A6, B34, B-217, DEC-007 | A band reads *from 68 measured arms, 4 of which were taken while the machine was not steady* rather than *from 68 measured arms*; no point is filtered, because filtering needs the threshold DEC-007 has not set | done `Point` carries what else the machine was doing (thousandths of a processor, the larger of the two readings), `band` returns a `Projection` that cannot be separated from its `Rested`, and both surfaces that render a band render it. Nothing is filtered — that needs DEC-007's figure — and *nothing recorded* renders as unknown rather than as quiet (A7). F75. |
| B-376 | The provisioned engine cannot be probed: it takes text on a command line and prints text back, so a turn built from token identifiers has nowhere to go, and it does not say why a generation ended | B-032, B-052, B-374, D42, F38 | A probe can name any engine MCF can drive, and its result belongs to that engine; a turn of identifiers reaches a provisioned engine unaltered, and the stop reason is observable through it | **done** (F39) — the same provisioned prefix's *server* rather than its completion tool, on a Unix socket under MCF's runtime directory so nothing listens on the network and no port is contended (§XVII). The model stays loaded between requests, which is F36's open residency. `mcf probe --engine <name>` names one, and the default is the server where there is one — changed only after both engines were shown to return the same verdict, not on the twentyfold speed alone (B29). A 270M model probes in eight seconds where it took tens of minutes |
| B-059 | Derived configuration carries the provenance of the capability that set it: which probe, when, under what conditions | §3.18, §6.19 | Every auto-set parameter answers "why this value" with a probe reference or a declared default | **done** (F43) — `mcf probe --apply` is the act D43 requires, and it is the only thing that writes a configuration. What it writes carries the probe, the moment, the build and the conditions; the daemon applies it and puts the provenance in the account; `mcf run` prints it on every run (§3.15). Inconclusive refuses to apply, and raw refuses too — a provenance on a default would make it look derived. The winning addressing is carried out of the probe rather than rebuilt from its name, and a test asserts the turn a configuration builds is the turn the probe sent, identifier for identifier |
| B-060 | Inconclusive handling: a probe that neither confirms nor denies leaves the capability unknown and says so | §3.18, §7.24 | No inconclusive probe result is ever coerced to a working default | **done** — `Outcome` offers `observed() -> Option<&T>` and nothing else, and `checks/tests/inconclusive_is_never_defaulted.rs` holds it: the type may not grow a fallback accessor, and no shipped caller may build one from the accessor it has. Shown to fail against a planted `.observed().unwrap_or_else(|| "raw")` and to pass without it (B-003's negative control). In a test file `expect` is an assertion that the probe decided, and is allowed there and nowhere else |
| B-061 | Reconfiguration policy enforcement: whatever DEC-025 decides, comparability across a configuration change is preserved or explicitly invalidated | §7.25, §3.4 | A configuration change either preserves comparability or marks prior results non-comparable | blocked (DEC-025) |
| B-062 | `mcf probe` / `mcf explain`: the M3 product — a model runs the way it is designed to run, and can prove it | §X | A model whose defaults were previously wrong measurably improves, and the improvement is attributable to a named probe | **done** (F43, F45) — `mcf probe` runs two probes and `--apply` acts on the first; `mcf explain` carries what was applied beside the declared defaults and says whether the conditions still hold. All three exit criteria are met: a model that produced nothing now answers and the improvement names the probe (F43), no probe result is ever coerced to a default (B-060, held mechanically), and every derived value answers *why this value* where it is used (F43, F44). What M3 does not have is a second *applied* parameter — the usable context is measured and nothing consumes it |

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
| B-080 | Benchmark runner distinct from the test suite: no pass condition, never a gate on correctness | §6.7 | Benchmarks cannot fail CI; correctness tests cannot produce measurements | **done** (F59). `mcf bench <model> --against <model>`: it builds the comparison the only way one can be built (B-250), stops when this run's own arithmetic decides (F55, F57), and records what it found whichever way that was (B-086). No verdict sets a failing exit status — *differ*, *same to a stated resolution* and *not decided* all exit zero, because all three are things the machine said. The refusals are about being unable to measure: no such model, no daemon, an engine that cannot report a speed. **The engine is asked rather than assumed** — one request per arm, and the account says which engine ran, so a stand-in is refused by name rather than marked (B65, D31). `checks/tests/benchmarks_never_gate.rs` holds both halves: no verdict reaches a failing status, no gating test bounds a wall-clock reading, no gating test writes to the machine's own record, and the gating tier does not run the benchmark. The tiers that may assert on a timing are named there, because an exemption written down is one somebody can argue with |
| B-081 | Non-adaptive measurement path: caching, reuse and adaptation are disabled while measuring, and warm-versus-cold is a recorded condition | §6.13, §6.2 | A measurement taken warm is distinguishable in the record from one taken cold | **done** (F64). `mcf_bench::warmth` reads each trial's warmth from the daemon's own account — a state this build has not been taught is *unstated* rather than whichever of the two it resembles — and a run is `Uniform` or **`Mixed`**, where mixed says *this is not one measurement* rather than averaging over it. One warm trial among a hundred and ninety-nine cold ones is mixed: §6.13's concern is hidden history, not proportion. It is the floor's thirteenth condition, written by `Interleaving::finish` so a caller cannot forget, and it flows into `Isolation` for nothing — a warm arm against a cold one is confounded and A8 withholds the delta. **What it made visible at once**: every benchmark trial is cold, even comparing a model with itself, because a prompt routes to a per-request subprocess and only a turn of identifiers reaches the server (B-376). Measured, that is 46 ms of overhead against 0.25 ms per token — **60% of a default trial and 74% of F59's** — which explains why that comparison could not see a difference between two quantizations. What remains is §6.13's corollary that the benchmark path may not *adapt*: making it warm needs the server path, which is B-376 and B-090 |
| B-082 | Real-hardware only for performance numbers: no performance figure may originate in simulation | §6.16 | The type system or the record prevents a simulated timing being published | **done** (F60). Putting the clock in the type stopped a simulated duration being *compared* with a real one, which nobody was attempting, and not being *written down* as one, which F59's encoder did in three lines. `mcf_core::time::Measurable` closes it: implemented for `Monotonic` and nothing else, and the comparison encoder is bounded by it, so encoding a laboratory comparison is a compile error. It bit at once — three of the encoder's own tests were on the simulated clock and stopped compiling. Checked three ways: the trait has exactly one implementation, the encoder keeps its bound, and no crate that writes to the record names the laboratory's clock at all |
| B-083 | Repeated trials with reported spread and sample count; single-shot timings are refused as anecdotes | §3.4 | No result is publishable with n=1 | **done** (F54, F55). Unblocked by DEC-007's first answer and F51: no result from one run, and seven repeats is what a five-percent claim needs on this machine measured rather than assumed. The count travels with the machine — `prototypes/timing-noise` is the instrument, and it is the instrument that ships rather than the number **Reshaped by F53:** the row's *no result with n=1* stands, but a fixed count does not. Six readings of one command needed seven to over a hundred repeats for the same claim, so a benchmark carries a stopping condition rather than a count, and reports the number it actually needed as part of the result **Built (F54):** `mcf_bench::enough` carries the rule. Three outcomes — differ, same to a stated resolution, not yet — with arms paired by construction and unequal ones refused rather than truncated. Its first real comparison stopped itself at seventeen paired trials, which is neither of the counts previously derived, and reported the count as part of the answer **Corrected by F55:** the *same to a stated resolution* branch was asking a circular question — *would a difference this big have shown?* of a null built out of the very differences such a difference would have moved — and on four real paired trials it declared a null result at a count where `Differ` was arithmetically unreachable, since a sign flip over four pairs cannot reach one in twenty. It asks about power now: centre this run's differences, add an effect of exactly the resolution asked for, and ask whether *that* would have been declared. The same correction went into the pooled path The row's condition — *no result is publishable with n=1* — is now held in two places at once: a `Measurement` cannot be built from one sample, and a comparison of fewer than two pairs is `NotYet` rather than an answer **And the test itself changed (F57):** the resampling could not recognize two arms with identical timings — flipping the signs of equal magnitudes cannot move the median's size — so the paired verdict is an exact sign test now, computed in whole numbers with no seed. Six pairs is the fewest that can reach one in twenty at all, which is F55's four-pair defect in its general form **And a third correction (F59):** `Differ` now requires the effect to be real *and* at least as large as the resolution the caller asked about. The first real benchmark reported *they differ by 0.8%* to a caller who had said five percent — a true statement answering a question nobody asked. Below the resolution the verdict is the null one, and the measured difference travels inside it (A1) |
| B-084 | Warm-up and thermal steady state per DEC-007's acceptance criteria | §3.4, §7.7 | Runs that fail the criteria are marked invalid, not silently included | blocked (DEC-007) |
| B-250 | Comparisons are constructed only from paired, interleaved, order-randomized trials sharing a session id; the reported quantity is the paired difference distribution | B53, §3.27, §3.4 | Block-then-subtract does not compile; a cross-session comparison is constructible but labelled weaker | **done** (F55). `mcf_bench::compare`: a `Comparison` has three constructors and no fourth — the `Interleaving` runner, which alternates the arms itself and draws which goes first per pair from a stated seed; `from_trials`, which reads a session back out of the record and *verifies* the interleaving from the positions, so blocked trials come back as `RanInBlocks` naming the arm and where it repeated; and `from_separate_sessions`, which is §3.27's *it may be all that exists*. The statistics moved behind that door and are `pub(super)`, because a function taking two slices cannot tell an interleaved comparison from two blocks — that is the *does not compile* half, held by `checks/tests/a_comparison_is_paired.rs` against a fourth way in. The cross-session half is weaker **in the type**: it answers `None` to `paired_differences()`, and the reverse is refused too, since two arms that share a session may not be assembled the weak way when the strong construction is available. The reported quantity is the paired difference distribution and the null is a sign flip rather than F54's pooled redraw, which was an unpaired test on paired data. **Now has both numbers behind it:** the blocked arrangement of one command against itself invents **88%** on this machine (0.233 s quiet against 0.439 s loaded), and the interleaved arrangement reports *no difference as large as 2%* across a run containing a seventy-percent load spike (F55, and F51 for why the shift is coherent rather than random) |
| B-252 | Every generalized statement resolves on demand to the measurements, conditions and spread behind it; no summary is written in place of its evidence | B55, §3.28, §3.15 | A recommendation expands to its evidence without leaving the interface; the record retains full precision | **done**. `mcf show <entry-id>` expands any line `mcf log` printed into what it rests on, **without leaving the interface** (A22): every pair's two raw durations, which arm ran first, what each drew, both arms' full condition floors, what the run reused, and what was cut short. Nothing is recomputed and nothing is summarized — the summary *is* the line in `mcf log`. `--full` was not this: it prints the record's own JSON, which is what a script reads, and a person asking *what is that number made of* was being handed nine hundred characters of one line. **`null` survives the expansion**, because a question asked and unanswered is not a question nobody asked, and dropping it would turn *eleven questions, nine unanswered* into *two conditions* — the strongest claim in the file, made by omission (A7). And the log's own line now names the quicker arm rather than a bare magnitude (F67) |
| B-085 | Isolation check: a comparison in which more than one variable differed reports "these are not comparable" rather than a delta | §3.4 | An intentionally confounded comparison is refused by the tooling | **done** (F56). An arm is a configuration rather than a name — `compare::UnderTest` carries the `Conditions` it was measured under, and every constructor takes two of them — and `mcf_core::measurement::Isolation` says what separates them, walking `Floor::entries()` rather than a second copy of the list so that a condition added to the floor is isolated on without anybody editing the check. Four answers: nothing differs (a control, which measures the machine), one does (the only shape a delta means what a reader takes it to mean), several do, or MCF has not read enough to say — because two unknowns are not a match (A7). **The refusal is in the type**: `Finding::verdict()` is `None` for a confounded comparison, so a caller cannot print a number by forgetting to ask. A confound the operator *declares* is science and comes back with its declaration and every differing variable beside it (A8), which MCF records rather than judges |
| B-086 | Null and negative results are stored and surfaced as results — "does not fit here", "no measurable speedup" | §3.4 | Both appear in the record and in the window as outcomes, not failures | **done** (F58). Two record kinds, neither of them a failure: `comparison` and `fitment_planned`. A comparison writes all four outcomes — differ, same to a stated resolution, not yet decided, not comparable — with the paired difference distribution and both arms' conditions beside them, so the verdict can be re-asked from the record rather than trusted (D16, B56; the stopping condition's own rule has changed twice, F55 and F57). A plan writes every variant whichever way it came out, because a record of only the refusals cannot answer *when was this last known to fit* (A1). `mcf log` reads a null result as a result in as many words, and a refused comparison as an outcome with no delta anywhere in the line. A test asserts of both kinds that they are not `Failure` and do not render as one |
| B-087 | Partial success representation: nine of ten tasks completing is nine data points | §3.1 | Partial runs are queryable as partial, with their per-unit outcomes intact | **done** (F66). The benchmark runner had A4's own violation in it — *an all-or-nothing return type on anything that can partially succeed* — and threw away every completed pair when one request failed. It keeps them now, and `Comparison::cut_short` carries why it stopped: an `Option`, so that *it finished* and *it was interrupted and nobody recorded why* stay different facts (A7). It reaches the record, where `null` means the run finished. Demonstrated by stopping the daemon five seconds into a run: fifty-eight pairs kept, the verdict over them standing, and what was lost said. **Two more defects fell out**: the interrupted request was being recorded as a trial of zero nanoseconds — a number nobody measured, entering the distribution — and the record was written from a finding recomputed at the *default* resolution, so a caller asking about half a percent was shown one verdict and the record kept another (A6) |
| B-088 | Contention governance: MCF knows the difference between a slow model and a busy machine, and says so when it cannot tell | §3.8, §7.9 | A deliberately contended run is marked unattributable rather than reported | blocked (DEC-009) |
| B-089 | Environment pinning to the tolerance DEC-006 sets | §3.12, §7.6 | Every result carries enough environment to be reproduced to the stated tolerance | blocked (DEC-006) |
| B-280 | A reported improvement traces to a workload split that was not used to select it; a winner failing validation reports *no improvement found* | B59, D18, A10 | Sweep results cannot be published from the selection split | open |
| B-290 | A trial cannot be constructed without its seed, and a run cannot declare one seed for every trial; the seed set is published and recorded as a condition | B61, D19, §3.4 | Identical-output runs are unrepresentable; comparisons refuse mismatched seed sets | **done** (F61). `Trial` has a fifth field with no default, so a trial that does not say what it drew does not exist. It is a `Draw` with two variants, because D19 gives the two laboratories opposite rules — a behaviour trial draws seed *i* at trial *i*, a timing trial holds its seed still and pins its generation length — and two variants rather than one with a flag, so a timing run's fixed seed can never be mistaken for a behaviour run's mistake. **The published set is arithmetic, not a list**: D19 assumed a trial count and F55 removed it, so a fixed list would run out and have only bad answers for what happens next. Six lines, unbounded, identical on every machine, and a bijection — two trials cannot draw the same seed by construction. A declared set is admitted, refused if it repeats a seed or holds fewer than two, and runs out rather than wrapping around. The set is the floor's twelfth condition, so it renders everywhere and enters the isolation check for free; `Comparison::from_trials` refuses two arms that drew from different sets, pair by pair, because the two runs of a pair must have drawn the same thing. And `mcf bench` pins a length now: it did not, which made every timing include the models' verbosity |
| B-291 | Seed-set validation: periodically compare the fixed set's distribution against a larger random set; divergence replaces the set and records a break in comparability | D19, §6.16, §7.13 | The standard set is shown to be representative rather than assumed | **done** (F62). §6.16 turned on MCF's own instrument: the published set is the first thirty-two draws of a stated stream, and the tier runs three hundred and twenty more from a million indices further along and asks whether the prefix behaves like the body. `mcf_bench::seeds` holds the arithmetic and inverts the polarity — here *the same* is the good news and *distinguishable* is the finding, so a clearance can never render as a discovery — and **not decided is not clearance**, because treating it as clearance is how an unvalidated instrument stays unvalidated. The two draws cannot be paired (different seeds by construction), so it uses the pooled null §3.27 already calls weaker, which is the honest one here. Scheduled: `scripts/ci.sh --with-seed-set`. **It samples with nucleus and has to** — MCF's shipped generation is greedy and greedy ignores the seed, so a validation against it would compare a constant with a constant and clear the set for a reason that is not about the set |
| B-281 | Recommended sampling renders as *declared* until a sweep promotes it; a lab that pins its own sampling declares it and its results stay apart | B60, D18, A21 | No global default sampling exists; divergence between recommended and best-measured is surfaced | **in progress** (F63). `mcf_core::configuration::Calibrated` holds the values **and whose choice they are** — declared by the artifact, measured by a sweep, pinned by a laboratory, or MCF's own with the reason — with four constructors and no fifth, and `Sampling` has no `Default`: together that is *no global default* as a shape rather than a habit. A laboratory's pin is quarantined from anything that inherited, in both directions and from another laboratory's pin. **And MCF now looks before it chooses**: `mcf_standin::recommended` reads the file's own metadata and `mcf_hub::recommendation` reads the repository's `generation_config.json`, each with *nothing declared* as a state rather than an empty set, because only the first justifies MCF choosing for itself. `mcf explain` says whose choice the sampler is instead of only what it is. **The measured finding is that there is usually nothing to adopt**: none of six GGUF repositories publishes a recommendation and no acquired file carries one in its metadata — it lives in the base repository the conversion came from. What remains is the half that needs a sweep: `Chosen::MeasuredHere` exists and is unreachable until B-280 |
| B-091 | Quantization frontier on the reference model: one model, one machine, the full GGUF quantization range — the cleanest available §3.4 comparison, a single variable across many points | §XII, §3.4, §IV | A frontier is produced across quantizations with one variable differing, and results state they characterize the instrument, not models in general | **done** (F67). `scripts/frontier.sh`: seven quantizations of one model against one reference arm, built as seven **paired comparisons** rather than seven absolutes on a chart (§3.27), each with its own stopping condition and conditions, `--cold` throughout because a warm two-model run comes out mixed (F65). It is monotone in file size — 88, 88, 92, 105, 112, 138, 145, 271 MB giving 26.1, 25.7, 24.4, 18.2, 16.6, ~3, 0, −61.9 percent — which is F64 arriving: every trial loads the model and load time goes with bytes, so **this is principally a frontier of file size** and says so. The script prints what it characterizes in as many words: the instrument, not quantization in general, and nothing about quality. Producing it found that `Verdict::Differ` carried a magnitude and no direction, which is a table nobody can read; it names the quicker arm now. Not a step of `scripts/ci.sh` and never will be — a benchmark has no pass condition (A18) |
| B-211 | Repro bundle: one file carrying a claim, its method, its full §3.4 conditions, its raw samples, the artifact's provenance chain, the §XV identifier, and a verification manifest | [PR2](proposals.md#pr2--the-repro-bundle), §II, A6 | A bundle is emitted for any published measurement and contains everything needed to re-run it | **done** (F68). `mcf bundle <entry-id>`: the claim and everything it rests on — both arms' provenance as recorded at acquisition, the provisioned engine by image digest and commit, and what this machine was when the claim was taken — selected out of the record rather than assembled beside it. **One mechanism**: `export::write_selected` is `write` with a predicate, so B-302's *the kinds differ in what is selected, never in how it is written* is a signature rather than a sentence. The **method** is recorded with every comparison now (the prompt, the resolution asked about, the ceiling, the engine, whether every trial loaded the model), because a floor full of hardware does not tell anybody what to run. Producing the first bundle found that the header's `contains_user_content` had been a constant `false` since before a comparison carried a prompt; it is computed from the entries now, over a named list a check keeps complete. What leaves is printed before it leaves (A24) and what is missing is named (A7) |
| B-212 | `mcf verify <bundle>`: reproduce the configuration, re-run the method, report agreement or divergence with conditions compared side by side | [PR2](proposals.md#pr2--the-repro-bundle), §II, A8 | A bundle from another machine either agrees, or names which conditions differ and refuses to attribute the gap | **done**. The digest is checked before a byte is read out of it — a bundle arrives from somewhere else and is an untrusted input (§3.7) — and then it says what it claims, what differs between the machine it was taken on and this one, and whether what it measured is here. The conditions comparison is `Isolation`'s, the same arithmetic that decides whether two arms of a benchmark are comparable (A8, B-085): a second way of answering *which conditions differ* would eventually disagree with the first. **The refusal is the point**: if a re-run gives a different number MCF names every difference and stops, because picking one and calling it the cause is A8's confound wearing a helpful voice. The re-run is **printed rather than run** — verifying a bundle is reading a file somebody sent, and running a model because a file said to is a different act with a different cost, which MCF does not take on the reader's behalf (§3.7, A16). A claim whose arms are not on this machine still gets the conditions comparison, which is PR2's *tells you exactly why your machine cannot* |
| B-214 | Expectation bands from local history: project throughput for unmeasured configurations from what this machine has measured, as a labelled estimate that can never sit beside a measurement unlabelled | [PR3](proposals.md#pr3--pre-acquisition-planning), A20, B34 | Projections are band-shaped, marked as estimates, derived from local history only, and absent where there is no history | **done** (F69). `mcf_bench::project` reads between two measured sizes and **never past them**: F67 established the relationship and also measured where it stops being straight, so a file outside the measured range gets no band and the refusal says why. Band-shaped (B46), `Basis::LocalHistory` (B34), and absent at a budget nothing was measured at — two requests of different lengths are two different things. `mcf explain` says the word *ESTIMATE* in the sentence a reader meets and says what A20 forbids doing with it. **Validated by predicting first**: a quantization this machine had never benchmarked was projected at 385.7–404.5 ms and then measured at 397.3 |
| B-215 | Every projection is scored against the measurement that eventually replaces it, and the score is reported | [PR3](proposals.md#pr3--pre-acquisition-planning), §6.16, §3.4 | Prediction error is tracked over time; a projection model whose error grows is a finding about the model | **done** (F69). Scored by **leaving each point out**: for every measurement in the history, the band its neighbours would have given is computed and compared with what it actually was. No stored predictions and no new record kind — it is recomputed from the record each time, so it moves as the history does, which is what *over time* means when the history is the thing changing. `mcf doctor` reports it: **42 of 49 inside, worst miss 150.1%**, with the five that could not be scored named rather than counted as hits — the points at the ends have nothing to be read between, and counting them would be scoring the refusal to extrapolate. The misses are real and the largest is large, which is a finding about the model rather than a reassurance |
| B-216 | Contention snapshot: on demand and on invalidation, sample per-process accelerator occupancy, memory pressure, thermal and clock state against baseline, and attach it to the invalidation record | [PR5](proposals.md#pr5--contention-diagnosis), §3.8, B24 | An unattributable run names what it was competing with; the snapshot persists with the record rather than on a screen | **done** (F70). `mcf_core::hardware::contention` reads two `/proc` samples a stated fifth of a second apart, turns accumulated processor time into a rate, and **names the processes** — a snapshot whose answer is *the machine was busy* is a number, one that says which processes and what they took is a diagnosis. MCF's own process is marked rather than filtered out: it is the one the reader can do something about. The kernel's own stall accounting is read for processor, memory and storage; per-process accelerator occupancy is `Unknown` because MCF has no vendor library here, and D25 makes *unknown* not *none*. **Taken because a run could not decide, and at no other time** — B4 refuses ambient sampling, and a check asserts there is exactly one caller, no timer and no thread, and that it is sampled *after* the run so MCF is not one of the competitors it reports. It goes into the record as `contention_snapshot`, because a finding printed and not written down does not survive the terminal (§3.1) |
| B-090 | `mcf bench`: the M5 product — a defensible performance number taken here, with its conditions and its uncertainty | §II, §IV | Two configurations of one model are compared on this machine with a stated method, spread and conclusion — including "within noise" | **done** (F65). The command builds the comparison the only way one can be built (B-250), stops when its own arithmetic decides (F55, F57), states what it reused (B-081) and records the whole of it (B-086). It sends the prompt as **identifiers**, which reaches the provisioned engine's server rather than a fresh process per request (B-376) — and that turned out to make a two-model comparison *mixed* rather than warm, because one model is resident at a time (DEC-001) and a paired comparison alternates two. So the delta is withheld from a mixed run and the refusal names the two uniform ways: `--cold`, which sends text and loads the model for every trial, and comparing a model with itself. Both were run: *no difference as large as 5.0%* at nine and at fifteen paired trials, with `reuse` stated either way. What is not answered is whether a *warm* two-model comparison is worth having, which needs two resident models and is DEC-001's question |

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
| B-181 | Exclusive window for timing-class work: announced, bounded by a declared maximum, schedulable, interruptible, closed automatically; drains the serving path by explicit decision | B35, D8, §6.33 | A timing result cannot be constructed from a run that overlapped serving or another lab | open Shaped by the operator's answer on DEC-007: the window's precondition is not an absolute quiet but **stability against this machine's own baseline**, because a machine that idles at forty percent is a machine whose normal is forty percent. What the window guarantees is that conditions did not *change* under the run, and a disturbance partway through invalidates the result rather than skewing it. **What a window may do to the machine (operator, 2026-08-27): this project has priority over whatever else the machine is doing, and may change what it needs to — bounded absolutely by *the user must never lose control of their own system*, no freezing, no locking them out of ordinary use.** That is a sharper rule than politeness to other tenants and it has three mechanical consequences, none of which is a preference. **(1) Never take a card that is driving a display.** Exclusive compute mode on the display adapter is the freeze itself; on this machine the card reports `display_active: Enabled` with two processes already holding contexts on it, which is the desktop. The card is asked before it is taken, and one carrying a display is declined by that fact. **(2) What is taken must come back even if MCF does not.** The safety cannot rest on MCF working correctly, because the case that distresses a user is the one where MCF has hung — so a window expires by construction and the restoration ledger (B-220) is what makes a governor survive a kill. **(3) A run yields immediately when asked**, not at the end of the current trial: `mcf stop` (B-210) must return the machine, and a partial result is a result (A4). |
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
| B-224 | A laboratory declares its work in countable units — trials, sweep points, tokens, documents — never in minutes | B46, D14 | No lab declares a duration; duration is derived from work × the machine's measured rate | done `mcf_bench::planned::Work` carries trials, arms and tokens and has no fourth field: no duration, no deadline, no timeout, because a type with one would let a lab declare in minutes. The duration is multiplied out of a per-generation band this machine measured, arrives as an `Estimate`, and is absent by name where there is no history — measured both ways (F72). Held by `checks/tests/work_is_counted_not_timed.rs`, since Rust cannot say *no field here means a duration*. |
| B-225 | Duration estimates are banded, marked as estimates, scored against actuals, and their error is tracked and reported | B46, D14, A20 | A lab whose estimates are persistently wrong surfaces as a finding | done Banded and marked at the point of rendering (F72). Scored by `project::score` and reported by `mcf doctor` (F68): an expectation is the per-trial band multiplied by a count both sides agree on, so it is inside its band exactly when the per-trial band contained the truth, and a second bookkeeping path could only disagree with the first about the same history. |
| B-226 | A time budget produces a proposal naming what will run and what is excluded and why; never a silent truncation | B47, D14, §3.1 | "Ran 6 of 20" is always accompanied by the fourteen | done `mcf bench --within <seconds>` — the budget is the operator's, never the laboratory's (F72). `planned::Proposal` has three shapes and no fourth: `Whole`, `Fewer` (which cannot be constructed without the excluded half, so the fourteen are in the type), and `NotEnough`, a refusal rather than a smaller question answered quietly. Planned against the slow edge, because a budget planned against the fast one is decorative. A budget with no measured rate behind it refuses and names what would fix it (A7). F73, and the refusal that looked wrong and was not: F74. |
| B-227 | Anytime results: every lab reports as it goes; a run stopped early keeps what it produced, marked incomplete | B47, A4, §3.1 | A multi-day lab interrupted at hour three yields three hours of marked data | done The keeping was already true (A4: the pairs stand, the reason travels, the comparison is marked cut short). The reporting is new: an interim line per pair on standard error — never standard output, which is where the result goes — carrying the pair count, the words *so far*, and the two arms' medians rather than repeating *not decided* forty times. It cannot change what the run does (A18), which a check holds. F76. |
| B-228 | Environment ladder: report, wait for quiet, suspend-and-restore only with per-run approval of a named list, never terminate; scope granted per DEC-041 | B48, §6.39, A27, DEC-041 | A scenario asserts nothing outside the approved list is touched and every suspension resumes, including when MCF is killed | open |
| B-222 | Every corpus statement renders its sample count; no filter removes a candidate from a listing | B44, §3.24 | An unreported option is ranked lower and annotated, never hidden | open |
| B-273 | Lab setup and teardown: each lab owns both, may use its own tooling, and leaves nothing behind — asserted by running two labs back to back, including after the first is killed mid-run | B58, A27, §XIII | The second lab sees no trace of the first, warm caches included | open |
| B-111 | Lab framework: a lab is named, versioned, reproducible, declares its class (timing or behaviour), declares its capability gate and its workload slot, and states what it does and does not establish | §XIII, §3.17, §6.26, B40, B42 | A lab that cannot state its class, gate, slot or validity boundary fails to register | blocked (DEC-029) |
| B-200 | Lab results are a sum type: *measured*, *not applicable*, *unknown*, *failed* — the middle two carry no score and cannot be averaged | B40, §3.23 | An absent capability cannot render as a low number anywhere | done `mcf_core::graded::Graded`. The three non-readings have nowhere to put a score; the only way out is `score() -> Option<&Score>`, fallible so that a caller meets them where they were about to flatten them. *Not applicable* and *unknown* stay distinct, because one is resolvable by probing and the other is not. Built before M6's laboratories exist, which is when a type is cheapest to get right. F77. |
| B-201 | No type combines results from two laboratories into a scalar | B41, D2, §3.9 | An overall quality score is unrepresentable | done `Score` carries the laboratory it is on and compares only within one; `Profile` holds one outcome per laboratory with no arithmetic across them and no `Ord`. Coverage travels instead — *measured by 1 of 3; inapplicable to tools* (B41). A check names each escape hatch by the string that would add it. F77. |
| B-203 | A custom-workload result cannot be constructed into a contribution, and is marked non-comparable at production | B42, §6.37, A25 | The marking exists before export, not at it | open |
| B-204 | Each lab declares what its workload slot accepts and refuses what it cannot grade | B42, [PR1](proposals.md#pr1--customizable-workloads) | An ungradable workload is refused at load, never run | open |
| B-205 | Workload slot format, loader and validator: a documented data format per slot kind — labels, documents, schemas, constraints, tasks, test suites — with authoring documentation aimed at someone who has never read the intent document | [PR1](proposals.md#pr1--customizable-workloads), B42, D7 | A user authors a workload for at least three labs from the documentation alone and gets a marked, non-comparable, local result | open |
| B-217 | Quiet-machine pre-flight: a laboratory refuses to begin on a contended machine rather than producing an invalid result, using B-216's snapshot | [PR5](proposals.md#pr5--contention-diagnosis), D8, B35, §3.8 | A lab started while another process holds the accelerator refuses with the contender named, and does not run | in progress The measurement is built and the refusal is not (F71). `contention::steadiness` reports a machine's own spread across successive readings, and every comparison now records the competing processor time before it and after it, rendered as a condition: *the level moved N% across the run, which is a condition and not a verdict*. Exercised both ways on this machine — 0.0% quiet, 1740% with load started mid-run, the latter still reporting its verdict beside the fact that the floor moved. The refusal itself stays blocked on DEC-007, which now has measurements to be decided from: a threshold invented here would be exactly the figure that decision exists to derive. |
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
| B-122 | Interrogable recommendation: reasoning, the measurements behind it, and the runners-up | §3.9 | Every recommendation answers "why not the other one" with data | open Shaped by DEC-002's partial answer: interrogable means the numbers travel *beside* the recommendation rather than behind it, because the operator's intent is that an ordinary reader learns to read them; and the unmeasured qualities are named in the same breath, so that a recommendation cannot imply the measured axes are all there is. |
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
| B-147 | Offline operation: as much as possible works with no network, loudly labelled; "no internet" and "no local network" are distinct conditions | §7.11, §3.2 | The offline scenario runs the whole of M1–M7 to the extent possible and labels every degradation | open — unblocked by D33, and half-built with it: everything but acquisition already runs in a container with no network (B-183), and a request that needs one reports which of three things it observed rather than guessing at a layer. What remains is the *labelling* across M2–M7's surfaces, which need those surfaces to exist |
| B-148 | Long-run endurance scenario: days of simulated operation with faults, restarts, thermal excursions and upgrades | §I, §3.17 | MCF remains coherent, queryable and restartable throughout, with no unclassified outcome | open |
| B-260 | Longitudinal regression detection: compare like with like, detect against historical spread rather than a threshold, correlate with the diff of everything that changed, report as a labelled hypothesis and never as a cause | [PR7](proposals.md#pr7--longitudinal-regression-detection), §6.7, A18 | A drop exceeding historical spread is surfaced with what changed alongside it; improvements are reported the same way | blocked (DEC-045, DEC-007) |
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

### Version 191 — the prompt as the model receives it

B-381 done. And a silent `unwrap_or` found in the process, which had been
reporting a whole Japanese phrase as the contribution of its last token: A2's
silent failure with a friendly name. F78.

### Version 190 — four outcomes and no total

B-200 and B-201 done, ahead of the laboratories that will produce them: the
shape is what makes *not measured* impossible to render as a low number, and a
shape is cheapest to get right before there are results to retrofit it around.
F77.

### Version 189 — a run reports as it goes

B-227 done. The interim lines say what moves — the two arms — rather than
repeating the verdict that has not changed. F76.

### Version 188 — a band says what it rested on

B-385 done, the day it was opened. A projection carries the machine conditions
of the two points it was read between; the expected duration and `mcf explain`
both render them. F75.

### Version 187 — a budget proposes, and a projection is found to be silent

B-226 done: a time budget produces a proposal naming both halves, or a refusal
that says why — never a quietly smaller run. F73.

And B-385 opened, from F74: MCF's own record caught an unrelated workload
holding twenty-six cores while a benchmark ran, and the projection that reads
that history says nothing about it. The entries stay (A1) and are not filtered
(DEC-007); the condition must travel.

### Version 186 — work is counted, and minutes are derived

B-224 and B-225 done. A laboratory's declaration is trials, arms and tokens,
in a type with no field that could hold a duration; the minutes are multiplied
out of a measured band, marked as an estimate, and absent by name where this
machine has no history at that budget. F72.

### Version 185 — the machine either side of a run

B-217 in progress. The quiet-machine pre-flight built as a measurement rather
than a gate: `contention::steadiness`, a `MachineHeld` on every comparison, and
the movement across a run rendered and recorded as a condition. The refusal
stays with DEC-007, which now has something measured to decide from. F71.

### Version 184 — a run that could not decide names what it competed with

B-216 done (F70). B24's refusal with a name attached: a snapshot of what was
taking the machine, read on demand and never on a timer, taken because a
comparison could not decide and at no other time. It names the processes and
marks MCF's own; what MCF cannot read is unknown rather than idle; and it goes
into the record rather than onto a screen.

### Version 183 — a band, and the score against it

B-214 and B-215 done (F69). MCF projects a band for an unmeasured file by
reading between two measured sizes and never past them, says the word
*estimate* in the sentence a reader meets, and is absent where there is no
history around the thing asked about. A prediction made before measuring landed
inside its band. And `mcf doctor` scores the projection against every
measurement in the record — 42 of 49 inside, worst miss 150% — recomputed each
time rather than stored.

### Version 182 — a bundle either agrees or says what differs

B-212 done. `mcf verify <bundle>` checks the digest first, then says what the
bundle claims, which conditions differ between the machine it was taken on and
this one, and whether what it measured is here. It prints the re-run rather
than running it, and it refuses to attribute any gap: naming one of the
differences as the cause is A8's confound wearing a helpful voice.

### Version 181 — one file that reproduces one claim

B-211 done (F68). `mcf bundle` is §II's fourth obligation: a claim and
everything it rests on, in one file, with what leaves printed before it leaves
and what is missing named. Producing the first one found the header claiming it
held no user content while carrying the prompt both arms were asked.

### Version 180 — a statement expands to its evidence

B-252 done. `mcf show <entry-id>` unfolds any line `mcf log` printed into the
measurements and conditions behind it, without leaving the interface: every
pair's raw durations, both arms' full condition floors, and every question that
was asked and not answered — `null` is printed rather than skipped, because
dropping it would make the strongest claim in the file by omission.

### Version 179 — the first frontier

B-091 done (F67). Seven quantizations of one model, one machine, one sitting,
as seven paired comparisons against one reference. Monotone in file size, which
is what F64 predicted — every trial loads the model — so it is principally a
frontier of file size and says so. Producing it found that a verdict carried a
magnitude and no direction.

### Version 178 — a partial run keeps what it produced

B-087 done (F66). The benchmark runner had A4's own violation in it and threw
away every completed pair when one request failed. It keeps them, says what
stopped it, and records no trial for a run that did not happen — a
zero-duration stand-in had been entering the distribution. And the record now
carries the verdict the operator was shown rather than one recomputed at the
default resolution.

### Version 177 — a defensible number, and the two ways to get one

B-090 done (F65). `mcf bench` sends identifiers so the request reaches the
engine's server — which made a two-model comparison *mixed* rather than warm,
because one model is resident at a time and a paired comparison alternates two.
A mixed run has no delta to give; `--cold` and comparing a model with itself are
the two uniform ways, and both were run. Whether a warm two-model comparison is
worth having is DEC-001's question.

### Version 176 — every trial was cold

B-081 done (F64). What a trial reused is the floor's thirteenth condition, and
reading it showed that every benchmark trial loads the model for itself —
because a prompt routes to a per-request subprocess. Three fifths of a default
trial is process start, which is why F59's comparison of two quantizations
could not see a difference: three fifths of what it timed could not differ
between the arms.

### Version 175 — whose choice the sampler is

B-281 in progress (F63). A sampling configuration cannot exist without saying
whose choice it is, `Sampling` has no `Default`, a laboratory's pin is
quarantined, and MCF looks at the artifact before choosing for it. The measured
finding is that there is usually nothing to adopt: the recommendation lives in
a different repository from the weights.

### Version 174 — the seed set is shown, not assumed

B-291 done (F62). The published set is validated against a draw ten times
larger from the same stream, on a schedule rather than on trust: §6.16's *the
instrument does not get to grade itself*, applied to the seed set. The polarity
is inverted in the type so a clearance cannot read as a discovery, and *not
decided* is not clearance.

### Version 173 — a trial says what it drew

B-290 done (F61). A trial cannot exist without its seed; the published set is
stated as arithmetic rather than as a list, because F55 removed the trial count
D19 assumed and a list that runs out has only bad answers; a timing run's fixed
seed is a different variant from a behaviour run's, not the same one with a
flag; and a comparison refuses arms that drew from different sets. Along the
way, EINTR was found to be classified as a cut-off transfer and is retried now.

### Version 172 — a simulated timing cannot be written down

B-082 done (F60). The clock has been in the type since the first week, and it
was stopping the wrong operation: comparison rather than publication.
`Measurable` is implemented for the monotonic clock alone, the comparison
encoder is bounded by it, and no crate that writes to the record names the
laboratory's clock.

### Version 171 — a benchmark that cannot fail

B-080 done (F59). `mcf bench` compares two models on an engine that can be
timed and has no verdict that sets a failing exit status; a stand-in is refused
by name rather than marked. Its first real comparison — two quantizations of one
model on a provisioned llama.cpp, acquired from a real hub — found a defect in
the stopping condition: a difference smaller than the resolution asked about was
being reported as a difference.

### Version 170 — a null result is a result on the disk

B-086 done (F58). `comparison` and `fitment_planned` are record kinds of their
own and neither is a failure; a null result carries the resolution that would
have shown, a refusal carries what differed and no delta, and a plan is kept
whichever way it came out. The paired verdict became an exact sign test on the
way (F57): the resampling it replaces answered *not decided* about two arms
with identical timings.

### Version 169 — a comparison knows what it is a comparison of

B-085 done (F56). An arm carries its conditions; `Isolation` reports whether
one variable differs, none, several, or something MCF has not read enough to
judge; and a confounded comparison's verdict is `None` rather than a number
with a warning beside it. Two unknowns are not a match, which matters because
almost every condition producer is still unbuilt.

### Version 168 — the pairing is structural

B-250 done (F55). A comparison can only be built from paired, interleaved,
order-randomized trials, and the record's positions are checked rather than
trusted: block-then-subtract has no constructor and blocked trials read back
out of the record are refused by name. The cross-session assembly stays
constructible and is weaker in the type rather than in a label. On this machine
the blocked arrangement of one command against itself invents 88%, and the
interleaved arrangement is unmoved by a seventy-percent load spike arriving
mid-run. B-083's null-result branch was corrected in the same work: it could
only answer one way at small counts.

### Version 167 — the first probe

B-052 and B-374 done (F38). The probe's first answer was the inverse of the
truth: a model refuses an addressing it does not recognise by ending its turn
at once, and the question *did it end at its own stop token* scored that above
a fluent reply. Three defects behind it — a tie-break that always chose `raw`,
five trials that were one trial under greedy sampling, and a budget too small
to reach the end of a turn. B-375 opened for what the tie on gemma-3-270m
needs. The earlier note stands:

B-051 done and B-052 in progress (F37). The probe found that MCF never used the
stop token it was reading, and then that its own first answer was an artifact
of MCF being unable to put a control token in a prompt. B-374 opened for the
addressing-by-identifiers that unblocks it.

### Version 166 — what a probe is

DEC-024 and DEC-025 resolved by D42 and D43 (PR10), which unblocks M3's
framework and its three configuring probes. The characterizing probes are
re-sited beside §XIII with the reason on each row.

### Version 165 — the host stands

B-030 done: killed at every stage it has, coherent each time. M2's exit
criteria are all exercised (roadmap v20); the register keeps what remains
under it honest — B-210's releasing, B-036's deliberate exposure, B-032's
long-lived child.

### Version 164 — the reference model answers

B-032 and B-033 done (F36). The provisioned engine is a supervised subprocess;
the reference model produced text through MCF. M2's first exit criterion in
the form §VI meant it.

### Version 163 — killed mid-stream

B-030 gains the stage serving created: a daemon killed mid-generation with a
model resident, the client saying so and keeping what it got, the next daemon
answering. With D41's zero-tick idle, M2's exit criteria stand at the surface
as it now is; what remains of the milestone is B-033's subprocess half, which
waits for an engine that is a subprocess.

### Version 162 — a model that stays

DEC-018 resolved by D41 (F35): the daemon holds the last model asked for, says
so in every account and in status, and releases it on displacement or stop.

### Version 161 — both halves of D24's latency figure

B-035 done: request to the engine's first token through the daemon measures
120.8 µs at p99 on the fixture, in the window, beside the 13.3 µs round trip.
The core-binary footprint baseline is re-recorded with its reason.

### Version 160 — the served token

B-034 done: the daemon serves generations in PR9's shape, `mcf run` is its
client, the account is recorded. B-032's recorded-condition half done with it;
DEC-018 is answered provisionally by loading per request and saying so.

### Version 159 — distributions against distributions

B-373 done (F34). The oracle's verdict on the forward pass now rests on a
divergence between distributions, measured on a clean engine and two known
defects, rather than on MCF's own margin.

### Version 158 — the last schemes

B-364 done (F33). B-373 opened: the text comparison's threshold has narrowed
three findings in a row, and a comparison of logits is the instrument that
does not narrow.

### Version 157 — a defect the oracle let through

B-364 advanced with five scheme-witness files (F32): Q3_K was decoded wrongly
and the oracle had excused it through the hole F27 named. The oracle's rule now
takes the margin where the texts part; Q5_0 and Q5_1 are decoded; the decoders
no file exercises are listed as such.

### Version 156 — provisioning is done, four runs later

B-367 done (F31). The fourth run stands: self-contained, loads bare, 95 MB, and
the oracle tier agrees 57 of 57 through the prefix MCF built. Removal was
exercised once for real with the reason in the record.

### Version 155 — provisioning is a command

B-367 in progress: `mcf provision` turns F30's manual run into a recorded,
removable, repeatable command (F31). Two new record kinds,
`component_provisioned` and `component_removed`, appended to the kind list so
the derived index's positions hold.

### Version 154 — the ceiling refuses from the header

B-372's refusal half done. The arithmetic lives in the library and the machine
observation at the surface (§3.15); the directory is read from a bounded prefix,
so refusing the reference model costs 0.16 s instead of 72 s of reading a file
the answer does not need; the laboratory produces `resource.memory.exhausted`
through the shipped judgement (A13).

### Version 153 — a controlled environment, measured

DEC-052 resolved by F30: a rootless container over a mounted prefix, decided
against the host route by the host route's own first use — the oracle was built
by a pyenv cmake nobody recorded. B-367 unblocked, with the manual run as its
specification. B-372 opened: the reference model dequantizes to 109.3 GB
against 62.9 usable, so the stand-in must refuse it by arithmetic rather than
be killed mid-load.

### Version 152 — the corpus is six for six

DEC-055 resolved and B-371 done (F29). The bert family embeds through its own
verb, checked against the reference the day it was written: tokenizer exact,
forward pass at a measured cosine floor, meaning by related-against-unrelated
sentences. Every family F21 acquired now runs or embeds through MCF's engine.

### Version 151 — the mask, past the boundary

F28 closes the hole F27 stated: the sliding-window mask is compared at 682
tokens, in a bland history and in one with a distinctive fact parked outside
the sliding blocks' view. Token-for-token agreement both ways.

### Version 150 — the forward pass is no longer unchecked

B-368 done for what it can reach. The forward pass is compared against a
tolerance that was measured rather than assumed (F27): noise divergences sit at
margins of 0.04–0.16 with the reference always choosing MCF's runner-up, and the
one real defect sat at 0.775.

That defect was the sliding-window rotary base, which F24 had flagged as
untested ground: gemma3 rotates five blocks in six at a frequency the file does
not state and the reference defaults to ten thousand. MCF was using the million
stated for the others. The model still answered correctly, which is why nothing
before the oracle caught it.

### Version 149 — the oracle found a defect on its first run

B-368 delivers its first result: 29 of 30 tokenizer comparisons agreed and the
thirtieth was a real defect in MCF — a length guard on user-defined tokens that
MCF invented and no implementation has (F26). Control tokens are no longer
matched in ordinary text, which is the reference's default and the safer
reading of a prompt.

F23's transcriptions are now verified for these vocabularies and texts, which is
what a check establishes and not more.

### Version 148 — an oracle, starting where it can be exact

B-368 in progress. MCF's tokenizer against llama.cpp's at a pinned commit, over
texts chosen for where tokenizers differ. Identifiers are integers, so a
disagreement is a defect rather than a judgement — which is what F20, F22, F24
and F25 have each asked for in turn.

Logits are not compared yet, on purpose: a correct implementation can flip an
argmax on a near-tie through summation order alone, so that comparison needs a
measured tolerance before it can be a verdict.

### Version 147 — the embedding family is a different kind of model

DEC-055 and B-371 opened. `all-MiniLM-L6-v2` refused, and reordering the checks
so the architecture is asked before the vocabulary made the refusal say the
useful thing: it is not that MCF cannot read a `bert` tokenizer, it is that
`bert` is a model with no next token to produce. `mcf run --prompt` has no
question to put to it.

Sequenced after B-368 on purpose. F25 said output quality is no evidence of
correctness, and this is the family least like anything MCF has checked.

### Version 146 — a mixture, and the case for the oracle closes

B-365: five of six. The mixture of experts is found by the file's own
`expert_count` rather than by a family name — the artifact declares `llama`.

B-368 is now the highest-value item in M2, on the strength of F25: deleting the
expert router entirely produces better-reading output than the correct
implementation. Every family added past this point is transcribed and
unverified, and only an oracle changes that.

### Version 145 — a third architecture

B-365 advanced: gemma3 runs, four of six corpus families now. Its two sandwich
normalizations needed no code path, only two more names in the list of optional
tensors — what the file states is read from the file. Its three unobservable
habits went into `architecture.rs` with the rest (F24).

### Version 144 — four expressions where MCF had two

B-365 advanced: SmolLM2 runs, which took a fourth pre-tokenizer and found three
defects in the two MCF had already shipped (F23) — two expressions treated as
one, a third claimed and not implemented, and a fourth assumed for files that
name none. None was findable by running anything.

The corpus tier gained a round-trip over every vocabulary that loads, which
found a fourth defect: the empty string had stopped encoding as anything for
unigram vocabularies. `Split` variants are named for the expression rather than
the family, which is what DEC-053 requires of everything outside the one module
that may name a family.

### Version 143 — the corpus is a tier

B-370 done. `scripts/check-corpus.sh` runs each corpus entry and checks it
against what the register says it does — failing when a runner regresses, when a
refuser starts running, and when a refusal stops naming what it wanted. All five
failure modes verified to fire.

### Version 142 — the question picks the model

DEC-054 restated after F22. Not a preference for small artifacts: each tier
names what it must establish, and that names the artifact. The floor for
refereeing behaviour was measured at between 160M and 0.6B, and the withdrawal
of F21's oracle-dependency claim is recorded with it.

### Version 141 — the reference model was asked to be two things

DEC-054 resolved and B-370 opened, against D40's amendment of §XII. The engine
is developed against a conformance corpus of the smallest trained model per
family — six of them, 1.4 GB against the reference model's 16.5 GB, each a
different quantization ([findings.md](findings.md) F21). Two run; the four that
refuse name what they want, and that list is B-365's remaining order of work.

The cost is recorded with the decision: a small model cannot referee what F20
turned on, so B-368's oracle moves ahead of the remaining families rather than
after them.

### Version 140 — what B28 forbids, exactly

DEC-053 resolved. Reading `general.architecture` is not recognizing an artifact,
and B28's check now distinguishes them: a family may be named in one declared
module, which may name no publisher, no artifact, and no function taking a name
and answering yes or no. B-365 advanced to in progress — llama and qwen3 both
run against real files (F20).

### Version 139 — a real model, and the two defects it found in an hour

The operator observed that their machine already holds a hundred gigabytes of
models acquired through another tool, three of which declare the architecture
MCF's engine implements. Running one is the cheapest test there is, and it found
two defects in an hour ([findings.md](findings.md) F19).

The tokenizer used SentencePiece's unigram *search* where these files are read
by a *merge*. In a vocabulary whose byte tokens score zero and whose words score
minus thousands — these scores are ranks, not log-probabilities — that search
spells every word out one byte at a time, which is why a five-word prompt was
twenty-six tokens.

And `Q6_K` decoded every value correctly into the wrong place: the format writes
the four values a byte-pair produces at strides of thirty-two, and MCF wrote them
consecutively. In that file the only `Q6_K` tensor is the projection to logits,
so a correct forward pass came out as noise. `Q2_K` and `Q3_K` had the same class
of error and were rewritten against the source.

Both had tests. Neither test could fail: a block where every value is identical
cannot show a permutation, and a fixture vocabulary of whole words with no
intermediate pieces cannot be tokenized by the correct algorithm at all. Both
fixtures are realistic now, and the decoder tests give every position a distinct
value.

MCF's own engine now answers *The capital of France is* with *Paris, but the
largest city is Marseille*.

### Version 138 — the reference model is here, and its file reads

B-019: `Qwen3.8-27B-UD-Q4_K_M.gguf` is on the content drive, 16.5 GB, acquired
by MCF from the real hub, verified against the hub's declared digest, pinned at
its revision, with a provenance that records §XII's hard case honestly — the
upstream weights are named, and MCF says plainly that it has not fetched them
and cannot vouch for them.

B-364 got most of the way in one sitting, because the file said what it needed:
five schemes MCF could not read, 128 of 866 tensors in three of them. The K
family went in first, then the non-linear family — and that one needed
codebooks, which are data rather than logic. They are transcribed from the
format's own source under MIT with the compatibility finding B-330 requires,
because a table cannot be derived and one MCF invented would decode a hundred
and twenty-eight tensors into confident nonsense.

Every decoder's test is computed by hand from the format's sentence rather than
recorded from the decoder, since a test that asks the decoder what it produces
proves nothing. What that does *not* establish is that the reading of the format
is right — decoder and test come from the same reading — which is what B-368's
oracle is for.

### Version 137 — where the large files go is the operator's choice

B-369, on the operator's instruction: a model store is not where MCF happens to
be installed, and one machine may want several — a fast small disk and a slow
large one are two places a model can belong, and which one it belongs in is a
decision at acquisition time rather than a setting.

`MCF_MODELS` holds an ordered list of absolute paths. The first is where a new
acquisition goes; all of them are searched for what is held; `--into` overrides
for one acquisition and may name somewhere MCF was never told about. It is an
environment variable rather than a configuration file because §5 refuses MCF a
configuration language, and a path list in a variable is the platform's own
idiom rather than a language.

Two details are the interesting ones. A relative path in the list is **dropped
and named** rather than resolved against whatever directory MCF was started in
— a data path that depends on the caller's working directory is a data path that
moves. And a model held in **two** stores is refused with both paths rather than
resolved first-wins: two files under one name are two artifacts with two
provenances, and a measurement against whichever MCF reached first is one nobody
could reproduce.

`mcf doctor` shows the stores, which one is next, and how much room each has —
asking the filesystem that *would* hold a store that does not exist yet, because
that is the number somebody deciding where to put sixteen gigabytes wants.

### Version 136 — the serving surface is decided

DEC-001 decided: [PR9](proposals.md#pr9--what-serving-looks-like) accepted as
proposed. It has been M2's gate since M2 was written, and the thing that decided
it was not ergonomics but what a schema has room for — the shape every client
speaks has no field for the engine build, the seed, or *this answer is degraded*,
and a surface that cannot carry the mark strips it.

One thing changed under it while it sat: D38 makes MCF's own engine the thing
being served, so the degradation mark the protocol carries stops being a
hypothetical. Every answer from MCF's own engine has one.

### Version 135 — the engine is ours, and setup is a thing MCF may do

Two instructions from the operator, written into the intent document as D38 and
D39, and this is what they do to the register.

**MCF's own engine becomes the one that runs everything.** B-360 grows from *a
model no vendored engine will run reaches a first token* to *every model MCF can
acquire does*, and three items appear underneath it: quantization coverage
(B-364), architecture coverage (B-365) and threads that do not change the answer
(B-366). The immediate gap is embarrassing and worth naming: MCF decodes five
quantization schemes and **not one of them is a scheme its own reference model
publishes**.

**An engine may be provisioned rather than shipped.** B-320 stops being a
question about what can be vendored — the musl-toolchain wall three findings
kept hitting is no longer what decides it — and B-367 appears for the mechanism,
with DEC-052 open on what *controlled* concretely means. B-368 is the part that
justifies the weight: a reference implementation MCF can provision is an oracle
to check MCF's own engine against, which is what A19 has wanted since it was
written and what D31 could only half-answer with a second implementation MCF
also wrote.

The self-contained artifact is untouched. What ships is still only what is
vendored, and MCF's own engine is the baseline that needs nothing installed.

### Version 134 — an unreachable hub is not a clean bill of health

Found by pointing `mcf check` at a port nothing listens on. Two things were
wrong, and both flattered MCF.

The finding said *that is the same answer it gives for a repository that is
private and one that never existed* — F17's sentence, which is about a **401**
— for a connection that was refused. A network failure does not license a
statement about what might be in a repository (A7).

And the summary counted the unanswered question among the checked, so the run
ended with *1 checked upstream, and nothing there has changed* when nothing had
been checked at all. Asked and answered are different counts now, and a run
where every question went unanswered says so.

### Version 133 — a check that outlived the state it was written for

`the_licence_is_what_it_says` asserted that the artifact's vendored list and
`doc/vendored.md` agreed about *whether anything is vendored*. That was the
right check when both were empty, and it has been passing vacuously since the
day fourteen crates were admitted: two lists of fourteen different things would
have satisfied it.

It compares them properly now — name by name and revision by revision — and was
verified by adding a crate to one list and not the other. B-330's condition is
that no component ships without a recorded compatibility finding, and a check
that only counts cannot hold it.

### Version 132 — a check that verified and said nothing

`mcf check --here` re-read every artifact against its recorded digest, told the
operator, and wrote nothing down: no record entry, no account. So *when was this
last known to be fine* — the question D37 exists for — had no answer for the
half of the check that needs no network.

Now every check records what it found, including that nothing was wrong, and the
entry keeps the two halves apart: the bytes as `matched`, `changed` or *no digest
was recorded to compare against*, and the upstream as its finding or `null` when
nobody asked. *Checked and fine* and *not checked* are different facts, and an
entry that ran them together would say the upstream was fine when nobody looked
(A7).

The provenance sidecar is unchanged by a `--here` check, and that is deliberate:
what the bytes did is an event, and where they came from is a property. Only the
second belongs beside the file.

### Version 131 — the engine MCF wrote, on somebody else's weights

Found by running the lifecycle by hand: MCF acquired a 260-thousand-parameter
model from the real hub and its own stand-in produced *. It was a big, shiny
blueble. The blue was very smarty.* — which for a model that size is exactly
what it should look like.

That had never been automated. The stand-in's tests all run a fixture the
laboratory writes, which is MCF checking its own arithmetic against its own
file; A19 asks for the other kind. The online check now runs what it acquired
and asserts the answer carries its mark and its sentence about what it cannot
be, at no extra download.

### Version 130 — the same reason, on the other side of an acquisition

`mcf pull` says why it cannot plan for a repository *before* it fetches
anything — that was v116's fix — and after the file arrived it still said the
generic sentence: *that needs the model's own configuration and this machine's
free memory*. Same question, same command, two different answers depending on
which side of the transfer you were on.

It now carries the reason through: *this repository publishes no configuration,
and a plan needs one*, which for a repository of deliberately tiny models is the
whole truth and is actionable in a way the generic sentence is not.

### Version 129 — the check that broke the tier it was protecting

The gating check added in v125 — every mutation in the catalogue can still be
placed — broke the mutation tier on its first run, and the way it broke is worth
recording.

The tier runs the **whole suite** against a mutated copy of the tree. For the
entry under test the original line is gone, so a check that reads the source and
compares it with the catalogue fails there. The runner read that failure as *the
suite killed the equivalent-mutant control*, concluded it could not tell a
killed mutant from a broken copy, and refused to produce a score — which is
exactly the right refusal and exactly the wrong cause.

The check now asserts each entry describes its file **either as written or as
mutated**, which is true in a clean tree, true inside the runner's copy, and
false when somebody moves the line. Verified in all three states, including the
control's own file, which the catalogue also names.

A check that reads source is a check that runs inside the mutant. That is worth
remembering before writing the next one.

### Version 128 — a column that ran into the next one

Still reading the surfaces as a stranger would. `mcf explain`'s defaults table
had a value exactly as wide as its column, so one row read `32 unless --limit
saystokens rather than seconds` — two columns with nothing between them — and
three rows ran past 130 characters, which is past any terminal.

Fixed by a space the column cannot eat and a wrap with a hanging indent, and
asserted: no line in the report exceeds 120 characters and the row that used to
collide has its space. §3.15 asks that every default be visible with its source;
a table a reader has to decode is not visible.

### Version 127 — the report knew about a daemon that had arrived

Same sweep, one more stale sentence. `mcf doctor` listed four D24 figures as
*not measurable here* because *there is no daemon until M2* — and there is one,
and two of those figures have since been measured against it (zero wakeups over
a minute, 10.4 µs of added latency at p99).

The reason it does not measure them is real and is now the reason given: they
are figures about a *running* daemon, and a report that started one to measure
it would be changing the machine it is describing. The tiers that measure them
start a daemon of their own.

A test asserts the old sentence cannot come back, in the same shape as the one
guarding the usage text: what MCF says it cannot do is read by somebody deciding
whether to try (D7), so a stale *cannot* is a defect rather than an
infelicity.

### Version 126 — reading the surfaces as a stranger would

Loop step five, done by running the binary and reading what it says rather than
by grepping for stale citations. Two things were wrong.

The usage text still said an encrypted hub *needs a TLS stack MCF has not
vendored yet*, weeks after B-322 vendored one and after `mcf pull` had acquired
from the real hub over it. A stale *cannot* is worse than a missing line,
because it is read by somebody deciding whether to try something (D7). It now
says what MCF actually cannot do — serve a model, time one — and mentions
`mcf-helper`, which an operator would otherwise never learn exists.

And `mcf check` called a machine that has never acquired anything *the model
store could not be read*, where `mcf list` calls it *does not exist yet*. Two
answers to one situation (A6), and the failing one was the wrong one: a store
nobody has created is not a store that cannot be read. Both are asserted now, on
every surface that looks at the store.

### Version 125 — the catalogue that stopped describing the code

The scheduled tiers finally got the machine. Four of five are green on a recent
source, and the budget tier's figures were taken in the exclusive window at
last: added latency **10.4 µs at p99** against a 5 ms ceiling, cold start
1.22 ms against 100 ms, the binary 3.83 MiB against 40 MiB, resident memory
8.1 MiB against 20 MiB.

The fifth found something, and it was mine. B-300's rewrite of the journal
replay moved a line the mutation catalogue names, so the tier could not place
that mutant and stopped with `cannot check` rather than a score — thirteen hours
after the refactor that caused it.

The entry is fixed, and the class of failure is now caught where it belongs: a
gating check reads the script's own arrays and asserts every `find` still
matches exactly one line in the file it names. It costs eighteen file reads and
it tells whoever moved the line while they still remember why. What it
deliberately does not check is that the mutants are still *killed* — that is the
scheduled tier's whole job, and it costs a suite run each.

### Version 124 — the property that nearly passed on nothing

Two properties added for what the last stretch built: the index agrees with a
replay about every journal, and a provenance carrying any shape of upstream
observation survives the record.

The first was written wrong and the fix is the interesting part. As first
written it opened the index twice and compared both against a replay — and it
passed with the offsets deliberately corrupted, because a corrupted index fails
its own staleness check and is *rebuilt*, so the second open handed back
freshly-correct values and the file was never read. The property now asserts
that the second open **loaded** what the first wrote. Without that line it was a
property about an index that need never have been written to disk at all.

Both were checked by breaking the code they are about: the round trip falsifies
at case 0 when the encoder drops an observation, and the index property
falsifies at case 0 when a byte offset is written one too high.

### Version 123 — the wall nobody priced

Three findings have each stopped at the same absence — no C compiler targeting
musl on this machine — and each treated it as a fact of the world. Asking the
package manager takes a second and nobody had: it is packaged, at about nine
megabytes installed ([findings.md](findings.md) F18).

Nothing was installed. What it changes is not the answer to B-320 but the
question: not whether a musl artifact with a C dependency is *possible* here,
but whether MCF is willing to require a build-time toolchain — which B36 does
not forbid, since it constrains what a user needs rather than what a build does.

### Version 122 — four more mutants, one per finding

The mutation catalogue is the test of the tests, and the code written in the
last stretch had none of it. Four entries added, and they are chosen the way the
existing fourteen were: each mutates something a rule — or in these cases a
*finding* — depends on.

Two of them are the defects F16 and F17 already caught once, frozen so they
cannot come back quietly: counting every block rather than only those that cache
(the fourfold KV cache), and reporting a hub that will not answer as a decay.
The other two are the invariants the helper and the index rest on: a governor
value chosen from the machine's own list rather than from an argument, and an
index thrown away when it describes a journal it does not cover.

Each was applied by hand and checked to compile and to be killed before it was
admitted, because a catalogue entry that does not compile is the compiler
noticing rather than the tests.

### Version 121 — the other half of the same question

Writing `mcf check` surfaced a claim that was not true. Its output said that an
artifact's own integrity is `mcf list`'s business, and it is not: B-301 built
the streaming re-verification and nothing ever called it. A capability MCF has
and an operator cannot ask for is exactly what A22 forbids.

So the command asks both halves of the one question — *is what I hold still what
it should be* — of the two things that can change independently. `--here`
re-reads the bytes against the digest recorded for them and touches no network;
without it, the hub is asked as well. The verdicts stay apart deliberately:
corruption is a fact about this disk and a decay is a fact about somebody else's
server, and running them together would invite a reader to think one caused the
other.

### Version 120 — looking upstream at what you already have

B-331. `mcf check` asks the repository an artifact came from whether it still
says what it said, compares against what was written down at acquisition, and
writes the answer down twice — beside the artifact and in the record.

Four decays, four findings, four tests. The one that took the most care is the
fifth case: a hub that will not answer. F17 measured that private, withdrawn and
never-existed are one answer, so `unreachable` is reported as what was observed
and is deliberately **not** a change — a hub declining to speak says nothing
about whether anything changed, and reporting an absence as an event is what A7
forbids.

Two things follow from D37 and are visible in the output. Nothing is
invalidated: the artifact is here, its digest is what it was, and the surface
says so rather than leaving an operator to wonder whether their measurements
just died. And *checked and unchanged* is written down, because a record that
only kept the bad news could not answer when something was last known to be
fine.

The record gains an entry kind — `artifact_checked` — which is a change to a
public shape (§7.30) and is why the index carries its kind dictionary: an index
written by a build that knew eight kinds is rebuilt rather than misread by one
that knows nine.

### Version 119 — a pin that goes bad underneath you

DEC-038 decided. What MCF can detect about a decayed pin is a property of the
hub rather than of MCF, so the hub was asked ([findings.md](findings.md) F17),
and the measurement constrained the decision more than it confirmed it.

A withdrawn revision is unambiguous, a gate that closed is visible in the card
while the file refuses, and a relicensing or a repointed tag refuses nothing at
all — a changed field beside a 200, findable only by comparing with what was
recorded at acquisition. But a repository that does not answer is
indistinguishable from a private one and from one that never existed: the hub
answers 401 to all three, with the body *Invalid username or password*.

So MCF's refusal on a 401 changed. It used to advise supplying a credential,
which is right for one of the three cases and misleading for the other two; it
now says what was observed and names the question it is not answering.

D37 also settles the part that matters most: decay is not retroactive. The
artifact is here and its digest is verifiable, and greying out local
measurements because somebody else deleted something would be destroying
evidence for a reason that is not scientific.

### Version 118 — the serving surface, argued rather than assumed

DEC-001 is M2's gate and it has been a single line since it was written. It is
now argued in full as [PR9](proposals.md#pr9--what-serving-looks-like), because
its three questions — the API surface, what happens when a runtime dies, and
whether two models may be resident — are one question wearing three hats.

The argument that decides the surface is not ergonomics but accountability: the
industry-standard shape has no field for the engine build, the seed, or *this
came from a stand-in and is marked degraded*, and a surface that cannot carry
the mark is one that strips it. So the proposal extends the protocol `mcf
status` already speaks and puts the conditions in the terminating line, leaving
a compatible adapter as a later and explicitly lossy decision.

The decision stays open: a proposal is where a recommendation waits for the
person whose project it is.

### Version 117 — the online tier meets a repository with a shape

F16's own conclusion was that the online check earns its place and does not go
far enough: it acquires 1.2 MB from a repository of tiny models, which exercises
the wire and says nothing about the *variety* of what a hub publishes. Every one
of the three defects it found was in code with tests, and none of them was
reachable from a tiny model.

So the check now also plans for the reference repository, which fetches no bytes
— a listing and a `config.json` — and asserts that every variant published comes
back classified. A plan missing a row nobody mentioned is worse than no plan.

### Version 116 — three defects, found by pointing MCF at a real repository

The operator asked which choices were still theirs, and one was which
quantization of the reference model to pin. MCF exists to answer that without
downloading anything, so the planner was pointed at the real repository rather
than the sizes being guessed at. It could not answer, and finding out why cost
three fixes ([findings.md](findings.md) F16).

The JSON reader refused the repository's configuration over `1e-06`. It was
written to keep floating point out of the record — the right property, and A6
still holds it — but the same reader reads documents MCF did not write, and §3.7
makes those untrusted input rather than MCF's own format. A number this format
does not carry is now kept byte for byte and is not a quantity to anybody who
asks; `checks/tests/no_float_reaches_the_record.rs` holds the property the
parser used to hold by accident.

The configuration was not where MCF looked: a multimodal repository publishes
one file describing several models, with the transformer's fields under
`text_config`.

And the third would have produced a wrong number rather than no number, which
makes it the worst: the model's 64 blocks include only 16 that hold a key/value
cache, and counting all of them overstates the cache fourfold — a gigabyte
instead of 256 MiB at 4096 tokens, which on a 16 GB accelerator is the
difference between a variant fitting and not.

### Version 115 — a first token on a machine with nothing on it

B-183 has been half-met since the from-scratch container was built: everything
except a first token ran in an image holding the binary and nothing else. The
missing half looked like it needed B-019's fifteen gigabytes, and it did not —
it needed a model file, and D31 put an engine in MCF that will read a small one.

So the machine that has a toolchain writes out the laboratory's four-token
fixture, the image is handed the bytes, and MCF produces text with no libc, no
shell, no package manager, no `/etc` and no `/tmp`. That is the whole path —
file, vocabulary, forward pass, sampler, tokens, text — demonstrating what B36
claims rather than arguing it.

Both caveats travel with the result, in the script's own output: the fixture is
not a model, and a stand-in's answer is never a speed.

### Version 114 — being hostile, rather than promising not to be

B-180. The row asks that untrusted code fail to reach an elevated path
*asserted by scenario rather than by policy*, and the distinction is the whole
value of it: a rule saying MCF does not shell out to `sudo` is worth nothing,
because the question is whether a hostile artifact can make it.

So the privileged programs are made observable — a `PATH` of sentinels that
record being started and exit — and then MCF is handed everything worth trying:
a model whose metadata is a shell command and a governor name, a store whose
file names are helper arguments, references that climb out of the store. Every
surface that touches an artifact is run against them. Nothing starts, and the
hostile strings come back exactly as written.

A fourth test starts a sentinel deliberately, because *nothing happened* is not
evidence unless something could have.

### Version 113 — the only program with rights

B-190. §6.32 has asked since it was written for a privileged surface that is
enumerable, auditable and short; D35 enumerated it, and this is that list as a
program: `mcf-helper`, three operations, performed and then gone.

What makes it auditable is what it does *not* have. It links `mcf-core` and
nothing else. It reads no environment — the arguments are the whole input. It
writes to no record, because a privileged program that writes to the operator's
record is a second thing to audit. And nothing links it back: the daemon starts
it as a process, which is what keeps the daemon unprivileged rather than a
promise that it is.

Two details are worth the reading. The value written to a governor is chosen
from the machine's own list of offered governors rather than passed through from
an argument — a helper that writes what it is told is a helper that writes
anything. And a failure part-way through is *partial* and names what was already
changed, because a restoration that does not know what was altered cannot be
made (A27).

Three scenarios drive it through a fixture root, which is how a laboratory
watches a privileged program work without letting it near the machine.

### Version 112 — the boundary, drawn where the parts now are

DEC-022 decided. §7.22 asked four questions about what whole-system testing
means for a daemon, and when it was written three of the four things it asked
about did not exist. They do now, so each answer is demonstrated by a test
rather than argued: `mcf pull` over loopback HTTP against a hub the laboratory
can make hostile, `mcf run` end to end through the engine MCF ships, the daemon
started and killed at eight moments of its life, and recovery read back off the
disk.

The rule they share is the useful part: **nothing on MCF's side is mocked**. A
simulated component appears only to produce a failure that is hard to cause on
purpose, which is D26's rule rather than a convenience.

M0 has no open gating decisions left.

### Version 111 — what a machine actually makes you ask for

DEC-039 decided. §6.32 has wanted the privileged surface enumerated since it was
written, and §7.39 recorded that nobody had done it; documentation cannot,
because the answer differs by kernel, distribution and how a machine was set up.
So a probe asked one — reading, and testing permission the way `open` does,
without changing a thing.

The answers were not the expected ones. Pinning cores and bounding memory need
no privilege at all, because cgroup v2 delegates them to the user's own slice.
Per-process accelerator occupancy is readable, so PR5's contention snapshot is
unprivileged. But *reading processor energy* is not — the counters were made
root-only after they were shown to leak what a machine is doing — and D11 makes
energy first-class, so a helper has to exist for a read.

The probe also got its own first row wrong in a useful way: `nice -n -5 true`
exits zero having failed, because the shell reports the command's status rather
than whether the priority was applied. Asking the child what it actually ran at
reverses the answer. A declaration is not an observation, at the level of a
shell script.

D35 has the list and the five things MCF declines to do to a machine at all.
B-190 is unblocked.

### Version 110 — the terms, where the model is

B-023 asks that a licence be surfaced *before use*. `mcf pull` has said it since
the transport landed, but the place an operator sees a model before using one is
`mcf list`, and the place they decide whether to run it is `mcf explain` —
neither of which said anything about terms.

Both do now, in the sentence `mcf_hub::licence::describe` already wrote for
`pull`: one way to say a thing, whichever surface says it. A model whose
provenance cannot be read says its terms are unknown rather than leaving the
line out, because an absent line reads as *no restrictions*.

What is still open under the item is the *forbidden use* half — whether a
licence forbids publishing measurements about a model — which is DEC-036 and
needs a decision rather than a surface.

### Version 109 — the daemon, killed at every stage

B-030's condition asked for the lab to kill the daemon at every lifecycle stage
and find it coherent and queryable each time. The stages a daemon has today are
the ones it can be killed *in*, and it is now killed in all of them: before the
socket exists, during the append that records it started, idle in `accept`, and
after it has answered.

What is asserted afterwards is what an operator would need to be true. The
socket a dead process left is taken over rather than refused. The record opens —
whole, or naming a loss that is inside the file rather than a shorter history.
And a new daemon answers `mcf status` with what it recovered.

The item stays open for what a daemon *holds*: a runtime to drain and an
accelerator to release are B-032's, and there is no engine to hold yet.

### Version 108 — what MCF interposes, as far as it goes

B-035's figure is D24's *added latency, request to the engine's first token*,
and half of it cannot be measured: there is no engine. The half that can be is
now measured and defended — a real `mcf serve` process, asked a hundred
questions from outside, timed from the write of the request to the read of the
answer, read against the 5 ms ceiling at p99 the way D27 says an event-class
figure is read.

What makes it honest rather than flattering is that the reading carries what it
leaves out, and the tier prints those lines beside the number: the engine's own
latency and the hand-off to it, model residency, and a network hop. A number
compared against D24's ceiling without them would be claiming to be the whole of
the thing D24 named.

### Version 107 — everybody writes, and says who they are

DEC-037 decided and B-332 done, in the shape the evidence pointed at rather than
the one the item was written in. The item said *a single writer*; F13 had
already measured that concurrent appends of whole lines do not tear, so the
alarming half of §7.37 was not real. What was real was the identifier: every
writer counted its own appends from zero, so two programs recording the same
kind of event in the same second minted one identifier for two events.

An identifier now carries a token for the writer that minted it, and only a
journal can mint one — an entry that has not been appended has none, and the
type says so. The load tier asserts uniqueness across four concurrent writers
alongside the absence of a torn line.

One defect fell out on the way: a replay was *recomputing* each identifier from
the envelope with a sequence number of zero, so `mcf log` showed identifiers the
record does not contain. An identifier is now read back exactly as written (A1).

D34 in the intent document has the reasoning, and §7.37 joins the answered table
where its number stays citable.

### Version 106 — the index that earned its bytes, and the database that did not

B-300 and B-042 are one question wearing two hats. D6 said the record is a
SQLite database; D20, written later, said the record is an append-only journal
with a derived index over it. Both cannot be the first thing MCF reaches for.

It was settled by measurement rather than by preference. A replay costs 8 µs an
entry — 7.89 s and 381 MiB at a million entries, paid at every daemon start —
and the queries a record actually gets are answered in 196 µs from 32 bytes an
entry. SQLite answers them in the same order of time and would cost 9.2 MiB of
C, 52 seconds of compile, and B-183's static musl container, which cannot be
built here at all for want of a C cross toolchain — the same wall F12 found for
both engine candidates. [findings.md](findings.md) F14 has both halves;
`prototypes/record-index/measure.sh` and
`cargo test -p mcf-record --test how_the_record_grows` re-take them.

So D6 is amended, the index is MCF's own, and what it holds is where an entry
is rather than what it says. B-042's condition is met by the journal it was
written about a different mechanism for: a format version before the first
entry, a truncated write that is a classified failure the record reopens after,
and a file a machine with no MCF on it can still read.

### Version 105 — one way to say a refusal

Five commands had grown five ways to print the same failure: `mcf pull`, `mcf
run`, `mcf serve`, `mcf list`/`mcf rm` and `mcf log` each assembled the failure
line and its context entries slightly differently, so that one refusal read
differently depending on which command produced it.

`Failure`'s `Display` is deliberately one line and deliberately without context
(C1: the record keeps the structure, a surface builds the view). That is right,
and it meant every surface rebuilt the actionable form by hand. Rebuilt once
now, in `crates/mcf-cli/src/say.rs`, and the chain of causes is printed rather
than dropped — the disk being full is what explains the model not arriving, and
a reader given only the second is guessing at the first (A1).

No behaviour changed except that a cause is now shown where four of the five
renderers had silently discarded it.

### Version 104 — an identifier that was already taken

B-363 is what B-333 should have been called. The register already had a B-333 —
workflow declaration, blocked on DEC-044 — and a second item took the number,
which is a register that cannot be cited: two rows, one identifier, and every
reference to it ambiguous.

Renamed, and the hole it went through is closed. `documents_conform` checked
that every citation *resolves* and never that every identifier is *unique*, so
the one thing a register must be was the one thing nothing asserted. It does
now, for the decisions and the build items alike.

### Version 103 — the record reads back

B-363 added and done. MCF has written to the record since M0 — machine
profiles, self-cost figures, failures, acquisitions, removals, the daemon's own
life — and the only way to read it was to open the file. §3.3 ranks
machine-readability first and legibility second, and second is not omitted; A22
makes the headless surface the complete one.

`mcf log` gives each kind its own sentence with the field a reader wants first,
`--full` gives the record's own JSON for anything that is not a person, and
`--kind` says what it counted so a filtered view cannot be mistaken for the
whole.

The part that matters is what it does with damage. A replay is not a `cat`: a
crash mid-append leaves a torn last line, and the log reports it at the end
while still showing everything that came before (B62). A whole-system test tears
a real record and asserts both halves — what was whole was read, and what was
not was said.

### Version 102 — a declaration is not an observation

B-050 done. §3.18 forbids MCF treating a model card's claims as facts, and the
rule is now a type: two separately optional halves, read as `declaration()` and
`observation()`, with no operation between them. There is no `unwrap_or`,
because a capability with a default is a capability whose absence looks like a
value — A7's substitution arriving through a method signature.

Three states, derived rather than stored: unknown, declared, verified. The
fourth situation is *diverged*, and it is B-058's finding rather than a fourth
kind of knowing — a model whose card says one thing and whose weights say
another is not a model with bad metadata, it is a model somebody should look at
before measuring anything on it.

Adopting it in `mcf_hub::inspect` found a real loss. The enum it replaced turned
*a card MCF believed and weights it could not read* into **unknown**, throwing
away what the card said — a small A1 violation that the type makes
unrepresentable, because a declaration with no observation is exactly the state
`Declared` exists for.

### Version 101 — the defaults, and the questions MCF cannot answer

B-038 in progress. `mcf explain <model>` puts everything MCF knows about a model
into three columns and says which each line is: **declared** by the file,
**read** from the bytes, or **chosen** by MCF. §3.15 forbids hidden choices and
A21 forbids treating a declaration as a fact, so a table whose provenance a
reader has to guess is the thing this replaces.

Every chosen line names where it is written down — `crates/mcf-cli/src/run.rs`
for the sampler, D19 for the seed, B49 for the token budget — so that disagreeing
with a default means opening a file rather than guessing at an intention.

The last section is the one worth having. It lists the questions MCF has no
basis to answer, with the reason and the milestone that would earn each: which
quantization to run (DEC-002 and M5–M7), how fast it is (B65 — a timing from the
stand-in measures the stand-in), what it is good at (M6, gated on capabilities
M3 verifies). A defaults screen that listed only what MCF chose would imply it
had a basis for choosing, which is exactly the fabricated report C7 is written
against.

### Version 100 — a model on this machine answers something

B-040 in progress, in the half MCF can do honestly. `mcf run <model> --prompt
<text>` drives the whole path — model file, vocabulary, forward pass, sampler,
tokens, text — through MCF's own stand-in engine, which D31 put there so that a
model no vendored engine will run still runs, *marked*.

The marking is the point rather than a caveat. B65 forbids a stand-in from
producing a speed; the type refuses to hand over a bare result; and the surface
prints the mark beside the answer with a sentence saying what it cannot be. A
tool that showed the tokens and left the reader to wonder which engine produced
them would be the demo this project exists not to be.

What arrives with the answer is what somebody would need to reproduce it: the
model, the sampler, the seed, the token budget, the engine and its build. §3.4's
habit at the smallest scale, and the reason `--seed` is a flag rather than a
default nobody sees.

The laboratory gained a fixture with it — a four-token, one-block model whose
embedding table is one-hot, so greedy decoding repeats what it is given and the
answer is something a person can state in advance. A whole-system test asks it
`yes` and expects ` yes yes yes`, which is the smallest end-to-end claim MCF can
make about inference and is now made as a process.

### Version 99 — two writers, measured before they were argued about

The daemon gave MCF a second thing that writes to the record, and §7.37 has
never said who may. Rather than reach for a lock,
[findings.md](findings.md) F13 measured what the platform gives with none:
eight processes appending whole lines to one journal, at three sizes up to
128 KiB, on both filesystems that matter here. Sixteen thousand lines every
time, none torn, none interleaved — and the load tier now asserts it, which is
half of B-332's condition met before its decision.

What the measurement leaves is narrower and sharper than the question that
prompted it. Not *how do we coordinate writes* — the cheap arrangement holds —
but *who mints an identifier*: each writer counts its own appends, so two
processes can produce the same `EntryId`. Nothing is lost and nothing is
unreadable; what is broken is the assumption that an identifier names one entry.
DEC-037 now has evidence to be decided against rather than guessed at.

### Version 98 — the daemon answers, and its stopping leaves a trace

Two gaps closed on the daemon, both small and both the kind that matter.

`mcf status` exists. The daemon has been able to say what it is and what it is
holding since it was written, and no command could ask — which is precisely the
capability-reachable-only-through-a-client that A22 forbids. It now reports the
build, how long it has been up, what it recovered, what it is holding, and what
it cannot do.

And starting and stopping are recorded. A process that can only be killed leaves
no account of why it stopped; `daemon_stopped` carries the reason the operator
gave, and `daemon_started` carries what was recovered — so *MCF was up between
these two moments* is answerable, which is a condition of anything measured in
between (§3.4). Both are events rather than ticks, and the idle measurement
still reads zero: sixty seconds, no context switches, no processor time, record
unchanged.

A record that cannot be written does not stop the daemon. It is reported and MCF
carries on, because a machine with a full disk still wants MCF up — A4's shape,
with A2's requirement that the loss be said rather than swallowed.

### Version 97 — four gates, two of them absences

B-039 done. §6.14 draws the line at *category rather than frequency* — untrusted
execution, large irrecoverable resource use, network exposure and destruction
are asked about every time, and everything else flows — and the four are now
enumerable in code, each saying where MCF asks.

Two are commands, and on a headless surface a command that names what it will do
is a better record of consent than a prompt anybody would click through: `mcf
rm` will not destroy without a stated reason and deletes nothing at all without
`--purge`; `mcf pull` acquires only what was named, and a repository asked for
without a file is answered rather than fetched.

The other two are *absences*, and they are the ones the check earns its place
on. MCF runs nothing it acquires and listens on nothing another machine could
reach — the strongest statements in the file and the easiest to falsify by
accident. A `TcpListener` added for a convenience now fails in
`checks/tests/the_four_gates.rs` rather than in review, and the loopback
listener the laboratory needs to test MCF's own client is declared with what it
is for, the way every deletion and every spawn already is.

### Version 96 — the engine's numbers, and the test that did not decide it

F12's second half, measured in the exclusive window. Both candidates build for
the ordinary target: llama.cpp produces 12.9 MiB of static libraries — a third
of D24's ceiling before the linker drops anything — and a minimal candle program
is 1.6 MiB needing only the three libraries §3a allows.

The finding worth having is the one that went the other way. F9.5 chose the TLS
provider on the musl target: the C candidate could not build for it, the Rust
one could, and an existing check stayed runnable everywhere. The same test was
expected to choose the engine. It does not — `candle-core` depends on
`tokenizers`, which depends on Oniguruma, which is C — so neither candidate
keeps B-183's container runnable without a cross toolchain.

That leaves the choice resting on things numbers do not settle: one upstream to
account for against a hundred and forty-three, and a second tokenizer MCF
already owns by D31. B-320 stays open on purpose. This is the largest single
thing MCF will ship, the register asks for a finding before an admission, and
the finding now exists.

### Version 95 — the engine question, opened with numbers rather than opinions

B-320's measuring has started. D32 settled that MCF delegates inference; §XVI
settles that what it delegates to must be vendorable; and F9 established the way
that question gets answered here — by asking the machine.

The cheap half is measured. llama.cpp is 464,000 lines of C++ from one project
under one licence, 35 MiB of what MCF would actually ship. candle is 152 crates
of Rust under many licences, 119 MiB before filtering. Neither is obviously the
smaller obligation: one is more code to read and less bookkeeping, the other the
reverse.

The half that decides it is unmeasured and the finding says so. Whether either
builds for the musl target B-183's container uses, and what each does to D24's
40 MiB ceiling, both need several minutes of a quiet machine — and on a machine
four projects share, taking it for a build of that size is a thing to schedule
rather than to grab. The script is committed; the table has a hole in it with
the shape of what is missing.

### Version 94 — the daemon, and an idle cost of zero

M2's foundation. D1 settled that MCF is a process with clients attached; this is
that process, and it cannot serve a model — there is no engine — and says so
when asked, which is the honest shape of a daemon that exists before the thing
it will host.

Three things about it are worth stating. It keeps **no state a crash could
lose**: what it knows on start is what the record and the model store say, read
fresh, so recovering across a restart is a property of the disk rather than of a
memory — and a damaged record is recovered *and said*. It is **local by
construction**: a Unix socket with no bind address, no port and no flag, so
exposure is not something a mistake can do because it is not something MCF can
do. And **nothing a client says can stop it**: a request that is not one gets a
classified answer and the daemon stays up.

The part worth the most is B-031, and it is a measurement rather than a design
note. D24 states idle cost as a prohibition — zero timer wakeups — and the soak
tier now runs a real daemon for the minute D24 names, with nobody talking to it,
and reads what the kernel keeps: **zero context switches, zero clock ticks of
processor time**, and a record byte-for-byte unchanged. The shape is what makes
it true, and the tier is what makes it a claim.

B-004's stated condition closes with it: it has said *a running idle daemon
writes zero records* since M0 and there was no daemon to measure.

### Version 93 — the plan, asked again once the model is here

B-213 done and B-028 with it. The plan was always made from what a hub declares,
which is the only thing available before a download; PR3's other half is that it
be *re-checked against reality on acquisition*, and it now is. The verdict for
the file that arrived is printed beside the acquisition, computed from the
machine as it is at that moment rather than as it was when the plan was made.

B-028 is done because there are two simulated hubs now and the second one was
not planned. `mcf_lab::hub` answers the four questions the `Source` interface
asks; `mcf_lab::serving` answers them over a socket, which became necessary the
day MCF gained code that talks to an operating system. Both simulate what MCF
observes and never the cause, and every M1 test runs against one of them with no
network — including the whole acquisition path, end to end.

### Version 92 — a full disk, decided rather than discovered

B-026 done, and it needed a measurement to be done properly. Writing to a full
filesystem through a buffered writer *succeeds* — the buffer takes the bytes and
`ENOSPC` arrives at the flush ([findings.md](findings.md) F11). A fetcher that
checked its writes and ignored its flush would verify a digest over bytes that
never reached the disk, rename a file that was never written, and record an
acquisition that did not happen.

So both halves are built. A file that will not fit is refused before a byte
moves, with the arithmetic in the refusal rather than a verdict: what it needs,
what is available, what it is short by, and which filesystem. That needed the
number the standard library does not expose, so `mcf_core::hardware::space` is
the second module in the workspace to take the `unsafe_code` opt-out — one
`statvfs` call, checked against `df`, and `Unknown` wherever it fails, because
*could not look* and *no room* are opposite answers.

And the case the check cannot catch — another process taking the room a moment
later — is classified rather than reported as a general write failure, with
`/dev/full` as the scenario: a device every Linux machine has that accepts
everything and stores nothing.

DEC-009's disk half is therefore built rather than pending. What remains of it
is M2's: several clients at once, and two resident models.

### Version 91 — offline, decided by what a machine actually says

DEC-011 resolved as D33, and it is the first decision this project has closed
by asking the operating system a question rather than by argument.

The half that needed no experiment: everything except acquisition already runs
with no network at all, and B-183's container has been demonstrating it on every
scheduled run — `doctor`, the laboratory, `list`, `rm`, `export`, `licence`.
§3.2 had suggested *most of it, loudly labelled*; nobody had stated it.

The half that did: [findings.md](findings.md) F10 asked what a platform tells a
program when a name will not resolve, and the answer is nothing — no error kind,
no errno, the same report whether the machine has no network, no resolver, or
asked for a name that does not exist. So MCF reports the observation and names
the question it is *not* answering, rather than saying *you are offline*, which
is wrong exactly when somebody has a mirror they could have used.

The three failures that are distinguishable are now kept apart, because they are
three different things to act on: refused (something answered), no route (this
machine cannot get there), silence (MCF's own deadline ended the wait). Two of
the three are asserted in the gating tier without a network at all.

And §V's wrinkle — no internet is not no local network — is answered by
declining to assert it. Telling them apart means making a request nobody asked
for, and §3.2 refuses unrequested traffic for better reasons than a nicer error
message. B-147 is unblocked.

### Version 90 — MCF runs nothing it downloaded

B-025 in progress, and the useful part is a check that is true *now* rather
than a design for later. §6.4 will one day allow repository code to be run
deliberately; today MCF runs none of it, and that is a stronger claim worth
holding while it holds.

Every place shipped code starts a process is declared with what it starts:
three, and each of them is MCF's own business — MCF starting itself to time how
long MCF takes to start, the compiler at build time recording what built this,
and `dlopen` of the vendor's management library to read an accelerator. That
third one is the interesting one, because loading a library is how bytes become
code without anything looking like execution, so the check on it is not that it
is absent but that what it opens is a constant: two library names, no path
joined to anything, and a test that fails if the module learns to build a path
at all.

The other half is what a model file *contains*, which no source check can
reach. A model whose metadata is a shell command, a path traversal and a format
specifier is read with MCF's own reader, and every field comes back exactly as
written — neither run, nor resolved, nor interpolated. That is what *data* has
to mean for the largest untrusted input MCF will ever read.

### Version 89 — a real model, from the real hub

B-029 done, and the second half of its condition is the one worth reporting:
*online against the real one*. `scripts/check-online.sh` acquires a 1.2 MiB GGUF
from Hugging Face over TLS — a real redirect to a CDN, a real LFS digest
declared with the listing and checked against the bytes that arrived — then
lists it with its provenance, purges it, and reads both events back out of the
record. It ran, and it passed, on the first machine that had a network.

Scheduled rather than gating, for the reason the fuzz tier is: a gate that needs
a network is a gate that fails on a train. What it acquires is small and really
a model, because a check that pulled 27 GiB would be one nobody runs and one
that pulled nothing would prove nothing.

Reproducibility needed defending on the way here. The first vendored dependency
put this machine's directory into every panic location rustc compiled, and two
checkouts at different paths stopped producing the same bytes.
`--remap-path-prefix` is the fix, it is part of the build MCF documents rather
than a flag the check adds to itself, and it also stops a panic message naming
somebody's home directory into whatever a panic is reported to (§XIV).

### Version 88 — the first dependency, admitted the way the rules ask

B-322 done. MCF has a TLS stack, and the interesting part is not that one was
vendored but that the choosing was a measurement.

The usual provider for `rustls` is `ring`, which is C and assembly. It cannot be
built for `x86_64-unknown-linux-musl` without a cross toolchain nobody here has
— and that is the target B-183's from-scratch container uses, so admitting it
would have quietly made an existing check runnable in fewer places. The provider
admitted instead is Rust and inline assembly by rustls's own author, builds for
both targets with nothing installed, compiles fourteen crates rather than
sixteen, and held a real TLS 1.3 session with the hub before it was let in.
[findings.md](findings.md) F9.5 and F9.6 are the evidence.

The tree is 18 MiB rather than 95, and that too was measured: a vendored tree
cannot be *deleted* down to the platform, because cargo resolves the whole lock
graph — but a crate nothing compiles can keep its manifest, its licence and an
empty `lib.rs`. `scripts/vendor.sh` does that and then checks both targets
against the result.

What it costs is written beside it rather than discovered later. `graviola` is
young where the mature providers are C, and the exchange is stated in
[vendored.md](vendored.md) §2 with the two things that bound it: the provider is
one struct behind `Wire`, and the digest MCF checks bytes against arrives with
the listing rather than with the file. Three crates declare their terms and ship
no copy of them, and that is recorded as a difference between declared and
verified rather than smoothed over (A21).

The checks moved with it. `the_workspace_declares_no_third_party_dependencies`
became `the_workspace_takes_exactly_what_the_register_admits`, which names three
crates and fails on a fourth anywhere; `mcf licence` lists what is compiled into
the binary, because that is what a redistributor conveys; and the from-scratch
container still runs MCF with no libc, no shell and no `/etc`.

### Version 87 — the chain, and the field that could only lie

B-019's chain is built, and building it found a defect in a type that has been
in the workspace since M0. `Provenance::retrieved_at` was a plain `Timestamp`,
which is right for something MCF acquired and false for the other half of
§XII's hard case: the upstream weights a third-party requantization was made
from are a real artifact with a real origin, and *when somebody else obtained
them* is not a thing MCF can know. The type could only say something untrue, so
the field is `Attested` now and `Provenance::known_of` is the constructor for a
link nobody fetched. The check that exempted the field from A7's rule — on the
grounds that MCF always knows when it acted — no longer does, because that
reasoning was about acquisitions and the chain is not all acquisitions.

The chain itself comes from the hub's own tags: a publisher who worked from
somebody else's weights says so, and `mcf pull` writes down the artifact, the
publisher's own word for what they did, and the upstream repository as a link
MCF has not fetched and cannot vouch for. What the publisher does not say stays
unsaid — which revision of the base, which tool, when — because an unpinned
base is exactly the break in the chain §XII is about, and filling it in would
hide the break rather than record it.

B-020 and B-021 are done with it. The reference parser reaches a named outcome
for every string and is driven through both simulated hubs; the fetcher resumes,
verifies, and refuses a source that answers a resumption from the wrong place
before a byte is appended.

### Version 86 — the operator's credential, and only theirs

B-024 done. What was missing was the surface, and the surface is where the rule
either holds or quietly stops holding: `mcf pull --token-from <file>` and
`--token-from-env <VARIABLE>` are the only two ways a credential reaches MCF,
and both of them are the operator naming a place rather than MCF finding one.

The nicest part is what happens when a repository asks for one. MCF says which
repository and what to do, and then says what it has *looked at and not used* —
a token sitting in the environment is reported with the flag that would offer
it, not spent on the operator's behalf. Finding a credential is not permission
to use it, and this is the one place where that distinction is visible to
somebody who has never read the rules.

A credential offered over a connection that cannot keep it is refused rather
than downgraded, and a whole-system test asserts the request never goes out at
all: the refusal happens before the socket, and the token is not in the output.

### Version 85 — which of these will run here

B-213's own product, from the hub's own metadata: `mcf pull` on a repository
with no file named now answers the question an operator is actually asking.
Not *what is published* — that was already there — but which of the twenty
quantizations will run on this machine, at a stated context, with how much left
over or how much short.

Three cheap questions and no weights: the card, the tree, and the model's own
`config.json`. A whole-system test asserts exactly that — a plan is made and
nothing ending in `.gguf` is ever requested.

The shape is read whole or not at all. A configuration that does not say how
many key/value heads a model has produces no shape, because the grouping factor
is precisely what a guess gets wrong: eight heads planned as thirty-two
overstates the cache fourfold, and the operator is told a variant does not fit
that does. There is a test that reads the same model both ways and asserts the
two answers differ by exactly the grouping factor, so the mistake cannot be
made quietly.

And a repository that publishes no configuration is reported as unplannable
rather than planned badly. A missing plan and a plan that found nothing are
different answers (A7).

### Version 84 — a model enters this machine

B-029's offline half is done, and it is the M1 product: `mcf pull` acquires a
model from a hub, `mcf list` says what is here and where it came from, `mcf rm`
takes it away deliberately. A whole-system test drives all three as separate
processes against a hub on the loopback address and a real record on a real
disk — which is B19's *every M1 test runs against it with no network*, now
including the network path itself.

`pull` refuses to choose for anybody. A repository publishing twenty
quantizations gets its contents put in front of the operator with the sizes and
the terms, and nothing is acquired: picking one would be picking what they
measure. When a file is named it is verified against the digest the hub
declared with the listing, and when the hub declared none the surface says
**HELD, NOT VERIFIED** in as many words — A21's distinction where somebody will
actually read it.

Two records, because they answer different questions. The provenance goes in a
sidecar beside the artifact, since that is what travels with the file; the
acquisition goes in the journal as `artifact_acquired`, since that is what
happened on this machine. A model somebody later moves by hand keeps the first
and cannot alter the second.

The laboratory gained a hub on a socket (`mcf_lab::serving`) to make all of
this testable. `mcf_lab::hub` simulates what MCF observes through the `Source`
interface; this simulates what MCF observes through a socket, and it exists
because code that talks to an operating system cannot be tested against a
value. Two scenarios now use it and the third new one needs no listener at all:
an `https` hub is refused before a connection is opened, because the refusal is
decided from what MCF has rather than from what the far end says.

### Version 83 — the economy that was not there

B-322's last step is now a single decision, and it is worth being exact about
what the decision costs. Most of a vendored TLS tree is Windows import
libraries a Linux build never compiles, so the obvious move is to trim them.
Cargo refuses: with a vendored directory it resolves the whole lock graph
before compiling any of it, and a tree with those crates removed fails to build
on Linux for reasons that have nothing to do with Linux.

So the number is about ninety megabytes of third-party source in this
repository, not fifteen. Fifteen is what gets compiled, which is the right
figure for reviewing what MCF ships and the wrong one for what a checkout
costs. [findings.md](findings.md) F9.4 has the measurement.

Everything else the transport needs is written: the protocol, the socket, the
hub as a source, the redirect that drops a credential, the resumption that
refuses to append to the wrong thing. What is left is one struct — and the
admission is left as its own deliberate step rather than folded into a commit
about something else.

### Version 82 — the hub, on the other end of the wire

B-322 further, and B-021's transport with it. `mcf_hub::client` answers the
four questions a source is asked by asking a real hub, in the shapes F9
measured — and the laboratory's simulated hub, which has been standing in for
one since B-028, now has something to have been standing in for.

Two questions before a byte of weights moves: the card, which names the
revision to pin and the terms; and the tree, which names every file, its size,
and the SHA-256 the hub declares for anything in LFS. That second answer is
what makes B-213's arithmetic possible before a download and B-021's
verification possible at all — a digest that arrives *with the listing* is one
the bytes can be checked against, rather than one the same connection could
have invented to match what it sent.

Writing it found two defects of the kind that only appear when something real
is on the other end, and both were the same mistake: acting before checking.
The client appended a resumption's bytes and *then* noticed the source had
started again from zero, and it wrote a 404's error page into the artifact
before reading the status. Both are now decided from the head of the answer,
before anything is written — an append undone afterwards is a file that was
wrong in between, and a crash in between leaves it wrong for good. A scenario
holds the first: it asserts the partial file is byte-for-byte untouched after
the refusal.

A redirect's own body is no longer read either, which is a smaller thing and
the same principle: the head said to go elsewhere, so what is underneath it is
bytes nobody asked for.

### Version 81 — the socket, and the shape the cryptography drops into

B-322 further. The wire is written, which means everything between `mcf pull`
and a TLS handshake now exists and is tested: a request goes out over a real
socket, redirects are followed, a body is streamed to a file rather than held
in memory, and every way that can go wrong ends somewhere named.

Writing the boundary before the cryptography rather than after is the point.
`Wire` is three methods — what are you, will you keep a secret, open a
connection — and the TLS implementation is one more of them. Everything above
that line is finished; what is missing is one struct.

Two things in it are worth saying out loud. A wire that cannot keep a secret
*refuses* to carry one rather than sending it anyway: a token on an
unencrypted connection is a token given to everything in between, and a silent
downgrade is exactly the leak nobody can see afterwards. And a redirect keeps
its range while dropping its credential — the first because a resumption that
lost its offset at the CDN would start again from zero and report progress that
did not happen, the second because the CDN is a host the network named.

The tests are over real loopback sockets rather than a mock, because what is
being tested is the part that talks to an operating system. One of them is a
server that accepts a connection and says nothing, which is also now a
scenario: B7's *a hang is a defined outcome* asserted against the real socket
path rather than a stand-in for it.

### Version 80 — the half of the transport that needs no socket

B-322 in progress. The client is written and it opens nothing: it turns a
request into bytes and bytes into an answer, which is precisely what makes the
two behaviours worth being careful about testable without a network.

The first is the one that would have been a defect nobody noticed. The hub
redirects a download to a signed URL on a CDN, so a client that forwarded the
`Authorization` header would hand an operator's token to whatever host the
answer named — and the answer comes from the network, which §3.7 says is
untrusted. So a redirect here does not return a destination; it returns *where
to go and whether the credential goes with it*, and the second half is a
decision at a call site rather than an oversight in a library. A host that
merely ends with the hub's name is a different host, and there is a test with
`evil-huggingface.co` in it saying so.

The second is that nothing a source claims is believed before it is read: a
length that is not a number, a range that starts somewhere other than where the
transfer resumed, a header block that never ends. Each is a refusal naming what
was seen, bounded so that a source cannot write a megabyte into MCF's own
record.

It went into the fuzz tier immediately and the tier immediately earned its
place: it found `Content-Range: bytes 0-15/2` accepted — a source contradicting
itself in the two numbers a fetcher acts on, the total it plans against and the
span it appends. Refused now, with the case written into the unit tier beside
the ones a person thought of.

### Version 79 — the transport, measured before it was argued

B-322 added, and it is the item four others were waiting on. B-021, B-213,
B-024 and B-029 all end with the same sentence — *what remains is the
transport* — and behind that sentence was an argument nobody had numbers for.
[findings.md](findings.md) F9 has them now.

The hub turns out to be simpler than feared and to say more than expected: it
answers HTTP/1.1, redirects to a signed URL on another host, serves ranges, and
publishes the file's size, its SHA-256 and the repository revision in the
headers of the download itself. So the parts B-021 and B-019 need are already
there for the asking.

What MCF lacks is not a protocol but TLS, and the cost of that is sixteen
crates and 15 MiB — not the 91 MiB a vendored tree reports, because most of a
vendored tree is Windows import libraries a Linux build never compiles. The
alternative that costs almost nothing to vendor is the one that makes the
binary demand `libssl` of the user's machine, and F9 keeps it as the control:
the shape that fails, failing where a check can see it.

So the item is written the way D32 wrote the engine question. Delegate what
specialists maintain — nobody at MCF is going to write a TLS 1.3
implementation, and A19 forbids claiming what is not tested. Own the wrapper
that has to be correct: the client is small, and the two behaviours that matter
— a redirect that must not carry a credential across hosts, and a resumption
that must verify — are exactly the ones the laboratory has to be able to
simulate.

### Version 78 — models live on this machine, and leave it

B-029 in progress: `list` and `rm`, which are two thirds of M1's product. What
was in the way was not the commands but the fact that nothing could write a
provenance down. A provenance that exists only in memory travels with an
artifact until the process ends, which is not what §3.6 means.

So `mcf_record` gained the other direction. Unknown goes out as `null` and
comes back as unknown; a record missing a field is refused *naming the field*
rather than filled in; an unreadable link refuses the whole chain, because a
chain with an invented link is worse than no chain. The property tier holds the
round trip over generated chains eleven links deep rather than over the one
example §XII names.

On the disk it is a sidecar beside the artifact and not an index. An index is a
second copy, and second copies drift: a model moved by hand, a directory
restored from a backup, a machine that lost its journal. In each of those the
sidecar is still there and still true.

`list` keeps three states apart, which is the whole reason it is not a `find`
command: provenance read, nothing there, and something there that cannot be
read. The third is the one that matters, and a listing that showed it as the
second would be telling an operator their model is unaccounted for when in fact
MCF is the one that cannot read.

`rm` is B-027 at the surface. Without a reason it previews and removes nothing;
with one it authorizes that exact plan, writes the record, and moves the
artifact and its provenance together — a sidecar left behind would record
something that is no longer there. Nothing is deleted without `--purge`.

There is still no `pull`: fetching needs a network stack, and the vendoring
decision that requires has not been made. The usage text says so rather than
leaving an operator to wonder.

### Version 77 — an artifact is not a cache entry

B-027 done. The failure it is against is a helpful one: a disk fills, and a
tool that wants to keep working deletes the oldest thing it can find. MCF will
not, because an artifact is what a measurement was made against and
re-acquiring it is not always possible — a pinned repository can be withdrawn,
gated or relicensed between one week and the next (DEC-038). A tool that
deletes to reclaim space has decided that disk is worth more than evidence.

So there is no path from *space is short* to *bytes are gone*. Removal is four
separate acts and each is a type: a preview that touches nothing, an
authorization somebody gives for a stated reason about a list of files at
particular sizes, a removal that writes the record and then *moves* the
artifact to a shelf, and a purge — the only function in MCF that destroys an
artifact, and it takes the authorization to do it.

The record goes first. After a removal the artifact is gone and the record is
all there is, so a record written afterwards is one a crash can lose along with
the thing it describes (A1). `artifact_removed` is a new kind in the journal
for exactly that reason: it is the one event whose record has to outlive its
subject.

Reversibility is measured rather than promised. A rename inside one filesystem
is free, so the shelf costs nothing where it works; where the shelf is on
another filesystem the plan says NOT recoverable in as many words, and the
operator authorizes that fact. Unknown reads as irreversible, because being
wrongly told a removal can be undone is how somebody loses a model.

What keeps this true next year is not the prose. `nothing_deletes_an_artifact`
reads every shipped source in the workspace and requires each deleting call to
be declared with what it destroys and why that is not an artifact. Adding a
deletion means writing that line, which is the point.

### Version 76 — a credential nobody handed over

B-024 in progress. The failure this is against is not one anybody writes on
purpose. It is a convenience: one line that reads a token out of the
environment because the tests were annoying, after which MCF acquires artifacts
under a condition nobody recorded. §3.4 makes conditions part of the
measurement, so an artifact fetched with a token that happened to be set was
fetched under different conditions from one fetched without it — and nothing
downstream can tell.

A comment cannot hold that line and a review will not, because the change that
breaks it is one line and looks helpful. So it is held by the compiler and by a
check: nothing in `mcf-hub` reads the environment at all, the survey is *handed*
a way to look, and the names of the token variables appear in exactly one file
across the workspace. An operator is told a credential is sitting there and
decides; MCF does not decide for them.

The secret redacts itself, because the rendering that leaks a token is never the
one somebody wrote deliberately — it is `Debug` on some struct three layers up.
What a record keeps is a fingerprint, which answers the question provenance
actually has (*was this the same credential?*) and answers nothing else.

Three refusals, written once so that three sources cannot phrase them three
ways: no credential, a credential refused, and terms not accepted. They are
different worlds and the difference is the point — a better token does not open
a gated repository, and a fourth scenario now holds the distinction in the fault
catalogue, which covers 24 of the 24 categories MCF's code constructs.

### Version 75 — three states, and the middle one is the one that matters

B-023 in progress. A licence has three states and MCF now keeps them apart: an
identifier it recognizes, terms that are present and unmatched, and nothing
declared at all. The middle one is why this is work rather than a field. A
repository whose licence MCF cannot parse is *not* a repository with no licence,
and a tool that collapsed the two would let somebody proceed past terms nobody
read — while a tool that treated the unparseable as a failure would refuse
models whose only sin is a licence written for themselves.

`inspect::terms_are_legible` returns the state instead of a string, so a caller
cannot receive a licence and forget which kind it was. Recognized identifiers
carry the family their own name puts them in — permissive, copyleft,
non-commercial, bespoke — which is reading a label, not reading terms.

What MCF refuses to do is the reason the module has an argument in it. It will
not say whether a use is allowed. That is a legal judgement about a specific
person and a specific use, MCF has no standing to make it, and a guess would be
worse than silence. §III's *a use the terms forbid is stated rather than
discovered* is honoured by putting the terms in front of a reader early enough
to read — and for the one forbidden use MCF could itself commit, publishing
measurements about a model, the question is DEC-036 and it is open.

### Version 74 — a transfer that cannot leave a half-model behind

B-021 in progress. The failure the fetcher exists to prevent is a
half-downloaded model sitting where a whole one would be, because everything
downstream reads that path and cannot tell. So the rule is structural: bytes
accumulate under a `.partial` name, and the artifact's own name is only ever
given to something that has been verified.

Resumption is an optimization; verification is not. A source that cannot
continue is restarted and said so, and a file that changed under the transfer is
refused with the mixture deleted — half of one file and half of another is the
one thing worse than no file.

### Version 73 — what a repository claims, against what is true

B-022 in progress. The interesting half is the deceptive card, and it is only
checkable because D31 gave MCF a second reader for the weights: a repository can
say anything in its metadata, and the tensors either have the shape that
architecture implies or they do not. A21's divergence, reported rather than
raised — whether a user wants the weights anyway, knowing, is theirs to decide.

Three scenarios now drive the whole path through the simulated hub: list, fetch,
read, compare. That is what B-028 was built to make possible.

### Version 72 — what will run here, before the bandwidth is spent

B-213 in progress: PR3's arithmetic half. A repository publishing twenty
quantizations can be classified without fetching any of them — fits, fits
without room for your context, does not fit — and the middle verdict answers
with the longest context that *would*, because that is a configuration somebody
can actually take.

Nothing here produces a number about speed. That is the estimate half (B-214),
and A20 keeps it apart.

### Version 71 — a hub that behaves badly on purpose

B-028 in progress. What MCF asks a hub is now an interface — four questions,
stated in `mcf_hub::source` — and the laboratory has a hub that answers them
badly in declared ways. That boundary is what makes B-028's condition
achievable at all: a simulation needs a thing to simulate.

It observes rather than causes (D26): there is no model of a rate limiter, only
a source that answers *throttled*, because what MCF must get right is what it
does with that answer. Where bytes matter they are real — a truncated transfer
writes the partial file, since the artifact on the disk is what a fetcher has
to notice.

### Version 70 — a reference is a thing MCF can read

B-020 in progress. Every way a reference is written reaches a named outcome —
typed, pasted, from a browser URL — and every string that is not one is refused
by name, including the traversals and the schemes §3.7 exists to stop. It is
total, offline and deterministic, which is what lets the laboratory exercise
every branch without a hub.

The fuzz tier found two things worth keeping while it was written: a file name
that could be read out of a URL and not written back, and an owner containing a
dot, which is what distinguishes an owner from a host. Both are refusals now.

B-028's status was stale — DEC-021 closed with D26 and the register still said
blocked. The fake hub is open work, and it is what B-020's done-when is waiting
for.

### Version 69 — MCF runs on a machine with nothing on it

B-183 in progress. The artifact's dependency list said MCF needs nothing
unusual; this runs it in a container that has nothing at all, which is the
difference between a claim about a file and a claim about what happens. All of
`doctor` runs there — the profile, the hundred-spawn cold-start measurement, the
laboratory — and what the container lacks is reported rather than assumed.

The first token is what remains, and it needs a model. That is M1's work, and
the script says so on every run.

### Version 68 — the stand-in reads text

B-360 gains the tokenizer, so the engine takes a prompt rather than a list of
identifiers: the unigram vocabulary a llama-family file carries, segmented by
the dynamic program its scores imply, with byte fallback for text the
vocabulary does not spell and a named refusal for the byte-pair kind this crate
does not implement.

Text in, tokens out, text back, and the result marked — on a model a test
constructs. What is left is the part no test can construct: a real artifact,
and the cross-check that would establish agreement rather than wiring.

### Version 67 — the stand-in runs a model

B-360 reaches a first token. The forward pass, the operations under it and the
sampler above it are all here, and what comes out is typed as a stand-in result:
a `Degraded<Behaviour<Generated>>` with no way out that drops the mark (A5).

The done-when is not met and the two things missing are named: a tokenizer, so
that a prompt rather than a list of identifiers reaches the model, and a real
artifact rather than one a test constructed — which is M1's acquisition work.
What the tests establish is the wiring, on models whose answers can be stated
without running them; agreement with a vendored engine is B-362 and is the only
thing that can establish the rest.

### Version 66 — the stand-in engine begins with what it must read

B-360 in progress. D31's second implementation starts where a model does: the
file. The GGUF reader reads metadata, the tensor directory and the alignment,
refuses by name what it does not read, and keeps a file whose one unknown
tensor type does not make the rest unreadable (A4).

It is untrusted input (§3.7), so it joined the fuzz tier the day it existed —
and the first campaign found something worth having: a directory whose
arithmetic overflowed was handed back rather than refused. The reader now
checks the directory against itself, and `read` checks it against the file's
length while `parse` does not, because a caller holding only the head of a
download is B-213's case rather than a defect.

### Version 65 — the artifact states its own terms

B-330 done. The repository held the licence and the register refused an
unrecorded component; what was missing was the half a redistributor actually
needs, since they have a binary rather than a repository. `mcf licence` states
the terms, the warranty position and the source obligation, and `--full` prints
the whole text, which is compiled in.

It cost 37 KiB of a 40 MiB ceiling and tripped B-011's regression detector at
5.7 %, which is the detector working: the growth was accepted in a commit
message rather than by raising a threshold.

### Version 64 — the engine question is answered

DEC-004 resolved by D32: MCF delegates inference and owns the wrapper. The
intent document called it the most consequential unanswered question in it and
asked not to be settled on a reading, so it was settled on a measurement —
[findings.md](findings.md) F8, one matrix multiply, four ways MCF could
maintain, against a tuned BLAS on the same machine.

What it unblocks is the engine line: B-320's vendored stack, B-183's first token
from a container, and the M2 items that were gestured at. What it does *not*
decide is which engine, which is D23's terms, D28's licence and B-330's matrix.

### Version 63 — the signal F5 was missing

B-193 done, which closes what F5 opened. The storage an artifact is read from
is the condition floor's eleventh question, and a measurement whose work took a
major page fault is refused as a reading of the device rather than asserted
against a ceiling.

The two signals answer different questions and a measurement can fail either:
*was this thread queuing for a processor*, and *did this work go to a device*.
F5's cold start failed the second while passing the first, which is why it was
judged clean at two and a half times its ceiling.

### Version 62 — the budget tier has a before

B-011 done. What it was waiting for was somewhere to keep a previous reading,
which B-185's stamps supplied: each figure is now compared with the last one
recorded and a regression fails with both readings and both condition sets.

Two of the three figures are judged. The cold start is reported and not judged,
because F5 established that its reading is decided by storage nothing records —
that is B-193, and naming it is better than a detector that fires on a page
cache.

### Version 61 — what the artifact requires of a machine

B-192 done against its stated condition. The artifact's dynamic dependencies
are read out of the file itself and refused if they name anything a stock
machine lacks; three libraries qualify and each is part of what a Linux machine
is. The vendored engine is B-320 and the from-scratch container is B-183, and
the check holds the line for both: a component that dragged in a prerequisite
would fail here rather than at a user's first run.

### Version 60 — the mutation score has a floor

B-186 done, which closes the last of B38's three. The floor is 100 % of a
hand-written catalogue rather than a percentage of everything a generator could
produce, and a run scoring below the floor or below the previous stamp exits
non-zero.

The catalogue grew by one while this was built, and the way it grew is the
argument for the tier: a mutant that multiplied kibibytes by 1000 instead of
1024 survived the whole suite, so a figure `mcf doctor` prints and B-011 asserts
rested on nobody having looked. The gap now has a test and the mutant is in the
catalogue.

### Version 59 — the tiers have ages

B-185 done. Every scheduled tier stamps what source it ran against, every
`ci.sh` run reports the ages, and `scripts/check-tier-ages.sh --release`
refuses a release on a tier that has not run against the code it would be a
release of.

The interesting half is what *stale* was allowed to mean. A maximum age in days
needs a figure nobody has stated, so the register would have acquired intent
nobody chose; the source a result was taken against is the thing that actually
invalidates it, and it is decidable. B-186's floor now has somewhere to keep a
previous score, which was the other thing blocking it.

### Version 58 — the budget that measures the filesystem

B-193 added, from [findings.md](findings.md) F5. The first run of every tier at
once failed the cold-start budget, and the cause is a mount whose p99
page-fault service time is a thousand times its median: the same binary read
from tmpfs is within its ceiling by two orders of magnitude. Two halves are
registered together because either alone would leave the figure meaningless —
the storage an artifact is executed from is a condition nothing records, and
D30's attributability signal watches the measuring thread, which during a cold
start is the one thing not doing the work.

### Version 57 — the suite has all ten tiers

B-191 done. D10's ten disciplines all exist and all run: five gate every change
and five are scheduled behind flags on the same command. What is not done is
named where it belongs — tier ages are B-185, the mutation floor is B-186, and
the end-to-end boundary is DEC-022, which the whole-system tier cites rather
than pre-empts.

Three of the new tiers found something on their first run, and each is recorded
with the work rather than in a commit message alone: a condition that did not
round-trip through the record ([build.md](build.md) §9, fixed), a replay whose
footprint is proportional to the journal, and the reason the soak tier can only
be read on one thread.

### Version 56 — the offset is read, at the moment it applies to

B-352 is done. Three choices in it are worth recording.

**The zone file is parsed rather than a platform call made.** The obvious route
is the C library's `localtime_r`, whose `tm_gmtoff` is a widely-implemented
extension rather than POSIX — which means declaring another platform's
`struct tm` by hand, in `unsafe`, for a field nobody standardized. The zone file
is a published, stable format MCF can read in safe Rust and test against a value
the machine itself can be asked for. B15 admits weight against a stated cost,
and this is the cheaper side.

**The offset is the one in force at the moment, not the one in force now.** That
is the whole reason a zone file has transitions, and getting it wrong would make
an old record legible as the wrong place.

**A moment beyond the file is unknown rather than extrapolated.** A version 2
footer carries a POSIX rule for the far future and this reader does not evaluate
it, so it says so instead of guessing — and never `+00:00`, which is a real
offset most machines do not have (A7).

The check is A19's: the system's own `date +%z` is an independent
implementation of the same question reading the same file, so agreement is
evidence and disagreement would be a defect here.

### Version 55 — the vendoring register, before there is anything to vendor

B-321 is done, and reaches further than the item asked. It wanted the *deferred*
list; B-330 wants a compatibility finding per vendored component. Those are the
same register seen from two sides, and writing one alone would have produced two
documents that eventually disagreed about what MCF ships.

Three things it states that were not written down anywhere.

**What a finding is**, with A21's distinction running through it: what a
component *declares*, what MCF *verified* and how, the verdict against
GPL-3.0-only, and the tier. A component whose licence MCF has not verified is
not admitted — which is stricter than it sounds and is the point.

**Why the candidates are still declared.** Their terms were fetched rather than
recalled, with digests recorded, and they remain declared: a project's `LICENSE`
is its statement about itself, and what MCF would ship is a *tree*, which can
contain files under other terms than the one at its root. Verification is a check
of the tree actually vendored at the revision actually pinned.

**That deferring an accelerator path does not weaken §III.** B7 governs attempt
and diagnosis rather than success, and D31's stand-in makes the outcome on
hardware with no vendorable backend *runs on the processor, marked* rather than
*does not run*.

### Version 54 — one portable file, and a citation the rename missed

B-302 is done. Its condition was the interesting part — *one mechanism serves
export, contribution and repro bundles* — and what it forced is that the three
differ in **what is selected** and never in how it is written. Three
serializations of the same evidence would eventually disagree about what the
evidence was.

Two decisions inside it. **Entries are carried verbatim rather than
re-encoded**, because a bundle whose digest depended on the version that wrote
it would defeat §XV. And **a damaged bundle is refused rather than read short**:
a bundle with rows missing looks exactly like a smaller bundle, and reading it
as one is B62's silent shortening arriving by post. A bundle from a
*legitimately* incomplete journal is a different thing and says so.

**This row also carried the defect B-353 was about.** Its title cited the repro
bundle as a bare `P2`, which the rename to `PR<n>` missed because only the link
beside it was rewritten — and the conformance check passed, because a bare `P2`
now resolves as the *precedence rule* P2. That is a citation that silently
started meaning something else, which is precisely what having one namespace per
letter was supposed to prevent, and the check could not see it because both
readings are valid identifiers. A sweep found this was the only one.

### Version 53 — the prohibition lands before the thing it constrains

B-361 is done. `Run<Vendored>` has a `timing` method and `Run<StandIn>` does
not, so a speed from a naive kernel is not a rule somebody might break but a
program that does not compile — and `Timing` has no constructor of its own, so
the only way to one is through an engine that is allowed to report one.

It lands before B-360 deliberately, for the reason B-220's restoration ledger
was built before anything was permitted to change the environment: a prohibition
added after the thing it prohibits is a prohibition somebody has already worked
around.

The fault-catalogue ratchet did its job twice while this landed. Constructing
`engine.unavailable` required a scenario, which is A13 working as D26 intends;
and the laboratory's own suite then refused that scenario for producing a
failure with no context, which is B21 — a mark that says something was lost
without saying what is a mark a reader cannot act on. The failure now names
which implementation ran, at which build, and what it was permitted to report.

### Version 52 — the stand-in engine is registered

PR8 is accepted by D31 and registers three items. B-361 lands first — the
prohibition, before the thing it constrains exists, for the same reason B-220's
restoration ledger was built before anything was allowed to change the
environment.

The ordering of the argument is what admits the work at all. Coverage sounds
like capability and B23 refuses weight admitted for capability; what justifies a
second implementation is A19, since for inference the only available
demonstration that software computes what it claims is another implementation
that agrees. Coverage is what it also buys.

B-360 is placed at M0 rather than M2 for the loaders it shares with provenance
and PR3's fitment arithmetic, and its first token belongs with the engine work.
B-362's tolerance question is D19's shape — identical inputs do not guarantee
identical outputs — and is open in the item rather than assumed.

### Version 51 — the budget tier can assert again

DEC-051 is resolved by D30, and the deadlock F2 found is gone rather than traded
off. Attributability is a property of a *reading* — measured as how long the
measuring thread spent runnable and waiting — rather than of the machine, which
is what B24 said all along.

Two things the work turned up are recorded in [findings.md](findings.md) F3 and
matter beyond this item.

**Every budget figure MCF had measured was contaminated.** Cold start's median
is 208 µs on a quiet machine, not the 6–8 ms F1 and F2 recorded. None of the
earlier readings was wrong about *passing*; all were wrong about the number,
which is what §3.4's conditions exist to prevent and what an unstated
contamination hides.

**The first implementation of the new signal could not fail.** It read a
process's accounting rather than a thread's, and a process's accounting is its
main thread's — so a measurement taken on any worker thread, which is every
measurement under a test harness, read as perfectly clean under any load. It was
caught by running the tier under deliberate load and checking that the verdict
*changed*, which is the negative control B-003 established for a lint, applied to
a measurement. A check that has only ever been observed to pass has not been
observed.

### Version 50 — the author answers two, and both were his to answer

DEC-047's licence half and DEC-035 are resolved, by D28 and D29. Neither was
derivable: the first is a commitment about distribution that only the copyright
holder can make, and the second is a scope decision that spends somebody's time.

**GPL-3.0-only.** `LICENSE` holds the verbatim text and a check asserts it stays
the text the manifest declares — the mundane failure being a licence file that
drifts from the terms a redistributor is told about. B-330 moves to in progress:
the matrix has no rows because nothing is vendored, and the check refuses a
vendored component with no recorded finding rather than passing an empty world
silently.

**All platforms, Linux first.** The reading that made this tractable is B7's:
coverage governs attempt and diagnosis, not success. So a platform gets D25's
three states for the same reason a device does. B-183 unblocks, though its *first
token* still waits on an engine.

### Version 49 — the clock anomaly closes B-184

The half that was waiting for the laboratory is built, and building it made
clear why the detector could only live where it does. **A wall-clock reading
alone cannot say whether time passed or the clock moved** — which is the whole
reason B37 keeps `Timestamp` and `Instant` apart — so the detector has to hold
both across an append. When the calendar advances further than the monotonic
clock, or goes backwards, it is the calendar that moved.

The anomaly does not stop the append, and that is D9's *events, not
corrections* read carefully. The event being recorded did happen and A1 forbids
losing it; what is unsound is anything being **measured** across the anomaly. So
the entry is written, the anomaly is written beside it, the caller is told, and
the disposition is `invalidated` — §3.4's word for a result that completed and
cannot be believed.

The tolerance is a stated judgement rather than a hidden one: one second, since
the two readings are taken microseconds apart and a second of divergence cannot
be scheduling.

### Version 48 — the machine comes back, including after a kill

B-220 is in progress: the mechanism is built and two of the four things its
condition names do not exist yet to be interrupted.

The design turns on one ordering. A `Drop` restores when a process unwinds and
does nothing when a process is killed — and A27's test is explicitly about the
worst moment, so the destructor is the convenience and not the guarantee. The
ledger is written **before** the change and recovered on the next open. A crash
between the two leaves an entry for a change that never happened, and restoring
it is harmless; a crash the other way round would leave a changed machine nobody
can put back, which is the failure A27 forbids.

Three consequences follow and each is a test. Recovery **reports** what it did,
because an operator whose machine was changed and changed back is owed the fact.
Overlapping changes unwind newest first, so a file replaced twice comes back to
what it was before the first replacement. And a change MCF could not first
capture is refused *before* it is made — A27's *what it cannot restore it does
not touch*, as a precondition rather than a rule.

What is deliberately short is the list of things that can be changed. One
variant today, because a replaced file is all MCF alters outside its own
directory; governors, priorities, exclusive modes and suspensions join it with
the environment ladder, and B48 requires each be approved per run, so each
arrives with the approval that admits it. The list being short is the point:
what is not in it, MCF cannot change.

### Version 47 — MCF measures what its own observing costs

B-012 is done, and what made it tractable at M0 is noticing what MCF's
observation currently *is*. There is no serving path to instrument and no
laboratory telemetry to reduce; the whole of what MCF does in order to observe
is append a line and wait for it to reach the medium. So that is the cost
measured, over D27's hundred trials, against D24's two-millisecond ceiling —
and beside the operator's real record rather than in a temporary directory,
because a durability barrier on a fast local disk and one on a network
filesystem are different costs and the second is the one that would surprise
somebody.

**The condition floor grows to ten.** B3 requires the instrumentation profile
travel with a figure, and `mcf doctor` and `mcf doctor --no-record` are the two
arms of that comparison — so a reader of two results can tell which was taken
while MCF was writing. That is the second time the floor has grown, and both
times the addition came from a rule that already required it rather than from a
convenience.

`mcf doctor` also now reads its timing at the percentile D27 names rather than
showing a median and a p95 with §7.50 marked open, and states whether the
machine was quiet enough for the reading to be about MCF at all.

### Version 46 — corruption is caught before the run, and the taxonomy grows by one

B-301 is done, and it exercised the extension machinery the taxonomy describes
rather than only using it.

**A code was missing.** Re-verification has three outcomes, not two: the bytes
verify, the bytes differ, or the file is there and cannot be read. The third had
no code. `artifact.corrupt` says *present and fails verification*, which is a
claim about the bytes; a permission error or a media error is a claim about the
**machine**, and filing one under the other puts the wrong attribution on a
failure — B24's difference between a slow model and a busy machine, one level
down. `artifact.unreadable` was added with its laboratory scenario in the same
change, which is what the extension policy requires (A13). 111 codes.

**SHA-256 is written out**, and the argument is B15's and stated: it is a fixed,
published algorithm with official test vectors, so the correctness a dependency
would buy is exactly the correctness a test establishes — and it is established
against the vectors in FIPS 180-4, including the million-character one that
catches a drifting block loop where the short ones cannot. What is *not*
claimed is a security boundary: this answers *are these the same bytes*, and
where MCF later needs to resist a deliberate collision that will want a reviewed
implementation. The distinction is recorded rather than assumed.

`Checksum::of` takes a computed digest and cannot fail, which removed an
unreachable arm from every call site. An unreachable arm is either a lie or a
panic waiting to be written.

The laboratory is at eight scenarios covering seven categories, and `mcf doctor`
runs all of them on the operator's machine before it reports.

### Version 45 — `P` means one thing, and the ADR item is dropped

B-353 is done. The letter `P` named both the precedence rules and the
proposals, and both appeared bare in prose — the same identifier meant *science
outranks speed* in one document and *the repro bundle* in another. A reader
could not tell, and neither could the conformance check, which had been
resolving `P` against both sets and calling that a pass.

C5 forbids renumbering and permits deprecating in favour of a named successor,
so the proposals become `PR<n>`, digit for digit, with the mapping stated where
they live. Every citation elsewhere is updated, including those inside changelog
entries: a changelog states what changed rather than the words used at the time.

**B-017 is dropped**, and the reasoning is kept per C6 so it is not re-proposed
as an oversight. It asked for an ADR format and index so that §7's resolutions
and their reasoning survive the code — and that already exists under another
name. §2.1 holds each resolution, the intent document's changelog holds the
reasoning that produced it, and §7's retired-void index points back. An ADR set
would be a second home for statements that have one, and a statement in two
places eventually says two different things. The item's own condition — *a
resolved void points at an ADR and the ADR points back at §7* — is met today by
documents that exist, with the retired-void table doing exactly that job.

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
when the machine changes underneath) registered. B-260 and B-261 for PR7's
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

PR1, PR2, PR3 and PR5 accepted from [proposals.md](proposals.md) and registered:
the workload slot format and its authoring documentation (M6), the repro bundle
and its verifier (M5, both blocked on DEC-033 since a bundle of summaries cannot
be re-analysed), pre-acquisition fitment (M1) with projection bands and their
scoring (M5), and the contention snapshot (M5) with the quiet-machine pre-flight
it enables (M6).

B-217 is the item worth noting: D8 made laboratories exclusive, which means a
lab must *begin* on a quiet machine or exclusivity is a claim rather than a
condition. That turns PR5 from a diagnostic convenience into a precondition for
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
