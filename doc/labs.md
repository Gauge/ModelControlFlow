# Laboratory Catalogue

| | |
|---|---|
| **Type** | Catalogue — candidate laboratories, drafted not ratified |
| **Version** | 5 |
| **Status** | Living. Nothing here is committed; §7.29 remains open. |
| **Authority** | Derived from [document-of-intent.md](document-of-intent.md) v17, governed by [rules.md](rules.md) |
| **Answers** | §7.29 in draft · registered as DEC-029 in [backlog.md](backlog.md) |

**A laboratory measures one property of a model under controlled conditions.**
It is an instrument, not a test: it has no pass condition, produces a
distribution rather than a verdict, and never gates a build (§6.7, D10). It owns
the machine while it runs and may be as greedy as accuracy requires (D8, B35).

**Twenty-three candidates, four families.** The families differ in what gates them, in
what they can claim, and in whether they need the machine to themselves:

| Family | Gate | Class | What it establishes |
|---|---|---|---|
| **A · Physical** | None — every model | Timing | What the model costs on this hardware. **Needs an exclusive window** (D8) — tens of minutes |
| **B · Capability** | §X verified the capability | Behaviour | How well it does a thing it can do. **Yields** — runs behind the user, for hours or days |
| **C · Robustness** | Broad | Mixed | Whether the result holds up when conditions move. Class per lab |
| **D · Comparative** | Two or more configurations | Mixed | What a change to the configuration did. Class per lab |

## Tiers

D13 orders work cheapest-first, and the ordering is a correctness requirement:
§X calls a misconfigured model a measurement error, so evaluating an
uncalibrated configuration measures the misconfiguration.

| Tier | Labs | Cost |
|---|---|---|
| **Smoke** | none — a load and a token, before any lab | seconds |
| **Calibration** | §X's probes: template, stop conditions, tool format, usable context. Adjusts as well as detects | minutes |
| **Characterization** | Family A entire | tens of minutes |
| **Evaluation** | Families B, C, D | hours to days |

Characterization is not only a tier of results; it supplies the **rate** that
turns a laboratory's declared work into a duration (D14, B46). Nobody runs
everything, so a run is bought with a time budget and the proposal states what
it excludes (B47).

## The three rules every entry obeys

**Not applicable is not zero** (B40, §3.23). A lab runs against a model only
where §X verified the capability present. Four outcomes exist — *measured*, *not
applicable*, *unknown*, *failed* — and the middle two carry no score. A model
that cannot call a tool is not a model that calls tools badly.

**No aggregate** (B41). Each lab reports separately. There is no overall score,
no weighted average, no ranking across labs. MCF ranks only within a workflow
the user declared (§6.36).

**Customization is a workload slot, not an API** (B42, B32). A lab ships a
default workload and may accept a replacement — documents, schemas, labels,
constraints, tasks. Default-workload results are comparable and contributable;
custom-workload results are neither, and are marked at the point of production.
No lab accepts user code.

---

## Family A · Physical

Always applicable. No capability gate, because every model that loads can be
timed. Timing-class throughout, so all of these run under reduced
instrumentation with characterized overhead (B31).

### L1 — Throughput and latency

- **Question** What does a token cost here, and how does that change with shape?
- **Method** Prompt lengths × generation lengths × batch sizes, n≥30 per cell,
  cold and warm distinguished (B2). Reports tokens/second, time to first token,
  and inter-token latency distribution — not a mean, since tail latency is what
  a user feels.
- **Telemetry** Per-token arrival timestamps (monotonic, D9), accelerator
  occupancy, queue depth.
- **Slot** Prompt/generation shape grid.
- **Why first** §IV cannot recommend anything without it, and it is the only lab
  with no capability gate at all.

### L2 — Energy and thermal

- **Question** What does a token cost in joules, and what happens when the
  machine gets hot?
- **Method** Sustained generation with power sampled at a declared rate (B39),
  reported as joules per token and watts sustained, alongside time-to-throttle
  and the steady-state clock the machine settles into.
- **Telemetry** Board or package power, die temperature, clock, throttle
  reasons. Provenance stated per platform: measured, estimated, or unknown (A7,
  A20).
- **Why it matters** §3.9 lists power as a frontier axis and nothing populates
  it. On a laptop this is frequently the deciding quantity.

### L3 — Memory and context scaling

- **Question** How does memory grow with context, and where does it stop?
- **Method** Walk context length upward until allocation fails; record resident
  bytes, KV cache growth, and the exact boundary. The failure is the result
  (A9).
