# Building

How MCF is built, tested and released.

## The toolchain

`rust-toolchain.toml` pins an exact compiler version, not a channel. `rustup`
reads the file and installs the pin on its own; nothing else is needed to
build. The pin and the workspace's `rust-version` are asserted equal, because a
declared minimum nothing has ever built with is an untested claim.

Raising the pin is its own commit. It invalidates byte-identical comparison
with anything built before it.

## Building

```
cargo build --locked                  # debug
cargo build --locked --release        # the shipped artifact
./target/release/mcf --version
```

`revision unknown` in the version string is correct rather than a defect. The
source revision reaches the binary through `MCF_BUILD_COMMIT`, set by whatever
performs the build. MCF does not fill it in by shelling out to `git`, because
that would make `git` a build dependency, and it does not invent a plausible
value when nothing set one. A release build sets it:

```
MCF_BUILD_COMMIT=$(git rev-parse HEAD) cargo build --locked --release
```

**A second artifact.** `x86_64-unknown-linux-musl` links statically — no
interpreter, no libc to resolve on the machine it lands on. The target triple
is part of the conditions every recorded row carries, so this is a different
artifact rather than a different build of the same one.

```
rustup target add x86_64-unknown-linux-musl
cargo build --locked --release --target x86_64-unknown-linux-musl -p mcf-cli
```

**The release profile is part of what is measured.** It aborts on panic, keeps
overflow checks on, and enables fat LTO with one codegen unit. A panic is an
invariant violation the type system was meant to prevent, and unwinding past
one invites catch-and-continue. Overflow checks stay on because an overflow
that wraps is a wrong number.

Cargo forces the test profile to unwind whatever the dev profile says, so the
suite and the shipped artifact genuinely differ in that one respect.

## Installing

```
scripts/install.sh
```

Puts the release binary at `~/.local/bin/mcf`, and beside it the launcher and
icon that put the window in the desktop's application search. It installs what
is in `target/release` and refuses when there is nothing there — it will not
build for you, because what is installed is meant to be the release artifact
this document describes. `--prefix` puts everything under another directory.

## The gating tier

One command gates every change:

```
scripts/ci.sh
```

It builds, lints and runs the fast hermetic tier. Three properties of that tier
are obligations rather than preferences:

- **Hermetic.** No network, no accelerator, no model file. `--offline` is
  passed rather than merely expected, so a check that starts reaching out fails
  here rather than on somebody's aeroplane.
- **Fast.** It runs on every change, and a gating tier people skip does not
  gate.
- **Complete about what it covers.** A check that could not run reports as such
  and fails, because "did not run" read as "passed" is a silent failure aimed
  at the suite.

## The tiers

Ten disciplines, declared in `checks/src/tiers.rs`.
`checks/tests/tiers_conform.rs` fails the build when that declaration, this
document and `scripts/ci.sh` disagree.

The gating tier covers `unit`, `property`, `functional`, `whole-system` and
`fault-injection` — everything hermetic and fast enough to run on every change.

The rest run on a schedule and before a release:

| Tier | Flag | What it does |
|---|---|---|
| `fuzz` | `--with-fuzz` | Mutates known-good input into the parsers that read bytes MCF did not write |
| `load` | `--with-load` | MCF's claims under many callers at once |
| `soak` | `--with-soak` | Drift over a long run: descriptors, directories, memory |
| `performance` | `--with-budget` | MCF's own cost against its stated ceilings, in release |
| `mutation` | `--with-mutation` | Breaks the code deliberately and reports what the suite failed to notice |

```
scripts/ci.sh --with-fuzz
scripts/ci.sh --all          # all of them; minutes, not seconds
```

Beyond the ten disciplines there are checks that need something the gating tier
refuses itself — a real network, a real accelerator, a container. They are run
the same way: `--with-online` acquires a real model over TLS, `--with-corpus`
runs the conformance corpus through the engine, `--with-oracle` compares MCF
against a reference implementation, `--with-from-scratch` runs the static
artifact in a container holding it and nothing else, and
`--with-reproducibility` builds twice and compares the bytes.

None of them is optional. All of them are scheduled rather than gating.

## Ages, and what a release refuses

A green `scripts/ci.sh` says nothing about whether the soak tier ever ran
against this code, so every scheduled tier carries an age.

```
scripts/check-tier-ages.sh              # report
scripts/check-tier-ages.sh --release    # refuse, naming each stale tier
```

**Stale means the source changed, not that a clock advanced.** A maximum age in
days is the obvious design and it needs a number nobody has. What actually
invalidates a result is that the code it was taken against is no longer the
code in the tree, so each tier stamps a digest of the manifests, the toolchain
pin, the crates, the checks and the scripts. The stamps are machine-local: a
fresh checkout says it has run nothing rather than inheriting a result.

## Making a release

1. Run the heavy tiers against the current source — `scripts/ci.sh --all`.
2. `scripts/check-tier-ages.sh --release`, which refuses if anything is stale.
3. Build with `MCF_BUILD_COMMIT` set.
4. `scripts/install.sh`.

Keep uncommitted drafts out of the tree while the mutation tier runs; it
rewrites the source deliberately and anything unsaved is at risk.

## Reproducibility

Two builds of the same source under the release profile produce the same bytes.
`scripts/ci.sh --with-reproducibility` checks it by building twice and
comparing. This is why the toolchain is pinned exactly and why the source
revision arrives through the environment rather than being read from the
working tree.

## Dependencies

The workspace compiles from the standard library alone. What is vendored is
vendored deliberately, with its terms recorded and a compatibility finding
written down — see [vendored.md](vendored.md). `scripts/vendor.sh` maintains
the tree, and `scripts/check-vendored-terms.sh` checks that what is shipped
still matches what is recorded.

## Generated code

The failure taxonomy in [taxonomy.md](taxonomy.md) and
`mcf_core::failure::Category` are one text. `checks/tests/taxonomy_agreement.rs`
asserts the codes are the same list in the same order, with the same meanings
and the same domains. Changing one without the other fails the build.
