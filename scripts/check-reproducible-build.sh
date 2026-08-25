#!/usr/bin/env bash
#
# B-001's own acceptance condition: `cargo build --locked` reproduces
# byte-identically from a clean checkout on the pinned toolchain.
#
# §3.12 makes reproducibility a precedence rule (P3), and an unchecked claim of
# reproducibility is an untested claim (A19). This script is the check. It
# takes two checkouts of the committed tree, at deliberately different paths
# and with deliberately different environments, builds both with the pinned
# toolchain, and compares the bytes.
#
# The two builds differ in every way a build is allowed to differ and still be
# the same build:
#
#   * the path of the source tree, and its length, which is what
#     `--remap-path-prefix` exists to defeat when it is needed;
#   * the target directory;
#   * `SOURCE_DATE_EPOCH`, so a build that embeds a timestamp is caught;
#   * `TZ` and the umask, which reach a build through file metadata.
#
# What is deliberately held still is the toolchain (rust-toolchain.toml), the
# lockfile (`--locked`) and `MCF_BUILD_COMMIT` — those are conditions of the
# artifact, not incidental facts about the machine, and a build that changed
# when they changed would be correct to change.
#
# Exit status is 0 when the artifacts agree, 1 when they do not, and 2 when the
# check could not be run. The three are distinct because "could not check" is
# not "checked and passed" (A2).

set -o errexit -o nounset -o pipefail

readonly EXIT_DIVERGED=1
readonly EXIT_CANNOT_CHECK=2

root=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)
cd "$root"

fail_cannot_check() {
    printf 'cannot check: %s\n' "$1" >&2
    exit "$EXIT_CANNOT_CHECK"
}

command -v git >/dev/null 2>&1 || fail_cannot_check "git is not on PATH"
command -v cargo >/dev/null 2>&1 || fail_cannot_check "cargo is not on PATH"
command -v sha256sum >/dev/null 2>&1 || fail_cannot_check "sha256sum is not on PATH"
git rev-parse --git-dir >/dev/null 2>&1 || fail_cannot_check "not a git repository"

# The check compares two checkouts of what is *committed*. Uncommitted work is
# not what a third party would build, so checking it would answer a question
# nobody asked.
if ! git diff --quiet HEAD -- . ':!target' || [ -n "$(git ls-files --others --exclude-standard)" ]; then
    fail_cannot_check "the working tree has uncommitted changes; commit them first"
fi

revision=$(git rev-parse HEAD)
workdir=$(mktemp -d "${TMPDIR:-/tmp}/mcf-repro-XXXXXX")
# A27's habit: what this script creates, it removes, including on the way out
# of a failure.
trap 'rm -rf "$workdir"' EXIT

# Two source paths of different lengths, so an embedded absolute path shows up
# as a difference rather than cancelling out.
declare -a names=("a" "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb")
declare -a digests=()

for index in 0 1; do
    src="$workdir/${names[$index]}/mcf"
    mkdir -p "$src"
    git archive "$revision" | tar -x -C "$src"

    printf 'build %d: %s\n' "$((index + 1))" "$src"
    (
        cd "$src"
        export CARGO_TARGET_DIR="$workdir/target-$index"
        export MCF_BUILD_COMMIT="$revision"
        # Deliberately different, and deliberately not conditions of the build.
        if [ "$index" -eq 0 ]; then
            export SOURCE_DATE_EPOCH=1000000000 TZ=UTC
            umask 022
        else
            export SOURCE_DATE_EPOCH=1700000000 TZ=Pacific/Kiritimati
            umask 077
        fi
        cargo build --locked --offline --release --bin mcf >/dev/null
    )

    artifact="$workdir/target-$index/release/mcf"
    [ -f "$artifact" ] || fail_cannot_check "build $((index + 1)) produced no artifact at $artifact"
    digests+=("$(sha256sum <"$artifact" | cut -d' ' -f1)")
done

printf '\nrevision   %s\n' "$revision"
printf 'toolchain  %s\n' "$(cargo --version)"
printf 'build 1    sha256:%s\n' "${digests[0]}"
printf 'build 2    sha256:%s\n' "${digests[1]}"

if [ "${digests[0]}" = "${digests[1]}" ]; then
    printf '\nreproducible: the two builds are byte-identical (B-001)\n'
    exit 0
fi

printf '\nNOT reproducible: the two builds differ (B-001)\n' >&2
printf 'The difference is a finding about the build, not a tolerance to widen.\n' >&2
exit "$EXIT_DIVERGED"
