# Documentation

| | |
|---|---|
| **Type** | Index and format contract |
| **Version** | 1 |
| **Status** | Living |
| **Authority** | Governs the form of every document in `doc/`, never their content |

Start with [document-of-intent.md](document-of-intent.md) if you want to know
what MCF is for, and [rules.md](rules.md) if you are about to write something.

## The documents

| Document | Type | Holds | Read it when |
|---|---|---|---|
| [document-of-intent.md](document-of-intent.md) | Intent | Why MCF exists, what it refuses to be, and every conflict and open question between its intents | A rule is ambiguous, two rules conflict, or no rule exists yet |
| [rules.md](rules.md) | Rules | 60 enforceable rules in three tiers, each with a citation and a check | You are writing code, a test, a specification or a review comment |
| [roadmap.md](roadmap.md) | Plan | Nine milestones, each a vertical MVP slice, with gating decisions and exit criteria | You are deciding what to build next |
| [backlog.md](backlog.md) | Register | Every outstanding decision and build item, with status | You are picking up work, or recording new work |
| [mockup/](mockup/) | Sketches | One picture per milestone of what finished looks like at that stage | You want to disagree with a design before it is code |

**The chain of authority is one-directional.** The intent document is the source.
Rules derive from it and carry its clauses forward with checks attached. The
roadmap sequences the work the rules imply, and the backlog registers it.
Mockups illustrate the result. A lower document never overrides a higher one: a
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
(A6, B12, P2), backlog IDs (B-019, DEC-016), void numbers (§7.16) and milestone
IDs (M3) are never reused and never renumbered. A clause whose substance moves
leaves an index entry behind pointing at where it went.

**Citation style.** Cite the source clause — `§3.4`, `§6.17`, `§IX`, `D4` — not
a paraphrase of it. Rules are cited by ID: `A6`, `B12`, `C5`, `P2`. Backlog
items by ID: `B-019`, `DEC-016`. [rules.md](rules.md) maps every clause to the
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
| M0 — The instrument | [M0-instrument.md](mockup/M0-instrument.md) |
| M1 — Custody | [M1-custody.md](mockup/M1-custody.md) |
| M2 — The host | [M2-host.md](mockup/M2-host.md) |
| M3 — Right by construction | [M3-capabilities.md](mockup/M3-capabilities.md) |
| M4 — The window | [M4-window.md](mockup/M4-window.md) · [M4-window.html](mockup/M4-window.html) |
| M5 — The measurement | [M5-measurement.md](mockup/M5-measurement.md) |
| M6 — The judgment | [M6-judgment.md](mockup/M6-judgment.md) |
| M7 — The loop | [M7-loop.md](mockup/M7-loop.md) |
| M8 — Endurance | [M8-endurance.md](mockup/M8-endurance.md) |

## Changelog

### Version 1 — the documentation is standardized

Created to end fragmentation. Five documents had grown independently, each with
its own header shape, its own way of citing, and its own mixture of present
statement and historical narrative. There was no index, so the reading order was
folklore, and no stated format, so every new document invented one.

This file supplies both: the map of what exists and the contract every document
holds to. It absorbs `mockup/README.md`, which held the same kind of content one
directory down and split the entry point in two.
