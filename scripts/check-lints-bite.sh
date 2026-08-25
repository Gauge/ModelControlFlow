#!/usr/bin/env bash
#
# The denials in Cargo.toml actually bite (B-003, A2, B16).
#
# B16 prefers the machine-checked form of a rule, and A19 says a claim nothing
# tests is not a claim. A lint table is a claim: it asserts that `unwrap`,
# `panic!`, a discarded `Result` and the rest cannot reach non-test code. This
# script checks it the only way that answers the question — by writing each
# violation into a copy of the workspace and requiring the build to refuse it.
#
# Without this, a lint silently dropped from the table, a `priority` that
# reorders the set, or a clippy release that renames one would all read as
# "green" and nobody would learn until a swallowed failure reached a
# measurement.
#
# The copy is of the *working* tree, not of HEAD, so the check answers a
# question about the code in front of you. Nothing is written inside the
# repository.
#
# Exit status: 0 when every construct is refused, 1 when one is admitted, 2
# when the check could not be run.

set -o errexit -o nounset -o pipefail

readonly EXIT_ADMITTED=1
readonly EXIT_CANNOT_CHECK=2

root=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)

fail_cannot_check() {
    printf 'cannot check: %s\n' "$1" >&2
    exit "$EXIT_CANNOT_CHECK"
}

command -v cargo >/dev/null 2>&1 || fail_cannot_check "cargo is not on PATH"

# Each entry is: the clippy or rustc lint that must fire, and the line of
# non-test code that must make it fire.
declare -a lints=(
    "clippy::unwrap_used"
    "clippy::expect_used"
    "clippy::panic"
    "clippy::todo"
    "clippy::unimplemented"
    "clippy::indexing_slicing"
    "clippy::let_underscore_must_use"
    "clippy::exit"
    "unsafe_code"
    "unused_must_use"
)
declare -a violations=(
    'pub fn a() -> u8 { fallible().unwrap() }'
    'pub fn b() -> u8 { fallible().expect("no") }'
    'pub fn c() -> u8 { panic!("no") }'
    'pub fn d() -> u8 { todo!() }'
    'pub fn e() -> u8 { unimplemented!() }'
    'pub fn f(v: &[u8]) -> u8 { v[0] }'
    'pub fn g() { let _ = fallible(); }'
    'pub fn h() -> ! { std::process::exit(1) }'
    'pub fn i() -> u8 { unsafe { *std::ptr::null::<u8>() } }'
    'pub fn j() { fallible(); }'
)

# A negative control. `clippy::dbg_macro` is a real lint that this workspace
# deliberately does not enable, so the check must report it as *not* refused.
# Without it, a probe that failed to compile, a grep that never matched or a
# clippy that never ran would all read as ten clean refusals — which is the
# vacuously green suite A19 exists to prevent.
readonly CONTROL_LINT="clippy::dbg_macro"
readonly CONTROL_VIOLATION='pub fn control(v: u8) -> u8 { dbg!(v) }'

workdir=$(mktemp -d "${TMPDIR:-/tmp}/mcf-lints-XXXXXX")
trap 'rm -rf "$workdir"' EXIT

# A27's habit: copy rather than mutate, and take nothing derived with us.
tar -c -C "$root" --exclude=./target --exclude=./.git . | tar -x -C "$workdir"

probe="$workdir/crates/mcf-core/src/lint_probe.rs"
admitted=()
control_failed=false

lints+=("$CONTROL_LINT")
violations+=("$CONTROL_VIOLATION")

for index in "${!lints[@]}"; do
    lint="${lints[$index]}"
    violation="${violations[$index]}"
    is_control=false
    [ "$lint" = "$CONTROL_LINT" ] && is_control=true

    {
        printf '#![allow(missing_docs, dead_code, unreachable_pub)]\n'
        printf 'fn fallible() -> Result<u8, ()> { Ok(0) }\n'
        printf '%s\n' "$violation"
    } >"$probe"
    printf '\npub mod lint_probe;\n' >>"$workdir/crates/mcf-core/src/lib.rs"

    # JSON, because the human-readable formats do not always name the lint
    # that fired, and a check that greps for a phrase is a check that passes
    # when the phrase is reworded.
    output=$(cd "$workdir" && cargo clippy -p mcf-core --offline --message-format=json 2>&1 || true)
    if printf '%s' "$output" | grep -q -F "\"code\":\"$lint\""; then
        if [ "$is_control" = true ]; then
            printf '  CONTROL FAILED  %s fired, but the workspace does not enable it\n' "$lint"
            control_failed=true
        else
            printf '  refused  %s\n' "$lint"
        fi
    elif [ "$is_control" = true ]; then
        printf '  control  %s not enabled, and correctly not reported\n' "$lint"
    else
        printf '  ADMITTED %s\n' "$lint"
        admitted+=("$lint")
    fi

    # Put the crate back the way it was found before the next probe.
    rm -f "$probe"
    sed -i '$ d' "$workdir/crates/mcf-core/src/lib.rs"
    sed -i '$ d' "$workdir/crates/mcf-core/src/lib.rs"
done

if [ "$control_failed" = true ]; then
    printf '\nthe negative control fired: this check cannot distinguish a denied\n' >&2
    printf 'construct from an admitted one, so its other results mean nothing\n' >&2
    exit "$EXIT_CANNOT_CHECK"
fi

if [ "${#admitted[@]}" -eq 0 ]; then
    printf '\nevery denied construct is refused, and the control is not (B-003)\n'
    exit 0
fi

printf '\n%d construct(s) the lint table claims to deny are admitted: %s\n' \
    "${#admitted[@]}" "${admitted[*]}" >&2
exit "$EXIT_ADMITTED"
