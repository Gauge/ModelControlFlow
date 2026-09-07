#!/usr/bin/env bash
#
# The test of the tests: mutation (B-191, B-186, D10, §6.34).
#
# D10 calls mutation testing *the test of the tests* — a suite that does not
# fail when the code is deliberately broken is a suite that proves nothing, and
# §3.5's credibility argument rests on knowing the difference. This applies a
# declared catalogue of mutations to a copy of the working tree and requires the
# suite to notice each one.
#
# **It refuses a score below the floor, and a score below the last one.** B-186:
# the mutation score is budgeted like any other property (B20) and may not
# regress silently. The floor is `MUTATION_FLOOR_PERCENT` in
# `scripts/lib-tiers.sh`, with the reasoning; the previous score comes from the
# tier's own stamp (B-185), which is where a run leaves what the next one
# compares against.
#
# **The catalogue is written by hand, not generated.** A generator produces
# thousands of mutants and most of them are noise — equivalent mutants, dead
# arithmetic, unreachable branches — and the tier's cost is one test run each.
# Each entry below is a mutation of something a rule in rules.md depends on, so
# a survivor names a rule nothing is checking.
#
# **There is an equivalent-mutant control.** A change with no semantic effect
# must NOT be killed. Without it, a runner whose test command always failed —
# a broken copy, a missing toolchain, a stray compile error — would report
# every mutant killed and mean nothing. That is the lesson
# scripts/check-lints-bite.sh states for lints, applied to this tier.
#
# **A mutant that does not compile is not a result.** It is reported as invalid
# and excluded from the score, because the question is what the *tests* notice,
# and a compiler error is the compiler noticing.
#
# **A mutant that hangs is killed, and is named as having hung.** This is not
# hypothetical: the digest mutant below takes the room left in a 64-byte buffer
# from 64 to 63, so the loop that fills it eventually takes zero bytes per pass
# and never terminates. Every suite run is therefore bounded by MUTANT_TIMEOUT,
# because a scheduled tier that one entry in its own catalogue can stop for ever
# is a tier nobody runs twice. A timeout is confirmed by a second run before it
# is believed, since a machine busy with something else can miss a deadline
# without the mutant having anything to do with it.
#
# Exit status: 0 when the score is at or above the floor and no lower than the
# last recorded one, 1 when it is below either, 2 when the run means nothing —
# the control was killed, a restore did not take, the tree would not build, or a
# tool is missing.

set -o errexit -o nounset -o pipefail

readonly EXIT_CANNOT_CHECK=2

root=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)
# shellcheck source=scripts/lib-tiers.sh
. "$root/scripts/lib-tiers.sh"

readonly EXIT_BELOW_FLOOR=1

fail_cannot_check() {
    printf 'cannot check: %s\n' "$1" >&2
    exit "$EXIT_CANNOT_CHECK"
}

command -v cargo >/dev/null 2>&1 || fail_cannot_check "cargo is not on PATH"
command -v python3 >/dev/null 2>&1 || fail_cannot_check "python3 is not on PATH"
command -v timeout >/dev/null 2>&1 || fail_cannot_check "timeout is not on PATH"

# How long one mutant's suite run may take before it is judged to have hung.
# The whole suite takes about three and a half minutes on this machine — the
# whole-system tests start real daemons and the window suite draws every page
# — and the equivalent-mutant control was judged killed by a deadline written
# when the tiers took five seconds (F261). Fifteen minutes is four times the
# suite, which is the difference between a slow machine and a mutant that does
# not terminate; a mutant that hangs still costs no more than that.
readonly MUTANT_TIMEOUT=900
readonly EXIT_TIMED_OUT=124

