# Use statistics

What MCF records about real use, and why that replaced diagnostics.

## The idea

A diagnostic asks *how does this model behave under conditions I have
constructed*. That is a fair question and an expensive one to answer honestly,
and the answer is about the constructed conditions rather than about your work.

MCF instead records what actually happened. Every hold produces a row saying:
the exact configuration it ran under, the exact hardware it ran on, and the
performance that combination delivered. Enough of those rows and you can answer
the questions people actually have — is this quantization worth it here, does
offloading more layers help on this card, did the last engine build make things
worse — from your own machine doing your own work.

Nothing here grades a model. A row is a record of what occurred, not a verdict.

## What a row holds

**The configuration, exactly.** The model file and its digest, the engine and
its build, context length, layer offload, thread count, batch size, attention
setting, draft head, rope scaling, and where each of those came from — chosen
by MCF or given by you.

**The hardware, exactly.** Processor model and core count, accelerator model and
memory, total memory, kernel, and the governor and power state in effect. Not a
class of machine — this machine, as it was at the time.

The hardware is read by `mcf doctor`, not by the daemon. A daemon that read
`/proc` and `/sys` while holding a model would be one of the things competing
for the machine it reports, so the reading is done by a command you ran and the
hold's row is matched to it by when it happened.

**The performance that resulted.** Prefill and generation rates, latency to
first token, the token counts from [monitoring.md](monitoring.md), memory and
video memory actually resident, watts drawn, and the wall-clock shape of the
hold: up, active, idle.

A row is only comparable to another row when the conditions match, so the
conditions travel with the figures rather than being recorded separately. A
number without its conditions is not something you can use later, and MCF does
not store one.

## What it is not

**Not telemetry.** The record is local. Nothing is sent anywhere as a
side effect of using MCF, there is no default-on reporting, and there is no
setting that quietly begins transmitting.

**Not user content.** Prompts and completions are not in the record. What is
recorded is the shape of the work — counts, rates, durations — never its text.

## Sending it somewhere

Nothing leaves on its own. Two commands move the record, and both are acts you
perform:

```
mcf export --to <path>      # the record as one portable file
mcf share [--into <path>]   # what would leave, row by row, before it does
```

`mcf export` writes the whole record with a digest over it, and says how many
entries hold a prompt or a completion recorded before MCF kept content out of
the record. Those rows are in the file. The record is not edited to look
better.

`mcf share` is the curated route: it shows the actual rows rather than a
description of them, states the terms, and states that the act cannot be
undone. **It has not caught up with this document.** `share` reads the paired
comparison rows the trials used to write, and nothing writes those any more,
so on a record produced by hosting it correctly reports that no row can
travel. Reshaping it around the hold rows above is open work.

Neither command sends anything anywhere. Both write a file, and carrying that
file somewhere is your act.

## Reading it back

```
mcf log [--kind <kind>] [--last <n>] [--full]   # what happened on this machine
mcf failures [--last <n>]                       # what went wrong, classified
```

Failures are classified against the codes in [taxonomy.md](taxonomy.md), which
the code is checked against, so a failure code means the same thing in the
record as it does in the source.

## Where the code is today

The record is append-only, survives a restart, and recovers from an interrupted
write. A hold writes both halves of the row above: the settings it ran under,
what it declared and what MCF recommended go down when the model is held; the
memory and card memory it gave back, the energy it drew and what that cost, and
the token counts the engine reported go down when it is let go. The hardware
comes from `mcf doctor`, for the reason above.

What is not built is the reading: `mcf log` shows the rows, and nothing yet
gathers them into an answer to "is this quantization worth it here". The rows
are being kept so that question can be asked later.

Two pieces of the old vocabulary are still in the source behind this. `mcf
share` speaks in comparisons and arms, as above, and the types under it —
`mcf_core::trial` and `mcf_core::contribution` — are the paired-trial
machinery the diagnostics used. They are reachable and they are not written
to any more. `mcf_core::measurement` is not part of that residue: `Bytes`,
`Conditions` and `Quantity` are the vocabulary the hardware, engine, cost and
record paths all speak.
