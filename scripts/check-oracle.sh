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
# **at the step where they part** — and only there. F27's first rule took the
# smallest margin anywhere in the generation, and F32 found what that lets
# through: a broken Q3_K decoder diverged at its first token with a margin of
# 0.449 and was excused by a 0.021 five tokens later. Now `margins --against`
# finds the step at which MCF's text stops being a prefix of the reference's
# and reports the margin there.
#
# Measured across sixteen files: noise divergences part at margins from 0.017
# to 0.320 — the largest on a Q2_K file, where the reference multiplies
# two-bit weights against eight-bit activations and MCF multiplies floats, so
# the arithmetic gap is widest (F33). The two real defects parted at 0.449
# (F32) and 0.775 (F27). The threshold sits between the largest noise and the
# smallest defect, and the gap it sits in is now 0.320 to 0.449: thin, stated,
# and the reason a comparison of logits rather than texts is on the register
# (B-373). It is provisional in the direction it has always been: a defect
# that happens to diverge at a genuine near-tie still passes.
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
readonly CLOSE_ENOUGH=0.40

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

# The distribution floor: the KL divergence of the reference from MCF over the
# reference's top-20 tokens at a step, above which the two are not the same
# distribution to within arithmetic. Measured (F34): a clean engine's maximum
# across sixteen files is 0.113; a swapped rotation's median is 0.32 and a
# broken Q3_K decoder's is in the tens. Under twice the clean maximum, a third
# of the subtle defect's median.
readonly LOGPROB_FLOOR=${MCF_LOGPROB_FLOOR:-0.20}

# Which sections run. Every one by default; `MCF_ORACLE_SECTIONS=distributions`
# runs only the distribution comparison, which is how its floor was measured
# without paying the ten minutes of text comparison each time (F34). A
# measuring convenience, not a tier setting: the tier is the whole file.
readonly SECTIONS=${MCF_ORACLE_SECTIONS:-all}
runs() { [ "$SECTIONS" = "all" ] || [ "$SECTIONS" = "$1" ]; }

# Runs of whitespace become one space, newlines included — `sed` works a line
# at a time and never sees a newline as whitespace, so the newline is folded
# to a space *first*. The instrument that finds where two texts part applies
# the same rule, and the two must agree or a paragraph break reads as a
# divergence (F32). Defined once, up here: it was inside the generation loop
# once, and a run of the distributions alone found it missing (F34).
flatten() { tr '\n' ' ' | sed -e 's/[[:space:]]\+/ /g; s/^ *//; s/ *$//'; }

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

# shellcheck source=scripts/lib-tiers.sh
. "$root/scripts/lib-tiers.sh"

# The binary this tier just built answers, not a daemon somebody left running
# (F104). `--engine stand-in` below pins which engine; this pins which *build*
# of it, which the account cannot tell a reader either way.
runtime=$(tier_private_runtime_dir)
trap 'rm -rf "$runtime"' EXIT
export XDG_RUNTIME_DIR="$runtime"

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

# **Where a generation divergence waits for the better instrument** (B-393,
# F107).
#
# The generation comparison decides with MCF's *own* margin: the gap between
# the token it chose and its runner-up. F27 measured that threshold on models
# up to 1.7B and it held four times. On a 4B model it produced a false
# positive, and the reason is that the margin is the wrong side's number — at
# the step where the two engines parted, MCF's margin was 0.71 and the
# *reference's* own top-two gap was 0.0199. The decision was a coin flip; MCF's
# logits, shifted by a third of a logit in each direction (well inside
# arithmetic), turned the tie into a clear win *in MCF's ranking* and the
# margin measured that.
#
# The distributions section already had the answer and was running beside it:
# KL 0.0520 against a floor of 0.20 — the same two engines, agreeing. Two
# instruments, one question, and the weaker one decided.
#
# So a generation divergence is now recorded here and resolved by the
# distribution comparison at that step. One that nothing resolves is still a
# disagreement, and says which instrument it rests on.
pending=$(mktemp "${TMPDIR:-/tmp}/mcf-oracle-pending-XXXXXX")
resolved=$(mktemp "${TMPDIR:-/tmp}/mcf-oracle-resolved-XXXXXX")
trap 'rm -f "$pending" "$resolved"' EXIT

