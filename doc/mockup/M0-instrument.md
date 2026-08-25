# M0 — The instrument

**Product:** `mcf doctor` — a static binary that reports what this machine is,
what MCF costs on it, and what MCF will and will not promise here.

**Nothing about models yet.** No acquisition, no inference, no network, no
interface. This milestone ships the instrument that every later number depends
on, and the smallest honest product that exercises it end to end.

---

## 1. The primary surface

```
$ mcf doctor

MCF 0.1.0-m0  (build 4f2ac91, rustc 1.83.0, target x86_64-unknown-linux-gnu)
Record: ~/.local/share/mcf/record  (1 entry, 4.1 KiB)

MACHINE
  CPU        AMD Ryzen 9 7900X · 12C/24T · 5.6 GHz max
  Memory     62.6 GiB total · 48.2 GiB available
  Disk       1.8 TiB total · 940 GiB available on record volume
  Accel #0   NVIDIA GeForce RTX 4090 · 24.0 GiB · driver 550.54.14 · CUDA 12.4
             status: recognized, characterized
  Accel #1   Intel UHD 770 (integrated)
             status: recognized, NOT characterized
             → MCF will attempt to use it and will label every result taken on
               it as uncharacterized. See rules.md R-014 (§3.2, §7.8).
  Thermal    CPU 41 °C · Accel #0 38 °C · both at idle steady state
  Power      profile "performance" · no throttling detected

MCF'S OWN COST ON THIS MACHINE            measured        budget      verdict
  Installed footprint                     11.4 MiB        ≤ 25 MiB    pass
  Resident memory, idle, nothing loaded   7.9 MiB         ≤ 12 MiB    pass
  Idle CPU over 60 s                      0.00 %          ≤ 0.10 %    pass
  Timer wakeups over 60 s, idle           0               = 0         pass
  Cold start to first command response    38 ms           ≤ 120 ms    pass
  Added request→first-token latency       not measurable at M0 — no serving path

  Budgets are asserted by the suite, not eyeballed (§3.13, §3.5). Their values
  are recorded in rules.md and were set by DEC-016.

WHAT MCF PROMISES HERE
  ✓ Every failure is classified, attributed and persisted with its context
  ✓ Every measurement carries its conditions, sample count and spread
  ✓ Every artifact carries its provenance, or records it as unknown
  ✓ The laboratory can reproduce all 41 failure categories on this machine
  ✗ Nothing about model quality, speed or fitness — that is M5 onward
  ✗ No claim about Accel #1 beyond "will attempt"

Written to record: obs_2026-08-24T18-04-11Z_machine-profile
```

## 2. What was written to the record

§3.3 requires the record be structured first and written at events, not on a
timer. Running `doctor` is an event.

```json
{
  "id": "obs_2026-08-24T18-04-11Z_machine-profile",
  "kind": "machine_profile",
  "recorded_at": "2026-08-24T18:04:11.442Z",
  "mcf": {
    "version": "0.1.0-m0",
    "build": "4f2ac91",
    "rustc": "1.83.0",
    "target": "x86_64-unknown-linux-gnu",
    "config_digest": "sha256:9c1f…a70b"
  },
  "machine": {
    "cpu": { "model": "AMD Ryzen 9 7900X", "cores": 12, "threads": 24 },
    "memory_bytes_total": 67217616896,
    "accelerators": [
      {
        "index": 0,
        "vendor": "NVIDIA",
        "model": "GeForce RTX 4090",
        "memory_bytes": 25757220864,
        "driver": "550.54.14",
        "runtime": { "cuda": "12.4" },
        "support": "characterized"
      },
      {
        "index": 1,
        "vendor": "Intel",
        "model": "UHD Graphics 770",
        "memory_bytes": null,
        "driver": "unknown",
        "runtime": {},
        "support": "attempted_uncharacterized"
      }
    ],
    "thermal": { "cpu_c": 41.0, "accel_0_c": 38.0, "state": "idle_steady" },
    "power_profile": "performance",
    "throttling_detected": false
  },
  "self_cost": {
    "footprint_bytes": 11954176,
    "rss_idle_bytes": 8283750,
    "idle_cpu_fraction": { "value": 0.0000, "n": 60, "window_s": 60 },
    "timer_wakeups": 0,
    "cold_start_ms": { "value": 38.2, "n": 20, "spread_p5_p95": [35.9, 41.7] }
  }
}
```

Two things to notice, because they are the whole point of M0:

- `"memory_bytes": null` and `"driver": "unknown"` on Accel #1. §3.6 forbids
  filling an unknown with a plausible value, and the type makes the null the
  only representable alternative to a real reading.
- `cold_start_ms` is not a number. It is a `Measurement` — value, sample count
  and spread — because §3.4 makes a bare number unrepresentable (B-005).

## 3. Failure, shown at full volume

