# M7 — The loop

**Product:** `mcf recommend` — a declared objective in, a configuration out,
with its reasoning, its measurements, its runners-up, and a re-measurement
confirming the prediction held.

**This is the reason MCF is worth building** (§1): *measure, compare, select,
re-measure*. It is also the milestone where MCF is most at risk of becoming an
oracle, which §3.9 forbids. Every screen below is built so the recommendation
can be interrogated rather than obeyed.

---

## 1. Declaring an objective

MCF refuses to invent one (§6.5), and ships a visible default rather than a
hidden one (§3.15).

```
$ mcf objective

CURRENT OBJECTIVE  "interactive"  ← MCF's default, not your choice

  MCF has no statement from you about what you value, so it is using its
  shipped default. This is a default, it is visible, it is overridable, and
  it travels with every recommendation made under it.

  interactive:
    require   first token ≤ 300 ms
    require   fits in accelerator memory with 20 % headroom
    maximize  agentic task success (M6)
    tie-break throughput, then memory

  Available:      interactive · batch · frugal · quality-at-any-cost
  State your own: mcf objective set --require "first_token<200ms" \
                                    --maximize "success" --tiebreak "memory"

  Why MCF has a default at all: §VI asks for frictionless ease, and a tool
  that refuses to have an opinion out of the box cannot be frictionless.
  Why it is loud about it: §6.5 requires that an unstated objective produce a
  visible default, never an emergent one.
```

## 2. The recommendation

```
$ mcf recommend

OBJECTIVE  "interactive" (MCF default — you have not stated one)
EVIDENCE   6 configurations across 3 models
           M5 performance: 30 trials each, thermal steady, uncontended
           M6 agentic:     280 trials each, suite core rev 4
           all measured on this machine between 2026-08-25 and 2026-08-27

RECOMMENDED

  qwen2.5-7b · Q4_K_M · ctx 32768 · llama.cpp b4321 · all layers on Accel #0

    first token     121 ms  [116–130]     requirement ≤ 300 ms      ✓
    memory          5.5 GiB of 24.0        requirement 20 % headroom ✓
    task success    71 %    [66–76]        maximized                 ← best
    throughput      41.2 tok/s [40.6–41.9] tie-break                 ← best

WHY NOT THE OTHERS

  qwen2.5-7b · Q8_0        success 73 % [68–78] — HIGHER, but the difference
                           from the recommendation is 2 points [−5 to +9],
                           which includes zero. MCF will not rank these on
                           quality. It falls to the tie-break, where Q8_0 is
                           7 % slower and uses 3.7 GiB more. Chosen against on
                           the tie-break alone, not on quality.

  mistral-7b-v0.3 · Q4_K_M success 64 % [59–70] — lower, and the difference
                           from the recommendation is 7 points [0.2–13.9]:
                           weak evidence, not strong. If you value
                           `stop-when-done`-shaped work, this model is BETTER
                           at it by 30 points and the overall ranking is
                           misleading for you.  detail ▸

  llama-3.1-70b · Q4_K_M   DOES NOT FIT — needs 43.8 GiB, this machine has
                           24.0 GiB. Not a failure; a fitment result.

  qwen2.5-7b · ctx 131072  NOT MEASURABLE — declared 131072, verified usable
                           32768 (M3 finding). MCF will not benchmark a
                           configuration it knows to be misconfigured.

HONEST CAVEATS
  · Two of the three arms are within noise on the maximized dimension. This
    recommendation is decided by the tie-break, not by quality. If memory and
    speed do not matter to you, Q8_0 is an equally defensible choice.
  · Task success comes from suite core rev 4, which is a proxy (§3.19). It
    resembles your work only to the extent the suite does.
  · Nothing here is a claim about any other machine (§5).

Record: rec_2026-08-27T16-08-30Z
```

The first caveat is the milestone's whole character. MCF's recommendation is
*correct* and *weak*, and it says both.

## 3. The frontier, because the recommendation is one point on it

