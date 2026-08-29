#!/usr/bin/env bash
#
# The seed set is shown to be representative, not assumed to be (B-291, D19,
# §6.16, §7.13).
#
# **What this is for.** D19 gives MCF a published seed set — trial *i* draws
# seed *i* — and then puts §6.16's discipline on it: *the instrument does not
# get to grade itself.* So the set is validated rather than trusted:
# periodically, a larger draw is run and its distribution compared with the
# fixed set's. Divergence means the set is unrepresentative, is replaced, and
# the replacement is recorded as a break in comparability (§7.13).
#
# **What runs.** `checks/tests/seed_set.rs`, in release, over a small model on
# this machine. Thirty-two seeds from the head of the published stream against
# three hundred and twenty from a million indices further along — two draws
# from one space, which is what *is the prefix representative of the stream*
# means. The per-trial outcome is how many distinct tokens the generation used,
# which is a behaviour statistic and never a speed (B65).
#
# **It samples with nucleus, and it has to.** MCF's shipped generation is
# greedy and greedy ignores the seed: every seed produces the same tokens. A
# validation against it would compare a constant with a constant and clear the
# set for a reason that has nothing to do with the set.
#
# **Scheduled rather than gating**, because it needs a model on the disk and
# runs hundreds of generations. `scripts/ci.sh --with-seed-set` runs it.
#
# Exit status: 0 when the set is shown representative; 1 when it is not, or
# when the run could not decide — *not decided* is not clearance; 2 when the
# check could not be made at all, which is no model on this machine.

set -o errexit -o nounset -o pipefail

readonly EXIT_FAILED=1
readonly EXIT_CANNOT_CHECK=2

root=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)
cd "$root"

fail_cannot_check() {
    printf 'cannot check: %s\n' "$1" >&2
    exit "$EXIT_CANNOT_CHECK"
}

# Which model. `MCF_SEED_SET_MODEL` names one outright; otherwise the smallest
# file in the store MCF itself would use, because the tier's cost is hundreds
# of generations and the question is about the seeds rather than about the
# model. Not invented: a check that guessed at a path would be a check that
# silently tested nothing (A7).
model=${MCF_SEED_SET_MODEL:-}
if [ -z "$model" ]; then
    data=${XDG_DATA_HOME:-${HOME:-}/.local/share}
    store=${MCF_CORPUS:-$data/mcf/models}
    [ -d "$store" ] || fail_cannot_check "no models at $store — \`mcf pull\` one, or set MCF_SEED_SET_MODEL"
    model=$(find "$store" -name '*.gguf' -type f -printf '%s\t%p\n' 2>/dev/null | sort -n | head -1 | cut -f2)
    [ -n "$model" ] || fail_cannot_check "no .gguf under $store — \`mcf pull\` one, or set MCF_SEED_SET_MODEL"
fi
[ -f "$model" ] || fail_cannot_check "no model at $model"

printf 'the seed set, against %s\n\n' "$model"

# Release, because hundreds of generations in a debug build is an hour rather
# than minutes. Nothing here is timed, so the profile is a cost and not a
# condition of the result.
if MCF_SEED_SET_MODEL="$model" cargo test --locked --offline --release \
    -p mcf-checks --test seed_set -- --ignored --nocapture --test-threads=1; then
    exit 0
fi
printf '\nthe published seed set was not shown representative of the stream it is a\n' >&2
printf 'prefix of. D19: replace the set, and record the replacement as a break in\n' >&2
printf 'comparability (§7.13). A run that could not decide is also a failure here,\n' >&2
printf 'because *not decided* is not clearance (§6.16).\n' >&2
exit "$EXIT_FAILED"
