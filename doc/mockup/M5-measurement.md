# M5 — The measurement

| | |
|---|---|
| **Type** | Mockup — what finished looks like at this stage |
| **Milestone** | [M5](../roadmap.md#m5--the-measurement) |
| **Version** | 2 |
| **Status** | Illustrative. Every figure is invented and none may be cited as a measurement (C7, A20). |
| **Source** | [document-of-intent.md](../document-of-intent.md) · rules: [rules.md](../rules.md) |

**Product:** `mcf bench` — a performance number taken on this hardware, with a
stated method, a sample count, a spread, and the conditions in force.

**The three honest outcomes this milestone makes possible,** and which most
tools of this kind cannot express: *within noise*, *not comparable*, and *does
not fit here*. §3.4 and §3.9 both require them, and a benchmark surface that
cannot say them will manufacture distinctions instead.

---

## 1. A run

```
$ mcf bench qwen2.5-7b --compare quantization=Q4_K_M,Q8_0

PRE-FLIGHT
  ✓ thermal steady state reached (CPU 41 °C, Accel 38 °C, drift < 0.5 °C/min)
  ✓ machine quiet — no process above 2 % CPU for 60 s
  ✓ adaptive paths disabled: no cache reuse, no warm process, no batch
    adaptation (§6.13 — the benchmark path may not adapt)
  ✓ instrumentation profile: measurement (reduced), overhead characterized
    at 0.31 % ± 0.04 and recorded as a condition (§6.2)
  ✓ isolation check: 1 variable differs between arms (quantization)

RUNNING   2 arms × 30 trials × 3 workloads, cold and warm      elapsed 24m 11s

RESULT  qwen2.5-7b · prompt 512 tok · generate 256 tok · batch 1 · cold

  quantization   tok/s              first token         accel memory
  Q8_0           38.4  [37.9–39.0]  148 ms [141–159]    9.2 GiB
  Q4_K_M         41.2  [40.6–41.9]  121 ms [116–130]    5.5 GiB
                 n=30 each · median with 5th–95th percentile

  Q4_K_M is 7.3 % faster in throughput [95 % CI 5.6–9.0], and the intervals
  do not overlap. This is a real difference under these conditions.

  ⚠ This says nothing about quality. Q4_K_M may be worse at the work you do
    and this measurement cannot see that. Agentic evaluation is M6, and
    choosing between them needs both plus a stated objective (M7).

CONDITIONS  (abbreviated — full set in the record)
  MCF 0.3.0-m5 build a41c02 · llama.cpp b4321 · Accel #0 driver 550.54.14
  CUDA 12.4 · KV cache f16 · context 32768 · thermal steady · power performance
  no other resident model · daemon idle before each trial · cold start per trial

Record: bench_2026-08-25T09-41-52Z_qwen-quant
```

## 2. Within noise — a refusal to rank

```
RESULT  qwen2.5-7b · context 8192 vs 16384 · generate 256 tok

  context   tok/s              first token
  8192      41.2  [40.4–42.1]  121 ms [115–129]
  16384     40.9  [40.1–41.8]  124 ms [117–132]
            n=30 each

  WITHIN NOISE. The difference in throughput is 0.3 tok/s [95 % CI −0.9 to 1.5],
  which includes zero. MCF will not rank these.

  If you need the larger context, take it; it costs nothing measurable here.
  Refusing to manufacture a distinction is the correct output (§3.9).
```

## 3. Not comparable — a refusal to subtract

```
$ mcf bench --compare qwen2.5-7b,mistral-7b-v0.3

  NOT COMPARABLE — 4 variables differ between the arms.

    variable          qwen2.5-7b        mistral-7b-v0.3
    quantization      Q4_K_M            Q5_K_M          ← differs
    context_length    32768             32768
    runtime           llama.cpp b4321   llama.cpp b4321
    tool_format       hermes            mistral         ← differs
    thermal state     steady 38 °C      steady 52 °C    ← differs
    driver            550.54.14         550.54.14
    parameters        7.6 B             7.2 B           ← differs

  MCF can run both and report both, and it will not present the difference as
  a delta. §3.4 requires one thing differ, and four do.

  To compare throughput specifically:
    mcf bench --compare qwen2.5-7b,mistral-7b-v0.3 \
      --hold quantization=Q4_K_M --wait-thermal --accept "parameters differ"

  The last flag is required, is recorded in the result, and appears on every
  surface that renders it. A confound you declared is science; a confound you
  did not is an error.
```

## 4. A null result, stored as a result

```
RESULT  qwen2.5-7b · flash-attention on vs off

  arm    tok/s              first token
  off    41.2  [40.4–42.1]  121 ms [115–129]
  on     41.4  [40.5–42.3]  120 ms [114–128]
         n=30 each

  NULL RESULT. No measurable speedup on this hardware for this workload.
  Effect size, if any, is below 2 % — this run could not have detected less.

  Stored and surfaced as a result, not discarded as a failure (§3.4). It is
  genuinely useful: it means the complexity of managing this flag buys nothing
  here, and MCF now has evidence for leaving it alone.
```

## 5. Does not fit here

```
$ mcf bench llama-3.1-70b --quantization Q4_K_M

  DOES NOT FIT HERE — this is a complete result, not a failure (§3.4, §6.3).

    weights at Q4_K_M          39.7 GiB
    KV cache at 32768 ctx       4.1 GiB
    required                   43.8 GiB
    Accel #0 available         24.0 GiB
    shortfall                  19.8 GiB

  Partial offload to host memory would run at an estimated 2–4 tok/s based on
  this machine's measured host-memory bandwidth. MCF has NOT measured that
  configuration and will not report an estimate as a measurement (§4).

    To measure it:  mcf bench llama-3.1-70b --offload host --accept-slow

  Recorded as a fitment result against this machine's profile. If the machine
  changes, this result is marked stale rather than silently reused.
```

## 6. Contention, marked unattributable

```
  ✗ RUN INVALIDATED — the machine was not quiet

    Category    bench.conditions.contended             (taxonomy 5.2.1)
    Detail      an unrelated process held 34–71 % of Accel #0 for 4m 12s of a
                9m 30s run, starting at trial 11
    Effect      trials 11–30 are marked CONTENDED and are excluded from the
                reported statistics. Trials 1–10 are retained and reported
                with n=10, which is below the n=30 acceptance threshold
                (DEC-007), so nothing is published from this run.
    Not done    MCF did not average across the contention. A number produced
                that way would be a claim about a busy machine presented as a
                claim about a model (§3.8).
    Retained    All 30 trials are in the record with their per-trial conditions.
                Nothing was discarded — only excluded from a claim.
    Next        mcf bench … --wait-for-quiet
    Record      bench_2026-08-25T11-02-44Z_invalid
```

## 7. The published-versus-estimated line

```
$ mcf bench --list qwen2.5-7b

PUBLISHED  (meets acceptance criteria — n ≥ 30, spread reported, thermal steady,
            adaptive paths disabled, single variable, uncontended)
  2026-08-25  throughput Q4_K_M vs Q8_0        real difference, 7.3 % [5.6–9.0]
  2026-08-25  context 8192 vs 16384            within noise
  2026-08-25  flash-attention on vs off        null result

ESTIMATES  (fast, labelled, never comparable with the above)
  2026-08-24  interactive session               ~41 tok/s, n=1, no warm-up

  An estimate cannot be promoted to a measurement. It can only be replaced by
  one (§4).
```

---

## Intent this stage satisfies

| Clause | How it shows up above |
|---|---|
| §II science | Every number carries a method, a sample count, a spread and its conditions |
| §3.4 repeatability before precision | Intervals, not decimal places; n=1 is refused |
| §3.4 isolate the variable | A four-variable comparison is refused; declared confounds are recorded |
| §3.4 null and negative results | Both are stored and surfaced as results |
| §3.9 refuse to manufacture | "Within noise, pick either" is a first-class output |
| §3.8 apparatus | Contention invalidates rather than being averaged away |
| §6.13 no adaptation while measuring | Adaptive paths disabled in pre-flight and recorded |
| §6.2 characterized instrument | MCF's own observation overhead is measured and travels as a condition |
| §6.16 no simulated timings | Every figure here comes from real hardware, by construction |
| §6.7 not a gate | Benchmarks have no pass condition and never fail CI |
| §4 estimates | Kept structurally separate and never promotable |

---

## Changelog

| Version | Change |
|---|---|
| 2 | Standardized to the format contract in [README.md](../../README.md): front matter, present tense, changelog. |
| 1 | Created alongside the roadmap, to make "done" at this stage a picture somebody can disagree with before it is code. |