```
$ mcf frontier --axes success,first_token

  success
  75 % ┤                       ○ Q8_0 (73 %, 148 ms)
       │                    ● Q4_K_M (71 %, 121 ms)  ← recommended
  70 % ┤
       │
  65 % ┤              ○ mistral Q4_K_M (64 %, 109 ms)
       │
  60 % ┤        ○ mistral Q5_K_M (62 %, 131 ms)   dominated by mistral Q4_K_M
       └────┬─────────┬─────────┬─────────┬────────
          100 ms    125 ms    150 ms    175 ms
                        first token

  ● recommended   ○ alternative   Pareto-dominated points are marked as such.
  Shaded intervals omitted in ASCII; each point carries [95 % CI] on both axes
  and the overlap is why two of these are not separable. Full data: mcf frontier --json
```

## 4. Constructing it, not just naming it

§6.6 reads "build" as compose-and-configure. That means materializing the
configuration and checking the prediction, not printing a suggestion.

```
$ mcf recommend --apply

APPLYING  qwen2.5-7b · Q4_K_M · ctx 32768 · llama.cpp b4321 · Accel #0

  Quantizing from F16 source weights        ████████████  6m 41s
    tool: llama.cpp quantize b4321 (supervised subprocess)
    provenance: derivative of art_qwen2.5-7b-instruct_a09a354
                transformation recorded, source weights retained
  Configuring from M3 probe results          ✓
  Re-probing capabilities under the new quantization    2m 12s
    tool_calling 92 % [80–98] — was 94 % at Q4_K_M/32768; within noise
    context_usable 32768 — unchanged
  Verifying the prediction                   9m 30s

  PREDICTED vs OBSERVED
    first token     121 ms predicted   ·  119 ms observed  [113–127]   ✓ within
    throughput      41.2 predicted     ·  41.0 observed    [40.3–41.8] ✓ within
    memory          5.5 GiB predicted  ·  5.5 GiB observed             ✓ exact

  The loop closed. The prediction held under re-measurement, which is the only
  thing that makes the recommendation more than an opinion (§1).

  Had it not held, that would be the finding: a divergence between predicted
  and observed is recorded and investigated, never smoothed (§6.1).

Record: rec_2026-08-27T16-08-30Z applied and verified
```

## 5. Not training on the test

```
$ mcf recommend --audit

  SELECTION suite   core rev 4        14 tasks   used to choose
  VALIDATION suite  holdout rev 2      6 tasks   used to confirm, never to choose

  The recommended configuration scores:
    selection   71 % [66–76]
    validation  69 % [61–77]
    gap +2 points [−8 to +12] — no signal of selection overfitting

  These suites share no task, no template and no generator seed. The separation
  is structural, because §3.4 warns specifically about MCF's own tendency to
  tune toward whatever it measures — and MCF is the thing doing the choosing
  here, so it is the thing most likely to overfit.
```

## 6. When there is nothing to recommend

```
$ mcf recommend

  NO RECOMMENDATION.

  Under objective "interactive", all three measured configurations are within
  noise of one another on the maximized dimension, and no tie-break separates
  them by more than 3 %.

    qwen2.5-7b Q4_K_M   71 % [66–76]   121 ms   5.5 GiB
    qwen2.5-7b Q8_0     73 % [68–78]   148 ms   9.2 GiB
    mistral-7b Q4_K_M   70 % [65–75]   109 ms   5.1 GiB

  Pick any of them. MCF could rank them and the ranking would not be evidence.

  If this is unsatisfying: it is the honest state of the measurements. More
  trials would narrow the intervals (≈ 4 h for 560 more per arm), or a sharper
  objective would let a tie-break decide. Both are choices you should make
  deliberately rather than have MCF make them for you.
```

---

## Intent this stage is trying to satisfy

| Clause | How it shows up above |
|---|---|
| §1 the closed loop | Measure → compare → select → apply → re-measure, with the prediction checked |
| §IV optimization | The frontier is mapped and a point on it is chosen by a declared objective |
| §6.5 declared objective | MCF refuses to invent one; its default is loud, attributed and overridable |
| §3.9 not an oracle | Every recommendation shows its reasoning, its runners-up and why each lost |
| §3.9 refuse to manufacture | "No recommendation — pick any" is a supported output |
| §6.6 "build" means compose | The configuration is materialized and validated, not merely named |
| §3.6 provenance | The quantized derivative traces to its source weights |
| §3.4 no training on the test | Selection and validation suites are structurally separate and audited |
| §6.1 honesty first | A prediction that failed would be a recorded finding, not a smoothed one |
| §5 not a leaderboard | Every claim is bounded to this machine and this suite |
