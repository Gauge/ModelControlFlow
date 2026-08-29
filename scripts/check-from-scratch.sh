#!/usr/bin/env bash
#
# MCF runs on a machine that has nothing (B-183, B36, §XVI, D29).
#
# B36: *the user obtains MCF and runs it — no runtime, interpreter, toolchain,
# framework or separately-fetched engine.* `crates/mcf-cli/tests/artifact.rs`
# checks that claim by reading the binary's own dependency list; this checks it
# the only way that settles it, by running the binary in a container built
# `FROM scratch` — an image containing the binary and **nothing else**. No
# libc, no shell, no package manager, no `/etc`, no `/tmp`.
#
# **The artifact under test is the statically linked one.** D29 makes a
# per-platform artifact normal and the target triple is already a §3.4
# condition, so `x86_64-unknown-linux-musl` is a second artifact rather than a
# replacement for the glibc one: static-pie, no interpreter, nothing to resolve.
#
# **What it establishes.** That MCF's *product* needs nothing from the machine:
# `--version`, `licence`, the whole of `doctor` — the hardware profile, the
# self-cost measurement, the laboratory — and **a first token**, which is
# B-183's condition. The token is produced by MCF's own stand-in engine (D31)
# from a four-token model the laboratory writes out, because the container has
# no toolchain to build one with and no network to fetch one over: the machine
# that has a toolchain writes the bytes, and the container is handed them.
#
# **What that first token is and is not.** It is the whole path — file,
# vocabulary, forward pass, sampler, tokens, text — running with nothing
# installed, which is exactly what B36 claims. It is not a real model and it is
# never a speed (B65): the model is a fixture with four tokens, and what a real
# artifact does here is B-019's, which needs fifteen gigabytes and somebody's
# decision to spend them.
#
# Exit status: 0 when MCF runs with nothing, 1 when it does not, 2 when the
# check could not be made — no container runtime, or no musl target.

set -o errexit -o nounset -o pipefail

readonly EXIT_FAILED=1
readonly EXIT_CANNOT_CHECK=2
readonly TARGET=x86_64-unknown-linux-musl
readonly IMAGE=mcf-from-scratch:check

root=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)

fail_cannot_check() {
    printf 'cannot check: %s\n' "$1" >&2
    exit "$EXIT_CANNOT_CHECK"
}

command -v cargo >/dev/null 2>&1 || fail_cannot_check "cargo is not on PATH"

engine=""
for candidate in podman docker; do
    if command -v "$candidate" >/dev/null 2>&1; then
        engine="$candidate"
        break
    fi
done
[ -n "$engine" ] || fail_cannot_check "neither podman nor docker is on PATH"

rustup target list --installed 2>/dev/null | grep -qx "$TARGET" ||
    fail_cannot_check "the $TARGET target is not installed (rustup target add $TARGET)"

printf 'writing the laboratory'"'"'s smallest runnable model\n'
fixture_source=$(mktemp -d "${TMPDIR:-/tmp}/mcf-fixture-XXXXXX")
cargo run --release --locked --offline -q -p mcf-lab --example write-fixture -- \
    "$fixture_source/model.gguf" >/dev/null ||
    fail_cannot_check "the fixture model could not be written"

printf 'building the statically linked artifact (%s)\n' "$TARGET"
cargo build --release --locked --offline --target "$TARGET" -p mcf-cli >/dev/null

binary="$root/target/$TARGET/release/mcf"
[ -x "$binary" ] || fail_cannot_check "the build produced no binary at $binary"

workdir=$(mktemp -d "${TMPDIR:-/tmp}/mcf-scratch-XXXXXX")
# A27: the image and the directory are both put back, whichever way this exits.
trap 'rm -rf "$workdir" "$fixture_source"; "$engine" rmi -f "$IMAGE" >/dev/null 2>&1 || true' EXIT

cp "$binary" "$workdir/mcf"
cp "$fixture_source/model.gguf" "$workdir/model.gguf"
cat >"$workdir/Containerfile" <<'CONTAINERFILE'
FROM scratch
COPY mcf /mcf
COPY model.gguf /model.gguf
ENTRYPOINT ["/mcf"]
CONTAINERFILE

printf 'building an image that holds the binary and nothing else\n'
"$engine" build -q -t "$IMAGE" "$workdir" >/dev/null ||
    fail_cannot_check "the container image could not be built"

failures=()

check() {
    local what="$1" phrase="$2"
    shift 2
    local output
    if ! output=$("$engine" run --rm "$IMAGE" "$@" 2>&1); then
        printf '  FAILED   %s: the binary would not run\n' "$what"
        printf '%s\n' "$output" | sed 's/^/           /' | head -5
        failures+=("$what")
        return 0
    fi
    if printf '%s' "$output" | grep -qF "$phrase"; then
        printf '  ran      %s\n' "$what"
    else
        printf '  FAILED   %s: ran and did not say %s\n' "$what" "$phrase"
        failures+=("$what")
    fi
}

printf '\nwith nothing on the machine but the binary:\n'
check "mcf --version" "$TARGET" --version
check "mcf licence" "GPL-3.0-only" licence
check "mcf doctor" "WHAT MCF PROMISES HERE" doctor --no-record
# The laboratory is the interesting one: it writes files, reads a clock and
# reproduces every failure MCF claims to handle, in an image with no `/tmp` to
# start from.
check "the laboratory, inside it" "scenarios" doctor --no-record
# B-183's condition, and the reason this script exists: a first token, on a
# machine with nothing on it. The prompt is a word the fixture's four-token
# vocabulary can represent; the answer is marked as a stand-in's, which is what
# B65 requires of every one of them.
check "a first token" "MARKED" run /model.gguf --prompt yes

if [ "${#failures[@]}" -gt 0 ]; then
    printf '\n%d thing(s) MCF claims to do on a bare machine did not: %s\n' \
        "${#failures[@]}" "${failures[*]}" >&2
    exit "$EXIT_FAILED"
fi

printf '\nMCF runs with nothing installed: no libc, no shell, no package manager,\n'
printf 'no /etc, no /tmp — and reaches a first token there (B-183, B36, §XVI).\n\n'
printf 'The token came from MCF'"'"'s own stand-in (D31) reading a four-token fixture:\n'
printf 'it is the whole path with nothing installed, and it is not a speed (B65).\n'
printf 'What a real artifact does here is B-019, which needs fifteen gigabytes.\n'
