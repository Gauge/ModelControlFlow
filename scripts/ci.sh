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
#   --with-corpus           (B-370) runs the conformance corpus through the engine
#   --with-seed-set         (B-291) shows the published seed set representative
#                           of the stream it is a prefix of, rather than
#                           assuming it
#   --with-oracle           (B-368) compares MCF's engine against a reference
#   --with-online           (B-029) acquires a real model from the real hub
#                           over TLS, verifies it, lists it and removes it.
#                           The only thing here that needs a network.
#   --all                   all of the above. Minutes, not seconds.
#
# None of them is optional; all of them are scheduled rather than gating,
# because each takes minutes and a gate people skip does not gate.
#
# THE MACHINE IS SHARED. Where `heavy` is on PATH, every scheduled tier runs
# inside an exclusive window: this machine hosts several projects with heavy
# test workloads, and four suites running at once do not run four times slower —
# they measure each other. B35 says a timing taken under contention measures the
# contention, and D30 makes MCF *refuse* such a reading rather than report it,
# so the budget tier is not merely slower without the window, it declines to
# assert. The gating tier does not take the window: it is seconds long, and a
# five-second check queued behind a five-minute mutation run is a check people
# stop running. `~/.local/bin/HEAVY.md` states the rest.

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
with_corpus=false
with_seed_set=false
with_oracle=false
with_online=false
for argument in "$@"; do
    case "$argument" in
        --with-reproducibility) with_reproducibility=true ;;
        --with-budget) with_budget=true ;;
        --with-fuzz) with_fuzz=true ;;
        --with-load) with_load=true ;;
        --with-soak) with_soak=true ;;
        --with-mutation) with_mutation=true ;;
        --with-from-scratch) with_from_scratch=true ;;
        --with-corpus) with_corpus=true ;;
        --with-seed-set) with_seed_set=true ;;
        --with-oracle) with_oracle=true ;;
        --with-online) with_online=true ;;
        --all)
            with_reproducibility=true
            with_budget=true
            with_fuzz=true
            with_load=true
            with_soak=true
            with_mutation=true
            with_from_scratch=true
            with_corpus=true
            with_seed_set=true
            with_oracle=true
            with_online=true
            ;;
        *)
            printf 'ci: no such option: %s\n' "$argument" >&2
            printf 'usage: scripts/ci.sh [--with-fuzz] [--with-load] [--with-soak] ' >&2
            printf '[--with-budget] [--with-mutation]\n' >&2
            printf '                     [--with-from-scratch] [--with-reproducibility] ' >&2
            printf '[--with-corpus] [--with-seed-set] [--with-oracle] ' >&2
            printf '[--with-online] | --all\n' >&2
            exit 2
            ;;
    esac
done

command -v cargo >/dev/null 2>&1 || {
    printf 'cannot run: cargo is not on PATH\n' >&2
    exit 2
}

step() { printf '\n=== %s\n' "$1"; }

# Runs a scheduled tier inside an exclusive window where one is available, and
# plainly without one where it is not — a machine with no `heavy` on it is a
# machine with one project on it, and MCF is not going to require a tool it does
# not ship (B36's habit, applied to a developer's machine rather than a user's).
#
# `--minutes` is per tier and generous: it is a deadline that ends a hung run,
# not an estimate of how long the tier takes.
#
# **A window that never comes is not a failed tier, and not a passed one.** The
# machine MCF is developed on is shared, and a project can hold the window for
# twelve hours ([build.md](../doc/build.md) section 12). Before this, the first
# tier to time out ended the whole run under `errexit` — so the tiers after it
# never ran, nothing said which, and the output stopped mid-sentence. That is
# the silent partial A4 forbids, in MCF's own build script. Now it is recorded
# and named at the end, and the tier's age is left stale, which is what
# `check-tier-ages.sh --release` refuses on (B38, B-185).
#
# How long to wait is `MCF_WINDOW_WAIT_SECONDS`, because how long is worth
# waiting is a property of the machine rather than of the tier: on a machine
# nobody else uses it is irrelevant, and on this one an overnight run wants
# hours.
readonly WINDOW_WAIT_SECONDS="${MCF_WINDOW_WAIT_SECONDS:-1800}"
readonly EX_TEMPFAIL=75
declare -a no_window=()

