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
| Draft head | Use the file's own, where it carries one |
| Rope scaling | Kind and factor, left to the engine unless set |
| Port | Where the endpoint listens |
| API key | Required on the endpoint, or not |

`mcf settings <model> --context <n>` also reports what a given window would
reserve in memory, which is the number worth checking before committing to a
context length.

## Machine settings

| Electricity price | Cost per kilowatt-hour, used to turn watts into money |

**Not implemented.** This is the setting [monitoring.md](monitoring.md)
describes as missing. Power is measured today; the price that converts it to a
running cost is not yet stored anywhere or editable.

## Environment

MCF has no configuration file today. What would be settings are environment
variables and command-line arguments.

| Variable | Read by | Meaning |
|---|---|---|
| `MCF_MODELS` | `pull`, `list`, `host`, `rm`, … | An ordered list of absolute paths, colon-separated. The first is where a new acquisition goes; all are searched for what is held. A relative path is dropped and named rather than resolved against the working directory. |
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
