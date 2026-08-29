#!/usr/bin/env bash
#
# The quantization frontier: one model, one machine, many quantizations
# (B-091, §XII, §3.4, §IV).
#
# **What this is.** The cleanest comparison §3.4 admits — one variable across
# many points. Every arm is the same weights, converted to a different
# quantization by the same publisher, run through the same engine on the same
# machine within one sitting. What differs between two arms is the
# quantization, and `mcf explain` reads that from each file's own tensor types
# rather than from its name (A21).
#
# **It is a benchmark, so it cannot fail** (A18, §6.7). Nothing here has a pass
# condition and nothing here gates a change: it is not a step of
# `scripts/ci.sh` and never will be. Its exit status says whether it could
# *run*, not what it found.
#
# **Each point is a comparison, not an absolute.** §3.27 makes the paired
# comparison the durable output and the absolute number the local one, so the
# frontier is built as *n* paired comparisons against one reference arm rather
# than as *n* independent timings put on a chart. Each carries its own
# stopping condition, its own count and its own conditions (F55, F57, B-081).
#
# **`--cold` is not a preference.** F65: the provisioned engine holds one model
# at a time and a paired comparison alternates two, so a warm run comes out
# *mixed* and has no delta to give. Every trial loading the model for itself is
# the uniform state available here, and F64 measured what it includes.
#
# Usage:  scripts/frontier.sh [<reference-quantization>]
#
# Exit status: 0 when the frontier was produced; 2 when it could not be — no
# models, no daemon, no engine that can be timed.

set -o errexit -o nounset -o pipefail

readonly EXIT_CANNOT_CHECK=2

root=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)
cd "$root"

cannot() {
    printf 'cannot produce a frontier: %s\n' "$1" >&2
    exit "$EXIT_CANNOT_CHECK"
}

# What the arms are asked. Held still across every point, because what must
# differ between two arms is the quantization and nothing else (A8).
readonly PROMPT='Once upon a time'
readonly TOKENS=128
readonly RESOLVING=5

reference=${1:-Q8_0}

mcf=$root/target/release/mcf
[ -x "$mcf" ] || cannot "no release build at $mcf — \`cargo build --release -p mcf-cli\` first"

data=${XDG_DATA_HOME:-${HOME:-}/.local/share}
store=${MCF_MODELS:-$data/mcf/models}
[ -d "$store" ] || cannot "no models at $store"

# Every variant of one repository, which is what makes this one model rather
# than several. The repository is an argument of the environment rather than a
# constant here, because naming one in the script would be MCF taking a view on
# whose model to characterize (B28).
family=${MCF_FRONTIER_FAMILY:-}
if [ -z "$family" ]; then
    family=$(find "$store" -name '*.gguf' -type f -printf '%h\n' 2>/dev/null | sort | uniq -c | sort -rn | head -1 | awk '{print $2}')
fi
[ -n "$family" ] && [ -d "$family" ] || cannot "no directory of variants under $store"

mapfile -t variants < <(find "$family" -maxdepth 1 -name '*.gguf' -type f | sort)
[ "${#variants[@]}" -ge 2 ] || cannot "only ${#variants[@]} variant(s) in $family; a frontier needs several"

# The reference arm every other point is compared against. §3.27: what travels
# is the comparison, so the frontier is n paired comparisons rather than n
# absolutes on a chart.
against=""
for held in "${variants[@]}"; do
    case "$held" in *"$reference"*) against=$held ;; esac
done
[ -n "$against" ] || cannot "no variant matching '$reference' in $family"

printf 'the quantization frontier\n\n'
printf '  family     %s\n' "$family"
printf '  reference  %s\n' "$(basename "$against")"
printf '  asked      %s, %s tokens, resolving %s%%\n' "$PROMPT" "$TOKENS" "$RESOLVING"
printf '  every trial loads the model for itself (F64, F65), so each figure\n'
printf '  includes what that costs — which is a condition, not an error.\n\n'

produced=0
for held in "${variants[@]}"; do
    [ "$held" = "$against" ] && continue
    line=$("$mcf" bench "$held" --against "$against" \
        --prompt "$PROMPT" --limit "$TOKENS" --resolving "$RESOLVING" \
        --engine provisioned --cold 2>&1 | head -1) || true
    said=${line##*gguf: }
    printf '  %-40s %s\n' "$(basename "$held")" "${said%% — paired*}"
    produced=$((produced + 1))
done

[ "$produced" -gt 0 ] || cannot "no comparison ran; is a daemon listening, and is an engine provisioned?"

cat <<'CAVEAT'

  WHAT THIS CHARACTERIZES
  This is one model, one machine, one prompt, one engine build and one
  sitting. It characterizes **the instrument** — that MCF can take a
  single-variable comparison across many points and say what it found —
  and it is not a statement about quantization in general, about these
  quantizations on other hardware, or about anything but latency. What a
  quantization costs in *quality* is not measured here and is not
  measured anywhere yet (§IV, DEC-002).

  Every figure includes the model load, because the engine holds one
  model at a time and a paired comparison alternates two (F64, F65).
CAVEAT
