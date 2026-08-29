#!/usr/bin/env bash
# Does a model generate at a constant rate, and for how long (B-396, B-397,
# D19, §3.4, A19, F117, F118).
#
# THE OPERATOR'S DESIGN, with one change. The proposal was: give every model the
# same near-empty prompt, let it generate for a fixed time — fifteen seconds,
# then thirty, a minute, two, four — and count what came out. Three things about
# it are right and better than the length ladder F117 used:
#
#   * a run long enough dilutes the per-request fixed cost instead of having to
#     fit it out: a one-second load is 7% of fifteen seconds and 0.4% of four
#     minutes;
#   * a prompt of one token cannot cost one vocabulary more than another, which
#     is the asymmetry F116 could only narrow;
#   * with the stop token suppressed no model can end the run early, so
#     verbosity cannot leak into the reading.
#
# THE CHANGE: a ladder of TOKEN COUNTS rather than of durations. Killing a
# generation at fifteen seconds loses whatever is in the output buffer, so the
# count becomes an estimate; running to a known count gives an exact one and the
# same information with the axis swapped. The durations map straight onto it.
#
# WHAT THIS IS REALLY ASKING. Generation is not a constant-rate process: each
# token attends to every token before it, so per-token cost should RISE as the
# run goes on. If it does, then "tokens in T seconds" is a rate *at a context
# depth*, a fixed-duration comparison is biased against the faster model — it
# reaches deeper context in the same time — and the decay curve is the thing to
# measure. If it does not, the operator's design is simply the better one and
# this says so.
#
# WHAT IS READ OUT at every rung: the wall time, the exact tokens produced, the
# MARGINAL rate since the previous rung — which is the rate at that context
# depth — and the processor die temperature and load before and after, because
# four minutes of sustained generation is a thermal event and a rate that fell
# because the machine got hot is not a fact about the model (§3.4).
set -u

root=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/../.." && pwd)
cd "$root"
out=${1:-/tmp/sustained-generation}
mkdir -p "$out"

# One token, the same for every model, carrying nothing: what is being timed is
# generation, and a prompt with content would be a prompt some vocabulary spells
# more expensively than another (F116).
readonly PROMPT="A"
# The ladder. Chosen to stay inside the smallest context in the corpus (8192)
# with room to spare, and to span the durations the operator named: at the rates
# F117 measured, 4096 tokens is roughly a minute for a slow model and eight
# seconds for a quick one.
readonly RUNGS=(128 256 512 1024 2048 4096)
readonly REPEATS=${MCF_SUSTAINED_REPEATS:-3}
# **The allocation, set rather than inherited** (the operator, 2026-08-29;
# F118). Left alone, the engine takes each model's own declared training
# context — 8,192 for SmolLM2 and 40,960 for Qwen3 — so two arms compared at
# their defaults differ by fivefold in how much key/value cache was allocated,
# unrecorded. It is not a detail: on SmolLM2 the difference between allocating
# 512 and 8,192 is 172 MB of resident memory, nearly twice the weights. And it
# is the operator's own constraint made concrete — how much context a model may
# have is bounded by the space it is allowed to occupy, so the part of the decay
# curve a person ever reaches is bounded by their memory rather than by the
# model.
#
# Held still across every arm so that what differs between two of them is the
# model, and stated in the readings so that a run at another allocation is not
# silently compared with this one.
readonly ALLOCATION=${MCF_SUSTAINED_CONTEXT:-8192}
readonly PER_RUN_SECONDS=900

prefix=$HOME/.local/share/mcf/provisioned
engine=$(find "$prefix" -name llama-completion -type f 2>/dev/null | head -1)
[ -x "${engine:-}" ] || { printf 'cannot measure: no provisioned engine\n' >&2; exit 2; }
tokenize="$root/target/release/examples/tokenize"
[ -x "$tokenize" ] || { printf 'cannot measure: build the tokenize example first\n' >&2; exit 2; }

models=()
for candidate in \
    "$HOME/.local/share/mcf/models/bartowski/SmolLM2-135M-Instruct-GGUF/SmolLM2-135M-Instruct-Q4_0.gguf" \
    "$HOME/.local/share/mcf/models/Qwen/Qwen3-0.6B-GGUF/Qwen3-0.6B-Q8_0.gguf"; do
    [ -f "$candidate" ] && models+=("$candidate")
done
[ "${#models[@]}" -gt 0 ] || { printf 'cannot measure: no models\n' >&2; exit 2; }

# The machine, watched. A die temperature rather than a package one: F91 found
# the package sensor reads 16 °C below the die under load, and a rate that fell
# because the machine got hot is a condition rather than a property.
die() {
    for i in 1 2 3 4 5; do
        label=$(cat "/sys/class/hwmon/hwmon4/temp${i}_label" 2>/dev/null || true)
        case "$label" in Tccd*) cat "/sys/class/hwmon/hwmon4/temp${i}_input" 2>/dev/null; return;; esac
    done
    printf '0\n'
}
load() { cut -d' ' -f1 /proc/loadavg; }

children=()
trap 'for pid in "${children[@]:-}"; do kill "$pid" 2>/dev/null || true; done' EXIT INT TERM

readings="$out/readings.tsv"
printf 'model\tallocation\trung\trepeat\tnanoseconds\tproduced\tdie_before_mc\tdie_after_mc\tload_before\tload_after\n' >"$readings"

for model in "${models[@]}"; do
    name=$(basename "$model")
    for rung in "${RUNGS[@]}"; do
        for repeat in $(seq 1 "$REPEATS"); do
            before_t=$(die); before_l=$(load)
            started=$(date +%s%N)
            said=$(timeout "$PER_RUN_SECONDS" "$engine" -m "$model" -p "$PROMPT" \
                -n "$rung" -c "$ALLOCATION" --temp 0 --seed 0 -ngl 0 -no-cnv \
                --no-display-prompt --no-warmup --ignore-eos 2>/dev/null || true)
            ended=$(date +%s%N)
            after_t=$(die); after_l=$(load)
            produced=$("$tokenize" "$model" "$said" --ids 2>/dev/null | wc -w)
            printf '%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\n' "$name" "$ALLOCATION" "$rung" "$repeat" \
                "$((ended - started))" "$produced" "$before_t" "$after_t" "$before_l" "$after_l" \
                >>"$readings"
        done
        printf '  %-34s rung %5s done\n' "$name" "$rung" >&2
    done
done

printf '\nreadings in %s\n' "$readings"
python3 "$root/prototypes/sustained-generation/analyse.py" "$readings"
