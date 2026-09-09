# Goals

MCF hosts one model on one machine, through a window or a terminal, and reports
what that costs. Everything here follows from that sentence.

## What MCF is for

**Hosting a single model, well.** You choose a model file. MCF obtains the
engine it needs, loads it with settings you can see and change, and holds it on
a port. When you are done, it gives the memory back.

**Complete control over how it runs.** Every setting the engine accepts is
exposed and editable, including which file is loaded. MCF chooses sensible
defaults and shows its reasoning, but it never makes a choice you cannot
inspect and override. A setting MCF picked and a setting you picked are
labelled differently and both are visible.

**An honest account of the cost.** While a model is held, MCF reports the work
going through it, the load on the machine, the power being drawn, and what that
power costs at your electricity price. The figures are the ones a person
actually needs to decide whether this model on this machine is worth running.

**Reliability first, then control, then speed.** In that order. A host that is
fast and drops a session has failed at the thing it exists to do. Performance
is a real concern and it is measured, but it never buys its way past
correctness or predictability.

## What MCF will not do

**Serve more than one model at once.** Hosting a second model releases the
first. Multi-model serving is a different product with different failure modes,
and refusing it keeps the memory story simple enough to reason about.

**Run diagnostics on models.** MCF does not benchmark, score, evaluate or grade
models. It does not tell you which model is better. Those questions need
controlled conditions that a hosting tool cannot honestly provide, and answers
produced casually are worse than no answer.

**Decide anything for you silently.** Where MCF picks a default it says so and
says why. Where it cannot determine something it says that too, rather than
substituting a plausible value.

**Send anything anywhere on its own.** Statistics are collected locally. Nothing
leaves the machine except by an explicit act that shows you the rows first.

## Why diagnostics were removed

MCF used to be a measuring instrument: a bench of laboratories that put models
through controlled trials and reported what they cost and what they were worth.
That work was careful, and it was the wrong shape for the problem.

A controlled trial answers *how does this model behave under these conditions*.
What a person hosting a model actually needs to know is *how is this model
behaving right now, on my hardware, under my load*. The second question is
answered by watching real use, and watching real use is cheap, continuous and
honest in a way a synthetic trial is not.

So the trials are gone and what replaces them is a record of what actually
happened: the exact configuration, the exact hardware, and the performance that
combination delivered. See [statistics.md](statistics.md).

## Where the code is today

This document describes the target. The diagnostics are gone — the
laboratory and bench crates, the commands that drove them, and the daemon
operations behind them — and what remains is the hosting tool. Two things it
describes are still to build:

- Engines are built through `mcf provision`. A model does not yet pull and
  build the engine it needs without being asked.
- The record holds what a measurement produced, not what a hold did. The row
  [statistics.md](statistics.md) describes is not written yet.

Each of these is work to be done, not a description of behaviour that exists.
Where a document in this directory describes something not yet built, it says
so in the same way.
