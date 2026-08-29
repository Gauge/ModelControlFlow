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
# The conditions columns come from depth_probe's CONDITIONS list, printed by
# the probe itself so the header cannot drift from the rows.
cond_header=$(python3 -c "import sys;sys.path.insert(0,'$root/prototypes/falloff');import depth_probe;print('\t'.join(depth_probe.CONDITIONS + depth_probe.TRAILING))")
[ -s "$readings" ] || printf 'model\tallocation\trepeat\tarrival\tkind\tfrom_token\tto_token\tnanoseconds\tprefill_ns\t%s\n' "$cond_header" >"$readings"

die() {
    for i in 1 2 3 4 5; do
        case "$(cat "/sys/class/hwmon/hwmon4/temp${i}_label" 2>/dev/null || true)" in
        Tccd*) cat "/sys/class/hwmon/hwmon4/temp${i}_input" 2>/dev/null; return;;
        esac
    done
    printf '0\n'
}

# A model that produced no reading at all is the gap that makes a table look
# complete. Whatever the reason, it goes in the file.
note() { # note <model> <allocation> <why>
    printf '%s\t%s\t0\tplan\trefused\t0\t0\t0\t0' "$1" "$2" >>"$readings"
    python3 -c "
import sys;sys.path.insert(0,'$root/prototypes/falloff');import depth_probe
sys.stdout.write('\t' + '\t'.join(['unknown'] * len(depth_probe.CONDITIONS)))" >>"$readings"
    printf '\t%s\n' "$3" >>"$readings"
}

server_pid=""
stop() { [ -n "$server_pid" ] && { kill "$server_pid" 2>/dev/null || true; wait "$server_pid" 2>/dev/null || true; server_pid=""; }; }
trap 'stop' EXIT INT TERM

start() { # start <model-path> <allocation>
    stop
    "$server" -m "$1" --host 127.0.0.1 --port "$PORT" -c "$2" -ngl 0 --no-warmup >/dev/null 2>&1 &
    server_pid=$!
    export MCF_FALLOFF_SERVER_PID="$server_pid"
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
# What depths this model can actually be asked about. A fixed ladder assumes
# every model has the context of the one it was written for: the first corpus
# sweep asked 4096 of a model trained to 128 and lost five models to HTTP 400.
plan() { # plan <model-path> -> "<allocation> <depth> <depth> ..."
    # the heredoc is quoted, so the prototype's path is passed as an argument
    # rather than interpolated
    python3 - "$1" "$root/prototypes/falloff" <<'PLAN'
import sys
sys.path.insert(0, sys.argv[2])
import physics
geo = physics.geometry(sys.argv[1])
trained = (geo or {}).get("trained") or 0
if not trained:
    # A state-space or recurrent model declares no context length, and
    # correctly so: it keeps a fixed state rather than a growing cache. That
    # is precisely the model whose curve should be FLAT, so it is the one
    # most worth probing. The depths are assumed rather than declared, which
    # the reading says (A21).
    if (geo or {}).get("verdict") == "no-growing-cache":
        print("4096 512 1024 2048 3584")
    else:
        print("")
    raise SystemExit
allocation = min(int(trained), 16384)
# 128 tokens are generated at each depth, and the engine needs headroom for
# them, so the deepest probe stops short of the allocation.
ceiling = allocation - 256
depths, d = [], 512
while d <= ceiling:
    depths.append(d)
    d *= 2
# One more depth as deep as the context allows. A doubling ladder stops at
# the last power of two that fits, which for a model whose sliding window IS
# a power of two means never probing past the window -- exactly the region
# where the window changes the slope. gemma-2's window is 4096 and its
# context 8192: without this it is measured only where it looks ordinary.
if depths and ceiling >= depths[-1] * 1.4:
    depths.append(ceiling)
if not depths and ceiling >= 64:
    depths = [max(64, ceiling // 2)]   # a small-context model still has a curve
print(" ".join(str(x) for x in [allocation] + depths))
PLAN
}

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

# ── corpus: the whole curve, measured, on every model here ───────────────────
# The arrival arm settled that a prefilled depth costs what a generated one
# costs (within 2.8%, inside the noise) and gets there 10-24x quicker. So the
# curve is MEASURED at each depth rather than fitted at shallow depths and
# extrapolated -- which the synthetic lab showed cannot state its own error.
# This is why the long version of this test is not needed: it was not buying
# accuracy, it was buying depth that prefill reaches for a twentieth of the
# cost.
if [ "$ARM" = corpus ] || [ "$ARM" = all ]; then
    printf '\n=== corpus: the measured curve, %d models\n' "${#models[@]}" >&2
    for model in "${models[@]}"; do
        name=$(basename "$model")
        case "$name" in ggml-vocab-*) continue;; esac
        read -r allocation depths <<<"$(plan "$model")"
        if [ -z "${allocation:-}" ]; then
            printf '  %-34s no declared context in the header; not planned\n' "$name" >&2
            note "$name" 0 "the header declares no context length"
            continue
        fi
        if [ -z "${depths:-}" ]; then
            printf '  %-34s context %s is too small to probe at depth\n' "$name" "$allocation" >&2
            note "$name" "$allocation" "trained context $allocation leaves no room to probe at depth"
            continue
        fi
        start "$model" "$allocation" || { printf '  %-34s no server; skipped\n' "$name" >&2; continue; }
        printf '  %-34s context %s, depths %s\n' "$name" "$allocation" "$depths" >&2
        for repeat in $(seq 1 "$REPEATS"); do
            for depth in $depths; do
                timeout 900 python3 "$probe" prefill "$PORT" "$depth" 128 \
                    "$name" "$allocation" "$repeat" "$readings" 2>&1 | sed 's/^/    /' >&2 || true
            done
        done
        stop
    done
fi

printf '\nreadings in %s\n' "$readings"
