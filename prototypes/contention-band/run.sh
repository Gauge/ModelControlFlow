#!/usr/bin/env bash
# The band DEC-007 leaves open, measured (B-217, B-084, F95).
#
# DEC-007 settled that quiet is relative: "a machine steady throughout a run is
# measurable wherever its baseline sits". What it left open is the band — how
# much competing load stops a machine being measurable — and it was explicit
# that the number must be measured rather than chosen.
#
# Until now it could not be. The contention instrument over-read by a fifth
# under load (F90), and the effect size carried no measure of itself (F92), so
# there was neither a trustworthy x-axis nor a y-axis. Both are fixed.
#
# WHAT THIS DOES. The same paired comparison, at six levels of competing load
# from idle to half again the machine's thread count, three times each. What is
# read out is not the effect size but the WIDTH of its interval: a machine that
# cannot be measured is one whose answers get wider, not one whose answers get
# bigger.
#
# THE CRITERION IS DERIVED, NOT CHOSEN (the operator, 2026-08-28). The quiet
# runs give a baseline width and its own run-to-run spread. The band is the
# highest load at which width is still indistinguishable from that baseline.
# MCF picks no threshold; it measures where load-induced widening exceeds the
# machine's own variability.
#
# THE MACHINE STAYS THE OPERATOR'S (B-181). Every load burst is bounded by
# `timeout`, every spinner is a child of this script, and the trap kills them
# on any exit including an interrupt. Nothing here can outlive the experiment.
set -u

root="$(cd "$(dirname "$0")/../.." && pwd)"
mcf="$root/target/release/mcf"
models="$HOME/.local/share/mcf/models/bartowski/SmolLM2-135M-Instruct-GGUF"
left="$models/SmolLM2-135M-Instruct-Q4_K_M.gguf"
right="$models/SmolLM2-135M-Instruct-Q6_K.gguf"
out="${1:-/tmp/contention-band.txt}"

threads="$(nproc)"
# Fractions of the machine's thread count. 125% is deliberate: F90's worst
# recorded readings were above saturation, and the band may well lie below it.
fractions="0 25 50 75 100 125"
repeats=3

spinners=()
stop_load() {
    if [ "${#spinners[@]}" -gt 0 ]; then
        kill -9 "${spinners[@]}" 2>/dev/null
        wait "${spinners[@]}" 2>/dev/null
        spinners=()
    fi
}
trap 'stop_load; exit 130' INT TERM
trap 'stop_load' EXIT

start_load() {
    local want="$1" i
    spinners=()
    [ "$want" -eq 0 ] && return
    for ((i = 0; i < want; i++)); do
        # Bounded absolutely: even if this script dies in a way the trap
        # cannot catch, every spinner ends by itself.
        timeout 600 sh -c 'while :; do :; done' >/dev/null 2>&1 &
        spinners+=($!)
    done
    sleep 3
}

printf 'fraction threads repeat verdict\n' > "$out"
for fraction in $fractions; do
    want=$(( threads * fraction / 100 ))
    for ((r = 1; r <= repeats; r++)); do
        start_load "$want"
        said="$(timeout 900 "$mcf" bench "$left" --against "$right" \
            --prompt Once --limit 32 --resolving 5 --engine provisioned --cold \
            2>/dev/null | head -1)"
        stop_load
        printf '%s %s %s %s\n' "$fraction" "$want" "$r" "${said:-NO OUTPUT}" >> "$out"
        printf '  %3s%% (%2s threads) run %s: %s\n' "$fraction" "$want" "$r" "${said:0:90}"
        # Let the machine settle so one level's heat does not become the
        # next level's condition (§3.4, and DEC-007's own open thermal half).
        sleep 10
    done
done
printf 'wrote %s\n' "$out"
