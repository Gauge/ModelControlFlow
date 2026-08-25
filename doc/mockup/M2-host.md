# M2 — The host

| | |
|---|---|
| **Type** | Mockup — what finished looks like at this stage |
| **Milestone** | [M2](../roadmap.md#m2--the-host) |
| **Version** | 2 |
| **Status** | Illustrative. Every figure is invented and none may be cited as a measurement (C7, A20). |
| **Source** | [document-of-intent.md](../document-of-intent.md) · rules: [rules.md](../rules.md) |

**Product:** `mcf serve` (the daemon) and `mcf run <model>` — the distance
between having a model and using a model is one command.

**The bar is §VI's:** near-zero friction, matched or beaten. **The divergence is
§3.15's:** fewer decisions, not hidden ones. Everything MCF chooses for the user
is visible, attributed and overridable, which is where MCF intends to be *better*
than the tools cited as its ease benchmark rather than merely equal to them
(§6.14).

---

## 1. First token, cold machine

```
$ mcf run Qwen/Qwen2.5-7B-Instruct

Starting daemon (not running)                                        31 ms
Selecting configuration                                              4 ms
Loading  Qwen/Qwen2.5-7B-Instruct  Q4_K_M  ████████████████  4.4 GiB   2.1 s
Ready    endpoint http://127.0.0.1:11711/v1  ·  name: qwen2.5-7b

> what is the boiling point of water at 3000 m?

Around 90 °C. Atmospheric pressure drops with altitude, so water reaches
its boiling point at a lower temperature…

  184 tok · 41.2 tok/s · first token 121 ms
  ⚠ These figures are not a measurement. n=1, no warm-up, no thermal
    steady state, ambient load unaccounted. Use `mcf bench` (M5) for a
    number MCF is willing to defend (§3.4, §4).

>
```

The warning is not decoration. §4 permits fast, clearly-labelled *estimates*
provided an estimate can never be mistaken for a measurement, and this is what
that permission looks like when it is honoured at the point of display.

## 2. Why these defaults

```
$ mcf explain qwen2.5-7b

CONFIGURATION IN FORCE                    VALUE          SOURCE
  Runtime                                 llama.cpp b4321  default (only
                                                           characterized engine
                                                           for GGUF at M2)
  Quantization                            Q4_K_M         MCF default
  Context length                          8192           MCF default
  Placement                               Accel #0, all layers   MCF default
  Sampling: temperature                   0.7            model config (declared)
  Sampling: top_p                         0.8            model config (declared)
  Chat template                           from tokenizer_config.json  DECLARED,
                                                                      UNVERIFIED
  Stop conditions                         <|im_end|>     DECLARED, UNVERIFIED

WHY Q4_K_M
  Rule: largest quantization whose weights fit in accelerator memory with
  headroom for the context, preferring higher precision where both fit.
  Here: F16 needs 15.2 GiB + 1.1 GiB context = 16.3 GiB of 24.0 GiB — fits.
        Q8_0 needs  8.1 GiB + 1.1 GiB = 9.2 GiB — fits.
        Q4_K_M needs 4.4 GiB + 1.1 GiB = 5.5 GiB — fits.
  All three fit, so the rule alone does not decide. The default preference at
  M2 is Q4_K_M.

  ⚠ This is a DEFAULT, not a MEASUREMENT. MCF has not measured what any of
    these cost or are worth on this machine. It has no basis at M2 for
    claiming Q4_K_M is the right choice for you — only for being transparent
    that it chose it. §6.5 forbids inventing an objective; the objective
    function is DEC-002, and the measurements are M5–M7.

  Override: mcf config set qwen2.5-7b quantization=Q8_0

WHY "DECLARED, UNVERIFIED"
  The chat template and stop conditions come from the artifact's own metadata,
  which §3.7 treats as untrusted and §3.18 forbids believing. A model whose
  template is wrong will look mediocre and MCF cannot currently tell. Probing
  is M3.
```

This screen is §3.15 in full: nothing blocked the first token, and every choice
that got there is interrogable afterwards. The warning inside it is §6.1 — MCF
would rather admit it has no basis for a claim than let a default look like a
finding.

## 3. The daemon

```
$ mcf status

DAEMON      running · pid 44121 · up 6 d 4 h 11 m · 3 restarts since install
            rss 9.2 MiB (idle budget ≤ 12 MiB) · idle CPU 0.00 % · wakeups/min 0

RESIDENT    qwen2.5-7b        4.4 GiB on Accel #0 · idle 41 m
                              residency policy: keep-until-pressure (see below)

ENDPOINTS   http://127.0.0.1:11711/v1     serving API   local only
            http://127.0.0.1:11711/ctl    control API   local only
            Network exposure: OFF. Turning it on is a deliberate act (§6.12).

RECENT      3 classified failures in 6 d · 0 unclassified · 0 silent
            last: child.exit.signal at 2026-08-22T04:11:09Z — recovered in 1.4 s
```

`0 unclassified · 0 silent` is the daemon's headline claim, and it is the one
§3.1 actually makes. "Never fail" was redefined in §6.1 as "never lose
information", and this is the line that reports on that promise rather than on
uptime.

## 4. A runtime dying, survived

```
2026-08-22T04:11:09.221Z  FAILURE  child.exit.signal            (taxonomy 2.1.0)
  Subsystem   mcf-serve::supervise
  Managed     llama.cpp b4321, pid 44903, serving qwen2.5-7b
  Detail      killed by SIGKILL 41 ms into token 12 of a 400-token completion
  Cause       host OOM killer; system memory 62.6 GiB, 61.9 GiB in use by
              an unrelated process (`<redacted>`, 48 GiB RSS)
  Attribution NOT MCF. The manager was using 9.2 MiB at the time.
  Effect      PARTIAL RESULT. 11 tokens were produced and are preserved.
              The request returned a partial completion marked truncated —
              not an error, and not a silent success (§3.1).
  Recovery    child respawned at +1.4 s; model reloaded from page cache;
              the endpoint was unavailable for 1.4 s and said so.
  Not done    The request was NOT retried. Retrying would have produced a
              different completion under different conditions and presented
              it as the same one (§6.1).
  Reproduce   mcf lab run scenario/child-oom-killed-midstream
  Record      fail_2026-08-22T04-11-09Z_9d02
```

`Attribution: NOT MCF` is §3.8 doing its job — MCF knows the difference between
"this model is slow" and "this machine was busy", and says which.

## 5. The API, which the window will also use

§6.21 requires the interface be a client of the same API a script uses, so the
control surface is designed at M2 for a consumer that does not exist yet.

```
$ curl -s localhost:11711/ctl/v1/models | jq '.[0]'
```
```json
{
  "name": "qwen2.5-7b",
  "artifact": "art_qwen2.5-7b-instruct_a09a354",
  "state": "resident",
  "resident_since": "2026-08-24T18:44:02Z",
  "config": [
    { "key": "quantization",   "value": "Q4_K_M",
      "source": { "kind": "mcf_default", "rule": "fit-with-headroom", "explain": "/ctl/v1/explain/qwen2.5-7b#quantization" } },
    { "key": "context_length", "value": 8192,
      "source": { "kind": "mcf_default", "rule": "conservative-8k" } },
    { "key": "chat_template",  "value": "<from tokenizer_config.json>",
      "source": { "kind": "declared", "verified": false } }
  ],
  "capabilities": { "state": "unprobed" },
  "measurements": []
}
```

`"measurements": []` is deliberate. At M2 MCF has nothing it is willing to
publish about this model, and an empty list is the honest rendering of that.

## 6. Serving is local until it deliberately is not

```
$ mcf expose --lan

  AUTHORIZATION REQUIRED — this makes MCF reachable from other devices.

  What becomes reachable:
    the serving API   — anything on this network can send prompts to your models
    the control API   — anything on this network can acquire models, execute
                        repository code you have authorized, delete artifacts,
                        and read every record MCF holds

  Access to the control plane is access to a service that downloads and runs
  code from the internet. MCF treats that with more seriousness than a
  localhost developer tool would (§6.12).

  This will bind 0.0.0.0:11711 on interface wlan0 (192.168.1.24).
  It is revocable with `mcf expose --off` and does not survive a reboot
  unless you pass --persist.

  Protection: <the mechanism DEC-017 chooses; unresolved at M2>

Expose? Type the interface name to confirm:
```

---

## Intent this stage satisfies

| Clause | How it shows up above |
|---|---|
| §VI hosting | One command from cold machine to first token; models addressed by name |
| §3.15 ease | Nothing blocked the token; `mcf explain` returns the actual reasoning afterwards |
| §6.14 gate by category | Exposure and destruction are asked; quantization and placement flow |
| §3.1 / §6.1 | Partial completions preserved and marked; no retry-until-pretty |
| §3.8 apparatus | The OOM failure is attributed away from MCF, with the manager's own footprint as evidence |
| §3.13 idle | `wakeups/min 0` with a model resident |
| §6.21 parity | The control API is designed at M2 for the M4 window and for scripts equally |
| §6.12 local by default | Exposure is a typed confirmation, revocable, non-persistent by default |
| §4 estimates | The chat figures are labelled as not-a-measurement at the point of display |
| §3.18 unverified | The chat template is shown as declared, with the consequence spelled out |

---

## Changelog

| Version | Change |
|---|---|
| 2 | Standardized to the format contract in [README.md](../README.md): front matter, present tense, changelog. |
| 1 | Created alongside the roadmap, to make "done" at this stage a picture somebody can disagree with before it is code. |