# The catalogue: three parallel arrays, the way check-lints-bite.sh states its
# lint table. Each `find` must occur exactly once in its file — the runner
# refuses a mutation it cannot place unambiguously, because a mutation applied
# somewhere other than where it was meant is a mutant nobody declared.
#
# Every entry mutates something a rule depends on, so a survivor names a rule
# nothing is checking. That is why the catalogue is written by hand: a generator
# produces thousands of mutants, most of them noise, and each one here costs a
# suite run.
declare -a files=(
    "crates/mcf-core/src/measurement/spread.rs"
    "crates/mcf-core/src/measurement/mod.rs"
    "crates/mcf-record/src/json.rs"
    "crates/mcf-record/src/journal/replay.rs"
    "crates/mcf-record/src/journal.rs"
    "crates/mcf-core/src/time/zone.rs"
    "crates/mcf-record/src/encode.rs"
    "crates/mcf-core/src/digest.rs"
    "crates/mcf-record/src/export.rs"
    "crates/mcf-core/src/trial/series.rs"
    "crates/mcf-core/src/failure/mod.rs"
    "crates/mcf-core/src/self_cost.rs"
    "crates/mcf-standin/src/ops.rs"
    "crates/mcf-standin/src/dequantize.rs"
    "crates/mcf-hub/src/fitment.rs"
    "crates/mcf-core/src/provenance/upstream.rs"
    "crates/mcf-helper/src/lib.rs"
    "crates/mcf-record/src/journal/index.rs"
)
declare -a finds=(
    # A6: the reported spread is a value that was observed, at the right rank.
    ".get(rank.saturating_sub(1))"
    # A6: the sample count is the number of samples.
    "self.rest.len().saturating_add(2)"
    # §3.3: one record, one meaning. A duplicate key is refused rather than
    # resolved differently by whichever reader gets there first.
    "if entries.insert(key, value).is_some() {"
    # B62: a torn last line is a loss, and is not read as a whole one.
    "let complete = raw.last() == Some(&b'\\n');"
    # §7.30: a journal written by a format this build does not read is refused
    # rather than appended to.
    "            Some(FORMAT_VERSION) => Ok(()),"
    # D9: the offset in force at a moment is the one the transition before it
    # set.
    "index => *self.offsets.get(index - 1)?,"
    # §3.3: a condition keeps the shape it was read in — the defect the property
    # tier found, kept here as its mutant.
    "Attested::Known(ConditionValue::Integer(number)) => Value::Integer(*number),"
    # B-301: a digest over a stream is the digest of the whole.
    "let room = 64 - self.buffered;"
    # A4: what a bundle states about itself is checked against what it holds.
    "if found != stated.digest || entries.len() != stated.entries {"
    # B-271: thinning keeps what its factor names.
    ".step_by(further.factor() as usize)"
    # A2: a failure carries the disposition it was classified with.
    "        self.disposition"
    # A19: a reported quantity is checked against an independently known value.
    # This entry survived when it was first tried, which is how the test that
    # kills it came to exist.
    "Bytes(kib.saturating_mul(1024))"
    # D31, A19: the rotary convention. Which two components of a head turn
    # together is the whole of it, and the two conventions differ in nothing
    # else — so making one of them the other is a mutation that changes only
    # the thing that matters. F20 measured what being wrong here costs: not a
    # crash and not gibberish, but English with no answer in it.
    "            Rotation::Halved => (pair, pair.saturating_add(pairs)),"
    # D31: the four-bit bias. A block decoded without it is the right size and
    # the wrong values, which is the kind of wrongness that looks like a model.
    "out.push(scale * (f32::from(value & 0x0F) - 8.0));"
    # F16: only the blocks that cache are counted. Counting every block
    # overstates the KV cache by the ratio between them — fourfold on the
    # reference model — which is a confident wrong number rather than no number.
    "blocks: caching_blocks(list(\"layer_types\"), declared_blocks)?,"
    # D37, F17: a hub that will not answer is not a decay. Reporting one as a
    # change would report an absence as an event, which is what A7 forbids.
    "                | Self::Replaced { .. }"
    # §6.32: the governor written is one the machine offers, chosen from its own
    # list. A helper that writes what it is told is a helper that writes
    # anything.
    ".find(|known| known.as_str() == wanted)"
    # B-300, D20: the index is a pointer and the journal is the record. An index
    # that describes a journal it does not cover answers questions about bytes
    # that are not there.
    "Some(last) if last.ends_at() > journal_bytes => ("
)
declare -a replaces=(
    ".get(rank)"
    "self.rest.len().saturating_add(1)"
    "if false {"
    "let complete = true;"
    "            Some(_) => Ok(()),"
    "index => *self.offsets.get(index)?,"
    "Attested::Known(ConditionValue::Integer(number)) => Value::text(number.to_string()),"
    "let room = 63 - self.buffered;"
    "if entries.len() != stated.entries {"
    ".step_by(1)"
    "        Disposition::Degraded"
    "Bytes(kib.saturating_mul(1000))"
    "            Rotation::Halved => (pair, pair.saturating_mul(2)),"
    "out.push(scale * f32::from(value & 0x0F));"
    "blocks: declared_blocks,"
    "                | Self::Unreachable { .. }"
    ".find(|known| !known.is_empty())"
    "Some(last) if last.ends_at() >= journal_bytes => ("
)

# The control: a mutation with no semantic effect at all — addition, the other
# way round. If the suite fails on this, the runner cannot tell a killed mutant
# from a broken copy, and its other results mean nothing.
readonly CONTROL_FILE="crates/mcf-core/src/measurement/mod.rs"
readonly CONTROL_FIND="self.rest.len().saturating_add(2)"
readonly CONTROL_REPLACE="2_usize.saturating_add(self.rest.len())"

workdir=$(mktemp -d "${TMPDIR:-/tmp}/mcf-mutants-XXXXXX")
trap 'rm -rf "$workdir"' EXIT

