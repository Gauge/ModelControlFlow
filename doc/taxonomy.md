# Failure Taxonomy

| | |
|---|---|
| **Type** | Reference — the classification scheme every failure is filed under |
| **Version** | 3 |
| **Status** | Living. Drafted, awaiting ratification. |
| **Authority** | Derived from [document-of-intent.md](document-of-intent.md) v23, governed by [rules.md](rules.md) |
| **Answers** | §7.10 · registered as DEC-010 in [backlog.md](backlog.md) |

**Every failure is classified, attributed and persisted with its context** (A2).
This file is the classification. It appears in the record, in every error type,
in the laboratory's fault catalogue, in the recommender and in the interface, so
it is designed once and deliberately rather than accreted.

**It is also code.** `crates/mcf-core/src/failure/` holds the three axes as
types — generated from this file, then committed — and
`checks/tests/taxonomy_agreement.rs` fails the build when a code, a meaning, a
domain or an axis value differs between the two. Editing one of them means
editing both, in the same change (B-003).

**It is also a public interface.** Once §XIV ships, these codes travel between
machines and versions. Codes are therefore stable for life (C5): never reused,
never renamed, deprecated only in favour of a named successor.

## Three axes, not one tree

A single deep tree tries to encode what happened, whose fault it was and what
MCF did about it, and collapses under the combinations. These are separated:

| Axis | Answers | Values |
|---|---|---|
| **Category** | *What failed* | the dotted codes below |
| **Attribution** | *Whose failure it is* | `mcf` · `managed` · `machine` · `hub` · `artifact` · `user` · `model-under-test` · `unattributable` |
| **Disposition** | *What MCF did* | `refused` · `degraded` · `partial` · `recovered` · `aborted` · `invalidated` |

Attribution matters because §3.8 requires MCF know the difference between "this
model is slow" and "this machine was busy", and B24 makes `unattributable` a
verdict rather than a gap. Disposition matters because §3.1 makes partial
success a real outcome and §3.2 makes degradation a marked one.

**Every leaf has a laboratory scenario that produces it** (A13, §3.17). A
category with no scenario is an untested claim, and the cross-check fails CI
(B-010).

## The domains

### 1 · `hub.*` — the model source
| Code | Meaning |
|---|---|
| `hub.unreachable` | No route to the hub |
| `hub.rate_limited` | Throttled, with or without a retry hint |
| `hub.auth.required` | Credentials absent |
| `hub.auth.rejected` | Credentials present and refused |
| `hub.access.gated` | Terms not accepted for this account |
| `hub.ref.not_found` | Repository or revision does not exist |
| `hub.ref.moved` | Tag repointed between resolve and fetch |
| `hub.metadata.absent` | No config, card or licence |
| `hub.metadata.malformed` | Present and unparseable |
| `hub.metadata.deceptive` | Declares an architecture the weights are not |
| `hub.licence.unparseable` | Licence text present, terms unmatchable |
| `hub.licence.forbids_use` | Terms forbid the attempted use |

### 2 · `transfer.*` — getting bytes here
| Code | Meaning |
|---|---|
| `transfer.stalled` | No progress past the deadline |
| `transfer.truncated` | Stream ended before the declared length |
| `transfer.mutated` | Content changed between manifest and fetch |
| `transfer.checksum_mismatch` | Bytes arrived and do not verify |
| `transfer.interrupted` | Connection lost; resumable |
| `transfer.tls` | Certificate or handshake failure |

