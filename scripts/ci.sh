#!/usr/bin/env bash
#
# The gating tier: the one command that fails the build (B38, B19).
#
# B38 tiers the suite, and this is the fast hermetic tier that gates every
# change. Three properties are obligations rather than preferences:
#
#   * **Hermetic.** No network, no accelerator, no model file. `--offline` is
#     passed rather than merely expected, so a check that starts reaching out
#     fails here instead of on somebody's aeroplane (B19).
#   * **Fast.** It is run on every change, and a gating tier people skip is a
#     gating tier that does not gate.
#   * **Complete about what it covers.** A check that could not run reports as
#     such and fails, because "did not run" read as "passed" is the silent
#     failure A2 forbids, aimed at the suite (B38).
#
# The heavy tiers B38 also requires — load, soak, mutation, the full fault
# matrix — do not exist yet; B-191 builds them and B-185 publishes their ages.
# They are named here so their absence is visible rather than assumed.
#
# Usage:  scripts/ci.sh [--with-reproducibility]
#
# The reproducibility check (B-001) rebuilds the workspace twice under the
# release profile and takes minutes, so it is out of the gating tier by
# default and run before a release and on demand. It is not optional; it is
# scheduled.

set -o errexit -o nounset -o pipefail

root=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)
cd "$root"

with_reproducibility=false
for argument in "$@"; do
    case "$argument" in
        --with-reproducibility) with_reproducibility=true ;;
        *)
            printf 'ci: no such option: %s\n' "$argument" >&2
            printf 'usage: scripts/ci.sh [--with-reproducibility]\n' >&2
            exit 2
            ;;
    esac
done

command -v cargo >/dev/null 2>&1 || {
    printf 'cannot run: cargo is not on PATH\n' >&2
    exit 2
}

step() { printf '\n=== %s\n' "$1"; }

step "formatting"
cargo fmt --all -- --check

step "lints (deny warnings)"
cargo clippy --workspace --all-targets --locked --offline -- -D warnings

step "unit and integration tests"
cargo test --workspace --locked --offline

step "documentation builds"
RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps --locked --offline >/dev/null

if [ "$with_reproducibility" = true ]; then
    step "reproducible build (B-001)"
    "$root/scripts/check-reproducible-build.sh"
else
    printf '\n=== not run in this tier\n'
    printf '  reproducible build (B-001)  — scripts/ci.sh --with-reproducibility\n'
    printf '  load, soak, mutation, fault matrix (B38) — not built yet: B-191, B-185, B-186\n'
fi

printf '\nci: green\n'
