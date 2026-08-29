#!/usr/bin/env bash
# Predicting the fall-off: what the variables are, and which of them a cheap
# diagnostic is allowed to ignore.
#
# Three arms, because there are three separate things not known:
#
#   order    Does the allocation change the rate at a fixed depth — or did the
#            context ladder only show that, because it walked the allocations
#            in ascending order and never recorded a temperature per run? The
#            two explanations are indistinguishable in that data. Here the
#            order is INTERLEAVED and the die temperature is written next to
#            every reading, so they separate.
#
#   arrival  Is the rate at depth D the same whether D was generated or
#            prefilled? Generating to depth is quadratic and is what makes a
#            full characterisation cost hours. Prefill is batched. If the two
#            agree, the deep part of the curve stops being expensive — and a
#            measured deep point beats an extrapolated one, which the
#            synthetic lab shows cannot be trusted to state its own error.
#
#   corpus   Across every model on this machine, fit the shallow readings and
#            predict the deep one. The spread of THAT error is the diagnostic's
#            accuracy. The lab is unambiguous that it cannot be computed from
#            the fit; it has to be calibrated against models.
#
# Usage: run.sh [order|arrival|corpus|all]
set -euo pipefail

readonly ARM="${1:-all}"
readonly REPEATS="${MCF_FALLOFF_REPEATS:-3}"
readonly PORT="${MCF_FALLOFF_PORT:-8081}"
readonly OUT="${MCF_FALLOFF_OUT:-/tmp/falloff}"

root=$(cd "$(dirname "$0")/../.." && pwd)
probe="$root/prototypes/falloff/depth_probe.py"

server=$(command -v llama-server || true)
[ -n "$server" ] || server=$(find "$HOME/.local/share/mcf" -name llama-server -type f 2>/dev/null | head -1)
[ -x "${server:-}" ] || { printf 'cannot measure: no provisioned llama-server\n' >&2; exit 2; }

mkdir -p "$OUT"
readings="$OUT/readings.tsv"
[ -s "$readings" ] || printf 'model\tallocation\trepeat\tarrival\tkind\tfrom_token\tto_token\tnanoseconds\tprefill_ns\n' >"$readings"

die() {
    for i in 1 2 3 4 5; do
        case "$(cat "/sys/class/hwmon/hwmon4/temp${i}_label" 2>/dev/null || true)" in
        Tccd*) cat "/sys/class/hwmon/hwmon4/temp${i}_input" 2>/dev/null; return;;
        esac
    done
    printf '0\n'
}

server_pid=""
stop() { [ -n "$server_pid" ] && { kill "$server_pid" 2>/dev/null || true; wait "$server_pid" 2>/dev/null || true; server_pid=""; }; }
trap 'stop' EXIT INT TERM

start() { # start <model-path> <allocation>
    stop
    "$server" -m "$1" --host 127.0.0.1 --port "$PORT" -c "$2" -ngl 0 --no-warmup >/dev/null 2>&1 &
    server_pid=$!
    for _ in $(seq 1 600); do
        curl -s "http://127.0.0.1:$PORT/health" 2>/dev/null | grep -q '"ok"' && {
            python3 "$probe" generate "$PORT" 8 warmup 0 0 /dev/null >/dev/null 2>&1 || true
            return 0
        }
        sleep 0.2
    done
    return 1
}

# MCF_FALLOFF_ONLY selects the models by substring. The order arm has to be
# pointed at the model whose allocation effect is in question, which is not
# whichever two sort first.
models=()
while IFS= read -r found; do models+=("$found"); done < <(
    find "$HOME/.local/share/mcf" "$HOME/.cache/mcf" -name '*.gguf' -type f 2>/dev/null \
        | grep -i -- "${MCF_FALLOFF_ONLY:-}" | sort
)
[ "${#models[@]}" -gt 0 ] || { printf 'cannot measure: no models\n' >&2; exit 2; }
printf 'found %d model(s)\n' "${#models[@]}" >&2

# ── order: the same depths, at several allocations, INTERLEAVED ──────────────
if [ "$ARM" = order ] || [ "$ARM" = all ]; then
    printf '\n=== order: is the allocation the cause, or was it the running order?\n' >&2
    for model in "${models[@]:0:2}"; do
        name=$(basename "$model")
        # Round-robin the allocations, so a drift over the arm cannot masquerade
        # as an allocation effect: each allocation is visited once per pass.
        for repeat in $(seq 1 "$REPEATS"); do
            for allocation in 1024 2048 4096 8192; do
                start "$model" "$allocation" || { printf '  %s c=%s: no server\n' "$name" "$allocation" >&2; continue; }
                before=$(( $(die) / 1000 ))
                python3 "$probe" generate "$PORT" 1024 "$name" "$allocation" "$repeat" "$readings" 2>&1 | sed 's/^/  /' >&2
                printf '  %-30s c=%-6s pass %s  die %s->%s C\n' "$name" "$allocation" "$repeat" "$before" "$(( $(die) / 1000 ))" >&2
            done
        done
    done
    stop
fi

# ── arrival: generated depth against prefilled depth ─────────────────────────
if [ "$ARM" = arrival ] || [ "$ARM" = all ]; then
    printf '\n=== arrival: does a prefilled depth cost the same as a generated one?\n' >&2
    for model in "${models[@]:0:2}"; do
        name=$(basename "$model")
        start "$model" 8192 || continue
        for repeat in $(seq 1 "$REPEATS"); do
            # generated: one run to 4096 gives every band below it
            python3 "$probe" generate "$PORT" 4096 "$name" 8192 "$repeat" "$readings" 2>&1 | sed 's/^/  /' >&2
            # prefilled: the same depths, reached by prompt, 128 tokens measured
            for depth in 512 1024 2048 4096; do
                python3 "$probe" prefill "$PORT" "$depth" 128 "$name" 8192 "$repeat" "$readings" 2>&1 | sed 's/^/  /' >&2
            done
        done
        stop
    done
fi

# ── corpus: the calibration, across every model here ─────────────────────────
if [ "$ARM" = corpus ] || [ "$ARM" = all ]; then
    printf '\n=== corpus: how wrong is a shallow fit, across %d models?\n' "${#models[@]}" >&2
    for model in "${models[@]}"; do
        name=$(basename "$model")
        start "$model" 8192 || { printf '  %-30s no server; skipped\n' "$name" >&2; continue; }
        before=$(( $(die) / 1000 ))
        for repeat in $(seq 1 "$REPEATS"); do
            python3 "$probe" generate "$PORT" 4096 "$name" 8192 "$repeat" "$readings" 2>&1 | sed 's/^/  /' >&2
        done
        # the deep anchor the shallow fit will be scored against
        for repeat in $(seq 1 "$REPEATS"); do
            python3 "$probe" prefill "$PORT" 7168 128 "$name" 8192 "$repeat" "$readings" 2>&1 | sed 's/^/  /' >&2
        done
        printf '  %-30s die %s->%s C\n' "$name" "$before" "$(( $(die) / 1000 ))" >&2
        stop
    done
fi

printf '\nreadings in %s\n' "$readings"
