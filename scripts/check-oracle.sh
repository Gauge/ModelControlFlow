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
# **The forward pass is compared too, and the tolerance was measured rather than
# assumed** (F27). Greedy generation is deterministic, so two correct
# implementations should produce the same tokens — except where the best and
# second-best logits are close enough that a different summation order picks a
# different winner. That is not a defect in either and never will be.
#
# What separates the two is the *margin* between the best and second-best logit
# at the step where they part. Measured over fifteen comparisons: the four
# divergences that were float noise happened at margins of 0.040, 0.098, 0.105
# and 0.159, in every case with the reference choosing exactly MCF's runner-up.
# The one real defect found — a sliding-window rotary base MCF was not applying
# — diverged at a margin of 0.775.
#
# So the rule below is: if the texts differ *and* every step had a comfortable
# margin, something is wrong. If any step was close, the divergence is
# explainable and is reported rather than failed. The threshold sits between the
# largest observed noise and the smallest observed defect, and it is provisional
# — a defect can hide under a near-tie, and only more comparisons narrow it.
#
# **The reference is a development instrument and is not vendored.** Nothing
# here ships, nothing here is on the path of any MCF command, and MCF's own
# engine runs with none of it present (D39's fourth condition). What MCF may
# *provision* for itself is DEC-052 and is not settled; until it is, this check
# asks for a build that is already there and says how to make one when it is
# not.
#
# Exit status: 0 when every comparison agreed, or differed only where a margin
# was close; 1 when one differed with room to spare; 2 when the check could not
# be made — no reference build, or no corpus.

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
# The margin below which a different choice is explained by arithmetic rather
# than by a defect. See the note above for the fifteen measurements this sits
# between; it is deliberately nearer the noise than the defect.
readonly CLOSE_ENOUGH=0.50

# What to generate from, and how far. Ten tokens is enough for a divergence to
# show and short enough that five models finish in a minute.
readonly GENERATE_FROM=(
    'The capital of France is'
    'The opposite of hot is'
    'Water freezes at a temperature of'
)
readonly GENERATE_TOKENS=10

# The embedding agreement floor, measured before it was set (F29). MCF
# dequantizes to floats and multiplies; the reference multiplies in quantized
# arithmetic, quantizing the activations too — so the two vectors are near and
# cannot be equal. Across five texts the worst cosine observed was 0.999596;
# a structural defect (a wrong normalization, a missed bias, the wrong pooling)
# moves cosine by orders of magnitude more than that. The floor sits well under
# the observed agreement and far above what any defect leaves standing.
readonly EMBED_FLOOR=0.999

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

# The reference comes from `mcf provision llama.cpp` (B-367): a prefix under
# MCF's data home holding the source at the pinned commit, the build, and the
# provenance of both. `MCF_ORACLE` names a checkout elsewhere for a machine
# that built one by hand — which F30 measured as the route that silently picks
# up whatever the PATH resolves, so the provisioned one is looked for first.
data=${XDG_DATA_HOME:-${HOME:-}/.local/share}
provisioned="$data/mcf/provisioned/llama.cpp@${REFERENCE_COMMIT:0:12}"
if [ -f "$provisioned/mcf-provenance.json" ]; then
    oracle="$provisioned"
    source_dir="$provisioned/source"
else
    oracle=${MCF_ORACLE:-}
    source_dir="$oracle"
