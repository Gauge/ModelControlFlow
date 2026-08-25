# ModelControlFlow

| | |
|---|---|
| **Type** | Repository README — what this is, and the format every document holds to |
| **Version** | 10 |
| **Status** | Living |
| **Authority** | Governs the form of every document in `doc/`, never their content |

**MCF makes the open weights ecosystem usable by one person on one machine
without that person becoming a full-time operator of it.** It acquires models
with their provenance intact, serves them as a dependable local endpoint,
puts them on a bench of purpose-built diagnostic laboratories, measures what
they cost and what they are worth *on this hardware for the work actually being
done*, and makes the result legible from anything with a browser. Evidence
leaves the machine only when the operator sends it, row by row.

The measuring is the point. Choosing a local model today is folklore — people
pick by leaderboard positions measured on someone else's hardware with someone
else's quantization against benchmarks that do not resemble their work. MCF
replaces folklore with measurement taken here.

**Status: M0 in progress.** `mcf doctor` works: it reports what this machine
is, what MCF costs on it against its stated ceilings, and what MCF will and will
not promise here, and writes the whole thing to an append-only record. Nothing
acquires, serves or measures a model yet.
Start with [doc/document-of-intent.md](doc/document-of-intent.md) to know what
MCF is for, [doc/rules.md](doc/rules.md) before writing anything,
[doc/roadmap.md](doc/roadmap.md) to see what gets built next, and
[doc/build.md](doc/build.md) to build it.

```
$ cargo build --locked --release && ./target/release/mcf doctor
```

## The documents

| Document | Type | Holds | Read it when |
|---|---|---|---|
| [document-of-intent.md](doc/document-of-intent.md) | Intent | Why MCF exists, what it refuses to be, and every conflict and open question between its intents | A rule is ambiguous, two rules conflict, or no rule exists yet |
| [rules.md](doc/rules.md) | Rules | 99 enforceable rules in three tiers, each with a citation and a check | You are writing code, a test, a specification or a review comment |
| [roadmap.md](doc/roadmap.md) | Plan | Ten milestones, each a vertical MVP slice, with gating decisions and exit criteria | You are deciding what to build next |
| [taxonomy.md](doc/taxonomy.md) | Reference | The failure classification: three axes, sixteen domains, 110 codes | You are handling an error, writing a lab scenario, or rendering a failure |
| [labs.md](doc/labs.md) | Catalogue | Twenty-three candidate laboratories in four families, with what gates each and what it can claim | You are deciding what to measure, or designing a lab |
| [findings.md](doc/findings.md) | Record | What a prototype or a run established, with the conditions it was established under | A decision cites a run, or you are about to reopen one |
| [proposals.md](doc/proposals.md) | Proposals | Seven features argued in full — the claim each enables, how it works, what it costs, what it collides with | You are considering a feature, or about to propose one |
| [backlog.md](doc/backlog.md) | Register | Every outstanding decision and build item, with status | You are picking up work, or recording new work |
| [build.md](doc/build.md) | Reference | The workspace, the toolchain pin, the crate layering, and the checks that gate a change | You are about to build, add a crate, or admit a dependency |
| [mockup/](doc/mockup/) | Sketches | One picture per milestone of what finished looks like at that stage | You want to disagree with a design before it is code |

**The chain of authority is one-directional.** The intent document is the source.
Rules derive from it and carry its clauses forward with checks attached. The
roadmap sequences the work the rules imply, and the backlog registers it.
Proposals argue for work not yet accepted; mockups illustrate the result, and
references such as the taxonomy and the build document state what a decision
settled. A lower document never overrides a higher one: a
disagreement means the lower document is wrong, or that an amendment to the
higher one is owed (§8).

## Format contract

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
place written in the past tense. "This was resolved in version 4" belongs in a
changelog entry; the clause it resolved simply states the resolution.

**Changelog.** Every document carries one. Each entry names the version, what
changed, and **why** — the reasoning is the point, not the diff, and §8 of the
intent document requires that the reasoning outlive the change. Entries are
newest first, so the current state is reached in one screen.

**Identifiers are stable for life** (rule C5). Clause numbers (§3.4), rule IDs
(A6, B12, P2), proposal IDs (PR3), backlog IDs (B-019, DEC-016), void numbers
(§7.16) and milestone
IDs (M3) are never reused and never renumbered. A clause whose substance moves
leaves an index entry behind pointing at where it went.

**Citation style.** Cite the source clause — `§3.4`, `§6.17`, `§IX`, `D4` — not
a paraphrase of it. Rules are cited by ID: `A6`, `B12`, `C5`, `P2` — where `P`
is always a precedence rule, and a proposal is `PR3`. Backlog
items by ID: `B-019`, `DEC-016`. [rules.md](doc/rules.md) maps every clause to the
rules that absorbed it, so a clause citation traverses to an enforceable rule in
one hop and neither document has to restate the other.

**Layout.** Tables for anything enumerable — rules, milestones, items,
coverage — because a table makes a missing row visible and a prose list does
not. Prose for reasoning, which is what tables cannot hold. Code fences for
anything a machine emits or consumes. No decoration that carries no information.

**Duplication is a defect.** A statement lives in exactly one document; every
other document cites it. A fact restated in two places will eventually say two
different things, and the second reader will not know which one is current.

## Mockups

`mockup/` holds one file per roadmap milestone, showing what MCF looks like when
that milestone is finished: what a command prints, what a record contains, what
the window renders. They exist so that "done" is a picture somebody can disagree
with *before* it is code, rather than a paragraph everyone reads differently.

Each ends with a table mapping its surfaces to the clauses of intent they
satisfy. A mockup that cannot show a measurement's conditions, a failure's
classification or a default's attribution has found a design error, not a
mockup error.

**What they are not.** Not committed designs — every field name, flag, number
and layout is illustrative and expected to be amended. Not a specification:
where a mockup and the intent document disagree, the document wins. Not real
data: every figure is invented, no figure may be cited as a measurement (rules
C7, A20), and the model names in them are illustrative rather than the reference
model of §XII. And not headless-optional — every mockup shows the headless
surface first, because a capability reachable only through an interface is one
the laboratory cannot test (A22).

**Conventions** (rules C7, C8): `$` is the operator's shell and output is
verbatim; records are shown as JSON regardless of on-disk form; placeholders are
written `<like-this>`; a surface that must show something it does not know shows
`unknown` rather than a plausible default (A7).

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

## Changelog

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