exclusively() {
    local what="$1" minutes="$2"
    shift 2
    if ! command -v heavy >/dev/null 2>&1; then
        "$@"
        return
    fi
    local status=0
    heavy run --for "mcf: $what" --minutes "$minutes" --wait "$WINDOW_WAIT_SECONDS" -- "$@" ||
        status=$?
    if [ "$status" -eq "$EX_TEMPFAIL" ]; then
        printf '  %s did not run: the exclusive window was not free within %ss\n' \
            "$what" "$WINDOW_WAIT_SECONDS"
        no_window+=("$what")
        return 0
    fi
    return "$status"
}

started=$SECONDS

step "formatting"
cargo fmt --all -- --check

# Paths do not travel into the artifact (B-001, §3.12, §XIV). A vendored crate
# is a path source, so without this rustc writes this machine's directory into
# every panic location it compiles — and two checkouts at different paths stop
# producing the same bytes, which is what `check-reproducible-build.sh` found
# the day the first dependency was admitted.
export RUSTFLAGS="${RUSTFLAGS:-}${RUSTFLAGS:+ }--remap-path-prefix=$root=."

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

step "the vendored tree's terms (B-330)"
# Cheap — it reads manifests — and it gates, because a dependency arriving under
# terms nobody looked at is the failure doc/vendored.md exists to prevent.
"$root/scripts/check-vendored-terms.sh"

step "documentation builds"
RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps --locked --offline >/dev/null

gating_seconds=$((SECONDS - started))

if [ "$with_fuzz" = true ]; then
    step "fuzz (B-191)"
    exclusively "fuzz tier" 10 \
        cargo test --locked --offline -p mcf-checks --test fuzz -- --ignored --nocapture
    tier_stamp "$root" fuzz
fi

if [ "$with_load" = true ]; then
    step "load (B-191)"
    exclusively "load tier" 10 \
        cargo test --locked --offline -p mcf-checks --test load -- --ignored --nocapture
    tier_stamp "$root" load
fi

if [ "$with_soak" = true ]; then
    # One thread: resident memory and open descriptors are properties of the
    # process, so a second test allocating in parallel reads as growth.
    step "soak (B-191)"
    exclusively "soak tier" 15 \
        cargo test --locked --offline -p mcf-checks --test soak -- --ignored --nocapture --test-threads=1
    tier_stamp "$root" soak
fi

if [ "$with_budget" = true ]; then
    step "performance budget (B-011, release profile)"
    # Release, because D24's ceilings are about the shipped artifact and a debug
    # binary is a different one. `--ignored` because the tier is scheduled.
    # This one needs the window most: every figure it asserts is a timing, and
    # D30 refuses a reading taken while something else had the processor.
    exclusively "performance budget" 20 \
        cargo test --release --locked --offline -p mcf-cli --test budget -- --ignored --nocapture
    # What the record costs as it grows (B-300, F14). It asserts only that the
    # index and the journal agree; the figures are read by a person deciding
    # whether the index still earns its bytes, which is why it prints them and
    # gates on none of them (A18).
    exclusively "how the record grows" 10 \
        cargo test --release --locked --offline -p mcf-record --test how_the_record_grows \
        -- --ignored --nocapture
    tier_stamp "$root" performance
fi

if [ "$with_mutation" = true ]; then
    step "mutation (B-191; the floor is B-186)"
    # The script itself refuses a score below the floor or below the last one
    # recorded (B-186); the score travels into the stamp so that the next run
    # has a previous one to compare against, which is B20's before and after.
    mutation_output=$(exclusively "mutation tier" 45 "$root/scripts/check-mutants.sh" | tee /dev/stderr)
    tier_stamp "$root" mutation \
        "$(printf '%s' "$mutation_output" | grep '^mutation score' || printf 'score not reported')"
fi