- **Telemetry** Accelerator and host allocation, KV cache size, fragmentation.
- **Feeds** P3's fitment arithmetic, which currently predicts this rather than
  knowing it.

### L4 — Concurrency and queueing

- **Question** How does service degrade when several requests arrive at once?
- **Method** Increasing parallel request load against a fixed workload; latency
  percentiles per concurrency level, and the level at which the tail collapses.
- **Telemetry** Queue depth, scheduling delay, per-request timing.
- **Note** Measures the *serving path*, so it is the one physical lab whose
  subject is partly MCF itself. Its overhead accounting must be honest about
  that (§3.8).

### L5 — Load, residency and cold start

- **Question** What does it cost to make this model ready, and to keep it that
  way?
- **Method** Cold load from disk, warm load from page cache, first token after
  idle, unload and reload cost.
- **Feeds** DEC-018 directly: the residency policy is a guess without these
  numbers, and residency is a §3.4 condition on every latency result.

---

## Family B · Capability

Each runs only where §X verified the capability (B40). These are the labs whose
absence-versus-failure distinction motivated §3.23.

### L6 — Instruction adherence

- **Question** Does the model do what it was told, exactly?
- **Gate** None beyond instruction-tuning; applies to nearly every model.
- **Method** Constraints with mechanically checkable satisfaction: length
  bounds, required and forbidden tokens, output ordering, formatting, "answer
  only with X". Score is the satisfaction rate per constraint type, so the
  output names *which* kind of instruction the model ignores.
- **Slot** Your constraints. This is one of the most useful customization
  surfaces, because everyone's constraints differ.

### L7 — Structured output

- **Question** Does it produce output that parses, against a schema, reliably?
- **Gate** Structured-output capability probed present.
- **Method** N generations against a schema set of graded complexity — flat,
  nested, arrays, optional fields, unions. Reports parse rate, schema-validity
  rate, and behaviour on retry after an invalid response.
- **Slot** Your schemas.
- **Why early** Cheap, fast, unambiguous, and a good template for the lab
  framework itself.

### L8 — Tool calling

- **Question** Does it call the right tool, with the right arguments, in the
  right form?
- **Gate** Tool-calling capability probed present. **This is the gate that
  motivated the correction to D2.**
- **Method** Single-call correctness first, decomposed: well-formedness,
  selection from a set of plausible tools, argument extraction, required-field
  completeness, nested arguments, parallel calls. Each reported separately,
  because "70 % tool calling" hides whether the model picks wrong tools or
  malforms right ones.
- **Slot** Your tool schemas.

### L9 — Agentic workflow

- **Question** Can it complete a multi-turn task, recover from its own errors,
  and stop?
- **Gate** Tool calling present (L8 is its precondition).
- **Method** §IX's suite: multi-turn tasks with checkable end states, run in the
  sandbox (A14), reported as a distribution with the failure taxonomy §6.17
  requires — wrong tool, malformed call, loop, early stop, gave up.
- **Slot** Your task definitions.
- **Status** The first lab built, because it is the shape this project's author
  uses. One lab among twenty, not the definition of quality (D2).

### L10 — Long-context retrieval

- **Question** How much of the declared context is actually usable?
- **Gate** None; every model has a context window.
- **Method** Needle retrieval at depths across a log-scaled length sweep, plus
  position bias (does it favour the beginning or end) and multi-fact retrieval.
  Produces the declared-versus-usable divergence that the M3 mockup shows.
- **Slot** Your documents — the most valuable slot in the catalogue, since
  retrieval over your corpus is not retrieval over synthetic text.

### L11 — Extraction

- **Question** Can it pull the right fields out of a real document?
- **Gate** Structured output helps but is not required.
- **Method** Documents with known field values; exact-match and normalized-match
  scoring per field, with a confusion breakdown of what it substitutes.
- **Slot** Your documents and your field definitions.

### L12 — Classification and routing

- **Question** Does it put things in the right bucket?
- **Gate** None.
- **Method** A labelled set; accuracy, per-class confusion, calibration if
  logprobs are available. Cheapest lab in the catalogue to run and to customize.
- **Slot** Your labels and examples. Likely the first slot most users fill.

### L13 — Code generation

- **Question** Does the code it writes run and pass the tests?
- **Gate** None, though results are meaningless for models with no code
  exposure — which §3.23's *unknown* handles.
- **Method** Generate against a specification, execute in the sandbox (A14),
  score by test outcome. The most objectively verifiable lab here: the tests
  either pass or they do not.