```
$ mcf doctor

MCF 0.1.0-m0  (build 4f2ac91)

MACHINE
  CPU        AMD Ryzen 9 7900X · 12C/24T
  Memory     62.6 GiB total · 48.2 GiB available
  Accel #0   NVIDIA GeForce RTX 4090 · DEGRADED

  ⚠ DEGRADED — accelerator interrogation failed
    Category    accel.driver.query_failed          (taxonomy 3.2.1)
    Subsystem   mcf-core::hardware::nvml
    Detail      NVML returned ERROR_DRIVER_NOT_LOADED after 3 attempts
    Context     driver 550.54.14 present on disk; kernel module nvidia not
                loaded; last successful query 2026-08-23T22:10:04Z
    Effect      Accelerator memory, thermal state and clock are UNKNOWN.
                Every measurement taken while this holds will be marked
                accel-state-unknown and is NOT comparable with measurements
                taken while it did not (§3.2, §3.4).
    Reproduce   mcf lab run scenario/accel-driver-unloaded
    Record      fail_2026-08-24T18-09-52Z_a71c

MCF'S OWN COST ON THIS MACHINE            measured        budget      verdict
  …                                       …               …           pass

WHAT MCF PROMISES HERE
  ✗ No accelerator claim of any kind while the above holds.

Exit status: 0  (doctor reports; it does not fail because the machine did)
```

The `Reproduce` line is §6.15's whole strategy in one field: MCF gave up ambient
telemetry, so the failure record must be rich enough for the laboratory to
rebuild the failure from it — and it names the scenario that does.

## 4. The laboratory

```
$ mcf lab list

41 scenarios · 41 taxonomy categories · 0 categories without a scenario

  accel.absent                        no accelerator present
  accel.driver.query_failed           driver present, module unloaded
  accel.driver.version_mismatch       runtime newer than driver
  accel.memory.exhausted              allocation refused mid-operation
  accel.thermal.ceiling               sustained throttle to 60% clock
  child.exit.signal                   supervised process killed by SIGKILL
  child.exit.nonzero_immediate        dies before first output
  child.exit.nonzero_midstream        dies after partial output
  child.hang.no_output                alive, silent, past deadline
  child.spawn.enoent                  binary missing at spawn
  clock.jump.backward                 wall clock steps backward mid-measurement
  disk.exhausted.during_write         no space left, record write in flight
  disk.readonly                       record volume remounted read-only
  record.corrupt.truncated_entry      last entry cut mid-write
  … 27 more

$ mcf lab run scenario/child-exit-midstream --repeat 100

scenario/child-exit-midstream
  100 runs · 100 identical outcomes · 0 divergences
  outcome: child.exit.nonzero_midstream, attributed to mcf-core::supervise,
           partial output preserved (2 of 5 expected chunks), manager alive
  determinism: PASS (§3.17 — a failure found once reproduces forever)
  wall clock: simulated; real elapsed 1.9 s
```

`0 categories without a scenario` is the M0 exit criterion. §3.17 says a
taxonomy category with no simulation is an untested claim, so the check that
produces that line fails CI when the two lists diverge (B-010).

## 5. The budget suite

```
$ cargo test --workspace -- --include-ignored

    Suite         Tests   Time
    unit            412   3.1 s
    lab              41   6.8 s
    budget            6   64.2 s   (60 s of it is the idle observation window)
    whole-system      9   11.4 s

  budget::idle_cpu ......................... 0.00 % ≤ 0.10 %      PASS
  budget::rss_idle ......................... 7.9 MiB ≤ 12 MiB     PASS
  budget::timer_wakeups .................... 0 = 0                PASS
  budget::cold_start ....................... 38.2 ms ≤ 120 ms     PASS
  budget::footprint ........................ 11.4 MiB ≤ 25 MiB    PASS
  budget::record_write_cost ................ 0.9 ms ≤ 5 ms        PASS

  0 tests required an accelerator, a network or a model file (§3.5).
  Total: 468 passed, 0 failed, 0 skipped. Offline.
```

A regression fails the build and prints a before/after under stated conditions,
because §3.13 says a performance change without one is a guess that also
increased complexity.

---

## Intent this stage is trying to satisfy

| Clause | How it shows up above |
|---|---|
| §3.1 no silent failure | Every failure is classified, attributed, contextualized and persisted; `except: pass` has no representation |
| §3.2 degrade and say so | The degraded run still reports, and marks every downstream result as non-comparable |
| §3.3 the record | Written at an event, structured first, small |
| §3.4 conditions travel | `cold_start_ms` is a `Measurement`, never a number |
| §3.6 never inferred | `null` and `unknown` for Accel #1 rather than a plausible value |
| §3.8 the apparatus | MCF measures its own cost, because a heavy instrument corrupts its own readings |
| §3.13 budgets | Six numbers, asserted in CI, failing the build on regression |
| §3.16 substrate enforces | The measurement and provenance types make the violations unrepresentable |
| §3.17 the laboratory | 41 scenarios, cross-checked against 41 taxonomy categories, deterministic over 100 runs |
| §6.15 reproduce, don't observe | The `Reproduce` field names the scenario that rebuilds the failure |
| §7.19 validate the substrate | The GPU probe and the badly-dying child are exactly the adversarial prototype §7.19 asked for |
