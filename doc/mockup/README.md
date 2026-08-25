# Stage Mockups

One mockup per roadmap milestone, showing what MCF looks like when that
milestone is finished. They exist so that "done" is a picture somebody can
disagree with *before* it is code, rather than a paragraph everyone reads
differently.

| Milestone | Mockup |
|---|---|
| M0 — The instrument | [M0-instrument.md](M0-instrument.md) |
| M1 — Custody | [M1-custody.md](M1-custody.md) |
| M2 — The host | [M2-host.md](M2-host.md) |
| M3 — Right by construction | [M3-capabilities.md](M3-capabilities.md) |
| M4 — The window | [M4-window.md](M4-window.md) · [M4-window.html](M4-window.html) |
| M5 — The measurement | [M5-measurement.md](M5-measurement.md) |
| M6 — The judgment | [M6-judgment.md](M6-judgment.md) |
| M7 — The loop | [M7-loop.md](M7-loop.md) |
| M8 — Endurance | [M8-endurance.md](M8-endurance.md) |

## What these are

- **Sketches of intent.** They show the shape of the finished surface: what a
  command prints, what a record contains, what the window renders.
- **A check against the principles.** Each mockup ends with the clauses of the
  Document of Intent it is trying to satisfy. If a mockup cannot show a
  measurement's conditions, a failure's classification, or a default's
  attribution, the design is wrong, not the mockup.
- **Headless-first, per §XI and §6.21.** Every mockup shows the headless surface.
  The window (M4) is drawn as a client of exactly what the earlier mockups show,
  because a capability reachable only through the interface is one the laboratory
  cannot test (§3.5).

## What these are not

- **Not committed designs.** Every number, field name, flag and layout here is
  illustrative. They will be wrong in detail and are expected to be amended.
- **Not a specification.** Where a mockup and the Document of Intent disagree,
  the document wins and the mockup is corrected.
- **Not real data.** Every figure shown is invented for illustration. Per §5,
  MCF is actively suspicious of numbers that did not originate locally — and
  none of these did. No figure here may ever be cited as a measurement.

## Conventions

Governed by **C7** and **C8** in [rules.md](../rules.md), which is where the
rules live. In short: `$` is the operator's shell and output is verbatim;
records are shown as JSON regardless of on-disk form; placeholders are written
`<like-this>`; a surface that must show something it does not know shows
`unknown` rather than a plausible default (**A7**); and no figure in these
files may ever be cited as a measurement.