fi
tokenize_reference="$oracle/build/bin/llama-tokenize"
[ -n "$oracle" ] && [ -x "$tokenize_reference" ] || fail_cannot_check "no reference build
  \`mcf provision llama.cpp\` builds one at the pinned commit (B-367), or set MCF_ORACLE
  to a checkout that already holds a build"

tokenize_mcf="$root/target/release/examples/tokenize"
[ -x "$tokenize_mcf" ] || fail_cannot_check "no MCF build at $tokenize_mcf
  cargo build --release -p mcf-standin --example tokenize"

# Which commit the reference build is, said rather than assumed: a build
# directory outlives the checkout that made it.
if built_at=$(git -c "safe.directory=$source_dir" -C "$source_dir" rev-parse HEAD 2>/dev/null); then
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

# ── the long context, on request ────────────────────────────────────────────
#
# `MCF_ORACLE_LONG=1` adds a comparison whose prompt is longer than the models'
# sliding windows, so that the window *mask* is exercised rather than only the
# rotary base (F28 closed that hole in F27). It is off by default because the
# stand-in pays one forward pass per prompt token: ~700 passes is minutes, and
# a tier that costs minutes by default is a tier that stops being run.

# ── the forward pass ────────────────────────────────────────────────────────

completion_reference="$oracle/build/bin/llama-completion"
margins="$root/target/release/examples/margins"
mcf="$root/target/release/mcf"

if [ ! -x "$completion_reference" ] || [ ! -x "$margins" ] || [ ! -x "$mcf" ]; then
    printf '\n  the forward pass was not compared: needs llama-completion, MCF'"'"'s margins\n'
    printf '  example and the mcf binary\n'
else
    printf '\n'
    for model in "${models[@]}"; do
        name=$(basename "$model")
        "$mcf" run "$model" --prompt "A" --limit 1 >/dev/null 2>&1 || continue

        prompts=("${GENERATE_FROM[@]}")
        if [ "${MCF_ORACLE_LONG:-0}" = "1" ]; then
            filler=$(printf 'Water flows down the hill and into the sea where the waves roll on. %.0s' \
                $(seq 1 40))
            prompts+=("My name is Konstantin Aurelio Blackwood and I live in a lighthouse in Norway. ${filler}My name is")
        fi
        for prompt in "${prompts[@]}"; do
            flatten() { sed -e 's/[[:space:]]\+/ /g' | tr -d '\n' | sed 's/^ *//; s/ *$//'; }
            mine=$("$mcf" run "$model" --prompt "$prompt" --limit "$GENERATE_TOKENS" 2>&1 |
                sed -n '/── what produced it/q;p' | flatten || true)
            theirs=$("$completion_reference" -m "$model" -p "$prompt" -n "$GENERATE_TOKENS" \
                --temp 0 --seed 0 --no-warmup -ngl 0 -no-cnv --no-display-prompt 2>/dev/null |
                flatten || true)
            # A model that produced nothing on either side is not a comparison.
            if [ -z "$mine" ] || [ -z "$theirs" ]; then
                printf '  %-40s not compared on %s: one side produced nothing\n' "$name" "$prompt"
                continue
            fi

            compared=$((compared + 1))
            if [ "$mine" = "$theirs" ]; then
                continue
            fi

            # They differ. Whether that is a defect depends on how close the
            # closest choice was, so ask.
            # `|| true` on every one of these: `pipefail` plus `errexit` would
            # otherwise end the whole run at the first model whose margins
            # cannot be taken, and a check that stops before its summary is a
            # check that cannot report (A4).
            closest=$("$margins" "$model" "$prompt" "$GENERATE_TOKENS" 2>/dev/null |
                awk 'NR > 1 && $2 != "" { if (min == "" || $2 < min) min = $2 } END { print min + 0 }' ||
                true)
            [ -n "$closest" ] || closest=0
            if awk -v m="$closest" -v t="$CLOSE_ENOUGH" 'BEGIN { exit !(m < t) }'; then
                printf '  %-40s differs on %s\n' "$name" "$prompt"
                printf '      explained: the closest choice had a margin of %s, under %s\n' \
                    "$closest" "$CLOSE_ENOUGH"
                continue
            fi
            disagreements=$((disagreements + 1))
            printf '  %-40s DIFFERS on %s WITH ROOM TO SPARE\n' "$name" "$prompt"
            printf '      closest margin %s, over %s — this is not a near-tie\n' \
                "$closest" "$CLOSE_ENOUGH"
            printf '      MCF:       %s\n' "$mine"
            printf '      reference: %s\n' "$theirs"
        done
    done
fi

# ── the embedding path ──────────────────────────────────────────────────────

embedding_reference="$oracle/build/bin/llama-embedding"
if [ -x "$embedding_reference" ] && [ -x "$mcf" ]; then
    printf '\n'
    for model in "${models[@]}"; do
        name=$(basename "$model")
        # Only models MCF can embed: the ones `mcf embed` serves.
        line=$("$mcf" embed "$model" --text "A" 2>/dev/null | sed -n '1p' || true)
        case "$line" in '{"width"'*) ;; *) continue ;; esac

        for text in "${TEXTS[@]}"; do
            rendered=$(printf '%b' "$text")
            [ -n "$rendered" ] || continue
            mine=$("$mcf" embed "$model" --text "$rendered" 2>/dev/null | sed -n '1p' |
                sed 's/.*"embedding":\[//; s/\]}//' || true)
            theirs=$("$embedding_reference" -m "$model" -p "$rendered" \
                --embd-output-format json --embd-normalize 2 -ngl 0 --no-warmup 2>/dev/null |
                grep -o '"embedding": \[[^]]*\]' | sed 's/.*\[//; s/\]//' || true)
            if [ -z "$mine" ] || [ -z "$theirs" ]; then
                printf '  %-40s not compared on %s: one side produced nothing\n' "$name" "$rendered"
                continue
            fi
            compared=$((compared + 1))
            cosine=$(printf '%s\n%s\n' "$mine" "$theirs" | awk -F',' '
                NR == 1 { for (i = 1; i <= NF; i++) a[i] = $i; n = NF }
                NR == 2 { dot = 0; for (i = 1; i <= NF && i <= n; i++) dot += a[i] * $i;
                          printf "%.6f", dot }')
            if awk -v c="$cosine" -v f="$EMBED_FLOOR" 'BEGIN { exit !(c >= f) }'; then
                continue
            fi
            disagreements=$((disagreements + 1))
            printf '  %-40s EMBEDS DIFFERENTLY on %s\n' "$name" "$rendered"
            printf '      cosine %s, under the %s floor — quantized-against-float arithmetic\n' \
                "$cosine" "$EMBED_FLOOR"
            printf '      does not reach this far down; something structural does (F29)\n'
        done
    done
fi

printf '\n'
if [ "$skipped" -gt 0 ]; then
    printf '%d model(s) were not compared because MCF refuses their vocabulary\n' "$skipped"
fi
if [ "$disagreements" -gt 0 ]; then
    printf 'the oracle: %d of %d comparisons disagreed\n' "$disagreements" "$compared" >&2
    printf 'each of these is either a list of identifiers, where there is no tolerance to\n' >&2
    printf 'argue about, or a generation that parted while every choice still had room to\n' >&2
    printf 'spare — which is not what a near-tie looks like (F27)\n' >&2
    exit "$EXIT_FAILED"
fi
printf 'the oracle: MCF and the reference agree on all %d comparisons, or differ only\n' "$compared"
printf 'where the choice was closer than %s (B-368, F27)\n' "$CLOSE_ENOUGH"
