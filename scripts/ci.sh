#!/usr/bin/env bash
#
# The tiered suite: the one command that fails the build, and the flags that
# run the heavy tiers (B38, B19, B-191).
#
# B38 tiers the suite. The fast hermetic tier gates every change; the heavy
# tiers run on a schedule and before a release. Three properties of the gating
# tier are obligations rather than preferences:
#
#   * **Hermetic.** No network, no accelerator, no model file. `--offline` is
#     passed rather than merely expected, so a check that starts reaching out
#     fails here instead of on somebody's aeroplane (B19).
#   * **Fast.** It is run on every change, and a gating tier people skip is a
#     gating tier that does not gate. Its elapsed time is printed at the end:
#     no D24 figure is a suite time, so there is nothing to assert against, and
#     a tier that grew slowly would otherwise become one people skip without
#     anybody noticing when.
#   * **Complete about what it covers.** A check that could not run reports as
#     such and fails, because "did not run" read as "passed" is the silent
#     failure A2 forbids, aimed at the suite (B38).
#
# The tiers are declared in `checks/src/tiers.rs`, and
# `checks/tests/tiers_conform.rs` fails the build when this script and that
# declaration disagree — when a tier's command is missing here, when a flag is
# not accepted, or when a tier that did not run is not reported as such.
#
# Usage:  scripts/ci.sh [--with-<tier>]...  |  scripts/ci.sh --all
#
#   --with-fuzz             (B-191) mutates known-good input into the four
#                           parsers that read bytes MCF did not write.
#   --with-load             (B-191) MCF's claims under many callers at once.
#   --with-soak             (B-191) drift over a long run: descriptors,
#                           directories, memory. One thread: the readings are
#                           properties of the process.
#   --with-budget           (B-011) measures MCF's own cost against D24's
#                           ceilings, in release, because those ceilings are
#                           about the artifact MCF ships (§3.4, D27).
#   --with-mutation         (B-191, B-186) breaks the code deliberately and
#                           reports what the suite failed to notice.
#   --with-from-scratch     (B-183) runs the statically linked artifact in a
#                           container that holds it and nothing else.
#   --with-reproducibility  (B-001) rebuilds the workspace twice under the
#                           release profile and compares the bytes.
#   --all                   all of the above. Minutes, not seconds.
#
# None of them is optional; all of them are scheduled rather than gating,
# because each takes minutes and a gate people skip does not gate.

set -o errexit -o nounset -o pipefail

root=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)
cd "$root"

# shellcheck source=scripts/lib-tiers.sh
. "$root/scripts/lib-tiers.sh"

with_reproducibility=false
with_budget=false
with_fuzz=false
with_load=false
with_soak=false
with_mutation=false
with_from_scratch=false
for argument in "$@"; do
    case "$argument" in
        --with-reproducibility) with_reproducibility=true ;;
        --with-budget) with_budget=true ;;
        --with-fuzz) with_fuzz=true ;;
        --with-load) with_load=true ;;
        --with-soak) with_soak=true ;;
        --with-mutation) with_mutation=true ;;
        --with-from-scratch) with_from_scratch=true ;;
        --all)
            with_reproducibility=true
            with_budget=true
            with_fuzz=true
            with_load=true
            with_soak=true
            with_mutation=true
            with_from_scratch=true
            ;;
        *)
            printf 'ci: no such option: %s\n' "$argument" >&2
            printf 'usage: scripts/ci.sh [--with-fuzz] [--with-load] [--with-soak] ' >&2
            printf '[--with-budget] [--with-mutation]\n' >&2
            printf '                     [--with-from-scratch] [--with-reproducibility] | --all\n' >&2
            exit 2
            ;;
    esac
done

command -v cargo >/dev/null 2>&1 || {
    printf 'cannot run: cargo is not on PATH\n' >&2
    exit 2
}

step() { printf '\n=== %s\n' "$1"; }

started=$SECONDS

step "formatting"
cargo fmt --all -- --check

step "lints (deny warnings)"
cargo clippy --workspace --all-targets --locked --offline -- -D warnings

# The five gating tiers are one command: unit, property, functional,
# whole-system and fault-injection all live in the workspace's own test
# targets, and separating them here would only make it possible to run some of
# them and believe the suite had run.
step "the gating tiers: unit, property, functional, whole-system, fault-injection"
cargo test --workspace --locked --offline

