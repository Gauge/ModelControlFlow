#!/usr/bin/env bash
#
# The conformance corpus (B-370, D40, DEC-054, F21, F22).
#
# **What this is for.** MCF's engine is developed against the smallest *trained*
# model of each family it covers or means to cover, one distinct quantization
# apiece, so that architecture coverage and quantization coverage come from the
# same handful of files. D40 amended §XII to say so: the reference model is the
# subject of provenance, frontier and residency work, and the size that makes
# residency real is the size that makes every engine iteration slow.
#
# **Every entry declares what MCF does with it today**, and the check fails
# either way round. A model that was running and now refuses is a regression. A
# model that was refusing and now runs is *also* reported, because the entry
# below is then out of date and somebody should say which family MCF covers —
# a check that quietly accepted good news would be a check that stops being
# read (A21: declared is not verified, in both directions).
#
# **A refusal is checked for what it says, not that it happened.** F21's point
# was that four refusals named four different missing things, and that list is
# the order of work. A refusal that stopped naming what it wanted would still
# be a refusal and would have lost the thing that made it useful (A2, A7).
#
# **What it is not.** Not a measurement. Nothing here is timed and nothing
# reported here is a speed: B65 forbids a stand-in from producing one, and the
# figures in F21 and F22 are recorded as what the *decision* was about rather
# than as properties of any model.
#
# **Scheduled rather than gating**, because it needs the corpus on the disk and
# a gate that needs 1.4 GB of models is a gate that fails on a fresh clone.
# `scripts/ci.sh --with-corpus` runs it.
#
# Exit status: 0 when every entry did what it says; 1 when one did not; 2 when
# the check could not be made — no corpus on this machine.

set -o errexit -o nounset -o pipefail

readonly EXIT_FAILED=1
readonly EXIT_CANNOT_CHECK=2

# The prompt, and why it is this one. Something a person knows the answer to,
# short enough to tokenize the same way under either vocabulary, and phrased so
# that the answer is the next word rather than a sentence away — F22 found that
# what separates a correct engine from a subtly wrong one is whether the answer
# appears at all, and a prompt whose answer nobody knows cannot show that.
readonly PROMPT='The capital of France is'
readonly BUDGET=12

# One line per entry: family | path under the store | state | what to expect.
#
# `runs` expects the text to contain the fourth field. `refuses` expects the
# refusal to contain it. The fourth field is what makes each line say something
# a change could contradict.
readonly CORPUS=(
    "llama, unigram vocabulary|Felladrin/gguf-Llama-160M-Chat-v1/Llama-160M-Chat-v1.Q4_K.gguf|runs|Paris"
    "qwen3|unsloth/Qwen3-0.6B-GGUF/Qwen3-0.6B-Q4_K_M.gguf|runs|Paris"
    "llama, byte-pair vocabulary|bartowski/SmolLM2-135M-Instruct-GGUF/SmolLM2-135M-Instruct-Q8_0.gguf|runs|Paris"
    "embedding|leliuga/all-MiniLM-L6-v2-GGUF/all-MiniLM-L6-v2.Q4_0.gguf|refuses|bert"
    "mixture-of-experts|RichardErkhov/Isotonic_-_TinyMixtral-4x248M-MoE-gguf/TinyMixtral-4x248M-MoE.Q5_K_M.gguf|refuses|ffn_gate"
    "gemma3|unsloth/gemma-3-270m-it-GGUF/gemma-3-270m-it-Q6_K.gguf|runs|Paris"
)

root=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)
cd "$root"

fail_cannot_check() {
    printf 'cannot check: %s\n' "$1" >&2
    exit "$EXIT_CANNOT_CHECK"
}

# Where the corpus is. `MCF_CORPUS` names it outright; otherwise the store MCF
# itself would use. Not invented: a check that guessed at a directory would be
# a check that silently tested nothing (A7).
store=${MCF_CORPUS:-}
if [ -z "$store" ]; then
    data=${XDG_DATA_HOME:-${HOME:-}/.local/share}
    store="$data/mcf/models"
