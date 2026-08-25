# M3 — Right by construction

**Product:** `mcf probe <model>` — MCF establishes what a model can actually do
by asking it to do the thing, configures accordingly, and reports where the
artifact's claims and its behaviour diverge.

**Why before any benchmarking.** §X states it: *a misconfigured model is a
measurement error*. Everything M5 and M6 measure would otherwise be partly a
measurement of MCF's ignorance of a chat template.

---

## 1. A probe run

```
$ mcf probe qwen2.5-7b

Probing qwen2.5-7b @ a09a354 · Q4_K_M · llama.cpp b4321 · Accel #0
Each probe is a bounded experiment with a method, a result and conditions (§3.18).

  chat_template            VERIFIED     8/8 turns well-formed          2.1 s
  stop_conditions          VERIFIED     20/20 stopped at <|im_end|>    3.4 s
  tool_calling             VERIFIED     47/50 well-formed calls        18.7 s
                                        ↳ 94 % [95 % CI 83–99], n=50
  structured_output/json   VERIFIED     50/50 parsed against schema    12.2 s
  context_usable           VERIFIED     32768 tokens                   41.9 s
                                        ↳ DIVERGENCE: declared 131072
  vision                   NOT PRESENT  no vision tower in weights     0.1 s
  embeddings               UNKNOWN      probe inconclusive             6.0 s
  reasoning_mode           NOT PRESENT  no reasoning tokens declared
                                        or observed                    4.4 s

  8 probes · 5 verified · 1 not present ×2 · 1 unknown · 88.8 s · 1 divergence

CONFIGURATION CHANGED BY THIS RUN
  context_length   8192 → 32768    source: probe/context_usable @ 2026-08-24
  tool_format      unset → hermes  source: probe/tool_calling @ 2026-08-24
  stop             declared → verified (unchanged value, now verified)

  Prior measurements taken under the old configuration are marked
  NOT COMPARABLE with measurements taken after it (§3.4). 0 affected.

Record: probe_2026-08-24T19-02-51Z_a09a354
```

## 2. The divergence, which is often the most useful thing MCF can say

```
$ mcf probe qwen2.5-7b --explain context_usable

PROBE  context_usable
  Question   How many tokens of context can this model use, as opposed to accept?
  Declared   131072   (config.json: max_position_embeddings)
  Verified    32768
  Divergence FACTOR OF 4

  Method
    Needle-in-haystack retrieval at 12 context lengths on a log scale from
    2048 to 131072. At each length, 8 trials, needle placed at 8 depths.
    A length passes when ≥ 7/8 trials retrieve the needle exactly.
    The reported value is the largest passing length.

  Result
     2048  8/8   4096  8/8   8192  8/8   16384  8/8   32768  7/8  ← last pass
    49152  3/8  65536  1/8  98304  0/8  131072  0/8 (also: 2 OOM at this length)

  Conditions
    quantization Q4_K_M · runtime llama.cpp b4321 · Accel #0 24.0 GiB
    rope scaling: none applied · KV cache f16 · thermal steady
    MCF 0.1.0-m3 build 1c8ef70

  What this does and does not say
    It says: under THIS quantization and THIS runtime, usable context is 32768.
    It does not say the model is incapable of more — a different quantization,
    KV cache precision or rope configuration may change this, and MCF has not
    tested those. This is a §3.4 conditioned claim, not a property of the model.

  Effect
    context_length was set to 32768. Requests above it are refused with a
    stated reason rather than silently truncated (§3.1).
```

The closing paragraph is the discipline. §3.4 says the unit of scientific output
is a number bound to its conditions, and a capability verdict is no different
from a timing in that respect.

## 3. Inconclusive is a real answer

§3.18 requires three states and §7.24 asks what MCF does with the third.

