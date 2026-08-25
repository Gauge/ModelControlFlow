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
# **What it establishes and what it does not.** It establishes that MCF's
# *product* needs nothing from the machine: `--version`, `licence` and the whole
# of `doctor` — the hardware profile, the self-cost measurement, the laboratory
# — run with nothing installed. It does not reach a first token, which needs a
# model artifact, which is M1's acquisition work (B-019, B-020). B-183's
# condition is not met until that exists, and this script says so rather than
# implying otherwise.
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

printf 'building the statically linked artifact (%s)\n' "$TARGET"
cargo build --release --locked --offline --target "$TARGET" -p mcf-cli >/dev/null

binary="$root/target/$TARGET/release/mcf"
[ -x "$binary" ] || fail_cannot_check "the build produced no binary at $binary"

workdir=$(mktemp -d "${TMPDIR:-/tmp}/mcf-scratch-XXXXXX")
# A27: the image and the directory are both put back, whichever way this exits.
trap 'rm -rf "$workdir"; "$engine" rmi -f "$IMAGE" >/dev/null 2>&1 || true' EXIT

cp "$binary" "$workdir/mcf"
cat >"$workdir/Containerfile" <<'CONTAINERFILE'
FROM scratch
COPY mcf /mcf
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

if [ "${#failures[@]}" -gt 0 ]; then
    printf '\n%d thing(s) MCF claims to do on a bare machine did not: %s\n' \
        "${#failures[@]}" "${failures[*]}" >&2
    exit "$EXIT_FAILED"
fi

printf '\nMCF runs with nothing installed: no libc, no shell, no package manager,\n'
printf 'no /etc, no /tmp (B36, §XVI).\n\n'
printf 'What this does not reach is a first token, which needs a model artifact —\n'
printf 'M1 acquires one (B-019, B-020), and B-183 is not done until it does.\n'