for model in "${models[@]}"; do
    runs tokenizer || break
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

# **Which engine `mine` comes from, stated rather than inherited (F103, F102,
# F46, §3.12).**
#
# This whole check is A19's mechanical form: `mine` must be *MCF's own engine*
# and `theirs` the reference. `mcf run` with no engine named uses whatever
# daemon is listening — and where that daemon serves the provisioned llama.cpp,
# `mine` and `theirs` become the same implementation and the oracle compares the
# reference with itself, reporting agreement while establishing nothing.
#
# That is worse than a flaky check. It is a check that passes *because* it has
# stopped testing, in the one place the rest of this repository's confidence
# rests on.
readonly MINE_ENGINE=stand-in
margins="$root/target/release/examples/margins"
mcf="$root/target/release/mcf"

if ! runs generation; then
    :
elif [ ! -x "$completion_reference" ] || [ ! -x "$margins" ] || [ ! -x "$mcf" ]; then
    printf '\n  the forward pass was not compared: needs llama-completion, MCF'"'"'s margins\n'
    printf '  example and the mcf binary\n'
else
    printf '\n'
    generation_skipped=0
    for model in "${models[@]}"; do
        name=$(basename "$model")
        # The gate is *can MCF's own engine read this model*, which is what the
        # comparison needs. Asked of a daemon it would be a question about
        # whatever engine that daemon serves (F103).
        #
        # **A skip is counted and reported** (A4). It was silent, and a section
        # that compared nothing looked exactly like one that passed: when a
        # wedged daemon made this gate fail for every model, the generation
        # comparison ran zero times and the summary still read "agree on all
        # 142 comparisons" — those 142 being distributions alone.
        if ! "$mcf" run "$model" --prompt "A" --limit 1 --engine "$MINE_ENGINE" \
            >/dev/null 2>&1; then
            printf '  %-40s not compared: MCF'"'"'s own engine will not read it\n' "$name"
            generation_skipped=$((generation_skipped + 1))
            continue
        fi

        prompts=("${GENERATE_FROM[@]}")
        if [ "${MCF_ORACLE_LONG:-0}" = "1" ]; then
            filler=$(printf 'Water flows down the hill and into the sea where the waves roll on. %.0s' \
                $(seq 1 40))
            prompts+=("My name is Konstantin Aurelio Blackwood and I live in a lighthouse in Norway. ${filler}My name is")
        fi
        for prompt in "${prompts[@]}"; do
            mine=$("$mcf" run "$model" --prompt "$prompt" --limit "$GENERATE_TOKENS" \
                --engine "$MINE_ENGINE" 2>&1 |
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
            verdict=$("$margins" "$model" "$prompt" "$GENERATE_TOKENS" --against "$theirs" 2>/dev/null ||
                true)
            case "$verdict" in
            "diverged at "*)
                closest=${verdict##* margin }
                where=${verdict#diverged at }
                where=${where%% *}
                ;;
            *)
                # The texts differed and the instrument found no divergence —
                # which means the difference is past the tokens generated, or
                # in something the flattening hid. Said, not failed.
                printf '  %-40s differs on %s beyond what was compared\n' "$name" "$prompt"
                continue
                ;;
            esac
            if awk -v m="$closest" -v t="$CLOSE_ENOUGH" 'BEGIN { exit !(m < t) }'; then
                printf '  %-40s differs on %s\n' "$name" "$prompt"
                printf '      explained: at step %s, where they part, the margin was %s, under %s\n' \
                    "$where" "$closest" "$CLOSE_ENOUGH"
                continue
            fi
            # Recorded rather than counted. MCF's margin says the model was
            # not indifferent *in MCF's arithmetic*; whether the two engines
            # actually disagree is a question about both of them, and the
            # distributions section is the instrument that asks it (F107).
            printf '%s\t%s\t%s\t%s\n' "$model" "$prompt" "$where" "$closest" >>"$pending"
            printf '  %-40s differs on %s with room to spare by MCF'"'"'s own margin\n' \
                "$name" "$prompt"
            printf '      at step %s, where they part, MCF'"'"'s margin was %s, over %s\n' \
                "$where" "$closest" "$CLOSE_ENOUGH"
            printf '      MCF:       %s\n' "$mine"
            printf '      reference: %s\n' "$theirs"
            printf '      pending the distribution comparison at that step, which asks about\n'
            printf '      both engines rather than about MCF'"'"'s ranking alone (F107)\n'
        done
    done
fi

# ── the distributions (B-373) ───────────────────────────────────────────────
#
# The comparison of texts above reads a distribution through one sample, and
# its threshold has narrowed three findings in a row (F27, F32, F33) because
# noise and defects are measured in the same unit — MCF's own margin. This
# compares the distributions themselves: the reference's top-N log-probabilities
# at a step, from `llama-server`'s `n_probs`, against MCF's log-softmax for the
# same tokens at the same step. A defect is a different vector; noise is the
# same vector to within arithmetic; and the two do not share a scale.
#
# It is measured at step 0 for every prompt — agreement or not — so the noise
# floor comes from every comparison rather than only from the divergences, and
# again at the parting step when the texts differ.

server_reference="$oracle/build/bin/llama-server"
if runs distributions && [ -x "$server_reference" ] && [ -x "$margins" ] && command -v jq >/dev/null 2>&1 && command -v curl >/dev/null 2>&1; then
    printf '\n'
    port=18765
    for model in "${models[@]}"; do
        name=$(basename "$model")
        # The gate is *can MCF's own engine read this model*, which is what the
        # comparison below needs. Asked of a daemon it would be a question about
        # whatever engine that daemon serves (F103).
        "$mcf" run "$model" --prompt "A" --limit 1 --engine "$MINE_ENGINE" >/dev/null 2>&1 ||
            continue

        # One server per model, on loopback, killed before the next.
        port=$((port + 1))
        "$server_reference" -m "$model" --host 127.0.0.1 --port "$port" -ngl 0 --no-warmup \
            --log-disable >/dev/null 2>&1 &
        server_pid=$!
        ready=0
        for _ in $(seq 1 100); do
            if curl -s "http://127.0.0.1:$port/health" 2>/dev/null | grep -q '"ok"'; then
                ready=1
                break
            fi
            sleep 0.2
        done
        if [ "$ready" -ne 1 ]; then
            printf '  %-40s the reference server did not come up; distributions not compared\n' "$name"
            kill "$server_pid" 2>/dev/null || true
            wait "$server_pid" 2>/dev/null || true
            continue
        fi

        for prompt in "${GENERATE_FROM[@]}"; do
            # The reference's generation with its top-20 at every step.
            response=$(curl -s "http://127.0.0.1:$port/completion" -H 'Content-Type: application/json' \
                -d "$(jq -cn --arg p "$prompt" --argjson n "$GENERATE_TOKENS" \
                    '{prompt: $p, n_predict: $n, temperature: 0, seed: 0, n_probs: 20, cache_prompt: false, return_tokens: true}')" \
                2>/dev/null || true)
            [ -n "$response" ] || continue
            theirs=$(printf '%s' "$response" | jq -r '.content' | flatten)

            # Which step to compare: 0 always; the parting step too when they differ.
            steps_to_compare="0"
            verdict=$("$margins" "$model" "$prompt" "$GENERATE_TOKENS" --against "$theirs" 2>/dev/null || true)
            case "$verdict" in
            "diverged at "*)
                parted=${verdict#diverged at }
                parted=${parted%% *}
                [ "$parted" != "0" ] && steps_to_compare="0 $parted"
                ;;
            esac

            for step in $steps_to_compare; do
                # The reference's top tokens at this step, and their log-probabilities.
                ref_ids=$(printf '%s' "$response" | jq -r ".completion_probabilities[$step].top_logprobs | map(.id) | join(\",\")" 2>/dev/null || true)
                [ -n "$ref_ids" ] && [ "$ref_ids" != "null" ] || continue
                # At step 0 the instrument prints step 0; at a parting step it
                # needs the reference text to find that step — without it the
                # first measurement compared MCF's step 0 against the
                # reference's step 9 and reported gaps of twenty (F34).
                if [ "$step" = "0" ]; then
                    mine_json=$("$margins" "$model" "$prompt" 1 --logprobs-of "$ref_ids" 2>/dev/null || true)
                else
                    mine_json=$("$margins" "$model" "$prompt" "$GENERATE_TOKENS" --against "$theirs" \
                        --logprobs-of "$ref_ids" 2>/dev/null || true)
                fi
                case "$mine_json" in '{"step"'*) ;; *) continue ;; esac

                # A distribution at step N is comparable only if both sides
                # reached step N through the same tokens. Text agreement does
                # not guarantee it — two tokenizations of one string — so the
                # reference's generated tokens are checked against MCF's chosen
                # ones up to the step, and a mismatch is said rather than
                # scored (F34).
                if [ "$step" != "0" ]; then
                    ref_prefix=$(printf '%s' "$response" | jq -c --argjson s "$step" '.tokens[:$s]' 2>/dev/null || echo '[]')
                    mine_prefix=$(printf '%s' "$mine_json" | jq -c '.chosen_before' 2>/dev/null || echo '[]')
                    if [ "$ref_prefix" != "$mine_prefix" ]; then
                        printf '  %-40s parts on %s at step %s through different tokens; distributions not comparable there\n' \
                            "$name" "$prompt" "$step"
                        continue
                    fi
                fi

                # Three views of the same two distributions over the
                # reference's top tokens: the largest gap anywhere in the top
                # twenty, the largest gap in the top five, and the divergence
                # of the reference from MCF over the top twenty (renormalized).
                # Which one the verdict uses was decided by measuring all three
                # against a clean engine and two known defects (F34).
                stats=$(printf '%s' "$response" | jq -r --argjson mine "$mine_json" --argjson s "$step" '
                    [ .completion_probabilities[$s].top_logprobs[]
                      | select(($mine.logprobs[(.id|tostring)]) != null)
                      | {r: .logprob, m: $mine.logprobs[(.id|tostring)]} ] as $pairs
                    | ($pairs | map((.r - .m) | fabs) | max // 0) as $max20
                    | ($pairs[:5] | map((.r - .m) | fabs) | max // 0) as $max5
                    | ($pairs | map(.r | exp) | add) as $zr
                    | ($pairs | map(.m | exp) | add) as $zm
                    | ($pairs | map( ((.r | exp) / $zr) * ((.r - ($zr|log)) - (.m - ($zm|log))) ) | add // 0) as $kl
                    | "\($max20) \($max5) \($kl)"' 2>/dev/null || true)
                [ -n "$stats" ] || continue
                read -r gap20 gap5 divergence <<<"$stats"
                distance=$divergence
                printf '  %-40s distributions on %s at step %s: top20 %.3f top5 %.3f kl %.4f\n' \
                    "$name" "$prompt" "$step" "$gap20" "$gap5" "$divergence" >&2
                compared=$((compared + 1))

                # **The reference's own top-two gap at this step** — the number
                # that says whether the *decision* was a coin flip, which MCF's
                # margin cannot say about the reference's ranking (F107).
                theirmargin=$(printf '%s' "$response" | jq -r --argjson s "$step" '
                    .completion_probabilities[$s].top_logprobs as $t
                    | if ($t | length) > 1 then ($t[0].logprob - $t[1].logprob) else 0 end' \
                    2>/dev/null || true)

                # A generation divergence waiting on this step, resolved by the
                # instrument that looks at both engines (B-393, F107).
                waiting=$(awk -F'\t' -v m="$model" -v p="$prompt" -v s="$step" \
                    '$1 == m && $2 == p && $3 == s { print; exit }' "$pending" 2>/dev/null || true)
                if [ -n "$waiting" ]; then
                    printf '%s\t%s\n' "$waiting" "$distance" >>"$resolved"
                    if awk -v d="$distance" -v f="$LOGPROB_FLOOR" 'BEGIN { exit !(d <= f) }'; then
                        printf '  %-40s resolved on %s at step %s: the two engines agree here\n' \
                            "$name" "$prompt" "$step"
                        printf '      KL %s, under %s, and the reference'"'"'s own top-two gap was\n' \
                            "$distance" "$LOGPROB_FLOOR"
                        printf '      %s — the decision was a near-tie for the reference, and MCF'"'"'s\n' \
                            "$theirmargin"
                        printf '      margin measured its own ranking rather than the disagreement\n'
                    else
                        printf '  %-40s CONFIRMED on %s at step %s: the generation parted and the\n' \
                            "$name" "$prompt" "$step"
                        printf '      distributions differ there too\n'
                    fi
                fi

                if awk -v d="$distance" -v f="$LOGPROB_FLOOR" 'BEGIN { exit !(d <= f) }'; then
                    continue
                fi
                disagreements=$((disagreements + 1))
                printf '  %-40s DISTRIBUTIONS DIFFER on %s at step %s\n' "$name" "$prompt" "$step"
                printf '      KL of the reference from MCF over its top twenty: %s, over %s\n' \
                    "$distance" "$LOGPROB_FLOOR"
                printf '      (largest log-probability gaps: %s over the top twenty, %s over the top five)\n' \
                    "$gap20" "$gap5"
            done
        done
        kill "$server_pid" 2>/dev/null || true
        wait "$server_pid" 2>/dev/null || true
    done
fi

# ── the embedding path ──────────────────────────────────────────────────────

embedding_reference="$oracle/build/bin/llama-embedding"
if runs embeddings && [ -x "$embedding_reference" ] && [ -x "$mcf" ]; then
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

# ── agreement at length, by teacher forcing (B-377, F40) ────────────────────
#
# Every comparison above stops being possible at the first divergence: past it
# the two engines are writing different sentences, and nothing downstream is
# the same question. So *whether agreement decays with position* had never been
# asked, and position is where a rotary encoding or a cache would go wrong.
#
# Here MCF is made to read the reference's own tokens rather than its own. At
# every position it sees exactly the reference's prefix and is asked what comes
# next, which stays comparable however far the free generations have drifted.
# F40 ran it across 700 positions of gemma-3-270m and found the sliding-window
# attention correct past its own 512-token window — 189 consecutive agreements,
# on a mechanism nothing had reached because nothing had generated that far.
#
# **This section reads the distribution rule, not the margin rule.** The 0.40
# margin threshold was measured on one parting step per file (F27, F33); at
# every position of a long comparison it is a different test with a different
# rate of false alarm, and F40 watched it raise two where the distributions
# agreed with room to spare. What is asserted here is the rank the reference's
# token got: a top-two order swap is arithmetic, and a reference token MCF
# ranks tenth is not.
#
# It is off by default because it costs minutes per model — `MCF_ORACLE_FORCED=1`
# asks for it. `MCF_ORACLE_FORCED_TOKENS` sets how far, and the default is past
# the widest sliding window in the corpus.
readonly FORCED_TOKENS=${MCF_ORACLE_FORCED_TOKENS:-700}
readonly FORCED_WORST_RANK=8
readonly FORCED_PROMPT='The history of the city of Paris begins'

if runs forced && [ "${MCF_ORACLE_FORCED:-0}" = "1" ] && [ -x "$server_reference" ] &&
    [ -x "$margins" ] && command -v jq >/dev/null 2>&1 && command -v curl >/dev/null 2>&1; then
    printf '\n'
    port=18766
    for model in "${models[@]}"; do
        name=$(basename "$model")
        # As above: the gate asks MCF's own engine (F103).
        "$mcf" run "$model" --prompt "A" --limit 1 --engine "$MINE_ENGINE" >/dev/null 2>&1 ||
            continue

        "$server_reference" -m "$model" --port "$port" --host 127.0.0.1 -ngl 0 \
            --no-webui >/dev/null 2>&1 &
        server=$!
        ready=0
        for _ in $(seq 1 120); do
            if curl -sf "http://127.0.0.1:$port/health" >/dev/null 2>&1; then ready=1; break; fi
            sleep 1
        done
        if [ "$ready" -ne 1 ]; then
            kill "$server" 2>/dev/null || true
            wait "$server" 2>/dev/null || true
            printf '  %-40s the reference server did not become ready\n' "$name"
            continue
        fi

        # `ignore_eos` is set, and has to be. Without it the reference stops
        # where the model stops — 377 tokens on gemma-3-270m — and a window of
        # 512 is never reached, so the check cannot exercise the mechanism it
        # exists for. That was found by running the negative control: the
        # sliding window was deliberately broken and this section returned
        # *identical* numbers (F41).
        #
        # What the flag costs is handled rather than accepted. A reference
        # forbidden to stop takes its best remaining token where MCF takes the
        # model's end of turn, and F40's largest apparent defect — a margin of
        # 6.59, an order of magnitude past any real one — was exactly that.
        # Those positions are set aside by name and counted, never silently
        # dropped (A1, A19).
        response=$(curl -sf "http://127.0.0.1:$port/completion" -H 'Content-Type: application/json' \
            -d "$(jq -cn --arg p "$FORCED_PROMPT" --argjson n "$FORCED_TOKENS" \
                '{prompt: $p, n_predict: $n, temperature: 0, seed: 0, cache_prompt: false, return_tokens: true, ignore_eos: true}')" \
            2>/dev/null || true)
        kill "$server" 2>/dev/null || true
        wait "$server" 2>/dev/null || true
        [ -n "$response" ] || continue

        ids=$(printf '%s' "$response" | jq -r 'if .tokens then (.tokens | join(",")) else empty end')
        [ -n "$ids" ] || continue

        forced=$("$margins" "$model" "$FORCED_PROMPT" --forced "$ids" 2>/dev/null || true)
        [ -n "$forced" ] || continue

        compared=$((compared + 1))
        total=$(printf '%s\n' "$forced" | jq -s 'length')
        # Positions where MCF chose the model's end of turn are not comparable:
        # the reference was forbidden to take it.
        aside=$(printf '%s\n' "$forced" | jq -s '[.[] | select(.mine_is_stop)] | length')
        disagreed=$(printf '%s\n' "$forced" |
            jq -s '[.[] | select(.mine_is_stop | not) | select(.agreed | not)] | length')
        worst=$(printf '%s\n' "$forced" |
            jq -s '[.[] | select(.mine_is_stop | not) | select(.agreed | not) | .rank_of_forced] | max // 0')

        if [ "$worst" -le "$FORCED_WORST_RANK" ]; then
            printf '  %-40s agreed at %s of %s comparable positions; where they differed\n' \
                "$name" "$((total - aside - disagreed))" "$((total - aside))"
            printf '  %-40s the reference token was never worse than MCF rank %s\n' '' "$worst"
            if [ "$aside" -gt 0 ]; then
                printf '  %-40s %s position(s) set aside: MCF took the model'"'"'s end of turn,\n' \
                    '' "$aside"
                printf '  %-40s which the reference was forbidden to take (F40)\n' ''
            fi
        else
            disagreements=$((disagreements + 1))
            printf '  %-40s RANKS THE REFERENCE TOKEN %s AT SOME POSITION\n' "$name" "$worst" >&2
            printf '      %s of %s comparable positions disagreed; a top-two swap is\n' \
                "$disagreed" "$((total - aside))" >&2
            printf '      arithmetic and this is not — the distributions differ in shape,\n' >&2
            printf '      not in order (F40, F41)\n' >&2
        fi
    done
fi

# ── the divergences nothing resolved ────────────────────────────────────────
#
# A generation divergence the distribution comparison never reached is still a
# disagreement, and it rests on MCF's own margin alone — which F107 measured
# wrong once on a 4B model. Saying which instrument a verdict rests on is A5's
# rule about degraded results applied to a check's own confidence.
unresolved=0
while IFS=$'\t' read -r model prompt step margin; do
    [ -n "$model" ] || continue
    if awk -F'\t' -v m="$model" -v p="$prompt" -v s="$step" \
        'BEGIN { found = 0 } $1 == m && $2 == p && $3 == s { found = 1 } END { exit found }' \
        "$resolved" 2>/dev/null; then
        unresolved=$((unresolved + 1))
        disagreements=$((disagreements + 1))
        printf '\n  %-40s DIFFERS on %s WITH ROOM TO SPARE\n' "$(basename "$model")" "$prompt"
        printf '      at step %s, MCF'"'"'s margin was %s, over %s — and the distributions at\n' \
            "$step" "$margin" "$CLOSE_ENOUGH"
        printf '      that step were not compared, so this verdict rests on MCF'"'"'s own\n'
        printf '      ranking rather than on both engines (F107)\n'
    fi
done <"$pending"

printf '\n'
if [ "$skipped" -gt 0 ]; then
    printf '%d model(s) were not compared because MCF refuses their vocabulary\n' "$skipped"
fi
# A4, and F103's lesson: a section that compared nothing must not read like one
# that passed. The headline counts what was compared; this counts what was not.
if [ "${generation_skipped:-0}" -gt 0 ]; then
    printf '%d model(s) were not compared on generation: MCF'"'"'s own engine would not\n' \
        "$generation_skipped"
    printf 'read them, so what came back would have been somebody else'"'"'s engine\n'
fi
# What the margin flagged and the better instrument cleared. Printed whether or
# not anything failed: a check that quietly drops a verdict it changed its mind
# about is a check nobody can audit (A1).
explained=$(wc -l <"$resolved" 2>/dev/null || echo 0)
if [ "${explained:-0}" -gt 0 ]; then
    printf '%d generation divergence(s) were flagged by MCF'"'"'s own margin and resolved by\n' \
        "$explained"
    printf 'the distribution comparison at the same step, which asks about both engines (F107)\n'
fi
if [ "$unresolved" -gt 0 ]; then
    printf '%d of them rest on MCF'"'"'s margin alone, because the distributions were not\n' \
        "$unresolved"
    printf 'compared there\n'
fi
if [ "$disagreements" -gt 0 ]; then
    printf 'the oracle: %d of %d comparisons disagreed\n' "$disagreements" "$compared" >&2
    printf 'each of these is a list of identifiers, where there is no tolerance to argue\n' >&2
    printf 'about; a generation that parted with room to spare (F27, F32); or two\n' >&2
    printf 'distributions further apart than arithmetic puts them (F34)\n' >&2
    exit "$EXIT_FAILED"
fi
printf 'the oracle: MCF and the reference agree on all %d comparisons, or differ only\n' "$compared"
printf 'where the choice was closer than %s (B-368, F27)\n' "$CLOSE_ENOUGH"