### 3 · `artifact.*` — a local model artifact
| Code | Meaning |
|---|---|
| `artifact.missing` | Referenced and not present |
| `artifact.corrupt` | Present and fails verification (§7.49's re-check) |
| `artifact.unreadable` | Present and cannot be read at all |
| `artifact.format.unsupported` | A format MCF does not read |
| `artifact.format.malformed` | A format MCF reads, malformed |
| `artifact.incomplete` | Some shards present, others absent |
| `artifact.archive.traversal` | Member escapes the extraction root |
| `artifact.archive.oversized` | Expands beyond its declared size |
| `artifact.provenance.incomplete` | Held, with unknown fields (§3.6) |

### 4 · `engine.*` — the supervised inference process
| Code | Meaning |
|---|---|
| `engine.spawn.not_found` | Binary absent at spawn |
| `engine.spawn.refused` | Permission or platform refusal |
| `engine.exit.immediate` | Died before first output |
| `engine.exit.midstream` | Died after partial output |
| `engine.exit.signal` | Killed by signal, including the OOM killer |
| `engine.hang.no_output` | Alive, silent, past deadline |
| `engine.protocol.malformed` | Output MCF cannot parse |
| `engine.protocol.version` | Incompatible engine interface |
| `engine.load.refused` | Engine declines the model |
| `engine.unavailable` | No vendored engine supports this artifact (D23) |

### 5 · `accel.*` — the accelerator
| Code | Meaning |
|---|---|
| `accel.absent` | None present |
| `accel.unrecognized` | Present, not characterized (§7.8) |
| `accel.driver.absent` | No driver |
| `accel.driver.query_failed` | Driver present, interrogation failed |
| `accel.driver.version_mismatch` | Runtime and driver disagree |
| `accel.memory.exhausted` | Allocation refused |
| `accel.memory.fragmented` | Free but unallocatable |
| `accel.thermal.ceiling` | Sustained throttle |
| `accel.reset` | Device reset mid-operation |
| `accel.lost` | Disappeared from the bus |

### 6 · `resource.*` — the machine's own limits
| Code | Meaning |
|---|---|
| `resource.disk.exhausted` | No space, operation in flight |
| `resource.disk.readonly` | Volume remounted read-only |
| `resource.disk.quota` | Quota refused the write |
| `resource.memory.exhausted` | Host allocation refused |
| `resource.memory.pressure` | Allocatable but degraded |
| `resource.fd.exhausted` | Descriptor limit |
| `resource.contended` | Another process holds what was needed (B24) |

### 7 · `record.*` — the record store
| Code | Meaning |
|---|---|
| `record.unwritable` | Store cannot be opened for writing |
| `record.corrupt.index` | Derived database damaged; journal intact (D20) |
| `record.corrupt.journal` | Journal damaged |
| `record.replay.incomplete` | Rebuilt, with a stated gap |
| `record.schema.unknown` | Written by a version this one cannot read (§7.30) |
| `record.budget.exhausted` | Retention limit reached (§7.5) |

### 8 · `config.*` — configuration
| Code | Meaning |
|---|---|
| `config.invalid` | Value outside the permitted domain |
| `config.unsatisfiable` | Coherent and impossible on this machine |
| `config.conflict` | Two settings that cannot both hold |
| `config.unverified` | Declared, never probed, and required to be (A21) |
| `config.identity.mismatch` | Realized differs from declared (D23, placement) |

### 9 · `probe.*` — capability probing
| Code | Meaning |
|---|---|
| `probe.inconclusive` | Neither confirms nor denies (§3.18) |
| `probe.timeout` | No result within its bound |
| `probe.malformed_response` | Output the probe cannot grade |
| `probe.unsupported` | Probe does not apply to this artifact |
| `probe.divergence` | Declared and verified disagree — a *finding*, not an error |

### 10 · `lab.*` — laboratory execution
| Code | Meaning |
|---|---|
| `lab.setup.failed` | Environment could not be constructed |
| `lab.teardown.failed` | Residue left behind (B58) |
| `lab.gate.not_applicable` | Capability verified absent (B40) — not a failure |
| `lab.gate.unknown` | Capability unestablished (B40) |
| `lab.precondition.contended` | Machine not quiet for a timing run (B-217) |
| `lab.budget.exceeded` | Declared maximum duration reached |
| `lab.interrupted` | Stopped by the operator; partial preserved |
| `lab.workload.ungradable` | Supplied workload cannot be graded (B42) |
| `lab.conditions.invalidated` | Conditions moved mid-run; result unsound |

### 11 · `model.*` — the model under test's behaviour
Not MCF failing. These are **measurements**, and §6.17 holds that *how* a trial
failed is more informative than the pass rate.

| Code | Meaning |
|---|---|
| `model.tool.malformed_call` | Unparseable or missing required arguments |
| `model.tool.wrong_selection` | Plausible but incorrect tool |
| `model.tool.hallucinated` | Tool that does not exist |
| `model.loop.no_progress` | Repeats an action with unchanged state |
| `model.stop.never` | Runs to the turn or token budget |
| `model.stop.premature` | Stops with the task incomplete |
| `model.recovery.none` | Receives an error and repeats unchanged |
| `model.format.violated` | Correct content, wrong required format |
| `model.instruction.ignored` | A stated constraint unmet |
| `model.refused` | Declined the task |
| `model.output.empty` | Produced nothing |

### 12 · `sandbox.*` — containment
| Code | Meaning |
|---|---|
| `sandbox.escape_attempted` | Reached for something absent (A14) — recorded, never permitted |
| `sandbox.quota.disk` | Environment quota refused a write |
| `sandbox.quota.memory` | Environment limit reached |
| `sandbox.turn_budget` | Turn limit reached |
| `sandbox.construct_failed` | Environment could not be built |

### 13 · `platform.*` — the operating system and privilege
| Code | Meaning |
|---|---|
| `platform.unsupported` | Outside §7.35's declared scope |
| `platform.privilege.denied` | Elevation refused (A26) |
| `platform.privilege.unavailable` | No mechanism on this platform |
| `platform.mechanism.unavailable` | Boxing, pinning or yielding unsupported here (§7.43) |
| `platform.restore_failed` | Something changed could not be restored (A27) |

### 14 · `time.*` — the clock
| Code | Meaning |
|---|---|
| `time.jump.backward` | Wall clock stepped back mid-measurement |
| `time.jump.forward` | Large forward step |
| `time.monotonic.unavailable` | No monotonic source (D9) |

### 15 · `exchange.*` — identifiers and contributions
| Code | Meaning |
|---|---|
| `exchange.identifier.malformed` | Unparseable |
| `exchange.identifier.unresolvable` | Names something unobtainable (§XV) |
| `exchange.reproduce.impossible` | Resolvable and cannot run here — a complete answer (§6.3) |
| `exchange.reproduce.divergent` | Reproduced; numbers differ — a *finding* (§6.29) |
| `exchange.schema.unreadable` | Contribution written by an uninterpretable version |
| `exchange.terms.absent` | Terms not shown before sending — a defect (D21) |

### 16 · `internal.*` — MCF's own invariants
| Code | Meaning |
|---|---|
| `internal.invariant_violated` | A state the type system was meant to prevent |
| `internal.unclassified` | A failure that fits nothing above |

`internal.unclassified` is **a tracked defect metric, not a bucket**. Its count
is reported and its target is zero: every occurrence is a missing category, and
adding the category is the fix.

## Extension policy

- **Adding a leaf is cheap** and needs a laboratory scenario in the same change
  (A13).
- **Renaming is forbidden.** Codes are stable for life (C5).
- **Deprecating requires a named successor**, and the old code remains readable
  forever because contributions and records carry it (§7.30).
- **Adding a domain is a decision**, recorded, because domains are the part
  consumers switch on.
- A category that never fires in a year is a candidate for review, not deletion —
  §3.17 wants the rare paths exercised, and the laboratory is where that happens.

## Changelog

### Version 3 — `artifact.unreadable` added

The re-verification path (B-301) needed a code for *the file is there and the
read failed*, and there was not one. `artifact.corrupt` says *present and fails
verification*, which is a claim about the bytes; a permission error or a media
error is a claim about the machine, and filing one under the other would put
the wrong attribution on it — B24's difference between "this model is slow" and
"this machine was busy", one level down.

Added with its laboratory scenario in the same change, as the extension policy
requires (A13). 111 codes.

### Version 2 — the classification becomes types

`crates/mcf-core/src/failure/` expresses the three axes so the compiler holds
them, and an agreement check fails the build when the document and the code
disagree (B-003). Nothing about the classification changed; what changed is
that it can no longer drift silently from the software that files against it.

### Version 1 — drafted

Created to answer §7.10, which had been recorded since version 1 of the intent
document and was blocking M0 by appearing in every error type.

The design decision worth noting is the split into three axes. A single tree
would have had to encode what failed, whose fault it was and what MCF did, and
the product of those is unmanageably large. Separating attribution and
disposition keeps the tree shallow — sixteen domains, none deeper than three
segments — and makes the two questions §3.8 and §3.2 care about answerable
without traversing it.
