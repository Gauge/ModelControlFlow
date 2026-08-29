#!/usr/bin/env bash
# Does a generation actually move the bytes the arithmetic says it moves?
#
# The slope argument says one generated token at depth d re-reads the whole KV
# cache: d x (KV bytes per token). That has been inferred from timings all
# along. With a calibrated counter it can be COUNTED.
#
# Two generations at the same depth, differing only in how many tokens are
# produced. The difference cancels the prefill, the model load and everything
# else that happens once, leaving the decode traffic alone.
set -euo pipefail
readonly PORT=8083
readonly DEPTH=${DEPTH:-4096}
readonly FEW=${FEW:-32}
readonly MANY=${MANY:-160}
root=$(cd "$(dirname "$0")/../.." && pwd)
model=$(find "$HOME/.local/share/mcf" -name "${MODEL:-Qwen3-0.6B-Q8_0}.gguf" | head -1)
server=$(command -v llama-server || find "$HOME/.local/share/mcf" -name llama-server -type f | head -1)

"$server" -m "$model" --host 127.0.0.1 --port "$PORT" -c 8192 -ngl 0 --no-warmup >/dev/null 2>&1 &
pid=$!
trap 'kill $pid 2>/dev/null || true' EXIT
for _ in $(seq 1 600); do curl -s "http://127.0.0.1:$PORT/health" 2>/dev/null | grep -q '"ok"' && break; sleep 0.2; done
python3 "$root/prototypes/falloff/depth_probe.py" prefill "$PORT" 256 8 warm 0 0 /dev/null >/dev/null 2>&1 || true

count() { # count <tokens>
    perf stat -a -x, -e l3_lookup_state.l3_miss -- \
        python3 "$root/prototypes/falloff/depth_probe.py" prefill "$PORT" "$DEPTH" "$1" \
        counted 8192 0 /dev/null 2>&1 | awk -F, '/l3_miss/{print $1}'
}
printf 'model %s, depth %s\n' "$(basename "$model")" "$DEPTH"
for repeat in 1 2 3; do
    few=$(count "$FEW"); many=$(count "$MANY")
    printf '  pass %s: %s tokens -> %s misses | %s tokens -> %s misses | difference %s\n' \
        "$repeat" "$FEW" "$few" "$MANY" "$many" "$((many - few))"
done
