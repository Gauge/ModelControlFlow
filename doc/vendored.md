# Vendored and Deferred

| | |
|---|---|
| **Type** | Register — what MCF ships, what it declined to ship, and why |
| **Version** | 2 |
| **Status** | Living |
| **Authority** | Governed by [rules.md](rules.md); the licence is D28, the tiers are D23, the stand-in is D31 |
| **Registers to** | B-192, B-320, B-321, B-330 in [backlog.md](backlog.md) |

**Nothing is vendored yet.** This file exists before the first component is
admitted, because B-330's condition is that *no component ships without a
recorded compatibility finding* — a register written after the fact is a
register that describes what happened rather than one that gated it.

Read it before admitting a component, and when asking why MCF does not support
something.

## Contents

| § | Section |
|---|---|
| 1 | [What a finding is](#1--what-a-finding-is) |
| 2 | [Vendored](#2--vendored) |
| 3 | [Deferred](#3--deferred) |
| 3a | [What the artifact requires of a machine](#3a--what-the-artifact-requires-of-a-machine) |
| 4 | [Candidates, not yet assessed](#4--candidates-not-yet-assessed) |
| — | [Changelog](#changelog) |

## 1 · What a finding is

D28 makes MCF GPL-3.0-only, so every vendored component's terms are MCF's
problem: one that is not GPL-3.0-compatible is one MCF cannot ship whatever its
merits. D23 gives the three tiers and intent v23 narrowed them — MCF vendors a
stack it controls end to end, and anything requiring a component it cannot
vendor is **deferred and recorded** rather than partially supported.

A finding states four things, and A21's distinction runs through it:

- **What the component declares.** Its own statement of its terms.
- **What MCF verified**, and how. A declared licence is not a verified one, and
  the two are different states rather than degrees of confidence.
- **The compatibility verdict** against GPL-3.0-only.
- **The tier**, and for a deferred component, what would change the answer.

A component whose licence MCF has not verified is **not admitted**. That is
stricter than it sounds and is the point: the check in
`checks/tests/the_licence_is_what_it_says.rs` refuses a vendored component with
no row here.

## 2 · Vendored

*Nothing.* No component has been admitted, so there is nothing to ship and
nothing to be compatible.

The first admission will be an inference engine, and DEC-004 is the decision
that names it. D31 already settles what MCF writes itself — a stand-in engine,
so that coverage is true and the vendored engine has something to be checked
against — and B65 settles that a stand-in can never report a speed.

## 3 · Deferred

Recorded rather than silent (B-321, C6), so that an omission is a decision
somebody can disagree with rather than a gap nobody noticed.

| Component | Why deferred | What would change it |
|---|---|---|
| **CUDA, cuBLAS, cuDNN** (NVIDIA) | Proprietary. §XVI instructs MCF to *ship* rather than link against, which weakens the system-library argument copyleft software normally relies on — the collision D22 recorded and intent v23 resolved by deferring the platform-provided tier entirely | Nothing MCF controls. The accelerator path is the open one instead, and F1 records that MCF can still *read* a vendor's management library for hardware state, since nothing measured passes through it (D25) |
| **Metal** (Apple) | The same shape: a platform framework MCF may use where it is present and may not redistribute | As above. D29 makes it a per-platform capability, present or stated absent |
| **Intel MKL** | Proprietary terms | An open BLAS is the alternative, and a stand-in needs none |
| **Any engine requiring a negotiated licence or payment** | D23's third tier. A normal outcome under §3.13 rather than a failure | Evidence that the performance gap changes which model a user should run, which is what B-321 asks this register be revisited on |

**Deferring an accelerator path does not weaken §III.** B7 governs attempt and
diagnosis rather than success, and D31's stand-in means the outcome on hardware
with no vendorable backend is *runs on the processor, marked* rather than *does
not run*.

## 3a · What the artifact requires of a machine

The other side of vendoring: not what MCF ships, but what it expects to find.
B36 refuses to make a missing prerequisite the user's errand, and B-192's
condition is the checkable form — **the artifact has no dynamic dependency a
stock machine lacks**.

| The artifact needs | Why this is not a prerequisite |
|---|---|
| `libc.so.6` | The C library is what "a Linux machine" means; a binary that did not use it would still be running on it |
| `libgcc_s.so.1` | The compiler's unwinding and arithmetic support, shipped with every toolchain's runtime and present on every distribution |
| `ld-linux-*.so.*` | The dynamic loader itself, which is the thing that reads the list above |

Nothing else. `crates/mcf-cli/tests/artifact.rs` reads the binary's own
`DT_NEEDED` entries — the file's requirement, rather than what happens to
resolve on the machine asking — and fails on anything outside that list, on
`DT_RPATH` or `DT_RUNPATH`, and on an interpreter that is not the platform's
own. It carries its own control: `/bin/sh` needs a terminal library that is not
on the list, so the predicate is watched rejecting something.

**A component admitted to §2 must keep this true.** A vendored engine that
dynamically links a maths library the user has to obtain would satisfy §2's
licence check and violate §XVI, and this is where that shows up.

## 4 · Candidates, not yet assessed

Terms as each project declares them, **fetched but not yet verified against a
vendored tree** — because nothing is vendored, so there is no tree to verify
against. A21: these are *declared*, and admitting any of them requires the
verified column to be filled first.

| Candidate | What it is | Declared | Fetched | Compatible with GPL-3.0? |
|---|---|---|---|---|
| **llama.cpp** | Inference engine; reads GGUF, which is what §XII's reference model is published in | MIT | `LICENSE` at `ggml-org/llama.cpp`, digest `94f29bbed6a22c35…` | Yes, if the declaration holds |
| **ggml** | The tensor library llama.cpp is built on | MIT | `LICENSE` at `ggml-org/ggml`, same digest as above | Yes, if the declaration holds |
| **mistral.rs** | Inference engine in Rust | MIT | `LICENSE` at `EricLBuehler/mistral.rs`, digest `b40bb09c7b3b6f69…` | Yes, if the declaration holds |
| **candle** | Tensor library in Rust; dual-licensed | MIT and Apache-2.0 | `LICENSE-MIT` at `huggingface/candle` | Yes, if the declaration holds |
| **tokenizers** | Hugging Face's tokenizer library, first-party to the ecosystem D4 notes | Apache-2.0 | `LICENSE` at `huggingface/tokenizers` | Yes, if the declaration holds |
| **OpenBLAS** | Dense linear algebra | BSD-3-Clause | `LICENSE` at `OpenMathLib/OpenBLAS`, digest `190b5a9c8d9723fe…` | Yes, if the declaration holds |

**"If the declaration holds" is doing real work in that column.** A project's
`LICENSE` file is its statement about itself; what MCF ships is a *tree*, and a
tree can contain files under other terms than the one at its root. Verification
is a check of the tree that is actually vendored, at the revision that is
actually pinned, and it is what turns a row in §4 into a row in §2.

## Changelog

### Version 2 — the other side of the register

Section 3a added with B-192. The register recorded what MCF ships and what it
declined to ship, and said nothing about what the artifact expects to *find* —
which is the same question from the machine's side, and the one B36 answers.
Three libraries, each of them part of what a Linux machine is, and a check that
fails on a fourth.

### Version 1 — the register exists before the first component does

Created for B-321, and reaching further than that item asked. B-321 wanted the
*deferred* list; B-330 wants a compatibility finding per vendored component; and
both are the same register seen from two sides, so writing one of them alone
would have produced two documents that disagreed about what MCF ships.

It is written before anything is vendored deliberately, for the reason B-361's
prohibition was written before the stand-in engine and B-220's ledger before
anything could change the environment: a gate added after the thing it gates is
a gate something has already gone through.

The declared terms in §4 were fetched rather than recalled, and the digests are
recorded — but they remain *declared* under A21, because a project's own licence
file is a statement about itself and what MCF would ship is a tree.
