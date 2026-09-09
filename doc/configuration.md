# Configuration

Everything MCF runs a model under is meant to be visible and editable. This
document is the list, and it separates what exists today from what is intended.

## The principle

Nothing that affects how a model runs is hidden. Where MCF chooses a value it
shows the value and says it chose it; where you set one it uses yours. A reader
looking at a running model should be able to see every setting in effect and
where each came from, and change any of them.

This includes the model file. Which file is loaded is a setting like any other,
not a fixed property of the session.

## Per-model settings

These are the settings a hold runs under. They can be given to `mcf host` as
flags — see [hosting.md](hosting.md) — and `mcf settings <model>` prints every
setting a model would run under and where each came from.

| Model file | Which file is loaded |
| Context | Window size in tokens |
| Placement | Processor or accelerator |
| Layer offload | How many layers go to the accelerator |
| Threads | Thread count |
| Batch size | Tokens per batch |
| Flash attention | On or off |
| Cache width | How wide each cached token is held |
| Draft head | Use the file's own, where it carries one |
| Rope scaling | Kind and factor, left to the engine unless set |
| Port | Where the endpoint listens |
| API key | Required on the endpoint, or not |

`mcf settings <model> --context <n>` also reports what a given window would
reserve in memory, which is the number worth checking before committing to a
context length. Add `--cache <type>` and it reports the same window at that
width instead.

**Cache width is the setting that moves the others.** Every token of the window
costs the cache width, so holding it narrower fits a longer conversation in the
same memory. The engine takes `f32`, `f16`, `bf16`, `q8_0`, `q5_1`, `q5_0`,
`q4_1`, `q4_0` and `iq4_nl`; MCF holds `f16`, which is what an engine holds
without being asked, and every figure MCF reports is computed at the width the
hold will actually use. A narrow width is held with flash attention on, because
the engine reads it no other way, and MCF turns it on rather than failing at
the point of loading.

## Machine settings

| Electricity price | `MCF_PRICE_PER_KWH` — what a kilowatt-hour costs here |

Set it the way a person writes a price: `0.28`, `.28`, `28`, up to six decimal
places. MCF holds no currency — the number is yours, and it comes back out
beside the energy it priced.

```
export MCF_PRICE_PER_KWH=0.28
```

Unset, MCF reports the energy and no cost. A price it invented would be a
number that looks like money and is not one. An energy MCF modelled or could
not read has no cost either, for the same reason.

## Environment

MCF has no configuration file today. What would be settings are environment
variables and command-line arguments.

| Variable | Read by | Meaning |
|---|---|---|
| `MCF_MODELS` | `pull`, `list`, `host`, `rm`, … | An ordered list of absolute paths, colon-separated. The first is where a new acquisition goes; all are searched for what is held. A relative path is dropped and named rather than resolved against the working directory. |
| `MCF_MEMORY_HEADROOM` | `host`, `explain`, `settings` | The share of a device's free memory MCF will plan to use, as a whole percentage from 1 to 100. Defaults to **100** — MCF does not hold back capacity you have. Lower it if you want room left for other work on the same device. A value outside the range is ignored rather than clamped silently. |
| `XDG_DATA_HOME` | the record, the model store fallback, engines | `$XDG_DATA_HOME/mcf/record.jsonl` for the record, `$XDG_DATA_HOME/mcf/provisioned` for engines. Falls back to `$HOME/.local/share/mcf`. |
| `XDG_RUNTIME_DIR` | `serve`, `status`, `stop` | Where the daemon's control socket lives. |
| `MCF_BUILD_COMMIT` | build time | The source revision stamped into the binary and into every recorded row's conditions. |

To make `MCF_MODELS` persist, set it in a shell profile:

```
export MCF_MODELS=/big/disk/models:/home/you/models
```

If neither `XDG_DATA_HOME` nor `HOME` is set, MCF refuses to invent a location
rather than guessing.

## Editing settings from the interfaces

**Partly built.** The window and the terminal interface show settings. The
intent is that both can edit every setting listed above — including swapping the
model file — and that a change takes effect on the next hold without anyone
touching an environment variable or retyping a flag.

This is a deliberate reversal of an earlier decision. MCF previously refused
itself a configuration language on the grounds that environment variables are
the platform's own idiom. That holds for a tool driven from a shell; it does not
hold for a tool whose primary surface is a window, where "edit your shell
profile and restart" is not an acceptable answer to "change the context
length". A settings store, and the interfaces to edit it, are open work.
