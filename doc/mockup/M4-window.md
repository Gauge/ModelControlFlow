# M4 — The window

**Product:** a page served by the daemon at `http://127.0.0.1:11711/` showing
the catalogue, serving state, capability findings and the failure record —
readable on the machine itself and on a handheld device once the operator
deliberately exposes it.

**Rendered wireframe:** [M4-window.html](M4-window.html) — open it in a browser.
It is a static file with no script, no framework and no external request, which
is the point being made rather than a shortcut taken.

**What the window is.** §3.14: a window, not an application. It holds no logic,
no authority, and nothing hides in it. §XI adds parity — it introduces no action
that exists only there — and §6.21 notes this is self-enforcing, because a
capability reachable only through the interface is one §VIII cannot test.

---

## 1. Overview

```
┌──────────────────────────────────────────────────────────────────────────────┐
│ MCF  0.2.0-m4                                    local only · expose: off  ⓘ │
├──────────────────────────────────────────────────────────────────────────────┤
│                                                                              │
│  DAEMON      running · up 6 d 4 h · 3 restarts since install                  │
│              rss 9.2 MiB / 12 MiB budget · idle CPU 0.00 % · wakeups/min 0    │
│                                                                              │
│  MACHINE     Ryzen 9 7900X · 62.6 GiB · RTX 4090 24.0 GiB (characterized)     │
│              + Intel UHD 770 — recognized, NOT characterized                  │
│              thermal: idle steady · no throttling                             │
│                                                                              │
│  RESIDENT    qwen2.5-7b   4.4 GiB · Accel #0 · idle 41 m · keep-until-pressure │
│                                                                              │
│  RECORD      3 classified failures in 6 d · 0 unclassified · 0 silent         │
│                                                                              │
├──────────────────────────────────────────────────────────────────────────────┤
│  MODELS                                                                      │
│                                                                              │
│  qwen2.5-7b                                          resident   ▸            │
│    a09a354 · apache-2.0 · provenance complete · probed 2026-08-24            │
│    tools 94 % ±8 (n=50) · json 100 % · ctx 32768 of 131072 declared ⚠        │
│                                                                              │
│  mistral-7b-v0.3                                     on disk    ▸            │
│    e0bc86c · apache-2.0 · provenance complete · probed 2026-08-24            │
│    tools 61 % ±13 (n=50) · json 88 % · ctx 32768 of 32768                    │
│                                                                              │
│  mystery-gguf                                        on disk    ▸            │
│    unknown revision · UNKNOWN licence · provenance partial (3 unknown)       │
│    never probed — nothing is known about what this model can do              │
│                                                                              │
├──────────────────────────────────────────────────────────────────────────────┤
│  Every figure on this page was measured on this machine under the            │
│  configuration in force. Hover or expand any of them for its conditions.     │
│  Nothing here is comparable with a number from another machine.              │
└──────────────────────────────────────────────────────────────────────────────┘
```

Three deliberate absences: no sparkline, no live graph, no auto-refreshing
counter. §6.11 forbids a UI that polls a busy machine to look responsive, and
§3.3 already refused the metric stream such a graph would need.

## 2. A model, expanded

```
┌──────────────────────────────────────────────────────────────────────────────┐
│  ◂ qwen2.5-7b                                                    resident    │
├──────────────────────────────────────────────────────────────────────────────┤
│  PROVENANCE                                                                  │
│    huggingface.co/Qwen/Qwen2.5-7B-Instruct @ a09a35458c70…                   │
│    pinned from main · retrieved 2026-08-24T18:23:14Z by MCF 0.1.0-m1         │
│    apache-2.0 (LICENSE file) · 8 files · all checksums verified              │
│    trust_remote_code: not required · transformations: none · unknown: none   │
│                                                                              │
│  CONFIGURATION                            value        source                │
│    runtime                                llama.cpp    mcf default   why ▸   │
│    quantization                           Q4_K_M       mcf default   why ▸   │
│    context_length                         32768        VERIFIED      why ▸   │
│    tool_format                            hermes       VERIFIED      why ▸   │
│    chat_template                          tokenizer    VERIFIED      why ▸   │
│    temperature                            0.7          declared      why ▸   │
│    embeddings                             —            UNKNOWN       why ▸   │
│                                                                              │
│  CAPABILITIES                                                                │
│    tool_calling      94 %  [83–99]  n=50   verified 2026-08-24  method ▸     │
│    structured/json  100 %  [93–100] n=50   verified 2026-08-24  method ▸     │
│    context_usable   32768                  verified 2026-08-24  method ▸     │
│      ⚠ declared 131072 — a factor of 4. This is a finding about the          │
│        artifact's metadata, not a defect in the model.            detail ▸   │
│    vision            not present                                             │
│    embeddings        unknown — probe inconclusive, nothing configured        │
│                                                                              │
│  MEASUREMENTS                                                                │
│    none. MCF has not benchmarked this model. (M5)                            │
│                                                                              │
│  [ serve ]  [ probe again ]  [ remove… ]                                     │
└──────────────────────────────────────────────────────────────────────────────┘
```