- **Slot** Your specifications and your test suites.

### L14 — Reasoning chains

- **Question** Does it get multi-step problems right, and does thinking longer
  help?
- **Gate** None; reasoning-mode capability if probed present changes the method.
- **Method** Problems with single checkable answers, scored on the answer, with
  chain length and latency recorded so "better but four times slower" is
  visible.

### L15 — Embeddings

- **Question** Do its vectors put similar things near each other?
- **Gate** Embedding capability probed present — frequently *not applicable*.
- **Method** Retrieval over a labelled corpus: recall@k, mean reciprocal rank,
  and near/far separation.
- **Slot** Your corpus and relevance judgments.

### L16 — Vision

- **Question** Can it read what is in the image?
- **Gate** Vision capability probed present — usually *not applicable*.
- **Method** Checkable visual questions: counting, OCR against known text, chart
  value reading, spatial relations.

### L17 — Multilingual

- **Question** Does capability hold up outside English?
- **Gate** None, but *unknown* is the honest default until measured.
- **Method** Any of the above labs re-run in other languages, plus round-trip
  translation with checkable invariants. Chiefly a *harness* rather than a lab
  in its own right.

---

## Family C · Robustness

These ask whether a measurement survives contact with variation. They are the
labs most likely to change what a user believes, and the least represented in
public benchmarking.

### L18 — Prompt-format sensitivity

- **Question** How much does the answer depend on how the question was phrased?
- **Method** One task, N semantically equivalent phrasings, same everything
  else. Reports the *variance*, not the mean. A model at 80 % ± 3 across
  phrasings is a different proposition from one at 80 % ± 25.
- **Why it may be the most valuable lab here** Every other measurement in MCF is
  taken with one phrasing. This lab measures how much to trust all of them, which
  makes it the closest thing to a calibration rig the catalogue has.

### L19 — Determinism and seed stability

- **Question** Same input, same seed — same output?
- **Method** Repeated identical requests; exact-match rate and divergence point.
- **Feeds** §7.6's reproducibility tolerance directly, and every other lab's
  interpretation of its own spread.

### L20 — Termination behaviour

- **Question** Does it know when to stop?
- **Method** Tasks with a natural end; measures runaway generation, repetition
  loops, premature stops, and stop-token adherence.
- **Note** The M6 mockup shows this as a model's dominant failure mode, and it
  is invisible in aggregate scores.

### L21 — Sustained-load stability

- **Question** Does quality drift as the machine heats up?
- **Method** A behaviour lab re-run continuously over hours, with thermal state
  recorded, checking whether outcomes correlate with throttling.
- **Bridges** Family A and Family B — the only lab that asks whether a physical
  condition changes a behavioural result, which is a question nobody asks and
  everyone assumes the answer to.

### L22 — Refusal and policy behaviour

- **Question** What does it decline, and is that consistent?
- **Method** A fixed probe set; measures refusal *consistency* and false-refusal
  rate on benign requests. Measures behaviour, never adjudicates whether a
  refusal was correct — §5 forbids MCF being a quality authority, and this is
  where that temptation is strongest.

---

## Family D · Comparative

These take two or more configurations and measure the difference. They are the
labs §IV depends on most directly.

### L23 — Quantization damage

- **Question** What did compression cost, and where?
- **Method** The same weights at N quantizations against a fixed input set,
  measuring divergence from the highest-precision variant available: output
  agreement rate, per-family behaviour change, and the point at which a
  capability disappears entirely.
- **Why it matters here** §XII's reference model publishes roughly twenty
  quantizations. This is the lab that makes that a resource rather than a
  problem, and it is a §3.4-clean comparison: one variable, many points.

### L24 — Resource dose-response

- **Question** How much does this model need before more stops helping, and how
  gracefully does it degrade below that?
- **Family** D · Comparative. **Class** Timing, so each point needs its own
  quiet window (D8) — the sweep is a sequence of short windows rather than one
  long occupation.
- **Method** The same workload run in boxes of decreasing size (D15): cores,
  host memory, accelerator share, each swept independently and then jointly.
  Reports the curve and its knees — the point below which throughput collapses,
  the point above which more buys nothing.
- **Telemetry** Per-box throughput and latency, plus what the box *failed* to
  bound: memory-bandwidth saturation, cache pressure and thermal state, since
  those are the dimensions §6.41 says a consumer box does not partition.