# A27's habit: copy rather than mutate, and take nothing derived with us.
tar -c -C "$root" --exclude=./target --exclude=./.git . | tar -x -C "$workdir"

# A pristine copy of every file the catalogue touches, to compare against at the
# end. Kept outside the workspace copy so that nothing in the build can see it.
pristine="$workdir.pristine"
mkdir -p "$pristine"
for file in "${files[@]}" "$CONTROL_FILE"; do
    cp "$workdir/$file" "$pristine/$(printf '%s' "$file" | tr / _)"
done
trap 'rm -rf "$workdir" "$pristine"' EXIT

# One warm build, so that each mutant costs a recompilation of what it touched
# rather than of the workspace.
printf 'building the unmutated copy\n'
(cd "$workdir" && cargo test --workspace --offline --no-run >/dev/null 2>&1) ||
    fail_cannot_check "the unmutated copy does not build"

# And it is green before anything is changed. A tree whose suite already fails
# would report every mutant killed.
printf 'running the unmutated suite\n'
(cd "$workdir" && cargo test --workspace --offline >/dev/null 2>&1) ||
    fail_cannot_check "the unmutated copy's suite does not pass"

run_suite() {
    local status=0
    # Reaped before as well as after: a spinning leftover from an earlier
    # mutant would compete with this run for the machine, and the deadline
    # below is what would notice it (B58's lesson from the laboratory —
    # clearing on the way in matters as much as on the way out).
    reap
    (cd "$workdir" && timeout "$MUTANT_TIMEOUT" cargo test --workspace --offline \
        >/dev/null 2>&1) || status=$?
    # `timeout` signals cargo, and cargo's test binaries are not its children to
    # signal. A mutant that hangs therefore leaves a test process spinning a
    # core for as long as the machine is up, which is a change to the machine
    # MCF did not put back (A27).
    reap
    return "$status"
}

# Kills anything still running out of this run's copy. By path, so nothing
# outside it is touched.
reap() {
    pkill -KILL -f "$workdir/target" >/dev/null 2>&1 || true
}

apply() {
    # $1 file, $2 find, $3 replace. Refuses anything but exactly one match.
    MUTANT_FIND="$2" MUTANT_REPLACE="$3" python3 - "$workdir/$1" <<'PY'
import os, sys
path = sys.argv[1]
find = os.environ["MUTANT_FIND"]
replace = os.environ["MUTANT_REPLACE"]
with open(path, encoding="utf-8") as handle:
    source = handle.read()
occurrences = source.count(find)
if occurrences != 1:
    sys.stderr.write(f"the text occurs {occurrences} times in {path}\n")
    raise SystemExit(3)
with open(path, "w", encoding="utf-8") as handle:
    handle.write(source.replace(find, replace))
PY
}

killed=(); survived=(); invalid=()

judge() {
    # $1 label, $2 file, $3 find, $4 replace, $5 "control" or "mutant"
    local label="$1" file="$2" find="$3" replace="$4" role="$5"
    local backup="$workdir/$file.original"

    # Before the build, not only before the run: a leftover from a hung mutant
    # is still executing the binary the linker is about to replace, and the
    # write fails with ETXTBSY. Reaping here is what makes the *next* mutant a
    # measurement of the next mutant.
    reap

    cp "$workdir/$file" "$backup"
    if ! apply "$file" "$find" "$replace"; then
        mv "$backup" "$workdir/$file"
        fail_cannot_check "could not place the mutation for $label"
    fi

    # Two steps rather than one, and in this order. `--no-run` answers whether
    # the mutant compiles; running the suite afterwards reuses what it just
    # built. Asking `cargo build` separately would compile the workspace a
    # second time under a different profile for an answer already in hand.
    local compiled=0
    (cd "$workdir" && cargo test --workspace --offline --no-run >/dev/null 2>&1) || compiled=$?
    local status=0
    if [ "$compiled" -eq 0 ]; then
        # `|| status=$?` rather than a bare call: under `errexit` a function
        # that returns non-zero in command position ends the script, and a
        # mutant being killed is exactly that.
        run_suite || status=$?
        # A timeout is confirmed before it is believed. A suite run competing
        # with something else on the machine can exceed a deadline without the
        # mutant having anything to do with it — B35's point, arriving at a
        # classification instead of at a measurement — and "hung" is a claim
        # about the mutant. The first draft of this script reported a mutant as
        # hanging that a unit test kills in a tenth of a second.
        if [ "$status" -eq "$EXIT_TIMED_OUT" ]; then
            printf '    (timed out; confirming)\n'
            status=0
            run_suite || status=$?
        fi
    fi

    # Restored, and given a new modification time. Cargo decides what to rebuild
    # from mtimes, and a restored backup is *older* than the build made from the
    # mutant — so without the touch the compiler considers the crate fresh, keeps
    # the mutated binary, and the next mutant is judged by the previous one's
    # code. That is how a hung digest mutant reappeared as a hung export mutant.
    mv "$backup" "$workdir/$file"
    touch "$workdir/$file"

    if [ "$role" = "control" ]; then
        if [ "$compiled" -eq 0 ] && [ "$status" -eq 0 ]; then
            printf '  control  an equivalent mutation is not killed\n'
        else
            printf '  CONTROL FAILED  an equivalent mutation was reported as killed\n'
            fail_cannot_check "the runner cannot tell a killed mutant from a broken copy"
        fi
        return 0
    fi

    if [ "$compiled" -ne 0 ]; then
        printf '  invalid  %s (does not compile; the compiler noticed, not the tests)\n' "$label"
        invalid+=("$label")
    elif [ "$status" -eq "$EXIT_TIMED_OUT" ]; then
        printf '  killed   %s (hung: the suite did not finish in %ds)\n' "$label" "$MUTANT_TIMEOUT"
        killed+=("$label")
    elif [ "$status" -ne 0 ]; then
        printf '  killed   %s\n' "$label"
        killed+=("$label")
    else
        printf '  SURVIVED %s\n' "$label"
        survived+=("$label")
    fi
}

