#!/usr/bin/env bash
#
# MCF against the real hub (B-029, B-322, §III).
#
# Every other test of acquisition runs against a hub the laboratory is holding
# on the loopback address, because B19 requires that M1's suite run with no
# network. That is the right default and it leaves one thing unchecked: whether
# the hub MCF was written against behaves the way MCF believes.
# an earlier finding answered that once, by hand, on one
# afternoon. This is the repeatable form.
#
# **It is scheduled rather than gating**, for the same reason the fuzz tier is:
# a gate that needs a network is a gate that fails on a train. `scripts/ci.sh`
# does not run it; `--with-online` does.
#
# **What it acquires is small and real.** A 1.2 MB GGUF from a repository of
# tiny test models — real weights, a real LFS digest, a real redirect to a CDN.
# A check that downloaded a 27 GiB model to prove a transfer works would be a
# check nobody runs, and a check that downloaded nothing would prove nothing.
#
# **And what it plans for is large and awkward, at no cost.** A transfer
# exercises the wire; it says nothing about the *variety* of what a hub
# publishes. an earlier finding found three defects in the
# planner the first time it was pointed at the reference repository — a
# configuration MCF could not parse, a configuration MCF was not looking in the
# right place for, and a cache overstated fourfold — every one of them in code
# with tests. So the plan is made for that repository too, which fetches no
# bytes at all: a listing and a `config.json`.
#
# **It is not a measurement.** Nothing timed is reported: the hub is somebody
# else's machine on somebody else's network, and A6 would want conditions MCF
# cannot state for any number taken here.
#
# Exit status: 0 when MCF acquired, listed and removed a real model; 1 when it
# did not; 2 when the check could not be made — no network, or the repository
# has changed under it.

set -o errexit -o nounset -o pipefail

readonly EXIT_FAILED=1
readonly EXIT_CANNOT_CHECK=2

# A repository of deliberately tiny models, and the smallest thing in it that is
# really a model. Named here rather than discovered, so that what this check
# acquires is a decision somebody made and can see.
readonly REPOSITORY=ggml-org/tiny-llamas
readonly FILE=stories260K.gguf

# The reference model (§XII), planned for and never fetched. It is here because
# it is *awkward* rather than because it is the reference: a multimodal
# configuration, a hybrid attention scheme, and an exponent in its metadata.
readonly AWKWARD=unsloth/Qwen3.8-27B-GGUF

root=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)
cd "$root"

fail_cannot_check() {
    printf 'cannot check: %s\n' "$1" >&2
    exit "$EXIT_CANNOT_CHECK"
}

fail() {
    printf 'ONLINE CHECK FAILED: %s\n' "$1" >&2
    exit "$EXIT_FAILED"
}

printf '=== building the binary this checks\n'
export RUSTFLAGS="${RUSTFLAGS:-}${RUSTFLAGS:+ }--remap-path-prefix=$root=."
cargo build --locked --offline --release --bin mcf >/dev/null
mcf="$root/target/release/mcf"
[ -x "$mcf" ] || fail_cannot_check "no binary was built"

machine=$(mktemp -d "${TMPDIR:-/tmp}/mcf-online-XXXXXX")
# And a socket of its own, so that the binary just built is the binary that
# answers rather than whatever daemon is listening (F104).
# shellcheck source=scripts/lib-tiers.sh
. "$root/scripts/lib-tiers.sh"
runtime=$(tier_private_runtime_dir)
trap 'rm -rf "$machine" "$runtime"' EXIT
export XDG_DATA_HOME="$machine"
export XDG_RUNTIME_DIR="$runtime"
# So that a mistake here writes nothing to the person's real record.
unset HOME

printf '=== what the repository publishes\n'
offered=$("$mcf" pull "$REPOSITORY" 2>&1) || {
    printf '%s\n' "$offered" >&2
    case "$offered" in
        *unreachable* | *"could not be resolved"* | *stalled*)
            fail_cannot_check "the hub could not be reached" ;;
        *) fail "listing $REPOSITORY did not succeed" ;;
    esac
}
printf '%s\n' "$offered" | sed 's/^/  /'
case "$offered" in
    *"$FILE"*) ;;
    *) fail_cannot_check "$REPOSITORY no longer publishes $FILE" ;;
esac
case "$offered" in
    *"nothing was acquired"*) ;;
    *) fail "a listing acquired something" ;;
esac

