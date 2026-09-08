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

If you want to share a record, you ask:

```
mcf share [--into <path>]   # what would leave, row by row, before it does
mcf export --to <path>      # the record as one portable file
```

`mcf share` shows you the actual rows, not a description of them, and states
that the act cannot be undone. Publication is per share and always explicit.

## Reading it back

```
mcf log [--kind <kind>] [--last <n>] [--full]   # what happened on this machine
mcf failures [--last <n>]                       # what went wrong, classified
```

Failures are classified against the codes in [taxonomy.md](taxonomy.md), which
the code is checked against, so a failure code means the same thing in the
record as it does in the source.

## Where the code is today

The record exists and works: it is append-only, it survives a restart, and it
recovers from an interrupted write. What it holds today is shaped around the
diagnostic runs that are being removed rather than around continuous hosting.

The work is to make a hold produce the row described above — configuration,
hardware and performance together, written as the hold proceeds rather than
assembled at the end of a trial. Until that is done, the rows in the record are
about measurements taken, not about use.