```
$ mcf probe qwen2.5-7b --explain embeddings

PROBE  embeddings
  Result     UNKNOWN — inconclusive, not negative
  Attempts   3 methods, 0 conclusive

  1. Pooled last-hidden-state via the runtime's embedding endpoint
     → runtime returned 501 Not Implemented for this model class.
  2. Similarity structure on a fixed 40-pair corpus (20 near, 20 far)
     → produced vectors, but near/far separation was 0.51 (chance ≈ 0.50).
       Consistent with "not an embedding model" AND with "extraction was
       wrong". Cannot distinguish.
  3. Declared metadata
     → silent. No embedding-related field present.

  MCF does not conclude. `embeddings` is UNKNOWN and nothing was configured
  from it. A default that made the model appear to work would corrupt every
  measurement taken under it (§3.6, §6.19).

  Next   This is a limitation of MCF's probe, not a finding about the model.
         Recorded as an open item against DEC-024.
```

## 4. Every setting answers "why this value"

```
$ mcf explain qwen2.5-7b

CONFIGURATION IN FORCE      VALUE      SOURCE            EVIDENCE
  runtime                   llama.cpp  mcf_default       —
                            b4321      (only characterized GGUF engine)
  quantization              Q4_K_M     mcf_default       rule fit-with-headroom
  context_length            32768      VERIFIED          probe/context_usable
                                                         2026-08-24T19-02Z
                                                         declared was 131072
  tool_format               hermes     VERIFIED          probe/tool_calling
                                                         47/50, 94 % [83–99]
  chat_template             tokenizer  VERIFIED          probe/chat_template 8/8
  stop                      <|im_end|> VERIFIED          probe/stop_conditions
                                                         20/20
  embeddings                —          UNKNOWN           probe inconclusive
  temperature               0.7        declared          model config, unverified

  6 of 8 settings are backed by an observation MCF made on this machine.
  2 are not, and say so.
```

## 5. The catalogue, now with earned information

```
$ mcf list --capabilities

NAME                 TOOLS      JSON    CTX (used/declared)  VISION  PROBED
qwen2.5-7b           94 % ±8    100 %   32768 / 131072 ⚠     no      2026-08-24
mistral-7b-v0.3      61 % ±13   88 %    32768 / 32768        no      2026-08-24
mystery-gguf         unknown    unknown unknown              unknown never

⚠ = declared and verified disagree. `mcf probe <name> --explain <capability>`

Percentages are success rates over n=50 trials with 95 % intervals, measured
here, under the configuration in force. They are not comparable with numbers
from any other machine, and MCF does not publish them anywhere (§5).
```

`mistral-7b-v0.3` at 61 % ±13 is the kind of thing this milestone exists to
surface: a model whose metadata claims tool support and which produces a
malformed call two times in five. Nothing before M3 could tell the difference.

## 6. Reconfiguration under a user, per DEC-025

```
$ mcf upgrade

MCF 0.1.0-m3 → 0.2.0-m3

  Probe methodology changed:
    probe/tool_calling  v1 → v2  (adds nested-argument and parallel-call cases)

  This affects 2 models whose configuration was derived from that probe.

  MCF will NOT silently reconfigure them. Yesterday's result and today's would
  not be comparable and the change would be invisible in both (§3.4, §7.25).

  Choose:
    [k] Keep current configuration. Results stay comparable. Configuration may
        now be worse than MCF could achieve. Models are marked probe-stale.
    [r] Re-probe now (≈ 3 min). Old results are marked NOT COMPARABLE with new
        ones. The invalidation is recorded and reversible.
    [d] Decide per model.
```

---

## Intent this stage is trying to satisfy

| Clause | How it shows up above |
|---|---|
| §X capability discovery | Eight capabilities established by observation and configured from it |
| §3.18 measured, not believed | Every verdict comes from asking the model to do the thing |
| §3.18 three states | VERIFIED / NOT PRESENT / UNKNOWN, with UNKNOWN acted on as unknown |
| §3.6 never inferred | The inconclusive embedding probe configures nothing |
| §6.19 derived config carries provenance | Every setting names the probe, the date and the evidence |
| §3.4 conditions | The context verdict states what it does and does not claim, and under what |
| §3.4 uncertainty | Capabilities are rates with intervals and n, never booleans |
| §7.25 / DEC-025 | An upgrade that changes a probe asks rather than drifts |
| §5 not a leaderboard | The numbers are local, conditioned and unpublished |