step "the lint denials bite (B-003)"
"$root/scripts/check-lints-bite.sh"

step "the fault signal can fail (B-193)"
# Exit 2 is *this machine cannot demonstrate it* — a tmpfs has no device to
# fault from, and some filesystems ignore the eviction hint. That is reported
# and does not fail the gate; exit 1 is the signal being wrong, and does.
fault_signal=0
"$root/scripts/check-fault-signal.sh" || fault_signal=$?
if [ "$fault_signal" -eq 1 ]; then
    printf 'ci: the fault signal does not distinguish a cold artifact from a warm one\n' >&2
    exit 1
fi

step "documentation builds"
RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps --locked --offline >/dev/null

gating_seconds=$((SECONDS - started))

if [ "$with_fuzz" = true ]; then
    step "fuzz (B-191)"
    cargo test --locked --offline -p mcf-checks --test fuzz -- --ignored --nocapture
    tier_stamp "$root" fuzz
fi

if [ "$with_load" = true ]; then
    step "load (B-191)"
    cargo test --locked --offline -p mcf-checks --test load -- --ignored --nocapture
    tier_stamp "$root" load
fi

if [ "$with_soak" = true ]; then
    # One thread: resident memory and open descriptors are properties of the
    # process, so a second test allocating in parallel reads as growth.
    step "soak (B-191)"
    cargo test --locked --offline -p mcf-checks --test soak -- --ignored --nocapture --test-threads=1
    tier_stamp "$root" soak
fi

if [ "$with_budget" = true ]; then
    step "performance budget (B-011, release profile)"
    # Release, because D24's ceilings are about the shipped artifact and a debug
    # binary is a different one. `--ignored` because the tier is scheduled.
    cargo test --release --locked --offline -p mcf-cli --test budget -- --ignored --nocapture
    tier_stamp "$root" performance
fi

if [ "$with_mutation" = true ]; then
    step "mutation (B-191; the floor is B-186)"
    # The script itself refuses a score below the floor or below the last one
    # recorded (B-186); the score travels into the stamp so that the next run
    # has a previous one to compare against, which is B20's before and after.
    mutation_output=$("$root/scripts/check-mutants.sh" | tee /dev/stderr)
    tier_stamp "$root" mutation \
        "$(printf '%s' "$mutation_output" | grep '^mutation score' || printf 'score not reported')"
fi

if [ "$with_from_scratch" = true ]; then
    step "from-scratch conformance (B-183)"
    "$root/scripts/check-from-scratch.sh"
fi

if [ "$with_reproducibility" = true ]; then
    step "reproducible build (B-001)"
    "$root/scripts/check-reproducible-build.sh"
fi

# Every scheduled tier's age, and what did not run in this invocation. B38: a
# tier that has not run is reported as such, never assumed green, and an age is
# the stronger form of the same statement — `scripts/check-tier-ages.sh
# --release` is what refuses on one (B-185).
printf '\n=== the scheduled tiers\n'
"$root/scripts/check-tier-ages.sh" | sed -n '3,$p' | sed '/^$/,$d'

printf '\n=== not run in this invocation\n'
not_run=false
report_absent() {
    # `if` rather than `[ … ] && printf`: under `errexit` a false test at the
    # end of a && list is a failing command, and a script that exited 1 because
    # every tier *did* run would be a gate that punished thoroughness.
    if [ "$1" = false ]; then
        printf '  %s\n' "$2"
        not_run=true
    fi
}
report_absent "$with_fuzz" "fuzz (B-191)                 — scripts/ci.sh --with-fuzz"
report_absent "$with_load" "load (B-191)                 — scripts/ci.sh --with-load"
report_absent "$with_soak" "soak (B-191)                 — scripts/ci.sh --with-soak"
report_absent "$with_budget" "performance budget (B-011)   — scripts/ci.sh --with-budget"
report_absent "$with_mutation" "mutation (B-191)             — scripts/ci.sh --with-mutation"
report_absent "$with_from_scratch" "from-scratch conformance     — scripts/ci.sh --with-from-scratch"
report_absent "$with_reproducibility" "reproducible build (B-001)   — scripts/ci.sh --with-reproducibility"
if [ "$not_run" = false ]; then
    printf '  nothing: every tier ran in this invocation\n'
fi

printf '\nci: green — the gating tiers took %ds\n' "$gating_seconds"
