# M1 — Custody

| | |
|---|---|
| **Type** | Mockup — what finished looks like at this stage |
| **Milestone** | [M1](../roadmap.md#m1--custody) |
| **Version** | 2 |
| **Status** | Illustrative. Every figure is invented and none may be cited as a measurement (C7, A20). |
| **Source** | [document-of-intent.md](../document-of-intent.md) · rules: [rules.md](../rules.md) |

**Product:** `mcf pull`, `mcf list`, `mcf show`, `mcf rm` — models enter, live
on, and leave this machine with their provenance intact and their licence
legible.

**Still no inference.** M1's claim is §6.3's: *no unhandled outcomes*, not *no
unsuccessful outcomes*. A model that will not run here is a complete success of
Intent III if MCF says exactly why.

---

## 1. The common path

```
$ mcf pull Qwen/Qwen2.5-7B-Instruct

Resolving  Qwen/Qwen2.5-7B-Instruct
  revision   a09a35458c702b33eeacc393d103063234e8bc28  (pinned from main @ 18:22:03Z)
  licence    apache-2.0  (LICENSE present, SPDX matched)
  files      8 · 15.2 GiB
  disk       940 GiB available → sufficient

  This repository contains no executable code. Nothing will be run. (§6.4)

Fetching   ████████████████████████  15.2 GiB / 15.2 GiB   214 MiB/s   71 s
Verifying  8/8 checksums match manifest
Validating safetensors headers · config · tokenizer · chat template   OK

Acquired   Qwen/Qwen2.5-7B-Instruct @ a09a354
           15.2 GiB at ~/.local/share/mcf/models/…
           provenance: complete · 0 unknown fields
           capabilities: NOT PROBED — declared only, unverified (§3.18, M3)

Record: acq_2026-08-24T18-23-14Z_a09a354
```

The last line before the record is the one that matters. §3.18 forbids MCF
treating a model card's claims as facts, and M1 has no probes, so it says so
rather than letting the catalogue imply otherwise.

## 2. The unsuccessful outcome, which is still a success of §III

```
$ mcf pull meta-llama/Llama-3.1-70B-Instruct

Resolving  meta-llama/Llama-3.1-70B-Instruct
  revision   945c8663693130f8be2ee66210e062158b2a9693
  licence    llama3.1  (custom; acceptance required — see §6 of the licence text)
  files      30 · 131.4 GiB

  ⚠ GATED — this repository requires accepted terms and an authenticated identity
    Category    hub.access.gated                    (taxonomy 1.4.0)
    Missing     (a) licence acceptance on the hub for this account
                (b) a token with `read` scope in MCF's credential store
    Action      mcf auth add --token <token>   then re-run
    Not done    Nothing was fetched. No partial state exists.
    Record      fail_2026-08-24T18-31-02Z_c4e1

  ℹ Even with access, this would not run here:
    weights need ≈ 131 GiB; Accel #0 has 24.0 GiB and the host has 62.6 GiB.
    A quantized derivative might. MCF will not pick one for you at M1.

Exit status: 4  (classified, actionable, nothing damaged)
```

Two §3.1 properties are visible: the outcome is *classified* (`hub.access.gated`),
and the second note is the honest one §6.3 demands — MCF states the hardware
verdict without being asked, because "this will not run here, because the weights
need 131 GiB and you have 24" is a complete answer.

## 3. Untrusted input, refused

```
$ mcf pull some-user/repackaged-model-v3

Resolving  some-user/repackaged-model-v3
  revision   7b1e0c9…

  ✗ REFUSED — archive member escapes the extraction root
    Category    hub.artifact.path_traversal         (taxonomy 1.2.3)
    Subsystem   mcf-hub::validate::archive
    Detail      member "../../.config/systemd/user/x.service" in extra_data.tar
    Context     detected during header scan, before any byte was written to disk
    Effect      Nothing extracted. Nothing written outside the staging directory.
                The partial download was discarded and its bytes are recorded.
    Reproduce   mcf lab run scenario/hub-archive-traversal
    Record      fail_2026-08-24T18-40-19Z_7b1e

  This is not a judgment about the publisher. It is the §3.7 posture: every
  fetched byte is hostile until validated.

Exit status: 5
```

## 4. Executing repository code — possible, never implicit

§6.4's resolution rendered as a gate. §6.14 puts this in the "asked every time,
no matter the friction" category.

```
$ mcf pull some-org/custom-arch-model

Resolving  some-org/custom-arch-model
  revision   3d02f11…
  licence    apache-2.0
  files      12 · 8.9 GiB

  ⚠ AUTHORIZATION REQUIRED — this model cannot be loaded without executing code
    from its repository.

    Files that would execute:
      modeling_custom.py        412 lines   (defines the model class)
      configuration_custom.py    88 lines
    They run with:
      a dedicated user, no network, read-only access to this model's directory,
      no access to MCF's records, the catalogue, or any other model.
    They can still:
      consume CPU and memory within that container, and produce wrong results.

    MCF's own records and state cannot be corrupted by this code — that
    containment is not a setting and is asserted by the laboratory.

    This choice is recorded in the model's provenance, permanently.

  Execute repository code for some-org/custom-arch-model? [y/N]
```

Answering `N` is a normal outcome; the model is acquired with
`trust_remote_code: denied` in provenance and is simply not loadable until the
answer changes.

## 5. The catalogue

```
$ mcf list

NAME                                REV      SIZE      LICENCE      PROVENANCE  CAPS
Qwen/Qwen2.5-7B-Instruct            a09a354  15.2 GiB  apache-2.0   complete    unprobed
mistralai/Mistral-7B-Instruct-v0.3  e0bc86c  14.5 GiB  apache-2.0   complete    unprobed
some-org/custom-arch-model          3d02f11   8.9 GiB  apache-2.0   complete    unprobed
                                                                    ↳ trust_remote_code: denied
local/mystery-gguf                  —        4.1 GiB  UNKNOWN      partial     unprobed
                                                                    ↳ 3 unknown fields

4 models · 42.7 GiB · 940 GiB free

local/mystery-gguf was imported from a local file. Its origin, revision and
licence are unknown and are recorded as unknown. MCF will serve it and will
mark every result taken from it as provenance-incomplete (§3.6, §3.2).
```

## 6. Provenance in full

```
$ mcf show Qwen/Qwen2.5-7B-Instruct --provenance
```
```json
{
  "id": "art_qwen2.5-7b-instruct_a09a354",
  "origin": {
    "hub": "huggingface.co",
    "repository": "Qwen/Qwen2.5-7B-Instruct",
    "revision": "a09a35458c702b33eeacc393d103063234e8bc28",
    "revision_pinned_from": "main",
    "retrieved_at": "2026-08-24T18:23:14.902Z",
    "retrieved_by": { "mcf_version": "0.1.0-m1", "build": "8be1d02" }
  },
  "licence": { "spdx": "Apache-2.0", "source": "LICENSE file", "state": "declared" },
  "integrity": {
    "manifest_digest": "sha256:2f77…c410",
    "files": [
      { "path": "model-00001-of-00004.safetensors",
        "bytes": 4128342016, "sha256": "…", "verified_at": "2026-08-24T18:24:19Z" }
    ],
    "resumed_transfers": 0,
    "mutation_detected": false
  },
  "trust_remote_code": { "state": "not_required" },
  "transformations": [],
  "unknown_fields": []
}
```

A quantized derivative created later appends to `transformations` and keeps this
whole block as its parent, because §3.6 requires a derivative trace back to its
source weights.

## 7. Deletion is a decision

```
$ mcf rm mistralai/Mistral-7B-Instruct-v0.3

Would remove  mistralai/Mistral-7B-Instruct-v0.3 @ e0bc86c
  14.5 GiB from ~/.local/share/mcf/models/…
  re-acquiring costs ≈ 68 s at the last observed rate (214 MiB/s)

  This model is referenced by:
    2 capability probe records   (kept — records outlive artifacts)
    0 benchmark results
    0 recommendations

  The provenance record is KEPT. Removing the weights does not erase the fact
  that they were here (§3.6, §3.11).

Remove? [y/N]
```

Nothing in MCF reclaims disk on its own initiative. §3.11 makes automatic
eviction a design MCF does not have.

## 8. The fake hub

Every scenario above runs offline against a constructed hub.

```
$ mcf lab list --group hub

14 scenarios

  hub.wellformed                  the happy path, byte-identical every run
  hub.unreachable                 DNS fails
  hub.stall_at_90                 transfer halts at 90% and never resumes
  hub.mutated_midflight           file content changes between manifest and fetch
  hub.truncated_file              content-length lies, stream ends early
  hub.access.gated                402/403 with terms-acceptance semantics
  hub.artifact.path_traversal     archive member escapes the root
  hub.artifact.zip_bomb           declared size 4 GiB, expands to 9 TiB
  hub.metadata.deceptive          config claims an architecture the weights are not
  hub.metadata.absent             no config, no card, no licence
  hub.licence.unparseable         licence text present, SPDX unmatchable
  hub.file.enormous               single file exceeding available disk
  hub.revision.moved              tag repoints between resolve and fetch
  hub.rate_limited                429 with and without Retry-After

  All 14 map to a taxonomy category. 0 categories in group `hub` lack a scenario.
```

---

## Intent this stage satisfies

| Clause | How it shows up above |
|---|---|
| §III custody | Fetch, verify, store, serve later, evict — one place, whole lifecycle |
| §6.3 "any" governs attempt | Gated, hostile, oversized and unparseable inputs all reach a defined, actionable outcome |
| §3.7 hub is untrusted | Path traversal caught before a byte is written; validation precedes extraction |
| §6.4 untrusted execution | Possible, per artifact, risk stated, contained, recorded in provenance |
| §6.14 gate by category | Code execution and destruction are asked every time; quantization and placement never are |
| §3.6 provenance | Complete or explicitly partial; `unknown_fields` is a first-class list |
| §3.11 nothing destroyed casually | `rm` previews cost, names references, keeps the record |
| §3.18 declared ≠ verified | The catalogue says `unprobed`, not "supports tools" |
| §3.17 the laboratory | 14 hub scenarios, all offline, all deterministic |

---

## Changelog

| Version | Change |
|---|---|
| 2 | Standardized to the format contract in [README.md](../../README.md): front matter, present tense, changelog. |
| 1 | Created alongside the roadmap, to make "done" at this stage a picture somebody can disagree with before it is code. |
