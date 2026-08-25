# M8 — Endurance

**Product:** a published fidelity report — what the laboratory models, what it
declines to model, how far its predictions diverge from real hardware, and how
much of MCF's confidence is therefore earned.

**Why this is a milestone and not background work.** §6.16 rates its own
confidence *low* and §7.20 is described as the question that decides "whether
§VIII is rigor or theatre." MCF chose to know itself through a model of the
world rather than observation of the world (§6.9, §6.15), and this milestone is
where it declines to fool itself about the difference.

---

## 1. The fidelity report

```
$ mcf lab fidelity

LABORATORY FIDELITY REPORT · MCF 0.5.0-m8 · 2026-09-14

WHAT THE LAB MODELS                                    validated against real
  accelerator absence                                  4 machines      ✓
  accelerator driver failure and version mismatch      2 machines      ✓
  accelerator memory exhaustion                        4 machines      ✓
  thermal throttling                                   2 machines      ⚠ see below
  supervised child death at 9 lifecycle stages         4 machines      ✓
  disk exhaustion, read-only volume, partial write     4 machines      ✓
  hub misbehaviour (14 scenarios)                      real hub, 3 mo  ✓
  network stall, partial transfer, mutation            4 machines      ✓
  clock jumps, simulated time                          n/a — structural
  memory pressure and host OOM                         4 machines      ✓
  contention between serving and benchmarking          4 machines      ⚠ see below
  upgrade and state migration                          4 machines      ✓

WHAT THE LAB DOES NOT MODEL — confidence is NOT claimed here (§6.16)
  · silent hardware data corruption (bit flips, failing VRAM)
  · driver bugs specific to versions we have not run
  · multi-accelerator topologies and peer-to-peer transfer
  · non-x86 hosts beyond the two ARM machines in the fleet
  · sustained multi-day thermal behaviour in constrained enclosures
  · adversarial models targeting MCF specifically rather than the sandbox
  · anything about vendors below: see the hardware scope table

DIVERGENCES FOUND, AND WHAT THEY MEAN

  thermal throttling — SIMULATOR DEFECTIVE
    The lab models throttling as a step to 60 % clock at a threshold. Real
    hardware ramps over 8–40 s and oscillates. Measured divergence in
    time-to-steady-state: lab 0 s, real 22 s [14–37] across 2 machines.
    Verdict: reality is right and the simulator is wrong (§6.16). The
    simulator is being rewritten; until it is, every claim that depends on
    throttle timing is marked LAB-ONLY and is not published.

  serving/benchmark contention — SIMULATOR OPTIMISTIC
    Lab predicts 3–5 % measurement perturbation from a concurrently served
    model. Real hardware shows 4–19 % depending on the accelerator's
    scheduling behaviour. The lab's model of scheduling is too simple.
    Verdict: DEC-009's arbitration rule was set from the lab's number and is
    being re-derived from the real one. Recorded as an amendment to §7.9.

  Divergence is a finding about the simulator, never a reason to adjust the
  world (§6.16). Both entries above are defects, and both are open.

CONFIDENCE STATEMENT
  Of 47 behaviours the suite asserts, 41 have a real-hardware counterpart
  that has been checked within the last 90 days. 4 are LAB-ONLY and labelled
  as such everywhere they appear. 2 are known-divergent and their claims are
  suspended.

  MCF's suite being green means: the code behaves correctly given the
  conditions the lab describes. It does not mean the lab describes the world.
  That second claim is exactly as strong as this report and no stronger.
```

## 2. Can the record rebuild the failure?

§6.15 made this a requirement rather than an aspiration: *the failure record's
sufficiency is measured by whether the lab can rebuild the failure from it.*

```
$ mcf lab reconstruct --sample last-90-days

  61 real failures recorded across 4 machines.
  For each: attempt to construct a lab scenario from the record ALONE, then
  check whether the scenario produces an identical classification and context.

    reconstructed exactly                    54  (89 %)
    reconstructed with divergent context      4  (7 %)
    could not reconstruct                     3  (5 %)

  THE THREE THAT COULD NOT BE RECONSTRUCTED — these are defects in the record,
  not in the lab (§6.15), and the fix is more context at the failure site:

    fail_…_3b1c  child.hang.no_output
      The record captured that the child was silent past deadline. It did not
      capture what the child had been sent, so the scenario cannot recreate
      the input that produced the hang.
      Fix: capture the request digest and the last-sent-bytes offset at the
      point of hang. Filed as B-152.

    fail_…_9e04  accel.memory.exhausted
      Allocation failed at 23.1 GiB of 24.0 GiB, but the record did not
      capture what else held accelerator memory at the time.
      Fix: capture the accelerator allocation table at failure. Filed as B-153.

    fail_…_c771  record.corrupt.truncated_entry
      Reconstructable only with the power-loss timing, which is not knowable.
      Verdict: accepted as unreconstructable. Documented rather than fixed.

  89 % is the number this project is actually rated on. It was 71 % at the
  start of M8. The target set by DEC-014 is 95 %.
```

