# M9 — The exchange

| | |
|---|---|
| **Type** | Mockup — what finished looks like at this stage |
| **Milestone** | [M9](../roadmap.md#m9--the-exchange) |
| **Version** | 1 |
| **Status** | Illustrative. Every figure is invented and none may be cited as a measurement (C7, A20). |
| **Source** | [document-of-intent.md](../document-of-intent.md) · rules: [rules.md](../rules.md) |

**Product:** `mcf share` — evidence leaves this machine deliberately, row by row,
and never comes back. `mcf import <identifier>` — a configuration found
elsewhere is reproduced here exactly, and believed only after this machine has
measured it.

**The two most irreversible acts MCF performs live here.** Publishing data
cannot be undone, and acting on a stranger's configuration downloads weights and
may execute their code. Both surfaces are built to make the operator feel the
weight of that rather than to be smooth.

---

## 1. What would leave, shown as rows

```
$ mcf share --preview

CONTRIBUTION  47 rows · 12.4 KiB · from record 2026-08-25 → 2026-09-14

WHAT LEAVES — these rows, not a description of them:

  measurements (18)
    model                 quant    metric        value    n   spread        accel
    qwen3.8-27b           Q4_K_M   tok/s          38.4    30  [37.9–39.0]   RTX 4090
    qwen3.8-27b           Q4_K_M   first_token   121 ms   30  [116–130]     RTX 4090
    qwen3.8-27b           Q8_0     tok/s          31.2    30  [30.6–31.9]   RTX 4090
    … 15 more                                                        [ show all ]

  capability verdicts (9)
    qwen3.8-27b  tool_calling      verified   94 % [83–99] n=50
    qwen3.8-27b  context_usable    verified   32768  (declared 131072) ⚠
    … 7 more                                                         [ show all ]

  lab results (14)
    lab/agentic-core  rev 4   success 71 % [66–76] n=280   bimodal
    lab/context-degradation rev 2   knee at 24k               n=96
    … 12 more                                                        [ show all ]

  conditions attached to every row above (6 distinct condition sets)
    accelerator model, driver version, CUDA runtime, host memory, thermal state
    at run, quantization, context length, KV precision, runtime build,
    harness version, MCF version, instrumentation profile

WHAT DOES NOT LEAVE — and cannot, because it was never in this database:
  prompts · completions · file paths · anything you typed
  These live in a separate store (A25). This is structural, not a filter: a
  filter can be misconfigured; a store that never held the data cannot leak it.

WHAT THIS DOES NOT PROTECT
  The rows above identify this machine. An RTX 4090 on driver 550.54.14 with
  62.6 GiB of host memory, running these particular models at these times, is
  close to unique. MCF does not claim this contribution is anonymous, because
  it is not, and claiming otherwise would be the more dangerous error (§7.27).

  Coarsened before sending:  driver → 550.x · host memory → 64 GiB bucket
  Withheld entirely:         hostname, username, local paths, wall-clock offsets
  Sent as-is:                accelerator model, quantization, all measurements

THIS CANNOT BE UNDONE. Once sent, these rows may be copied, indexed, aggregated
and retained by people who cannot be reached. There is no recall.

Nothing has been sent. `mcf share --send` sends exactly what is above.
```

The last three blocks are the mockup's whole argument. §3.20 requires the user
see the rows, know what is not protected, and be told at the moment of the
decision that the act is irreversible — not in a document they will read later.

## 2. Sending

```
$ mcf share --send

  AUTHORIZATION REQUIRED — publication (gated category 5 of 5, A16)

  47 rows · 12.4 KiB · destination: <endpoint from DEC-027>
  Reviewed: 2026-09-14T10:02:11Z (the preview above, unchanged since)

  This is opt-in, once, for these rows. It does not enable future sharing, does
  not persist a preference, and does not survive a restart as a setting.

  Type SEND to confirm:
```

There is no `--yes`, no configuration key that makes this automatic, and no
prompt MCF raises on its own initiative. §7.31 records the open question of
whether MCF may *ever* ask rather than wait to be asked; until it is answered,
MCF waits.

## 3. Importing an identifier

```
$ mcf import mcf1:qwen3.8-27b:q4km:c32768:llamacpp-b4321:7f3a91c2

Resolving identifier                                                    41 ms
  binds        weights   unsloth/Qwen3.8-27B-GGUF @ 3f9c1a2
               quant     UD-Q4_K_M
               context   32768
               runtime   llama.cpp b4321
               sampling  temperature 0.7 · top_p 0.8 · repeat_penalty 1.05
  self-describing: yes — this identifier resolves without contacting a server
  carries        1 measurement set from the machine that emitted it

  ⚠ AUTHORIZATION REQUIRED — this downloads 16.4 GiB and may execute
    repository code. An identifier pasted from a website earns no exemption
    from the gates any acquisition passes (§6.29, A15, A16).

  Proceed? [y/N] y

Acquiring  ████████████████████████  16.4 GiB                          78 s
Applying   quantization · context · runtime · sampling                  ✓

REPRODUCED — and not yet believed.

  Every parameter above reads DECLARED, attributed to this identifier. The
  numbers that travelled with it belong to the machine that emitted them:

    metric        theirs                    yours
    tok/s         44.1  [43.2–45.0]         not measured
    first_token   98 ms [92–107]            not measured

  MCF has measured nothing here. Nothing on this configuration is verified
  until this machine takes its own numbers (§3.21).

    Verify:  mcf probe qwen3.8-27b && mcf bench qwen3.8-27b
```

## 4. Verification, and the finding it produces

```
$ mcf bench qwen3.8-27b --against-import

RESULT  qwen3.8-27b · Q4_K_M · ctx 32768 · n=30 · thermal steady

  metric        imported claim         measured here          divergence
  tok/s         44.1  [43.2–45.0]      38.4  [37.9–39.0]      −13 % ⚠
  first_token   98 ms [92–107]         121 ms [116–130]       +23 % ⚠

  DIVERGENCE RECORDED — this is a finding, not a failure of the import.

  The configuration reproduced exactly; the numbers did not. Comparing the
  conditions shows why the comparison was never going to hold:

    condition            theirs              yours
    accelerator          RTX 4090            RTX 4090
    driver               560.x               550.54.14        ← differs
    host memory          128 GiB             64 GiB
    thermal at run       steady 34 °C        steady 41 °C     ← differs
    instrumentation      unknown             reduced          ← unknown

  Four variables differ, one of them unknown, so MCF does not attribute the
  gap (A8, B24). What it can say: this configuration runs on your machine, and
  it runs slower here than where it was measured.

  This is evidence about how far results travel between machines, which is
  precisely what §XIV exists to accumulate. Recorded and contributable.
```

## 5. Emitting an identifier

```
$ mcf identify qwen3.8-27b

mcf1:qwen3.8-27b:q4km:c32768:llamacpp-b4321:7f3a91c2

  binds       the exact weights revision, quantization, context, runtime and
              sampling parameters in force
  omits       your measurements, your hardware, your record — an identifier
              names a configuration, never a claim about it (§6.29)
  resolves    offline, by construction: everything needed to reproduce is in
              the string, so nobody needs to trust a lookup service

  Sharing this identifier publishes nothing. It is a name for a setup, and it
  is the only thing MCF emits that is not a contribution.
```

## 6. What foreign data may never do

```
$ mcf recommend

OBJECTIVE  "interactive" (MCF default — you have not stated one)
EVIDENCE   6 configurations · all measured on this machine

  Contributed and imported measurements available: 1 204
  Contributed and imported measurements used:          0

  MCF contributes outward and decides inward (B34, §6.28). An aggregate from
  other machines may tell you what to try; only a measurement taken here tells
  you what to run. MCF does not compute or display a ranking, and no number it
  did not take can reach this recommendation.

RECOMMENDED  qwen3.8-27b · Q4_K_M · ctx 32768 …
```

`used: 0` is not a limitation being apologized for. It is the anti-leaderboard
rule (§5) surviving contact with a crowd-sourcing intent, stated where a user
can check it.

---

## Intent this stage satisfies

| Clause | How it shows up above |
|---|---|
| §XIV the shared record | A contribution the user inspects row by row before it leaves |
| §XV reproduce by identifier | An identifier resolves offline to an exact configuration and reproduces it |
| §3.20 publication is irreversible | The rows are shown, the irreversibility is stated at the decision, the act is per-share |
| A16 five gated categories | Publication is gated like untrusted execution and destruction |
| A25 no content, structurally | What cannot leave, cannot leave because it was never written to this database |
| §7.27 honest about anonymity | MCF states what the contribution does *not* protect rather than implying protection it cannot deliver |
| §3.21 declared until verified | An imported configuration reads `declared`; the emitter's numbers stay attributed to them |
| §6.29 reproduce, then measure | Exact reproduction is the obligation; identical results are not |
| A8 confounded comparisons | Four differing conditions, one unknown — so the gap is recorded, not attributed |
| §6.28 contribute outward, decide inward | `used: 0`, shown to the user rather than asserted in a document |
| §5 not the website | MCF emits and resolves; it does not host, rank or display |

---

## Changelog

| Version | Change |
|---|---|
| 1 | Created with M9, when §XIV and §XV added the exchange to the roadmap. |
