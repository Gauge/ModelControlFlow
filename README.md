# ModelControlFlow

**MCF hosts one model on your machine and shows you what it costs to run.**

You point it at a model file, it fetches and builds the engine that model
needs, and it holds it on a port where anything that speaks OpenAI's API can
reach it. There is a window, a terminal interface, and a command line — all
three are clients of the same daemon, and all three show the same thing.

Every setting the engine takes is yours to edit, including which file is
loaded. Nothing is hidden behind a heuristic you cannot see or override.

While a model is held, MCF reports what it is doing and what it is drawing:
tokens through the endpoint, processor and accelerator load, memory and video
memory, watts and what those watts cost at your electricity price, and how long
the machine has been up, busy and idle.

## Contents

| 1 | [What it is for](#1--what-it-is-for) |
| 2 | [Requirements](#2--requirements) |
| 3 | [Getting started](#3--getting-started) |
| 4 | [The three interfaces](#4--the-three-interfaces) |
| 5 | [What it measures](#5--what-it-measures) |
| 6 | [Where things live](#6--where-things-live) |
| 7 | [Working on MCF](#7--working-on-mcf) |
| 8 | [The documents](#8--the-documents) |
| — | [Licence](#licence) |

## 1 · What it is for

One model, held well. MCF is not a model manager, a benchmark suite or a
research instrument. It runs a single model at a time and gives you complete
control over how it runs, plus an honest account of what that costs.

The full statement of what MCF does and does not try to do is in
[goals.md](doc/goals.md). The short version:

- **One model at a time.** Hosting a second releases the first.
- **The engine is MCF's problem, not yours.** The engine a model needs is
  fetched, built and installed on demand.
- **Every setting is exposed.** Context length, layer offload, thread count,
  batch size, attention, draft head, rope scaling, the port, the API key, and
  the model file itself.
- **Reliability and control come first.** Performance matters and is measured,
  but a fast host that drops a session is a failure.

## 2 · Requirements

| | |
|---|---|
| **Operating system** | **Linux.** MCF reads `/proc` and `/sys` throughout and has no `target_os` guards. It will not build usefully elsewhere. |
| **Rust** | 1.98.0, pinned in `rust-toolchain.toml`. `rustup` honours the pin; you do not need to select a toolchain. |
| **`podman`** | Needed to build engines, at `/usr/bin/podman` or `/usr/local/bin/podman`. Docker is not used. Without it, MCF reports a classified refusal rather than failing obscurely. |
| **SDL3** | Needed for the window (`mcf desk`), and only for that. It must be provisioned before MCF is built. The terminal interface needs nothing. |
| **Network** | Not needed to build. Needed to fetch models and engines. |
| **Disk** | The binary is around 4.3 MiB. Models are not — point `MCF_MODELS` at a drive with room. |
| **An accelerator** | Optional. MCF reports what is present and says plainly when nothing is. |

## 3 · Getting started

Build the binary and install it:

```
cargo build --release
scripts/install.sh
```

`install.sh` puts the release binary on your path along with a launcher and
icon, so the window appears in your desktop's application search. It installs
`target/release`; it will not build for you.

Bring a model here, then hold it:

```
mcf pull unsloth/Qwen3-4B-GGUF:Qwen3-4B-Q4_K_M.gguf
mcf host Qwen3-4B-Q4_K_M
```

`mcf host` prints the settings it chose and what they cost. If the engine that
model needs is not built yet, MCF builds it first and says so.

Ask it something without leaving the terminal:

```
mcf ask --prompt "what is this machine good at?"
```

`mcf ask` talks to whatever is held; name a model to ask that one instead. It
prints the answer as it arrives, then what produced it — the engine, the
sampler and seed, the token counts, and which daemon served it.

Or skip the command line and open the window:

```
mcf desk
```

## 4 · The three interfaces

All three talk to the same daemon over a control socket, so what you do in one
is visible in the others immediately.

| `mcf desk` | The window. Needs SDL3 at build time, and says so if it is missing. |
| `mcf tui` | The same screens with no display attached — over SSH, or on a headless box. |
| `mcf` | With no arguments, opens the terminal interface where there is a terminal to draw in. Piped or redirected, it prints usage instead, so scripts see what they expect. |
| `mcf <command>` | One-shot commands for scripting and for anything you would rather type. |

Put the window's launcher on your desktop and type `mcf` when there isn't
one — the same daemon either way.

The daemon itself is `mcf serve`. It stays up, recovers what is on disk after a
restart, and costs nothing while idle. Starting the window or the terminal
interface starts it if it is not already running.

## 5 · What it measures

Monitoring is meant to be light enough to leave on. The full set, with the
definition of each figure and how it is obtained, is in
[monitoring.md](doc/monitoring.md).

| **Throughput** | Tokens received, sent, processed and generated; active sessions |
| **Machine** | Processor and accelerator utilisation, memory and video memory |
| **Energy** | Watts drawn by the held model, and the running cost at your price per kilowatt-hour |
| **Time** | How long the machine has been up, how much of that was active, and how much idle |

MCF does not run diagnostics on models. What it collects instead is a record of
real use: the exact configuration, on the exact hardware, with the performance
that combination actually delivered. That record and what it is for are
described in [statistics.md](doc/statistics.md).

## 6 · Where things live

| Models | `$MCF_MODELS`, or `$XDG_DATA_HOME/mcf` or `$HOME/.local/share/mcf` |
| Engines | Built into `~/.local/share/mcf/provisioned` |
| Control socket | `/run/user/<uid>/mcf/control.sock` |
| Settings | Editable from any of the three interfaces — see [configuration.md](doc/configuration.md) |

## 7 · Working on MCF

One command gates every change:

```
scripts/ci.sh
```

It builds, lints and runs the fast hermetic tier — no network, no accelerator,
no model file. Heavier tiers run on a schedule and before a release; they are
described with the rest of the build process in [build.md](doc/build.md).

## 8 · The documents

| [goals.md](doc/goals.md) | What MCF is for, and what it deliberately will not do |
| [hosting.md](doc/hosting.md) | How a model is held, and how engines are obtained |
| [configuration.md](doc/configuration.md) | Every setting, where it lives, and how to change it |
| [monitoring.md](doc/monitoring.md) | The figures MCF reports while a model is held |
| [statistics.md](doc/statistics.md) | The record of real use that replaces diagnostics |
| [build.md](doc/build.md) | Building, the test tiers, and how a release is made |
| [vendored.md](doc/vendored.md) | Every vendored component, its terms, and the compatibility finding |
| [taxonomy.md](doc/taxonomy.md) | The failure codes, which the code is checked against |

## Licence

GPL-3.0-only. The full text is in [LICENSE](LICENSE), and `mcf licence` states
the terms and what conveying the binary obliges you to.
