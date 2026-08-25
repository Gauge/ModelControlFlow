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
# **It reports a score and does not enforce a floor.** B-186 is the floor and
# it is a separate item: a mutant that survives is a stated gap in the suite,
# printed by name, and the number is what B-186 will assert against. Failing
# here on the first survivor would make the tier unrunnable before there is
# anything to compare it with (B20's before-and-after).
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
# Exit status: 0 when the run is meaningful (whatever the score), 2 when it is
# not — the control was killed, the tree would not build, or cargo is absent.

set -o errexit -o nounset -o pipefail

readonly EXIT_CANNOT_CHECK=2

root=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)

fail_cannot_check() {
    printf 'cannot check: %s\n' "$1" >&2
    exit "$EXIT_CANNOT_CHECK"
}

command -v cargo >/dev/null 2>&1 || fail_cannot_check "cargo is not on PATH"
command -v python3 >/dev/null 2>&1 || fail_cannot_check "python3 is not on PATH"

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
    "let complete = line.ends_with('\\n');"
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

    cp "$workdir/$file" "$backup"
    if ! apply "$file" "$find" "$replace"; then
        mv "$backup" "$workdir/$file"
        fail_cannot_check "could not place the mutation for $label"
    fi

    local status=0
    (cd "$workdir" && cargo test --workspace --offline >/dev/null 2>&1) || status=$?
    local compiled=0
    (cd "$workdir" && cargo build --workspace --offline >/dev/null 2>&1) || compiled=$?

    mv "$backup" "$workdir/$file"

    if [ "$role" = "control" ]; then
        if [ "$status" -eq 0 ]; then
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
    printf '\nThe floor is B-186. This tier reports; it does not yet refuse.\n'
fi
