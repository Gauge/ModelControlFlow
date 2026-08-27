#!/usr/bin/env bash
#
# MCF's engine against a reference implementation (B-368, D39, A19, F25).
#
# **Why this exists, in one measurement.** F25 removed the expert router from
# MCF's mixture-of-experts entirely — never scoring an expert, always taking the
# first two — and the model produced `Paris. It is located on the River Seine
# in`. The correct implementation produced `Paris, France is Paris, Paris is
# Paris is`. The broken engine read *better* than the right one. Output quality
# is not evidence about correctness in either direction (F20, F22, F24, F25), so
# every family MCF has added is transcribed from somebody else's source and
# unverified in A21's exact sense. This is what verifies it.
#
# **What is compared, and why it starts with the tokenizer.** Identifiers are
# integers. Two tokenizers either agree about them or do not, with no tolerance
# to argue about and no floating-point arithmetic in the way — which makes this
# the one part of an engine that can be checked against a reference *exactly*.
# It is also the part F23 found three defects in, all of them by reading rather
# than running, and all of them still unverified by anything but that reading.
#
# **What is deliberately not compared yet.** Logits and generated text. A
# correct implementation can still flip an argmax on a near-tie through nothing
# worse than a different summation order, so a disagreement there is a finding
# to investigate rather than a verdict, and a tier that failed on it would be a
# tier people learn to ignore. That comparison wants a tolerance nobody has
# measured yet, and measuring it is its own piece of work.
#
# **The reference is a development instrument and is not vendored.** Nothing
# here ships, nothing here is on the path of any MCF command, and MCF's own
# engine runs with none of it present (D39's fourth condition). What MCF may
# *provision* for itself is DEC-052 and is not settled; until it is, this check
# asks for a build that is already there and says how to make one when it is
# not.
#
# Exit status: 0 when every comparison agreed; 1 when one did not; 2 when the
# check could not be made — no reference build, or no corpus.

set -o errexit -o nounset -o pipefail

readonly EXIT_FAILED=1
readonly EXIT_CANNOT_CHECK=2

# The reference, pinned. A comparison against "whatever was on master that day"
# is a comparison nobody can repeat (§3.4, §3.12).
readonly REFERENCE_REPOSITORY=https://github.com/ggml-org/llama.cpp.git
readonly REFERENCE_COMMIT=925e1179947ea0c0ebfb0032df18af3a729822be

# The texts to agree about. Chosen for where tokenizers differ rather than for
# what they mean: digit runs, which the four expressions cut four different ways
# and which F23 found MCF cutting wrongly; punctuation against letters, where a
# lead character either joins or does not; contractions; characters outside
# ASCII; runs of whitespace; and a plain sentence, so that a total disagreement
# is distinguishable from an edge case.
readonly TEXTS=(
    'The capital of France is'
    'In 2024 there were 365 days and 1234567 seconds'
    "it's (parenthesized), \"quoted\" and hyphen-joined!"
    'café naïve 你好'
    'tabs	and  spaces'
    'A'
)

root=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)
cd "$root"

fail_cannot_check() {
    printf 'cannot check: %s\n' "$1" >&2
    exit "$EXIT_CANNOT_CHECK"
}

oracle=${MCF_ORACLE:-/home/gauge/Content/mcf-oracle/llama.cpp}
tokenize_reference="$oracle/build/bin/llama-tokenize"
[ -x "$tokenize_reference" ] || fail_cannot_check "no reference build at $tokenize_reference
  git clone $REFERENCE_REPOSITORY && git checkout $REFERENCE_COMMIT
  cmake -B build -DCMAKE_BUILD_TYPE=Release -DGGML_NATIVE=OFF && cmake --build build -j
  or set MCF_ORACLE to a build that is already there"

tokenize_mcf="$root/target/release/examples/tokenize"
[ -x "$tokenize_mcf" ] || fail_cannot_check "no MCF build at $tokenize_mcf
  cargo build --release -p mcf-standin --example tokenize"

# Which commit the reference build is, said rather than assumed: a build
# directory outlives the checkout that made it.
if built_at=$(git -c "safe.directory=$oracle" -C "$oracle" rev-parse HEAD 2>/dev/null); then
    if [ "$built_at" != "$REFERENCE_COMMIT" ]; then
        printf 'the reference is at %s, and this check is written against %s\n' \
            "$built_at" "$REFERENCE_COMMIT"
        printf 'a disagreement below may be a change in the reference rather than in MCF\n\n'
    fi
else
    printf 'the reference checkout could not be identified; comparing anyway\n\n'
fi

store=${MCF_CORPUS:-}
if [ -z "$store" ]; then
    data=${XDG_DATA_HOME:-${HOME:-}/.local/share}
    store="$data/mcf/models"
fi
[ -d "$store" ] || fail_cannot_check "no corpus at $store"

# Every corpus model whose vocabulary MCF can read. Discovered rather than
# listed, so that a model added to the corpus is compared without this file
# being edited.
mapfile -t models < <(find "$store" -name '*.gguf' | sort)
[ "${#models[@]}" -gt 0 ] || fail_cannot_check "no models under $store"

compared=0
disagreements=0
skipped=0

for model in "${models[@]}"; do
    name=$(basename "$model")
    # A model whose vocabulary MCF refuses is not a disagreement — it is a
    # refusal, which the corpus tier already checks. Saying so and moving on is
    # A4: what was not done is reported rather than counted as done.
    if ! "$tokenize_mcf" "$model" "A" --ids >/dev/null 2>&1; then
        printf '  %-40s not compared: MCF refuses this vocabulary\n' "$name"
        skipped=$((skipped + 1))
        continue
    fi

    for text in "${TEXTS[@]}"; do
        rendered=$(printf '%b' "$text")
        mine=$("$tokenize_mcf" "$model" "$rendered" --ids 2>/dev/null || true)
        # `--ids` on the reference prints one identifier per line; `--no-bos` is
        # not passed because MCF adds the beginning token the file asks for and
        # the comparison must be of the same thing.
        # The reference prints `[785, 6722, 315, 9625, 374]`. Only the brackets
        # and commas are removed — stripping the spaces too would run every
        # identifier into one number, which is a comparison that fails on
        # everything and looks like a total disagreement.
        theirs=$("$tokenize_reference" -m "$model" -p "$rendered" --ids 2>/dev/null |
            tr -d '[],"' | tr '\n' ' ' | tr -s ' ' | sed 's/^ *//; s/ *$//' || true)

        compared=$((compared + 1))
        if [ "$mine" = "$theirs" ]; then
            continue
        fi
        disagreements=$((disagreements + 1))
        printf '  %-40s DISAGREES on %s\n' "$name" "$rendered"
        printf '      MCF:       %s\n' "$mine"
        printf '      reference: %s\n' "$theirs"
    done
done

printf '\n'
if [ "$skipped" -gt 0 ]; then
    printf '%d model(s) were not compared because MCF refuses their vocabulary\n' "$skipped"
fi
if [ "$disagreements" -gt 0 ]; then
    printf 'the oracle: %d of %d comparisons disagreed\n' "$disagreements" "$compared" >&2
    printf 'a disagreement here is exact: identifiers are integers and there is no tolerance\n' >&2
    exit "$EXIT_FAILED"
fi
printf 'the oracle: MCF and the reference agree on all %d comparisons (B-368)\n' "$compared"