printf '\nthe control\n'
judge "the equivalent mutant" "$CONTROL_FILE" "$CONTROL_FIND" "$CONTROL_REPLACE" control

printf '\n%d mutants\n' "${#files[@]}"
for index in "${!files[@]}"; do
    judge "${files[$index]}: ${finds[$index]:0:56}" \
        "${files[$index]}" "${finds[$index]}" "${replaces[$index]}" mutant
done

# Every mutated file is what it was before the run. A restore that silently
# failed would make every judgment after it a judgment about the wrong code —
# 4.5 of findings.md is that defect, found the hard way — and the control cannot
# see it, because the control runs before anything has leaked.
#
# Compared against the pristine copies taken at the start rather than against
# the repository, which may have been edited while a four-minute tier was
# running. The question is whether *this run* put back what it changed.
for index in "${!files[@]}"; do
    file="${files[$index]}"
    if ! cmp -s "$pristine/$(printf '%s' "$file" | tr / _)" "$workdir/$file"; then
        fail_cannot_check "$file was not put back, so these results are about some other code"
    fi
done

scored=$(( ${#killed[@]} + ${#survived[@]} ))
printf '\nmutation score: %d killed of %d scored' "${#killed[@]}" "$scored"
if [ "${#invalid[@]}" -gt 0 ]; then
    printf ' (%d excluded as invalid)' "${#invalid[@]}"
fi
printf '\n'

if [ "${#survived[@]}" -gt 0 ]; then
    printf '\nsurvivors — each is a claim no test is checking:\n'
    for entry in "${survived[@]}"; do
        printf '  %s\n' "$entry"
    done
fi

# The floor, and the previous score (B-186, B20).
if [ "$scored" -eq 0 ]; then
    fail_cannot_check "no mutant was scored, so there is no score to judge"
fi
percent=$(( ${#killed[@]} * 100 / scored ))
printf 'that is %d %%, against a floor of %d %% (B-186)\n' "$percent" "$MUTATION_FLOOR_PERCENT"

previous=$(tier_stamp_field "$root" mutation detail)
previous_percent=""
if [ -n "$previous" ]; then
    # The stamp holds this script's own summary line, so the two numbers come
    # back out the way they went in.
    previous_killed=$(printf '%s' "$previous" | awk '{ print $3 }')
    previous_scored=$(printf '%s' "$previous" | awk '{ print $6 }')
    if [ -n "$previous_killed" ] && [ -n "$previous_scored" ] && [ "$previous_scored" -gt 0 ]; then
        previous_percent=$(( previous_killed * 100 / previous_scored ))
        printf 'the last recorded run scored %d %% (%s)\n' "$previous_percent" "$previous"
    fi
fi

refused=false
if [ "$percent" -lt "$MUTATION_FLOOR_PERCENT" ]; then
    printf '\nrefused: %d %% is below the floor of %d %% (B-186, B20).\n' \
        "$percent" "$MUTATION_FLOOR_PERCENT" >&2
    printf 'Each survivor above is a claim a rule rests on and no test checks.\n' >&2
    refused=true
fi
if [ -n "$previous_percent" ] && [ "$percent" -lt "$previous_percent" ]; then
    printf '\nrefused: %d %% is below the %d %% the last run scored (B20: no silent\n' \
        "$percent" "$previous_percent" >&2
    printf 'regression). A score may only be lowered deliberately, by lowering the\n' >&2
    printf 'floor in scripts/lib-tiers.sh with the reasoning in the commit.\n' >&2
    refused=true
fi

if [ "$refused" = true ]; then
    exit "$EXIT_BELOW_FLOOR"
fi
