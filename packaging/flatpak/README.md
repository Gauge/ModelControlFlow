# The Flatpak

One file that installs MCF on another Linux machine: the window, the console,
the daemon, MCF's own engine, and the reference engine (llama.cpp, at the
commit MCF provisions, with the same configure flags). B-448. It is not
published anywhere; DEC-032 — how MCF is distributed at all — is still open,
and this is the artifact that lets the question be tried rather than argued.

## Building it

```
scripts/flatpak.sh
```

writes `dist/ModelControlFlow-<version>.flatpak` and a `.sha256` beside it.
With `flatpak-builder` on the host it uses that; without, it runs the same
steps inside the Fedora image MCF pins for its engines, under rootless podman
with `--privileged` (flatpak-builder sandboxes each module with bwrap, and a
user namespace inside a user namespace is allowed nowhere else). The two
freedesktop 24.08 runtimes (~1.5 GB) and the builder's cache persist under
`~/.cache/mcf-flatpak`; the first build downloads them, the next does not.
Every source is pinned in the manifest — the Rust 1.98.0 tarball by checksum,
SDL3 and llama.cpp by commit, MCF's dependencies from the vendored tree — so
what the bundle carries is what the manifest says.

## Installing it elsewhere

```
flatpak remote-add --user --if-not-exists flathub https://dl.flathub.org/repo/flathub.flatpakrepo
flatpak install --user ./ModelControlFlow-0.1.0-m0.flatpak
```

The first line is needed once: the bundle depends on
`org.freedesktop.Platform//24.08`, which is fetched from Flathub at install
time. Then:

```
flatpak run io.github.gauge.ModelControlFlow            # the window (`mcf desk`)
flatpak run io.github.gauge.ModelControlFlow doctor     # any mcf command
flatpak run io.github.gauge.ModelControlFlow pull unsloth/Qwen3-VL-2B-Instruct-GGUF:Qwen3-VL-2B-Instruct-UD-Q4_K_XL.gguf
flatpak run io.github.gauge.ModelControlFlow stop --because "done for today"
```

A shell alias — `alias mcf='flatpak run io.github.gauge.ModelControlFlow'` —
makes the commands read as they do in the documents.

## What to expect

- **Where things go.** The sandbox's data home is
  `~/.var/app/io.github.gauge.ModelControlFlow/data/mcf`: the record, the
  model store, the addressing on file. A store already at
  `~/.local/share/mcf/models` is not seen unless it is granted and named:
  `flatpak override --user --filesystem=~/.local/share/mcf:ro
  --env=MCF_MODELS=~/.local/share/mcf/models io.github.gauge.ModelControlFlow`.
- **The daemon and the terminal.** The first command that needs the daemon
  starts it, and a `flatpak run` that started it does not return until it
  stops — the sandbox lives as long as anything in it. Every later instance
  finds the same daemon (the control socket is under the directory Flatpak
  shares between instances of one application), so `mcf stop` from another
  terminal ends both.
- **Engines.** The one built in is llama.cpp at `304665fe7ac9`, processor
  only, portable code rather than tuned for the machine — the same as
  `mcf provision llama.cpp` produces, so figures from the two are under one
  condition (§3.4), and its provenance says the Flatpak SDK built it. No
  engine can be provisioned into a container from inside the sandbox (D39):
  `mcf provision` needs podman on the host, and there is none in here.
- **Fonts.** The window rasterises its own text from a font file; it looks in
  the runtime (DejaVu is there) and at the host's fonts, which Flatpak exposes
  under `/run/host/fonts`.
- **The window did not open.** Without a display the window refuses in words
  and exits 1; `mcf tui` is the same screens in a terminal.

## What was tried

Built here inside podman on 2026-09-03 and installed into a fresh Fedora 44
container with nothing but `flatpak` in it: `--version`, `doctor` (a full
machine profile, 8.6 MiB core binary, 4.3 MiB resident), `desk` without a
display (refused in words), `status` from a second instance while the
first's daemon was up (found it, *up for 6 seconds*), `list`, `stop`. No
model was pulled or run inside the sandbox. The bundle is 13 MB; installed,
59.6 MB.
