#!/usr/bin/env bash
# What a generation actually costs, and how to measure it so the number means
# something (B-081, D19, §3.4, A19, F117).
#
# WHY THIS EXISTS. MCF's timing comparison pins the generated length — D19's
# reason is that "a timing that varies because one run stopped earlier is
# measuring the stop, not the speed" — and the pin was declared and never
# enforced. Asked for 32 tokens, SmolLM2 produced 5 and Qwen3 produced 32, and
# the comparison between them would have reported a six-fold difference in
# WORK as a difference in SPEED. That is the question this measures rather than
# argues about.
#
# WHAT IT MEASURES, in one matrix:
#
#   * generated length     8, 16, 32, 64, 128 tokens
#   * the pin              on (--ignore-eos) and off
#   * repeats              enough for a median and a spread at each cell
#   * two models           one that stops early and one that does not
#
# WHAT THE MATRIX ANSWERS.
#
#   1. Does the pin change the answer? Off, a model that stops early is timed
#      for the tokens it felt like producing; on, every arm does the same work.
#   2. Is time linear in generated length? If it is, the line has a SLOPE — the
#      marginal cost of a token — and an INTERCEPT — everything that happens
#      once per request: loading, prefill, process start. The intercept is the
#      part that does not belong in a rate.
#   3. What is a token worth in the units MCF publishes? The slope makes a
#      prompt-length or generated-length mismatch convertible into a percentage
#      of a comparison, which is what decides whether it matters.
#   4. How repeatable is a cell? Which is the floor under any difference this
#      machine can resolve.
#
# THE OUTPUT IS NOT A SPEED FOR A MODEL. Every figure here is one machine, one
# engine, one afternoon, and A20 keeps a prototype's reading out of anything
# MCF publishes. What it produces is a *method*: which quantity to report so
# that two people measuring the same thing get comparable numbers.
#
# THE MACHINE STAYS THE OPERATOR'S. Every run is bounded by `timeout`, every
# child is this script's, and the trap kills them on any exit.
set -u

root=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/../.." && pwd)
cd "$root"

out=${1:-/tmp/generation-timing}
mkdir -p "$out"

readonly QUESTION="In three sentences, describe what happens to a river between its source in the mountains and the sea."
readonly LENGTHS=(8 16 32 64 128)
readonly REPEATS=${MCF_TIMING_REPEATS:-7}
readonly PER_RUN_SECONDS=120

prefix=$HOME/.local/share/mcf/provisioned
engine=$(find "$prefix" -name llama-completion -type f 2>/dev/null | head -1)
if [ ! -x "${engine:-}" ]; then
    printf 'cannot measure: no provisioned engine — `mcf provision llama.cpp` builds one\n' >&2
    exit 2
fi
tokenize="$root/target/release/examples/tokenize"
[ -x "$tokenize" ] || cargo build --locked --offline --release -p mcf-standin --example tokenize >/dev/null 2>&1

models=()
for candidate in \
    "$HOME/.local/share/mcf/models/bartowski/SmolLM2-135M-Instruct-GGUF/SmolLM2-135M-Instruct-Q4_0.gguf" \
    "$HOME/.local/share/mcf/models/Qwen/Qwen3-0.6B-GGUF/Qwen3-0.6B-Q8_0.gguf"; do
    [ -f "$candidate" ] && models+=("$candidate")
done
[ "${#models[@]}" -gt 0 ] || { printf 'cannot measure: no models\n' >&2; exit 2; }

children=()
cleanup() { for pid in "${children[@]:-}"; do kill "$pid" 2>/dev/null || true; done; }
trap cleanup EXIT INT TERM

readings="$out/readings.tsv"
: >"$readings"
printf 'model\tpinned\tlength\trepeat\tnanoseconds\tproduced\n' >>"$readings"

for model in "${models[@]}"; do
    name=$(basename "$model")
    for pinned in off on; do
        flag=""
        [ "$pinned" = on ] && flag="--ignore-eos"
        for length in "${LENGTHS[@]}"; do
            for repeat in $(seq 1 "$REPEATS"); do
                started=$(date +%s%N)
                said=$(timeout "$PER_RUN_SECONDS" "$engine" -m "$model" -p "$QUESTION" \
                    -n "$length" --temp 0 --seed 0 -ngl 0 -no-cnv --no-display-prompt \
                    --no-warmup $flag 2>/dev/null || true)
                ended=$(date +%s%N)
                # What came back, counted in the model's own vocabulary. A
                # generation is only comparable with another if the same amount
                # of it happened, and that is the whole point of this run.
                produced=$("$tokenize" "$model" "$said" --ids 2>/dev/null | wc -w)
                printf '%s\t%s\t%s\t%s\t%s\t%s\n' \
                    "$name" "$pinned" "$length" "$repeat" "$((ended - started))" "$produced" \
                    >>"$readings"
            done
            printf '  %-34s pin %-3s length %3s done\n' "$name" "$pinned" "$length" >&2
        done
    done
done

printf '\nreadings in %s\n' "$readings"
python3 "$root/prototypes/generation-timing/analyse.py" "$readings"
