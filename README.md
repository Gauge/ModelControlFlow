# ModelControlFlow

| | |
|---|---|
| **Type** | Repository README — what this is, how to run it, and the format every document holds to |
| **Version** | 19 |
| **Status** | Living |
| **Authority** | Governs the form of every document in `doc/`, never their content |

**MCF makes the open-weights ecosystem usable by one person on one machine
without that person becoming a full-time operator of it.** It acquires models
with their provenance intact, serves them as a dependable local endpoint, puts
them on a bench of purpose-built diagnostic laboratories, measures what they
cost and what they are worth *on this hardware for the work actually being
done*, and writes every result to an append-only record. Evidence leaves the
machine only when you send it, row by row.

The measuring is the point. Choosing a local model today is folklore — people
pick by leaderboard positions measured on someone else's hardware, with someone
else's quantization, against benchmarks that do not resemble their work. MCF
replaces folklore with measurement taken here.

**New to the repository?** [Quick start](#1--quick-start) gets you a running
binary in four commands. [Command reference](#5--command-reference) is the
operating manual for each subsystem. [The documents](#9--the-documents) is the
map of everything else.

## Contents

| | |
|---|---|
| 1 | [Quick start](#1--quick-start) |
| 2 | [Requirements](#2--requirements) |
| 3 | [Building](#3--building) |
| 4 | [Configuration](#4--configuration) |
| 5 | [Command reference](#5--command-reference) |
| 6 | [The systems](#6--the-systems) |
| 7 | [Working on MCF](#7--working-on-mcf) |
| 8 | [What MCF will not do](#8--what-mcf-will-not-do) |
| 9 | [The documents](#9--the-documents) |
| 10 | [Format contract](#10--format-contract) |
| — | [Licence](#licence) · [Changelog](#changelog) |

## 1 · Quick start

```
$ git clone <this repository> && cd ModelControlFlow
$ cargo build --locked --release
$ ./target/release/mcf doctor
$ ./target/release/mcf --version
```

`cargo build` needs no network: every dependency is vendored into `vendor/` and
`.cargo/config.toml` replaces the registry with it. The toolchain is pinned in
`rust-toolchain.toml`, so `rustup` installs the right compiler on first use.

`mcf doctor` is the command to run first. It reports what this machine is, what
MCF costs on it against its own stated ceilings, what it will and will not
promise here, and reproduces every failure it claims to handle — then writes the
lot to the record. It is the fastest way to know whether your machine is set up
and what MCF can honestly do on it.

Then, to actually use a model:

```
$ export MCF_MODELS=/big/disk/models                     # where models go
$ ./target/release/mcf pull bartowski/SmolLM2-135M-Instruct-GGUF
$ ./target/release/mcf pull bartowski/SmolLM2-135M-Instruct-GGUF:SmolLM2-135M-Instruct-Q4_K_M.gguf
$ ./target/release/mcf run <that file> --prompt "Once upon a time"
```

The first `pull` names no file: it says which of the published variants would
run on this machine and downloads nothing, which is usually the question being
asked.

## 2 · Requirements

| | |
|---|---|
| **Operating system** | **Linux.** MCF reads `/proc` and `/sys` in nineteen source files and has no `target_os` guards. It will not build usefully elsewhere today. |
| **Rust** | 1.98.0, pinned in `rust-toolchain.toml`. `rustup` honours the pin automatically; you do not need to select a toolchain. |
| **Network** | Not needed to build. Needed for `mcf pull` and for the scheduled `--with-online` tier. |
| **Disk** | The binary is ~4.3 MiB. Models are not: budget for what you intend to acquire, and point `MCF_MODELS` at a drive that has room. |
| **`podman`** | Only for `mcf provision`, at `/usr/bin/podman` or `/usr/local/bin/podman`. Docker is not used. Without it, provisioning reports a classified refusal rather than failing obscurely. |
| **An accelerator** | Optional. MCF characterizes what is present and says so when nothing is. |

Nothing else is required. There is no configuration file, no daemon manager, no
database, and no build step beyond `cargo`.

## 3 · Building

```
$ cargo build --locked                  # debug
$ cargo build --locked --release        # the artifact the cost ceilings govern
$ ./target/release/mcf --version
MCF 0.1.0-m0  (revision unknown, rustc 1.98.0 (...), target x86_64-unknown-linux-gnu, profile release)
```

**`revision unknown` is correct, not a defect.** The source revision reaches the
binary through `MCF_BUILD_COMMIT`, and MCF refuses to invent one when nothing
sets it. A release build sets it:

```
$ MCF_BUILD_COMMIT=$(git rev-parse HEAD) cargo build --locked --release
```

Do set it for anything you intend to keep measurements from. Every measurement
records the binary that took it, and the revision is how you later tell which
build that was.

**A statically linked artifact**, for a machine with nothing on it:

```
$ rustup target add x86_64-unknown-linux-musl
$ cargo build --locked --release --target x86_64-unknown-linux-musl -p mcf-cli
```

**Two binaries are produced.** `mcf` is the tool. `mcf-helper` sits beside it
and performs three operations that need rights `mcf` deliberately does not
have — setting the processor governor, taking or releasing a device's exclusive
mode, and reading the processor's energy counter. The daemon starts it as a
separate process rather than calling into it, which is what keeps the daemon
unprivileged rather than a claim that it is.

Full detail, including the release profile's panic and overflow settings and why
they are part of what is measured, is in [doc/build.md](doc/build.md).

## 4 · Configuration

MCF has no configuration file. §5 of the intent document refuses it a
configuration language, so what would be settings are environment variables —
the platform's own idiom — and everything else is a command-line argument.

| Variable | Read by | Meaning |
|---|---|---|
| `MCF_MODELS` | `pull`, `list`, `run`, `bench`, `rm`, … | An **ordered list of absolute paths**, colon-separated. The first is where a new acquisition goes; all are searched for what is held. A relative path is dropped and named rather than resolved against the current directory. |
| `XDG_DATA_HOME` | the record, the model store fallback | Where the record lives: `$XDG_DATA_HOME/mcf/record.jsonl`, falling back to `$HOME/.local/share/mcf`. |
| `XDG_RUNTIME_DIR` | `serve`, `status`, `stop` | Where the daemon's control socket lives. |
| `MCF_BUILD_COMMIT` | build time | The source revision stamped into the binary and into every measurement's conditions. |

Where you want `MCF_MODELS` to persist, a shell profile is how a machine
persists an environment variable:

```
$ export MCF_MODELS=/big/disk/models:/home/you/models
```

`mcf doctor` prints the stores in order, says which one new models go to, and
gives the free space of each — including for a store that does not exist yet, by
asking the filesystem that would hold it.

**Where things are written.** The record is `$XDG_DATA_HOME/mcf/record.jsonl`
with a derived index beside it. The index can be deleted at any moment: the
journal is the record and the index is rebuilt from it. Provisioned components
go under `$XDG_DATA_HOME/mcf/provisioned`. If neither `XDG_DATA_HOME` nor
`HOME` is set, MCF refuses to invent a location rather than guessing.

## 5 · Command reference

Run `mcf` with no arguments for the built-in usage summary. Every command below
is real and current.

### Getting oriented

#### `mcf doctor [--no-record] [--json]`

What this machine is, what MCF costs here against its stated ceilings, and what
it will and will not promise. Run this first, and run it again whenever the
machine changes.

```
MACHINE
  processor: AMD Ryzen 9 9950X 16-Core Processor · 16 cores · 32 threads
  memory: 98700095488 B (91.9 GiB) total · 74303545344 B (69.2 GiB) available
  accelerator #0: NVIDIA GeForce RTX 5080 · driver 610.57.04 · ... · characterized
  k10temp/Tccd1 50.7 °C (processor die), critical point not published by this chip
  card1 (amdgpu) 5% busy
  card2 (nvidia) unknown: this driver publishes no occupancy in sysfs; NVML is where the vendor put it

WHAT MCF COSTS HERE
  core binary, no engines                4.3 MiB — ceiling 40.0 MiB — within
  resident memory, nothing loaded        8.0 MiB — ceiling 20.0 MiB — within
  cold start to first command response   p99 496960 ns over n=100 — within

WHAT MCF PROMISES HERE
  ✓ Every measurement carries its conditions, its sample count and its spread
  ✗ Nothing about model quality, speed or fitness — that is M5 onward
  ✓ Every failure MCF claims to handle was reproduced on this machine just now
```

`--no-record` runs it without writing; `--json` emits it for a machine. Note
that a `✗` is not an error — it is MCF stating a boundary.

#### `mcf explain <model>`

What a model file declares about itself, what MCF read from it, what MCF would
choose, and what it cannot tell you. Includes what each of a declared set of
languages costs on that model's vocabulary.

#### `mcf support [--into <path>]`

Writes a file describing this machine's sensors and naming anything MCF could
not read, for you to review and send as a support request. **Nothing is sent**:
it is a file, and there is no upload path in the code. It carries driver names,
sensor readings and kernel version, and no prompt, model output or file content.

### Models: getting them, holding them, checking them

#### `mcf pull <owner/name[:file]> [--into <dir>] [--from <hub>] [--token-from <file>] [--token-from-env <VAR>]`

Acquire a model with its provenance. **Without a `:file`** it reports which of
the published variants would run on this machine and downloads nothing.
**With one** it acquires that file: resumed where the source allows, verified
against the digest the hub declared, provenance written beside it, acquisition
written to the record.

MCF reads a credential only where you name one — `--token-from` or
`--token-from-env`. It never searches your home directory for tokens.

#### `mcf list`

What this machine holds and where each thing came from. Keeps *nothing says*
and *something says and MCF cannot read it* apart.

```
11 model file(s) in /home/you/.local/share/mcf/models
  .../SmolLM2-135M-Instruct-Q4_K_M.gguf (105454432 bytes) — bartowski/SmolLM2-135M-Instruct-GGUF@09816acd...
    licence: apache-2.0 (permissive)
```

#### `mcf check [<model>] [--here] [--from <hub>]`

Is what you hold still what it should be? `--here` re-reads every artifact
against the digest recorded for it, catching silent disk corruption that would
otherwise arrive as a strange measurement rather than a bad file. Without
`--here` it also asks the source repository whether it still says what it said —
a withdrawn revision, a closed gate, a relicensing, a file replaced under its
own name.

**A finding invalidates nothing.** The artifact is here and its digest is what
it was; what a decay costs is somebody else's ability to reproduce.

#### `mcf rm <model> [--because <why>] [--purge]`

Stop holding something. **Without a reason this previews and removes nothing.**
Nothing is deleted without `--purge`. The removal and its reason go to the
record, because after a removal that line is all there is.

### Running a model

#### `mcf serve`

Start the daemon. It stays up, recovers what is on the disk, and costs nothing
while idle — measured, not intended: across ninety seconds, a real idle daemon
used **zero processor ticks and issued zero read syscalls** (F85).

#### `mcf status` · `mcf stop [--because <why>]`

Ask the daemon what it is, what it recovered and what it is holding; ask it to
stop and put the reason in the record. A process that can only be killed leaves
no account of why it stopped.

```
$ mcf status
mcf is up on /run/user/1000/mcf/control.sock
  build: 0.1.0-m0 (x86_64-unknown-linux-gnu)
  recovered 3039 record entries and 11 model files
  it cannot: serve a model: no inference engine is vendored yet
```

#### `mcf run <model> --prompt <text> [--limit <n>] [--seed <n>]`

Ask a model something. With no daemon, this drives MCF's own implementation of
inference — deliberately slow, written to be read. With `mcf serve` up, it is a
client of the daemon: tokens stream over the socket as they are produced, the
account follows, and the daemon records it.

The answer arrives with its mark, its sampler, its seed, and a sentence saying
what it cannot be. **A timing taken from MCF's own engine would measure the
engine, so it reports none** — that is what `mcf bench` and a provisioned engine
are for.

#### `mcf embed <model> --text <text>`

Ask an embedding model for a vector: JSON first, conditions after.

### Measuring

#### `mcf provision <component> [--list] [--remove <c> --because <why>] [--into <dir>]`

Build a pinned component in a container, everything recorded, removable without
residue. Requires `podman`.

```
$ mcf provision --list
MCF can provision 1 component(s)
  llama.cpp@925e1179947e  —  provisioned
    the reference implementation MCF's own engine is checked against
$ mcf provision llama.cpp
```

This is the prerequisite for timing anything: MCF's own engine cannot report a
speed, so a real engine has to be built first.

#### `mcf bench <model> --against <model> --prompt <text> [--limit <n>] [--seed <n>] [--resolving <%>] [--engine <name>] [--cold]`

Compare two models on an engine that can be timed. Needs `mcf serve` running
and a provisioned engine.

**There is no pass condition.** *They differ*, *they are the same to a stated
resolution* and *not decided* all exit successfully, because all three are
things the machine said. What fails is MCF being unable to run the benchmark at
all.

- `--resolving <%>` — the difference **you** care about. MCF will not report a
  0.8 % difference to someone who said 5 %.
- `--cold` — every trial loads the model for itself, so all trials are alike.
- `--engine <name>` — which engine; asked rather than assumed.

Trials are paired and interleaved, the run stops when its own evidence settles
rather than at a fixed count, and the reported effect size is a **range** with
its coverage, not a point estimate. A run on a machine busier than the measured
band is recorded in full and marked *not fit to contribute*.

#### `mcf probe <model> [--engine <name>] [--apply]`

Ask a model to do the thing and report what it did, **configuring nothing**.
`--apply` is the separate, recorded act of changing how MCF addresses that model
afterwards.

#### `mcf cross-check <model>`

Read one engine's tokens with the other and say whether they agree. This is how
MCF checks its own engine against the reference it provisioned.

#### `mcf segment <model> --prompt <text>`

The prompt as the model actually receives it, fragment by fragment: where text
breaks, where this vocabulary has no word for what you wrote, which of your
markers are real control tokens, and what the prompt spends against the model's
context.

### The record

#### `mcf log [--kind <kind>] [--last <n>] [--full]`

What happened on this machine, one line an event, with anything unreadable named
rather than skipped. Goes through a derived index — answering *the last twenty
acquisitions* costs microseconds where replaying the journal costs seconds.

#### `mcf show <entry-id>`

One recorded entry expanded into the measurements and conditions it rests on.

#### `mcf bundle <entry-id> [--into <path>]` · `mcf verify <bundle>`

`bundle` writes one file that reproduces one claim: the method, the conditions,
every trial, the provenance. `verify` asks whether **this** machine agrees and,
if not, which conditions differ — it will not say which difference caused it.

#### `mcf export --to <path>`

The whole record as one portable file.

#### `mcf licence [--full]`

The terms, and what conveying this binary obliges you to.

## 6 · The systems

**The record** is an append-only JSONL journal with a derived index. Nothing is
ever rewritten. Where an instrument is later found to have been wrong, an
*erratum* is recorded and every measurement taken before the correction renders
with it attached — the measurements stay exactly as taken, because they are what
the instrument said.

**The daemon** is the long-lived process. It holds models resident, serves
generation over a Unix socket, records what it did, and costs nothing when idle.
It holds no privilege; `mcf-helper` is the only program that does.

**The engines** are two. MCF's own is written to be read rather than to be fast
and is forbidden from reporting a speed. A provisioned engine — built by
`mcf provision` in a container from a pinned commit — is what timings come from,
and is named in every account.

**The measurements** carry their conditions or they do not travel. Every
comparison records what the machine was doing, what was competing for it, how
hot the processor was, which binary took the reading, and what stops the result
being contributed. An effect size is a range with stated coverage.

**The instruments are cross-checked against something that is not themselves.**
`scripts/ci.sh --with-instruments` compares the contention reading against the
kernel's own accounting, the processor sensor against the physical fact that a
die warms when it works, and the statistics against a brute-force enumeration.
Every module that measures declares what checks it; the five that are checked
against nothing say so in a count that may fall and may not rise.

## 7 · Working on MCF

```
$ scripts/ci.sh          # what gates a change: seconds, offline, hermetic
```

That is the whole gate. It runs formatting, `clippy -D warnings` over every
target, the test suite, and `cargo doc`, all with `--offline` passed rather than
merely expected.

**The heavy tiers run on a schedule rather than on every change**, because a
gating tier people skip does not gate:

| Flag | What it does |
|---|---|
| `--with-fuzz` | mutates known-good input into the parsers that read bytes MCF did not write |
| `--with-load` | MCF's claims under many callers at once |
| `--with-soak` | drift over a long run: descriptors, directories, memory |
| `--with-budget` | MCF's own cost against its ceilings, in release |
| `--with-mutation` | breaks the code deliberately and reports what the suite failed to notice |
| `--with-from-scratch` | the static artifact in a container holding it and nothing else |
| `--with-reproducibility` | builds twice and compares the bytes |
| `--with-corpus` | the conformance corpus through the engine |
| `--with-seed-set` | shows the published seed set representative |
| `--with-instruments` | each measuring instrument against an independent source |
| `--with-oracle` | MCF's engine against the provisioned reference |
| `--with-online` | acquires a real model from the real hub over TLS |
| `--all` | all of the above. Minutes, not seconds. |

**If your machine runs other projects' test suites too**, [build.md](doc/build.md)
section 12 covers it: the scheduled tiers take an exclusive window where one is
available, and the tier that measures timings needs it. A reading taken while
something else had the processor is one MCF refuses rather than reports.

**The workspace** is nine crates, layered so that nothing below depends on
anything above it:

| Crate | Holds |
|---|---|
| `mcf-core` | measurements, conditions, failures, hardware, time, capabilities |
| `mcf-record` | the append-only journal, its index, encode/decode |
| `mcf-lab` | the failure scenarios that reproduce every category |
| `mcf-standin` | MCF's own inference: GGUF, tokenizers, kernels, sampling |
| `mcf-hub` | HTTP, TLS, acquisition, provenance, fitment |
| `mcf-serve` | the daemon, its protocol, the probes |
| `mcf-bench` | comparison, the stopping condition, projection |
| `mcf-helper` | the three privileged operations |
| `mcf-cli` | every command |

Before writing anything, read [doc/rules.md](doc/rules.md) — 100 enforceable
rules, each with a citation and a check. Most of them are enforced
mechanically, and a surprising number of this project's findings are about
checks that passed without checking.

## 8 · What MCF will not do

Stated plainly, because a tool that hides its boundaries is worse than one that
lacks features.

- **It will not report a speed from its own engine.** A timing taken from a
  stand-in measures the stand-in.
- **It will not invent a value it could not read.** Unknown stays unknown; there
  are no plausible defaults.
- **It will not present an estimate as a measurement**, or promote one into one.
  An estimate can only be replaced by a measurement.
- **It will not give you an overall quality score.** Quality is plural; there is
  no type that combines two laboratories into one number.
- **It will not send anything anywhere on its own.** Every export is a file you
  read first, and there is no retraction, because there is no such act.
- **It will not compare two things that differ in more than one way** without
  saying so and withholding the delta.
- **It will not silently drop a trial, a row or a reading.** What could not be
  used is counted and named.

## 9 · The documents

| Document | Type | Holds | Read it when |
|---|---|---|---|
| [document-of-intent.md](doc/document-of-intent.md) | Intent | Why MCF exists, what it refuses to be, and every conflict and open question between its intents | A rule is ambiguous, two rules conflict, or no rule exists yet |
| [rules.md](doc/rules.md) | Rules | 100 enforceable rules in three tiers, each with a citation and a check | You are writing code, a test, a specification or a review comment |
| [roadmap.md](doc/roadmap.md) | Plan | Ten milestones, each a vertical MVP slice, with gating decisions and exit criteria | You are deciding what to build next |
| [taxonomy.md](doc/taxonomy.md) | Reference | The failure classification: three axes, sixteen domains, 111 codes | You are handling an error, writing a lab scenario, or rendering a failure |
| [labs.md](doc/labs.md) | Catalogue | Twenty-six candidate laboratories in four families, with what gates each and what it can claim | You are deciding what to measure, or designing a lab |
| [vendored.md](doc/vendored.md) | Register | What MCF ships, what it declined to ship, and the compatibility finding for each | You are about to admit a component, or want to know why MCF does not support something |
| [findings.md](doc/findings.md) | Record | What a prototype or a run established, with the conditions it was established under | A decision cites a run, or you are about to reopen one |
| [proposals.md](doc/proposals.md) | Proposals | Features argued in full — the claim each enables, how it works, what it costs, what it collides with | You are considering a feature, or about to propose one |
| [backlog.md](doc/backlog.md) | Register | Every outstanding decision and build item, with status | You are picking up work, or recording new work |
| [build.md](doc/build.md) | Reference | The workspace, the toolchain pin, the crate layering, and the checks that gate a change | You are about to build, add a crate, or admit a dependency |
| [mockup/](doc/mockup/) | Sketches | One picture per milestone of what finished looks like at that stage | You want to disagree with a design before it is code |

**The chain of authority is one-directional.** The intent document is the
source. Rules derive from it and carry its clauses forward with checks attached.
The roadmap sequences the work the rules imply, and the backlog registers it.
Proposals argue for work not yet accepted; mockups illustrate the result, and
references such as the taxonomy and the build document state what a decision
settled. A lower document never overrides a higher one: a disagreement means the
lower document is wrong, or that an amendment to the higher one is owed (§8).

### Mockups

`doc/mockup/` holds one file per roadmap milestone, showing what MCF looks like
when that milestone is finished. They exist so that "done" is a picture somebody
can disagree with *before* it is code.

**What they are not.** Not committed designs — every field name, flag, number
and layout is illustrative. Not a specification: where a mockup and the intent
document disagree, the document wins. Not real data: every figure is invented
and none may be cited as a measurement.

| Milestone | Mockup |
|---|---|
| M0 — The instrument | [M0-instrument.md](doc/mockup/M0-instrument.md) |
| M1 — Custody | [M1-custody.md](doc/mockup/M1-custody.md) |
| M2 — The host | [M2-host.md](doc/mockup/M2-host.md) |
| M3 — Right by construction | [M3-capabilities.md](doc/mockup/M3-capabilities.md) |
| M4 — The window | [M4-window.md](doc/mockup/M4-window.md) · [M4-window.html](doc/mockup/M4-window.html) |
| M5 — The measurement | [M5-measurement.md](doc/mockup/M5-measurement.md) |
| M6 — The judgment | [M6-judgment.md](doc/mockup/M6-judgment.md) |
| M7 — The loop | [M7-loop.md](doc/mockup/M7-loop.md) |
| M8 — Endurance | [M8-endurance.md](doc/mockup/M8-endurance.md) |
| M9 — The exchange | [M9-exchange.md](doc/mockup/M9-exchange.md) |

## 10 · Format contract

Every document in `doc/` follows this. It exists so that documents stay
comparable, so that duplication is visible when it appears, and so that a reader
learns one shape rather than five.

**Structure, in order.**

1. **Title** — the document's name, not the project's.
2. **Front matter** — a two-column table: type, version, status, authority, and
   what it derives from or derives.
3. **Lead** — what the document is and when to read it, in a few lines. The most
   important information is promoted here rather than discovered later.
4. **Contents** — a table linking the sections, for documents past roughly 200
   lines. Omitted below that, where it is noise.
5. **Body** — sections numbered where they are cited from elsewhere.
6. **Changelog** — last, newest first.

**Tense.** Present, throughout. A document states the position that holds now.
The changelog is the only place that describes what changed, and it is the only
place written in the past tense.

**Changelog.** Every document carries one. Each entry names the version, what
changed, and **why** — the reasoning is the point, not the diff. Entries are
newest first, so the current state is reached in one screen.

**Identifiers are stable for life** (rule C5). Clause numbers (§3.4), rule IDs
(A6, B12, P2), proposal IDs (PR3), backlog IDs (B-019, DEC-016), void numbers
(§7.16) and milestone IDs (M3) are never reused and never renumbered.

**Citation style.** Cite the source clause — `§3.4`, `§6.17`, `§IX`, `D4` — not
a paraphrase of it. Rules are cited by ID: `A6`, `B12`, `C5`, `P2`, where `P` is
always a precedence rule and a proposal is `PR3`. Backlog items by ID: `B-019`,
`DEC-016`. [rules.md](doc/rules.md) maps every clause to the rules that absorbed
it, so a clause citation traverses to an enforceable rule in one hop.

**Layout.** Tables for anything enumerable, because a table makes a missing row
visible and a prose list does not. Prose for reasoning, which is what tables
cannot hold. Code fences for anything a machine emits or consumes. No decoration
that carries no information.

**Duplication is a defect.** A statement lives in exactly one document; every
other document cites it. A fact restated in two places will eventually say two
different things, and the second reader will not know which one is current.

**Mockup conventions** (rules C7, C8): `$` is the operator's shell and output is
verbatim; records are shown as JSON regardless of on-disk form; placeholders are
written `<like-this>`; a surface that must show something it does not know shows
`unknown` rather than a plausible default.

## Licence

**GPL-3.0-only.** The full text is in [LICENSE](LICENSE), and `mcf licence`
states what conveying the binary obliges you to.

## Changelog

### Version 19 — a front door somebody can follow

Rewritten around setup and use. Version 18 said what MCF is and mapped the
documents, and a reader arriving with the intention of *running* it had to
assemble the answer from `doc/build.md`, the usage text and the source.

Added: requirements as a table, including the fact that MCF is Linux-only, which
the repository knew and never said. A quick start that reaches a working binary
in four commands. A configuration section, because environment variables were
the configuration and nothing enumerated them. A command reference covering
every command with what it needs, what it refuses and what its output looks
like. A description of each system — the record, the daemon, the engines, the
measurements, the instrument cross-checks — for a reader who wants to know how
the parts fit before reading nine crates.

Added *What MCF will not do*, stated plainly. The boundaries were previously
scattered through the status narrative, where they read as apology rather than
as design.

Removed the running status narrative. It had drifted — it described the daemon
as unable to serve a model when a provisioned engine had been serving one for
some time, and named benchmarking as absent when `mcf bench` exists. A front
page that must be re-edited whenever the work moves is a front page that will be
wrong; the roadmap and backlog hold status, and this document now points at
them rather than restating them.

Kept unchanged: the document map, the chain of authority, the mockup table and
the format contract, which are this document's standing content and are what its
authority line refers to.

### Version 18 — what is here, and what it came from

The summary had fallen behind the work. Added: `mcf check`, which asks whether
what this machine holds is still what it should be — the bytes against the
digest recorded for them, and the hub against what it published — and which
invalidates nothing when it finds something. `mcf log`, which reads the record
back through a derived index rather than by replaying a history. And
`mcf-helper`, the one program with rights, whose three operations were measured
rather than chosen.

The status line moved too: M1 is finished except for its reference model, which
is fifteen gigabytes and somebody's decision to spend them.

### Version 17 — something to ask a model

`mcf run` exists, which makes the status paragraph a shorter distance from
§VI's bar: having a model and using a model are one command apart, for the
behaviour half. The other half — how fast — is still absent, and now absent for
a stated reason rather than for want of a command.

### Version 16 — there is a daemon, and its idle cost is a measurement

M2's foundation, stated with the same care as its absence was. `mcf serve`
starts the long-lived process, recovers what the disk says, and answers three
questions — one of which is *what can you not do*. Its idle cost is the part
worth reading: zero context switches and zero processor ticks over a minute,
measured on a real process by the soak tier, rather than a claim about a design
that has no timers in it.

### Version 15 — the first dependency, and what it was chosen for

The status paragraph said MCF could not reach an encrypted hub. It can now, and
the interesting half is not that a TLS stack was vendored but how it was
picked: the usual provider is C and cannot be built for the target the
from-scratch container uses, so admitting it would have made an existing check
runnable in fewer places. The one admitted is Rust throughout and held a real
session with the hub before it was let in. That is the shape every future
admission is meant to have.

### Version 14 — models can arrive now

The status paragraph said *nothing acquires a model yet*, and that stopped being
true. `mcf pull`, `mcf list` and `mcf rm` are here: a model enters this machine
verified against the digest its hub declared, lives here with its provenance
beside it, and leaves against a stated reason. Asked for a repository and no
file, `pull` says which of the variants published there would run on this
machine and downloads none of them.

What the paragraph now says it cannot do is reach an encrypted hub, and *why* —
no TLS stack is vendored, the cost of admitting one is measured in F9, and the
refusal says so rather than failing obscurely. A status line that named the
capability without naming the boundary would be the fabricated report C7 is
written against.

### Version 13 — the gating command, and a shared machine

The front page gave the command that builds MCF and not the one that decides
whether a change may land, which is the one somebody arriving to work on it
needs first. Both are here now, with a pointer to what to do when the machine is
not MCF's alone.

### Version 12 — the status says what runs now

MCF has a second command and an engine since version 11: `mcf licence` carries
the terms a redistributor is obliged to convey, and D31's stand-in runs a model
end to end from a file it is given. The status line said neither, and said
"nothing measures a model yet" where the honest statement is narrower — nothing
*acquires or serves* one, and no number about a model exists.

The lab catalogue's count is corrected here too: twenty-six, not twenty-three.

### Version 11 — the vendoring register joins the map

`doc/vendored.md` added. D28 makes every vendored component's terms MCF's
problem and D23 makes a deferral a decision to record rather than a gap; both
are the same register seen from two sides, and it exists before the first
component is admitted so that it gates rather than describes.

### Version 10 — `P` means one thing

The format contract's citation style now says which namespace `P` belongs to. It
named both the precedence rules and the proposals, which is an ambiguity a
contract about citation could not afford; the proposals are now `PR<n>` and
`P<n>` is deprecated there in favour of the named successor (B-353).

### Version 9 — the M0 product runs

The status line says what `mcf doctor` does rather than that it is unwritten,
and the example command runs it. A front door that describes a plan when the
thing exists is a front door that will be read as a plan.

### Version 8 — evidence gets a document

`doc/findings.md` added. B-002's condition is that a void be *confirmed in
writing*, and neither the intent document nor the register was the right place
for the evidence behind one: the first states positions, the second states
status, and a run's conditions and readings are a third kind of thing.

### Version 7 — there is code

`doc/build.md` added and the status line corrected: the repository is no longer
documentation alone. B-001 landed the workspace, so the front door has to say
what building it involves rather than that there is nothing to build.

### Version 6 — the taxonomy joins the map

`doc/taxonomy.md` added, answering the failure-classification void that had been
open since the first version of the intent document.

### Version 5 — the lab catalogue joins the map

`doc/labs.md` added, drafting the laboratories the project measures with, after
intent v9 made quality plural and the question undeferrable.

### Version 4 — proposals join the map

`doc/proposals.md` added. A feature needs somewhere to be argued before it is
committed to: B15 admits weight only against a stated cost, and a backlog row
cannot hold that argument.

### Version 3 — the exchange joins the map

M9 and its mockup added, and the rule and milestone counts corrected, following
version 7 of the intent document.

### Version 2 — one README for the repository

Moved from `doc/README.md` to the root. A repository needs exactly one front
door, and a reader arriving at the project should not have to open a directory
to find out what the project is. Gains a lead stating what MCF does and that no
implementation exists yet; keeps the document map and the format contract.

### Version 1 — the documentation is standardized

Created to end fragmentation. Five documents had grown independently, each with
its own header shape, its own way of citing, and its own mixture of present
statement and historical narrative. There was no index, so the reading order was
folklore, and no stated format, so every new document invented one.

This file supplies both: the map of what exists and the contract every document
holds to. It absorbs `mockup/README.md`, which held the same kind of content one
directory down and split the entry point in two.
