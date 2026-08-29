#!/usr/bin/env bash
# The performance picture: rate against context depth, at every allocation a
# model can be given (B-398, B-399, D19, §3.4, A19, F117, F118).
#
# THE OPERATOR'S DESIGN, 2026-08-29. Step the ALLOCATION from 1024 up to the
# model's own trained context by powers of two; at each allocation step the
# GENERATION from 128 up to that allocation; and do it WARM, because warm is how
# MCF serves. Cold runs are kept, but only to measure what a cold start costs —
# never to measure speed.
#
# WHY BOTH AXES. F118 measured the rate falling 37% and 55% between 128 and 4096
# tokens, and then the operator identified the axis that measurement had left
# uncontrolled: how much context a model is *allocated* is a separate thing from
# how much it has *used*, it is bounded by the memory the machine can spare, and
# on one model the difference between allocating 512 and 8192 is 172 MB —
# nearly twice the weights. So depth and allocation are two axes and this walks
# both.
#
# ONE GENERATION PER RUNG, NOT A LADDER OF THEM. Streaming a single generation
# to the deepest rung and timing every token as it arrives gives the rate in
# every band from one continuous run — the same clock, the same cache, the same
# process — where separate runs would cost twice the tokens and force each
# band's rate to be differenced out of two independently noisy runs. It also
# gives the time to the *first* token, which is the fixed cost with everything
# after it removed.
#
# WARM MEANS THE MODEL IS RESIDENT. One server per allocation, a throwaway
# generation to warm it, then the measured ones. What is not reused is the
# key/value cache: each request starts fresh, or the second one would be
# measuring the first one's leftovers.
#
# WHAT THIS COSTS. Roughly the sum of the allocations, times the repeats, in
# tokens. For a model trained to 8192 that is about 46,000 tokens; for one
# trained to 40960 it is about 194,000, which at the rates F118 measured is over
# an hour. The window is taken for the whole of it.
#
# THE MACHINE STAYS THE OPERATOR'S. Every server is a child of this script and
# the trap kills them on any exit, including an interrupt.
set -u

root=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/../.." && pwd)
cd "$root"
out=${1:-/tmp/context-ladder}
mkdir -p "$out"

readonly REPEATS=${MCF_LADDER_REPEATS:-3}
readonly SMALLEST_ALLOCATION=1024
readonly PORT=${MCF_LADDER_PORT:-18771}

prefix=$HOME/.local/share/mcf/provisioned
server=$(find "$prefix" -name llama-server -type f 2>/dev/null | head -1)
completion=$(find "$prefix" -name llama-completion -type f 2>/dev/null | head -1)
[ -x "${server:-}" ] && [ -x "${completion:-}" ] || {
    printf 'cannot measure: no provisioned engine\n' >&2; exit 2; }
mcf="$root/target/release/mcf"
[ -x "$mcf" ] || { printf 'cannot measure: no release build\n' >&2; exit 2; }

models=()
for candidate in \
    "$HOME/.local/share/mcf/models/bartowski/SmolLM2-135M-Instruct-GGUF/SmolLM2-135M-Instruct-Q4_0.gguf" \
    "$HOME/.local/share/mcf/models/Qwen/Qwen3-0.6B-GGUF/Qwen3-0.6B-Q8_0.gguf"; do
    [ -f "$candidate" ] && models+=("$candidate")
done
[ "${#models[@]}" -gt 0 ] || { printf 'cannot measure: no models\n' >&2; exit 2; }

server_pid=""
cleanup() { [ -n "$server_pid" ] && kill "$server_pid" 2>/dev/null; }
trap cleanup EXIT INT TERM

warm="$out/warm.tsv"
cold="$out/cold.tsv"
printf 'model\tallocation\trepeat\tkind\tfrom_token\tto_token\tnanoseconds\n' >"$warm"
printf 'model\tallocation\trepeat\tnanoseconds\n' >"$cold"

die() {
    for i in 1 2 3 4 5; do
        case "$(cat "/sys/class/hwmon/hwmon4/temp${i}_label" 2>/dev/null || true)" in
        Tccd*) cat "/sys/class/hwmon/hwmon4/temp${i}_input" 2>/dev/null; return;;
        esac
    done
    printf '0\n'
}

for model in "${models[@]}"; do
    name=$(basename "$model")
    # The model's own trained context, read from the file rather than assumed:
    # the ceiling of this ladder is a property of the artifact (A21).
    trained=$("$mcf" explain "$model" 2>/dev/null |
        awk '/^  context length/ { print $3; exit }')
    [ -n "${trained:-}" ] || { printf '  %s: no declared context; skipped\n' "$name" >&2; continue; }
    printf '\n%s — trained context %s, die %s °C\n' "$name" "$trained" "$(( $(die) / 1000 ))" >&2

    allocation=$SMALLEST_ALLOCATION
    while [ "$allocation" -le "$trained" ]; do
        # COLD: what a start costs at this allocation, and nothing else. One
        # token, so that what is timed is loading and allocating rather than
        # generating (F117's intercept, isolated).
        for repeat in $(seq 1 "$REPEATS"); do
            started=$(date +%s%N)
            timeout 600 "$completion" -m "$model" -p "A" -n 1 -c "$allocation" \
                --temp 0 --seed 0 -ngl 0 -no-cnv --no-display-prompt --no-warmup \
                >/dev/null 2>&1 || true
            ended=$(date +%s%N)
            printf '%s\t%s\t%s\t%s\n' "$name" "$allocation" "$repeat" "$((ended - started))" \
                >>"$cold"
        done

        # WARM: one server, resident, for every measured generation here.
        "$server" -m "$model" --host 127.0.0.1 --port "$PORT" -c "$allocation" \
            -ngl 0 --no-warmup >/dev/null 2>&1 &
        server_pid=$!
        ready=0
        for _ in $(seq 1 300); do
            if curl -s "http://127.0.0.1:$PORT/health" 2>/dev/null | grep -q '"ok"'; then
                ready=1; break
            fi
            sleep 0.2
        done
        if [ "$ready" -ne 1 ]; then
            printf '  %-34s allocation %6s: the server did not come up\n' "$name" "$allocation" >&2
            kill "$server_pid" 2>/dev/null; server_pid=""
            allocation=$((allocation * 2)); continue
        fi
        # One throwaway generation: what stays warm is the model, and the first
        # request after a start pays for whatever the server does once.
        python3 "$root/prototypes/context-ladder/probe.py" "$PORT" 8 "$name" \
            "$allocation" 0 /dev/null >/dev/null 2>&1 || true

        printf '  %-34s allocation %6s, warm\n' "$name" "$allocation" >&2
        for repeat in $(seq 1 "$REPEATS"); do
            python3 "$root/prototypes/context-ladder/probe.py" "$PORT" "$allocation" \
                "$name" "$allocation" "$repeat" "$warm" >&2 || true
        done
        kill "$server_pid" 2>/dev/null; wait "$server_pid" 2>/dev/null; server_pid=""
        allocation=$((allocation * 2))
    done
done

printf '\nwarm readings in %s\ncold readings in %s\n' "$warm" "$cold"
python3 "$root/prototypes/context-ladder/analyse.py" "$warm" "$cold"
