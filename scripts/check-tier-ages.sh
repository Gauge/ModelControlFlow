#!/usr/bin/env bash
#
# Every tier's age, and the refusal a stale one earns before a release
# (B-185, B38).
#
# B38: *a heavy tier that has not run recently is reported as **stale**, never
# assumed green — an unstated staleness is A2's silent failure aimed at the
# suite.* The gating tiers run on every change and need no register of ages.
# The scheduled ones do: nothing about a green `scripts/ci.sh` says whether the
# soak tier has ever run against this code.
#
# Run with no arguments it reports. Run with `--release` it *refuses*: a
# scheduled tier that has never run against this source, or that ran against
# different source, exits non-zero with the age and the reason stated.
#
# What makes a tier stale is in `scripts/lib-tiers.sh`, with the reasoning: the
# source it ran against is not the source in the tree. A wall-clock age is
# reported beside that and is not the verdict, because no clause of intent
# states how old a soak result may be and inventing one is how a project
# acquires intent nobody chose (A23).
#
# Exit status: 0 when nothing was refused, 1 when a release is refused, 2 when
# the check could not be made.

set -o errexit -o nounset -o pipefail

readonly EXIT_STALE=1
readonly EXIT_CANNOT_CHECK=2

root=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)
# shellcheck source=scripts/lib-tiers.sh
. "$root/scripts/lib-tiers.sh"

refuse=false
for argument in "$@"; do
    case "$argument" in
        --release) refuse=true ;;
        *)
            printf 'usage: scripts/check-tier-ages.sh [--release]\n' >&2
            exit "$EXIT_CANNOT_CHECK"
            ;;
    esac
done

# The scheduled tiers, and the flag that runs each. This list is compared
# against `checks/src/tiers.rs` by `checks/tests/tiers_conform.rs`, so a tier
# added to the register and not to this file fails the gating suite rather than
# quietly having no age.
declare -a scheduled=(fuzz load soak performance mutation)
declare -a flags=(--with-fuzz --with-load --with-soak --with-budget --with-mutation)

current=$(tier_source_digest "$root")
if [ "$current" = "unknown" ]; then
    printf 'cannot check: sha256sum is not on PATH, so "the same source" cannot be decided\n' >&2
    exit "$EXIT_CANNOT_CHECK"
fi

printf 'source %s\n\n' "$current"

stale=()
for index in "${!scheduled[@]}"; do
    tier="${scheduled[$index]}"
    flag="${flags[$index]}"
    at=$(tier_stamp_field "$root" "$tier" at)
    epoch=$(tier_stamp_field "$root" "$tier" epoch)
    source_seen=$(tier_stamp_field "$root" "$tier" source)
    detail=$(tier_stamp_field "$root" "$tier" detail)

    if [ -z "$at" ]; then
        printf '  %-12s never run against any source\n' "$tier"
        stale+=("$tier ($flag): never run")
        continue
    fi

    age=$(tier_age_in_words "${epoch:-0}")
    if [ "$source_seen" = "$current" ]; then
        printf '  %-12s %s, on this source' "$tier" "$age"
    else
        printf '  %-12s %s, on OTHER source (%s)' "$tier" "$age" "${source_seen:0:12}"
        stale+=("$tier ($flag): last ran $age, against source ${source_seen:0:12}")
    fi
    if [ -n "$detail" ]; then
        printf ' — %s' "$detail"
    fi
    printf '\n'
done

if [ "${#stale[@]}" -eq 0 ]; then
    printf '\nevery scheduled tier has run against this source\n'
    exit 0
fi

printf '\n%d tier(s) have not run against this source:\n' "${#stale[@]}"
for entry in "${stale[@]}"; do
    printf '  %s\n' "$entry"
done

if [ "$refuse" = true ]; then
    printf '\nrefused: a release does not go out on a tier that has not run against\n' >&2
    printf 'the code it is a release of (B38, B-185). Run the flags above.\n' >&2
    exit "$EXIT_STALE"
fi

printf '\nreported, not refused. `--release` refuses (B38).\n'