printf '\n=== acquiring it\n'
acquired=$("$mcf" pull "$REPOSITORY:$FILE" 2>&1) || {
    printf '%s\n' "$acquired" >&2
    fail "acquiring $FILE did not succeed"
}
printf '%s\n' "$acquired" | sed 's/^/  /'
case "$acquired" in
    *"verified against the hub's digest"*) ;;
    *) fail "the artifact was not verified against the digest the hub declared" ;;
esac

held="$machine/mcf/models/$REPOSITORY/$FILE"
[ -f "$held" ] || fail "the model is not where MCF said it put it"
# The bytes are a model, not an error page wearing its name: GGUF's own magic.
[ "$(head -c 4 "$held")" = "GGUF" ] || fail "what arrived does not begin with GGUF"
[ -f "$held.mcf-provenance.json" ] || fail "no provenance was written beside it"

printf '\n=== what MCF is holding\n'
listed=$("$mcf" list 2>&1) || fail "listing what is held did not succeed"
printf '%s\n' "$listed" | sed 's/^/  /'
case "$listed" in
    *"$REPOSITORY"*) ;;
    *) fail "the model is held and the listing does not say where it came from" ;;
esac

# D31's claim, against weights MCF did not write. Everywhere else the stand-in
# runs a fixture the laboratory built, which is MCF checking its own arithmetic
# against its own file; this is a real model from a real publisher, read and run
# end to end. A19: nobody should believe numbers from software that cannot
# demonstrate it computes what it claims — and the first thing to demonstrate is
# that it computes anything at all on somebody else's weights.
printf '\n=== and running it, with the engine MCF wrote\n'
# **The engine is named** (F103). The heading says *the engine MCF wrote*, and
# with no engine named this ran whatever daemon happened to be listening — which
# on a machine with a provisioned one would have demonstrated somebody else's
# engine under MCF's own claim.
answered=$("$mcf" run "$REPOSITORY:$FILE" --prompt "once upon a time" --limit 16 \
    --engine stand-in 2>&1) \
    || fail "the stand-in would not run a real model: $answered"
printf '%s\n' "$answered" | head -3 | sed 's/^/  /'
case "$answered" in
    *MARKED*) ;;
    *) fail "an answer from the stand-in arrived without its mark (B65, A5)" ;;
esac
case "$answered" in
    *"can never be a speed"*) ;;
    *) fail "an answer from the stand-in did not say what it cannot be (B65)" ;;
esac
# Something came out: the first line is the text, and an empty one would mean
# the reader and the forward pass agreed on nothing.
first=$(printf '%s' "$answered" | head -1)
[ -n "$first" ] || fail "the model produced no text at all"

printf '\n=== and removing it\n'
removed=$("$mcf" rm "$REPOSITORY/$FILE" --because "the online check is done with it" --purge 2>&1) \
    || fail "removing it did not succeed"
printf '%s\n' "$removed" | sed 's/^/  /'
[ -f "$held" ] && fail "the model is still there after a purge"

printf '\n=== the record\n'
record="$machine/mcf/record.jsonl"
[ -f "$record" ] || fail "nothing was recorded"
grep -q artifact_acquired "$record" || fail "the acquisition is not in the record"
grep -q artifact_removed "$record" || fail "the removal is not in the record"
printf '  %s lines, both events present\n' "$(wc -l <"$record")"

printf '\n=== planning for a repository nobody would download to test with\n'
planned=$("$mcf" pull "$AWKWARD" 2>&1) || fail "listing $AWKWARD did not succeed"
printf '%s\n' "$planned" | grep -E 'publishes|licence|at [0-9]+ tokens' | sed 's/^/  /'
case "$planned" in
    *"cannot say which of these would run here"*)
        fail "the plan for $AWKWARD could not be made: $(printf '%s' "$planned" | tail -3)" ;;
esac
case "$planned" in
    *"at 4096 tokens of context"*) ;;
    *) fail "no plan was produced for $AWKWARD" ;;
esac
# Every variant classified, none fetched: the count is the check. A plan missing
# a row nobody mentioned is worse than no plan (A1).
variants=$(printf '%s' "$planned" | grep -c ' — fits\| — does NOT fit\| — fits at a shorter context')
published=$(printf '%s' "$planned" | grep -c '\.gguf — [0-9]* bytes')
[ "$variants" -eq "$published" ] ||
    fail "$published models are published and $variants were classified"
printf '  %s variants classified, nothing fetched\n' "$variants"

printf '\nMCF acquired a real model from the real hub, verified it against the digest\n'
printf 'the hub declared, listed it with its provenance, ran it, and removed it —\n'
printf 'and planned for a repository whose shape found three defects (F16).\n'