`MEASUREMENTS — none` is the honest state at M4 and is shown rather than hidden,
because an absent section reads as "nothing to report" while an empty one reads
as "nothing measured", and only the second is true.

## 3. A failure, expanded

The window renders every taxonomy category, and its job is legibility, not
softening.

```
┌──────────────────────────────────────────────────────────────────────────────┐
│  ◂ FAILURE  child.exit.signal                          2026-08-22T04:11:09Z  │
├──────────────────────────────────────────────────────────────────────────────┤
│  Subsystem     mcf-serve::supervise                                          │
│  Managed       llama.cpp b4321 · pid 44903 · serving qwen2.5-7b              │
│  Detail        SIGKILL 41 ms into token 12 of 400                            │
│  Cause         host OOM killer · 61.9 of 62.6 GiB in use by another process  │
│  Attribution   NOT MCF — the manager held 9.2 MiB at the time                │
│  Effect        partial result: 11 tokens preserved, marked truncated         │
│  Recovery      respawned +1.4 s · endpoint unavailable 1.4 s and said so     │
│  Not done      the request was NOT retried — a retry would have produced a   │
│                different completion and presented it as the same one         │
│                                                                              │
│  Reproduce     mcf lab run scenario/child-oom-killed-midstream   [ copy ]    │
│  Record        fail_2026-08-22T04-11-09Z_9d02                    [ export ]  │
└──────────────────────────────────────────────────────────────────────────────┘
```

## 4. Parity is checked, not asserted

```
$ mcf lab run parity

Enumerating control API operations                              41 found
Enumerating interface actions                                   41 found
Matching interface actions to control API operations            41/41

  0 actions reachable only through the interface.
  0 control operations the interface cannot express.

  PASS (§XI, §6.21)

  Note: the interface consumes the control API over HTTP, the same surface a
  script uses. There is no second code path, which is also why this check is
  cheap enough to run in CI (§VII).
```

## 5. The client budget

```
$ mcf lab run budget::client

  Document, gzipped                        14.1 KiB   ≤ 40 KiB     PASS
  CSS, inline                               3.2 KiB   ≤ 10 KiB     PASS
  JavaScript                                    0 B   ≤ 15 KiB     PASS
  External requests                               0   = 0          PASS
  Fonts shipped                                   0   = 0          PASS
  Cold render, reference client             184 ms    ≤ 400 ms     PASS
  Daemon CPU with an idle tab open           0.00 %   ≤ 0.02 %     PASS
  Daemon wakeups/min with an idle tab             0   = 0          PASS

  Reference client (DEC-016): a 2016-era phone browser over local Wi-Fi.
  A smart fridge is not a commitment this project makes (§6.11).
```

Zero JavaScript is a target the M4 surface can meet because it renders state and
posts forms. §3.14 predicts this: the surface can be radically thin precisely
because it carries no logic worth weight. Where a later milestone genuinely needs
script, it is a budgeted addition with a stated reason, not a threshold quietly
crossed.

## 6. Exposure, from the window

```
┌──────────────────────────────────────────────────────────────────────────────┐
│  NETWORK EXPOSURE — currently OFF                                            │
│                                                                              │
│  Turning this on makes MCF reachable from other devices on wlan0             │
│  (192.168.1.24). Two things become reachable, and they are not equally       │
│  dangerous:                                                                  │
│                                                                              │
│    the serving API — anything on this network can prompt your models         │
│    the control API — anything on this network can acquire models, run        │
│                      repository code you have authorized, delete artifacts,  │
│                      and read every record MCF holds                         │
│                                                                              │
│  Access to the control plane is access to a service that downloads and runs  │
│  code from the internet.                                                     │
│                                                                              │
│  Protection: <mechanism from DEC-017>                                        │
│  Revoke:     mcf expose --off, or the button here. Off at reboot unless      │
│              you choose otherwise.                                           │
│                                                                              │
│  [ Expose on wlan0 ]   ← requires typing the interface name                  │
└──────────────────────────────────────────────────────────────────────────────┘
```

---

## Intent this stage is trying to satisfy

| Clause | How it shows up above |
|---|---|
| §V minimal, any device | 14 KiB, no framework, no build, no install; reference client is an old phone |
| §XI parity | 41/41 checked in CI, not asserted in prose |
| §3.14 a window | No logic, no authority; every button is a control API call |
| §3.14 corollary | Minimal chrome, never minimal truth — no figure appears without its conditions |
| §6.11 no idle cost | Zero daemon CPU and zero wakeups with a tab open; no live graphs |
| §3.1 failures legible | The full failure card, including what MCF deliberately did *not* do |
| §3.6 unknown shown | `mystery-gguf` displays UNKNOWN licence and partial provenance prominently |
| §6.12 exposure | Deliberate, informed, revocable, non-persistent by default |
| §5 not a leaderboard | The footer states the numbers are local and incomparable |