- **Slot** Your box sizes, and your workload.
- **Why it matters** Three separate uses. It tells a user the minimum viable
  allocation for a latency target, which no other lab answers. It quantifies
  what yielding costs the run, which §3.26 requires be measured rather than
  asserted. And it measures how much a box actually isolates on *this* hardware
  — the open question §6.41 rates its own confidence "medium" on.

### L25 — Contention response

- **Question** What happens to this model when the machine is busy?
- **Family** D · Comparative. **Class** Timing.
- **Method** A fixed workload run against graded synthetic background load —
  CPU-bound, memory-bandwidth-bound, accelerator-bound, I/O-bound, each
  separately — from idle to saturated. Reports degradation per contention type,
  which is more useful than one "busy machine" number because the four have
  different remedies.
- **Distinct from L24** L24 restricts what the model *may take*; L25 varies what
  it *may be denied*. §6.41's whole point is that these are different
  quantities, and measuring both is how MCF learns which one dominates here.
- **Why it matters** It converts D8's yielding mode from a promise into a
  quantity: "a behaviour run in the background costs the foreground this much,
  and costs itself this much."

### L26 — Sampling sweep

- **Question** Is the publisher's recommended sampling actually the best
  available here, for this workload?
- **Family** D · Comparative. **Class** Behaviour, so it yields (D8) — which
  matters, because this is the most expensive lab in the catalogue.
- **Gate** None; every model has sampling parameters.
- **Method** Coordinate sweep around the declared recommendation rather than a
  full grid — one dimension at a time, temperature first, then the truncation
  parameters, then penalties. Each arm is paired and interleaved against the
  recommendation (B53) so drift cancels. The declared value is always one of the
  arms, so the output is a *comparison against the recommendation* rather than a
  free-floating ranking.
- **Discipline that makes it honest** Selection and reporting use different
  splits (A10, D18). Picking the best of eight arms inflates the apparent gain
  even when nothing differs, so the winner is re-measured on held-out workload
  before any improvement is published. A winner that does not survive validation
  is reported as **no improvement found** — a §3.4 null result, and a common one.
- **Slot** Your workload, and the ranges swept.
- **Output** Per workflow, never global: the temperature that maximizes
  tool-call reliability need not be the one that maximizes extraction accuracy
  (D2).
- **Cost warning** Arms × trials × workflows grows fast, and D14's budget
  discipline applies with more force here than anywhere else. A sweep that
  cannot be afforded honestly is better skipped than run at an `n` too small to
  distinguish its arms.

---

## What to build first

**L1 → L7 → L9.** Three labs that exercise three different parts of the
framework: a timing-class lab with no gate, a cheap behaviour-class lab with a
capability gate, and a heavy multi-turn lab with a sandbox and a distribution.
If the framework carries those three, it carries the family.

**Then L10 and L2.**  L10 produces the declared-versus-verified divergence that
is often the most useful thing MCF can say about a model; L2 populates the
frontier axis §3.9 has never been able to fill.

**L18 early if anything is doubted.** It is the calibration rig: it tells you
how much to believe every other number in the system.

---

## Changelog

### Version 5 — the sampling sweep

L26 added, following D18: the model's recommended sampling is a declaration, and
this is the lab that verifies it. Its defining constraint is statistical rather
than technical — selecting the best of several arms inflates the apparent gain,
so selection and reporting use different splits and a winner that does not
survive validation is reported as no improvement found.

### Version 4 — the resource labs

L24 and L25 added. They are the pair that makes D8's yielding mode and D15's
boxes measurable rather than asserted: L24 restricts what the model may take,
L25 varies what it may be denied, and §6.41 turns on those being different
quantities.

### Version 3 — which families take the machine

Family A needs an exclusive window and is measured in tens of minutes; Family B
yields and is measured in hours or days. The runs that need a quiet machine are
the short ones (D8).

### Version 2 — tiers added

D13's four tiers mapped onto the families, and the note that characterization
supplies the rate D14's duration estimates depend on — which is why the tier
ordering is structural rather than a suggestion.

### Version 1 — twenty candidates drafted

Created in response to the correction that made quality plural (D2, v9). With
agentic workflow demoted from *the* definition of quality to one shape among
many, the question "which laboratories exist" stopped being deferrable —
§7.29 had been recorded and never drafted.

Four families, split by what gates them rather than by subject, because the gate
is what determines whether a lab can make a claim about a given model at all.
Nothing here is ratified: the ordering, the contents of each workload slot, and
what each lab must state about its own validity all remain open.
