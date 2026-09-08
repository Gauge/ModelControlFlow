# Monitoring

What MCF reports while a model is held, and what each figure means.

Monitoring is meant to be light enough to leave running. It is a continuous
account of real use, not a measurement taken under controlled conditions —
nothing here is a benchmark and nothing here grades a model.

## Throughput

Four token counts, which are deliberately not the same number. Where they
differ tells you something.

| **Received** | Tokens arriving from clients in requests |
| **Processed** | Tokens the engine actually ran through prefill |
| **Generated** | Tokens the model produced |
| **Sent** | Tokens delivered back to clients |

Received and processed differ when a prefix is reused: a conversation that
resends its history costs far less on the second turn than the token count
suggests, and the gap between these two is where that saving shows up.

Generated and sent differ when a client goes away mid-stream. Work that was
done and thrown away is still work the machine paid for, and it stays visible
here rather than being quietly dropped from the total.

**Active sessions** is the number of clients currently holding an open request.

## Machine

| **Processor** | Utilisation across cores |
| **Accelerator** | Utilisation, where one is present |
| **Memory** | Resident bytes, and what is free |
| **Video memory** | Resident bytes, and what is free |

Where no accelerator is present MCF says so rather than reporting zero, because
zero and absent are different facts and confusing them is how a reader draws a
wrong conclusion.

## Energy and cost

| **Watts** | Power drawn attributable to the held model |
| **Energy** | Accumulated over the hold |
| **Cost** | Energy at your electricity price |

Power is read from the hardware's own counters — the processor's energy
counter through the helper binary, and the accelerator's through its vendor
interface. Where a counter is not readable, MCF reports that it is not readable.
It does not estimate watts from utilisation and present the result as a
measurement.

"Attributable to the held model" means MCF separates the machine's idle draw
from the additional draw under load, and reports the second. A machine that
burns 40 W doing nothing and 140 W while generating is drawing 100 W for the
model.

**Cost is not implemented yet.** The electricity price setting described in
[configuration.md](configuration.md) does not exist, and neither does the
running cost derived from it. Power and energy are measured and recorded today.

## Time

| **Up** | How long the daemon has been running |
| **Active** | How much of that was spent serving requests |
| **Idle** | How much of that was spent holding a model and doing nothing |

Idle time is worth reporting because it is not free. A model resident in memory
is occupying memory and, on most accelerators, drawing power whether or not
anyone is asking it anything. The idle figure and the idle watts together are
what tell you the cost of keeping something loaded that nobody is using.

## Where to see it

All three interfaces show the same figures, because all three read them from
the same daemon:

- `mcf desk` — the window, updating live
- `mcf tui` — the same screens over a terminal
- `mcf status` — a snapshot from the command line

The figures are also written to the record, which is what
[statistics.md](statistics.md) describes.