## 3. Long-run endurance

```
$ mcf lab run endurance --simulated-days 30

30 simulated days · real elapsed 41m 12s · simulated clock

  INJECTED
    child deaths                     412  at all 9 lifecycle stages
    thermal excursions                88  including 6 sustained ceilings
    disk-full events                  24  during record writes and downloads
    network stalls                   140  including 31 at >85 % of a transfer
    hub mutations                     18  revision moved under an in-flight fetch
    daemon restarts                   62  30 clean, 32 by SIGKILL
    MCF version upgrades               4  including one with a probe change
    clock anomalies                    9  backward jumps and large forward steps

  OUTCOMES
    unclassified outcomes                  0        ← the claim §3.1 makes
    silent failures                        0        ← the forbidden failure mode
    corrupted records                      0
    unrecoverable states                   0
    partial results preserved            311 of 311
    measurements marked non-comparable    47 correctly, 0 incorrectly
    record growth over 30 days          18.4 MiB   (budget 50 MiB / 30 d)  PASS
    resident memory drift                 +0.1 MiB (budget +1.0 MiB)       PASS
    idle CPU, day 30                     0.00 %                            PASS

  MCF remained coherent, queryable and restartable throughout.
```

## 4. Upgrade and comparability

```
$ mcf upgrade 0.5.0 → 0.6.0

  MEASUREMENT CODE CHANGED
    bench::first_token   now excludes the runtime's own prompt-eval time,
                         which was previously included. Every first-token
                         figure taken before this upgrade is systematically
                         higher than one taken after.

  This invalidates comparability for 41 measurements across 3 models.

  MCF will not silently carry them forward. Historical results remain in the
  record and remain readable; they are marked NOT COMPARABLE with results
  taken under 0.6.0 and above, and every surface that renders them says so
  (§3.4, §7.13).

    [k] keep and mark  (recommended — nothing is lost, nothing is conflated)
    [r] re-measure now (≈ 3 h 20 m; the old results are still kept and marked)

  MCF's own version has been part of every result's conditions since M0, which
  is the only reason this invalidation can be computed at all.
```

## 5. What MCF holds about you, and removing it

```
$ mcf record inspect

  SYSTEM RECORD                      18.4 MiB · 4 211 entries · kept indefinitely
    machine profiles       142   hardware, thermal and driver state
    failure records         61   classification, context, configuration
    provenance              12   one per artifact, outliving the artifacts
    measurements           891   performance and capability, with conditions
    agentic results       3105   per-trial outcomes from suite runs

  CONTENT RECORD                      2.1 MiB · retained 7 days · your setting
    prompts and completions from your own traffic
    ↳ structurally separate from the system record, in a different store, with
      a different retention. Not a flag on the same rows (§6.8).
    ↳ never included in an export unless you name it explicitly
    ↳ 0 bytes have ever left this machine

  BENCHMARK FIXTURE CONTENT           41.2 MiB · kept indefinitely
    suite task definitions and model outputs under test — fixture data, not
    your data, which is exactly why it can be kept in full

$ mcf record purge --content --older-than 0d
  Removes 2.1 MiB of prompt and completion content. The system record is
  untouched: measurements keep their conditions, failures keep their context.
  Proceed? [y/N]
```

## 6. Offline

```
$ mcf doctor --offline

  NETWORK: none detected.

  WORKS OFFLINE, unlabelled — this is normal operation
    serving · probing · benchmarking · agentic evaluation · recommending
    the window · the record · the laboratory

  WORKS OFFLINE, LABELLED DEGRADED
    catalogue freshness — local cache, last synchronized 4 d ago; a model may
                          have new revisions MCF cannot see
    licence checks       — cached; a licence changed upstream is invisible here

  DOES NOT WORK OFFLINE
    acquiring a model not already on this machine

  Note: "no internet" and "no local network" are different conditions and MCF
  distinguishes them. You have no internet. Your local network is up, so the
  window is still reachable from your other devices if exposure is on (§7.11).
```

---

## Intent this stage is trying to satisfy

| Clause | How it shows up above |
|---|---|
| §VIII verification | The suite's authority is itself measured rather than assumed |
| §6.16 reality outranks the lab | Two divergences are recorded as simulator defects, and their claims suspended |
| §6.16 bounded fidelity | An explicit list of what the lab does not model, so confidence is claimed only where earned |
| §6.15 reproduce from the record | 89 % reconstruction rate, with the failures treated as defects in the record |
| §3.1 no unclassified outcome | The endurance run's headline: 0 unclassified, 0 silent, over 30 simulated days |
| §3.17 determinism | 30 days in 41 minutes on simulated time |
| §7.13 comparability | An upgrade that changes measurement code invalidates history explicitly |
| §6.8 content vs system | Two stores, two retentions, structurally separate |
| §3.10 the user's data | Inspectable, purgeable, never exported by default, never left the machine |
| §7.11 offline | Three tiers, labelled; "no internet" and "no local network" distinguished |
| §7.14 definition of done | The reconstruction rate is the number the project is rated on, with a stated target |