fi
[ -d "$store" ] || fail_cannot_check "no corpus at $store — \`mcf pull\` the models named in findings.md F21, or set MCF_CORPUS"

mcf=$root/target/release/mcf
[ -x "$mcf" ] || fail_cannot_check "no release build at $mcf — \`cargo build --release -p mcf-cli\` first"

# The round-trip is a separate question from whether the model answers, and it
# is asked of every vocabulary that loads: a tokenizer that drops a byte changes
# the question the model was asked, and that is checkable against the text
# itself rather than against anybody's opinion of the answer (A1). What it
# cannot show is that the *cut* was the right one — see F23.
roundtrip=$root/target/release/examples/roundtrip

printf 'the conformance corpus, from %s\n\n' "$store"

failures=0
absent=0
for entry in "${CORPUS[@]}"; do
    IFS='|' read -r family relative state expected <<<"$entry"
    path="$store/$relative"

    if [ ! -f "$path" ]; then
        printf '  %-30s NOT HELD — %s\n' "$family" "$relative"
        absent=$((absent + 1))
        continue
    fi

    said=$("$mcf" run "$path" --prompt "$PROMPT" --limit "$BUDGET" 2>&1) || true
    # `mcf run` prints the answer first and its conditions after a rule; a
    # refusal prints the classified failure. Either way the first lines are
    # what this reads.
    head=$(printf '%s' "$said" | sed -n '1,6p')

    case "$state" in
    runs)
        if printf '%s' "$head" | grep -q 'did not run'; then
            printf '  %-30s REGRESSED — it ran before and now refuses:\n%s\n' "$family" "$head"
            failures=$((failures + 1))
        elif printf '%s' "$head" | grep -qF "$expected"; then
            if [ -x "$roundtrip" ] && ! "$roundtrip" "$path" >/dev/null 2>&1; then
                printf '  %-30s runs and says %s, BUT ITS VOCABULARY LOSES TEXT:\n' "$family" "$expected"
                "$roundtrip" "$path" 2>&1 | sed 's/^/      /'
                failures=$((failures + 1))
            else
                printf '  %-30s runs, says %s, and its vocabulary loses nothing\n' "$family" "$expected"
            fi
        else
            printf '  %-30s RAN AND SAID SOMETHING ELSE — expected %s:\n%s\n' "$family" "$expected" "$head"
            failures=$((failures + 1))
        fi
        ;;
    refuses)
        if ! printf '%s' "$head" | grep -q 'did not run'; then
            printf '  %-30s NOW RUNS — the entry says it refuses, and it did not. If MCF\n' "$family"
            printf '  %-30s covers this family now, say so here and in B-365.\n' ''
            failures=$((failures + 1))
        elif printf '%s' "$head" | grep -qF "$expected"; then
            printf '  %-30s refuses, naming %s\n' "$family" "$expected"
        else
            printf '  %-30s REFUSES WITHOUT SAYING WHAT IT WANTED — expected %s:\n%s\n' "$family" "$expected" "$head"
            failures=$((failures + 1))
        fi
        ;;
    *)
        printf '  %-30s the corpus entry states an unknown state: %s\n' "$family" "$state"
        failures=$((failures + 1))
        ;;
    esac
done

printf '\n'
if [ "$absent" -eq "${#CORPUS[@]}" ]; then
    fail_cannot_check "none of the corpus is held under $store"
fi
if [ "$absent" -gt 0 ]; then
    printf '%d of %d corpus entries are not held here and were not checked (A4: what was\n' "$absent" "${#CORPUS[@]}"
    printf 'not done is said rather than counted as done)\n'
fi
if [ "$failures" -gt 0 ]; then
    printf 'the corpus: %d entr%s did not do what the register says\n' \
        "$failures" "$([ "$failures" -eq 1 ] && printf 'y' || printf 'ies')" >&2
    exit "$EXIT_FAILED"
fi
printf 'the corpus: every entry held here did what it says (B-370)\n'
