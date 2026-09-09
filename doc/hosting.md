# Hosting

How a model is held, and how the engine it needs is obtained.

## One model at a time

MCF holds one model. Asking it to hold another releases the first and says so
before it does. This is a deliberate limit, not a missing feature — see
[goals.md](goals.md).

The consequence worth knowing: memory is either free or belonging to exactly
one model, so "how much have I got left" always has an answer, and releasing a
model always returns everything it took.

## Holding a model

```
mcf host <model>
```

MCF resolves the model file, works out which engine can run it, makes sure that
engine is present, chooses settings for this machine, and starts holding it on
a port. It prints the settings it chose and what they cost in memory before it
commits to them.

Anything that speaks OpenAI's chat completions API can then reach it. `mcf
hosted` says what is being held and where; `mcf unhost` stops holding it and
gives the memory back.

Every setting can be given explicitly:

| `--context <n>` | Context window in tokens |
| `--port <n>` | Port to listen on |
| `--engine <name>` | Force a particular engine |
| `--on cpu\|gpu` | Where to put it |
| `--gpu-layers <n>` | How many layers to offload |
| `--threads <n>` | Thread count |
| `--batch <n>` | Batch size |
| `--slots <n>` | How many conversations at once; they share the window |
| `--cache-reuse <n>` | Smallest run of tokens recovered rather than read again |
| `--prompt-cache on\|off`, `--prompt-cache-memory <mib>` | Whether what was read is kept, and how much memory for it |
| `--idle-slots on\|off` | Write an unused conversation to that cache |
| `--context-shift on\|off`, `--keep <n>` | Carry on past a full window, and what survives |
| `--checkpoints <n>`, `--checkpoint-spacing <n>` | Places a conversation can return to |
| `--flash-attention` | Enable flash attention |
| `--cache <type>` | How wide each cached token is held; a narrower one fits a longer conversation in the same memory |
| `--draft-head on\|off` | Use the file's own draft head, where it carries one |
| `--rope-scaling <kind>`, `--rope-scale <n>` | Rope scaling, left to the engine unless asked for |
| `--api-key <key>` | Require a key on the endpoint |

### Across several devices

Where a model is too large for any single device, MCF divides it across the
cards it can see, in proportion to what each has free, and tells the engine the
split. A model needing 135 GB runs on two cards holding 116 GB and 101 GB.

One card holds the whole model wherever it fits — crossing between cards costs
time, so spreading is what MCF does when it must, not by preference. The
`spread over` setting says which it did.

MCF plans to all of a device's free memory. `MCF_MEMORY_HEADROOM` lowers that
share if you want room left for other work; see
[configuration.md](configuration.md).

A setting you give is used as given. A setting you omit is chosen, and the
choice is shown alongside where it came from. See
[configuration.md](configuration.md) for how to change these without passing
flags every time.

## Engines

An engine is the thing that actually runs the model. Different model files need
different engines, and engines need building against the hardware they will run
on, which is why MCF builds them rather than shipping binaries.

Engines are built in a container with `podman`, from pinned sources vendored
into this repository. Everything about the build is recorded, and an engine can
be removed without residue. The components and their terms are listed in
[vendored.md](vendored.md).

```
mcf provision              # build the engine the model here needs
mcf provision <component>  # build a named one
mcf provision --list       # what is built, and what is available
mcf provision --remove <component> --because <why>
```

Built engines live in `~/.local/share/mcf/provisioned`.

**On demand.** `mcf host` builds the engine a model needs without being asked.
Where no built engine runs the model, MCF says which component it is building,
builds it, and then holds the model — one command rather than two. Running
`mcf provision` yourself still works, and is how you build a component before
you need it.

## Asking it something

```
mcf ask --prompt <text>              # whatever is held
mcf ask <model> --prompt <text>      # a model you name
mcf ask --prompt <text> --limit <n> --seed <n> --engine <name>
```

The answer streams as it arrives. After it, MCF prints what produced it: the
model, the prompt and produced token counts and why it stopped, the sampler and
seed, the engine, and which daemon served it. Where the engine dies part way,
what arrived stays on the page and the failure is named under it — a partial
answer is an answer, not a blank.

A file MCF's own reader cannot read is refused with the engine that might read
it named. The window's chat box asks the same way through the same daemon.

## The daemon

`mcf serve` is the daemon. It holds the model, owns the endpoint, and is what
the window, the terminal interface and the command line all talk to over a
control socket at `/run/user/<uid>/mcf/control.sock`.

It stays up, recovers what is on disk after a restart, and costs nothing while
idle. Starting the window or the terminal interface starts it if it is not
already running. `mcf status` asks a running daemon what it is and what it is
holding; `mcf stop --because <why>` asks it to stop.

The daemon holds no privilege it does not need. The three things that require
rights it does not have — the processor governor, a device's exclusive mode,
and the processor's energy counter — are done by a separate helper binary
beside it.

## Getting models here

```
mcf pull <owner/name>              # what this repository publishes, and what will run here
mcf pull <owner/name>:<file>       # bring one file here
mcf list                           # what this machine is holding
mcf rm <model> --because <why>     # stop holding it
```

Models are stored where `MCF_MODELS` says, or under `$XDG_DATA_HOME/mcf` or
`$HOME/.local/share/mcf`. `mcf pull` without a file says which variants exist
and which of them this machine could actually run.

A credential is read only where you name one, with `--token-from <file>` or
`--token-from-env <VAR>`.
