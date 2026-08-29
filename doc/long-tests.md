# Long tests, and the burden they carry

| | |
|---|---|
| **Type** | Discipline — the burden a slow test carries, and an audit of every slow test here |
| **Version** | 1 |
| **Status** | Living. Verdicts follow measurement; a test's entry changes when something cheaper is measured. |
| **Authority** | Derived from [document-of-intent.md](document-of-intent.md) v17, governed by [rules.md](rules.md) |
| **Answers** | Raised by the operator: long tests must prove no cheaper route exists |

A test that takes hours is not simply an expensive test. It is a test most
people will never run, which makes it a test that finds nothing on most
machines. MCF has one long-test discipline, and it is a burden of proof:

> **A test may take a long time only where the result it seeks cannot be
> obtained any other way. The alternative that was rejected is named, with the
> measurement that rejected it.**

Slowness is never a property to be accepted because a test is thorough. It is
a claim — *this cannot be had faster* — and like every other claim in this
repository it is either measured or it is unknown (A21).

## Why the burden is real: the context ladder

The ladder characterised generation speed against context depth by generating
to each depth at each allocation, three times over. It ran for 100 minutes,
was killed before finishing, and would have taken about three hours.

Three separate things were wrong with it, and each was found by asking what
the test was actually buying.

**It measured the same thing five times.** It swept five allocations because
an allocation effect appeared to exist. Run with the allocations interleaved
instead of ascending, the effect vanished — spread across allocations 2.5%
against 2.2% across passes (F119). What the ladder had measured was its own
first readings being slow. Four of the five sweeps bought nothing.

**It generated its way to depth.** To measure the cost of a token at depth
4096, the cache must hold 4096 tokens, and the ladder got there by generating
them — which is quadratic, and spends nearly all of its time on the tokens
nobody is timing. A prompt reaches the same depth by prefill, which is
batched. Measured on both models: the two agree within −2.1% to +2.8%, inside
the noise, and prefill arrives **10–24× faster** (F120).

**It was buying depth, not accuracy.** Once a depth is cheap to reach, the
rate there is *measured* rather than extrapolated — which matters, because the
estimator lab showed a fitted curve cannot state its own error honestly: a
two-sigma interval covered the truth between 4% and 79% of the time instead of
95%, and where the curve bends past the fitted range every candidate form is
about 41% wrong with no way to know it.

The replacement measures five depths on a model in about six seconds. It is
not a cheaper approximation of the ladder; it is strictly better evidence,
because every point in it is a measurement and none is an extrapolation.

## The audit

| test | what it seeks | cheaper route? |
|---|---|---|
| context ladder | rate against depth | **retired** — prefill, 10–24× quicker, measured not fitted |
| sustained generation | rate against generated length | **superseded** — the depth axis is the real one |
| generation timing | intercept vs slope | **kept, shortened** — the intercept needs a real cold start; the slope no longer needs generating to depth |
| memory hierarchy | where the cache edges are | **kept** — 4 minutes, once per machine, and every fall-off slope depends on it |
| soak | drift over a long run: descriptors, memory, clocks | **irreducible** — the claim *is* about duration; a short soak tests nothing |
| reproducible build | two builds agree bit for bit | **irreducible** — the second build is the evidence |
| from-scratch | the static artifact runs in an empty environment | **irreducible** — the empty environment is the point |
| mutation | that the suite notices deliberate damage | **bounded, not shortened** — cost is the mutant catalogue; keep the catalogue small and justified |
| load | claims hold under many callers at once | **irreducible** — concurrency is the variable |
| oracle | MCF's engine against a reference | **irreducible** — two engines must both run |
| online | a real model from the real hub | **irreducible** — the network is the variable |
| corpus | conformance across many models | **partly reducible** — anything computable from the file header should not be measured per model |
| fuzz | mutated input lands in a known category | **reducible in principle** — coverage-guided input selection would find more per second; not built |
| budget | MCF's own cost against its ceiling | **kept** — needs the exclusive window, which is most of its wall clock |
| seed set | the published seeds are representative | **kept** — bounded by the seed count |

## What replaced the long path here

The fall-off is attention re-reading the whole KV cache once per generated
token. That gives three ways to know a model's curve, in increasing cost, and
the discipline is to take the cheapest one that applies:

1. **From the file, free.** KV bytes per token of depth is layers × KV heads ×
   (key + value length) × 2, all in the GGUF header. Divided by this machine's
   measured DRAM bandwidth it gives the slope, with no generation at all.
   Applies where each layer's read is large enough to stream — also decidable
   from the header.
2. **From a prefilled probe, seconds.** Prefill to a depth, generate 128
   tokens, measure. Five depths in about six seconds on a small model.
3. **From generating to depth, minutes to hours.** Only where the first two do
   not apply, and the reason is stated.

A diagnostic that reaches for (3) without an argument against (1) and (2) is
not being careful. It is being slow.

## Changelog

### Version 1 — the ladder that was twenty times longer than it needed to be

Created because the operator asked that a long-running test be made to prove it
cannot get its result any other way. The occasion was the context ladder, which
ran for 100 minutes, was killed unfinished, and would have taken three hours:
four of its five allocation sweeps measured an effect that turned out to be its
own running order ([F119](findings.md)), and the fifth reached depth by
generating when a prompt reaches it 10-24x faster and agrees within 2.8%
([F120](findings.md)).

The audit table is a first pass. Seven tiers are irreducible for a stated
reason — duration, concurrency, the network or a second build is the variable
they vary — and two, fuzz and corpus, are marked reducible in principle with
nothing yet measured to say by how much. Those two are claims awaiting
evidence, not verdicts.