if [ "$with_corpus" = true ]; then
    step "the conformance corpus (B-370)"
    # No exclusive window: nothing here is timed and nothing here may be
    # (B65). What it costs is a minute of one processor, and a tier that took
    # the window to produce no number would be taking it from work that has one.
    corpus=0
    "$root/scripts/check-corpus.sh" || corpus=$?
    if [ "$corpus" -eq 1 ]; then
        printf 'ci: the conformance corpus did not do what the register says\n' >&2
        exit 1
    fi
fi

if [ "$with_seed_set" = true ]; then
    step "the seed set is representative (B-291)"
    # No exclusive window: nothing here is timed and nothing here may be
    # (B65). The quantity is a count of distinct tokens, which is a behaviour
    # statistic, and a tier that took the window to produce no number would be
    # taking it from work that has one.
    seed_set=0
    "$root/scripts/check-seed-set.sh" || seed_set=$?
    if [ "$seed_set" -eq 1 ]; then
        printf 'ci: the published seed set was not shown representative (B-291, D19)\n' >&2
        exit 1
    fi
fi

if [ "$with_oracle" = true ]; then
    step "against a reference implementation (B-368)"
    # No exclusive window: what runs here is two tokenizers over six short
    # strings, and nothing is timed. Building the reference is the heavy part,
    # and this check does not build it.
    oracle=0
    "$root/scripts/check-oracle.sh" || oracle=$?
    if [ "$oracle" -eq 1 ]; then
        printf 'ci: MCF and the reference implementation disagree\n' >&2
        exit 1
    fi
fi

if [ "$with_online" = true ]; then
    step "the real hub (B-029)"
    # No exclusive window: nothing here is timed, and what it waits for is
    # somebody else's server rather than this machine's processor.
    online=0
    "$root/scripts/check-online.sh" || online=$?
    if [ "$online" -eq 1 ]; then
        printf 'ci: MCF did not acquire a model from the real hub\n' >&2
        exit 1
    fi
fi

if [ "$with_from_scratch" = true ]; then
    step "from-scratch conformance (B-183)"
    exclusively "from-scratch conformance" 15 "$root/scripts/check-from-scratch.sh"
fi

if [ "$with_reproducibility" = true ]; then
    step "reproducible build (B-001)"
    exclusively "reproducible build" 30 "$root/scripts/check-reproducible-build.sh"
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
report_absent "$with_corpus" "conformance corpus (B-370)   — scripts/ci.sh --with-corpus"
report_absent "$with_seed_set" "the seed set (B-291)         — scripts/ci.sh --with-seed-set"
report_absent "$with_oracle" "against a reference (B-368)  — scripts/ci.sh --with-oracle"
report_absent "$with_online" "the real hub (B-029)         — scripts/ci.sh --with-online"
report_absent "$with_reproducibility" "reproducible build (B-001)   — scripts/ci.sh --with-reproducibility"
if [ "$not_run" = false ]; then
    printf '  nothing: every tier ran in this invocation\n'
fi

if [ "${#no_window[@]}" -gt 0 ]; then
    printf '\n=== asked for and could not run\n'
    for what in "${no_window[@]}"; do
        printf '  %s — the exclusive window was not free within %ss\n' "$what" "$WINDOW_WAIT_SECONDS"
    done
    printf '  Somebody else has the machine. These tiers are as stale as they were,\n'
    printf '  which `scripts/check-tier-ages.sh --release` refuses on (B38, B-185).\n'
    printf '  `heavy status` says who holds it; MCF_WINDOW_WAIT_SECONDS says how long\n'
    printf '  this run is willing to queue.\n'
fi

# The last line is the one people read, so it says both things or neither: a
# run that could not get the machine is not the same as one where everything
# asked for ran, and "green" on its own would be read as the second (A4).
if [ "${#no_window[@]}" -gt 0 ]; then
    printf '\nci: the gating tiers are green in %ds; %d scheduled tier(s) could not get the machine\n' \
        "$gating_seconds" "${#no_window[@]}"
else
    printf '\nci: green — the gating tiers took %ds\n' "$gating_seconds"
fi
