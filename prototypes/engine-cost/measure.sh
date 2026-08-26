#!/bin/sh
# What admitting an inference engine would cost (B-320, D23, D32, §XVI).
#
# D32 settled that MCF delegates inference rather than implementing kernels, so
# an engine is going to be vendored. *Which* one is [vendored.md]'s question,
# and the register asks for a compatibility finding before anything ships. This
# is the measuring half of that finding, written the way F9 measured the
# transport: ask the machine rather than argue.
#
# **The question is not "which is fastest".** It is what each costs MCF as a
# *thing to ship*: how much source, under what terms, needing which toolchain,
# and what it does to two claims MCF already makes — the 40 MiB footprint
# ceiling (D24) and the from-scratch container that runs the musl artifact with
# nothing installed (B-183, B36).
#
# Two candidates, chosen because §XVI's constraint is vendoring rather than
# convenience:
#
#   * llama.cpp — C++ and CMake, one upstream, one licence, reads GGUF natively;
#   * candle — Rust, no C++ toolchain, but a hundred and fifty crates.
#
# Everything is built OUTSIDE this repository. Measuring a candidate is not
# admitting one, and nothing here vendors anything into MCF's tree.
#
#   ./prototypes/engine-cost/measure.sh [scratch directory]
#
# Reports to doc/findings.md F12. The build half needs several minutes of a
# quiet machine: where `heavy` is on PATH it takes the exclusive window, because
# a build this size run beside somebody's measurements is a build that spoils
# them (B35, build.md §12).

set -eu

scratch="${1:-${TMPDIR:-/tmp}/mcf-engine-cost}"
targets="x86_64-unknown-linux-gnu x86_64-unknown-linux-musl"

say() { printf '%s\n' "$*"; }
rule() { printf '\n=== %s\n' "$*"; }

mkdir -p "$scratch"
say "measuring in $scratch"

rule "what this machine has to build with"
for tool in cc g++ cmake cargo x86_64-linux-musl-gcc x86_64-linux-musl-g++; do
    if command -v "$tool" >/dev/null 2>&1; then
        say "  $tool: $(command -v "$tool")"
    else
        say "  $tool: ABSENT"
    fi
done
say "  (a cross toolchain is what the musl artifact needs; F9.5 is where its"
say "   absence first cost a candidate)"

# ------------------------------------------------------------- llama.cpp
rule "llama.cpp: what a vendored tree would weigh"
if [ ! -d "$scratch/llama.cpp" ]; then
    git clone --depth 1 --quiet https://github.com/ggml-org/llama.cpp.git "$scratch/llama.cpp" \
        || say "  could not be fetched"
fi
if [ -d "$scratch/llama.cpp" ]; then
    (
        cd "$scratch/llama.cpp"
        say "  revision: $(git rev-parse HEAD | cut -c1-16)"
        say "  licence: $(head -1 LICENSE 2>/dev/null || echo unknown)"
        say "  whole checkout: $(du -sh --exclude=.git . | cut -f1)"
        say "  the parts an engine needs:"
        du -sh ggml src common include vendor 2>/dev/null | sed 's/^/    /'
        lines=$(find ggml src include common \
            \( -name '*.c' -o -name '*.cpp' -o -name '*.h' -o -name '*.hpp' \) \
            2>/dev/null | xargs wc -l 2>/dev/null | tail -1 | awk '{ print $1 }')
        say "  lines of C and C++ in them: ${lines:-unknown}"
        say "  build system: $(grep -m1 cmake_minimum_required CMakeLists.txt 2>/dev/null || echo unknown)"
    )
fi

# ---------------------------------------------------------------- candle
rule "candle: what a vendored tree would weigh"
if [ ! -d "$scratch/candle-cost" ]; then
    ( cd "$scratch" && cargo new --quiet --bin candle-cost ) || true
    ( cd "$scratch/candle-cost" \
        && cargo add candle-core candle-transformers --no-default-features >/dev/null 2>&1 ) \
        || say "  could not be resolved"
fi
if [ -d "$scratch/candle-cost" ]; then
    (
        cd "$scratch/candle-cost"
        crates=$(grep -c '^name = ' Cargo.lock 2>/dev/null || echo 0)
        say "  crates in the lock graph: $crates"
        [ -d v ] || cargo vendor --quiet v >/dev/null 2>&1 || true
        if [ -d v ]; then
            say "  vendored tree, unfiltered: $(du -sh v | cut -f1)"
            say "  C or assembly source files in it: $(find v \( -name '*.c' -o -name '*.cpp' -o -name '*.S' \) | wc -l)"
            say "  licences declared:"
            grep -h '^license' v/*/Cargo.toml 2>/dev/null | sed 's/license *= *//' \
                | sort | uniq -c | sort -rn | head -8 | sed 's/^/    /'
        fi
    )
fi

# ------------------------------------------------------- the two claims
rule "the two claims an engine has to keep"
say "  D24: the core binary stays inside 40 MiB. Measure the artifact, not the"
say "       source: a vendored tree is what a checkout costs and a linked binary"
say "       is what a user gets."
say "  B-183: the musl artifact runs in a container with nothing in it. A"
say "       candidate that cannot be built for that target does not fail — it"
say "       makes an existing check runnable in fewer places, which is what"
say "       F9.5 measured and what decided the TLS provider."
say ""
say "  Both need a build, and a build of this size needs a quiet machine."
say "  Run this script inside the exclusive window and fill in F12's second"
say "  table; until then that table says what it does not know (A7)."
