#!/usr/bin/env bash
#
# What the vendored tree says its terms are (B-330, D28, vendored.md §1).
#
# D28 makes MCF GPL-3.0-only, so every vendored component's terms are MCF's
# problem: one that is not GPL-3.0-compatible is one MCF cannot ship whatever
# its merits. `doc/vendored.md` is the register and
# `checks/tests/the_licence_is_what_it_says.rs` refuses a vendored crate with no
# row in it. This is the other half — reading the tree rather than the register:
#
#   * every vendored crate declares terms, and they are in the compatible set
#     below, which is a list somebody decided rather than a pattern;
#   * every crate that is *compiled* carries a copy of those terms, or is named
#     here as one that does not — a declaration is not a text (A21).
#
# It does not read the licences and decide whether they mean what they say. That
# is a human judgement, it is recorded in `doc/vendored.md`, and a script that
# claimed to have made it would be the fabricated report C7 is written against.
#
# Exit status: 0 when every declaration is in the compatible set, 1 when one is
# not, 2 when the check could not be made.

set -o errexit -o nounset -o pipefail

readonly EXIT_FAILED=1
readonly EXIT_CANNOT_CHECK=2

root=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)
cd "$root"

[ -d vendor ] || {
    printf 'nothing is vendored, so there are no terms to read\n'
    exit 0
}
command -v python3 >/dev/null 2>&1 || {
    printf 'cannot check: python3 is not on the path\n' >&2
    exit "$EXIT_CANNOT_CHECK"
}

python3 - <<'TERMS'
import pathlib
import re
import sys

# Every set of terms MCF has decided it can ship under GPL-3.0-only. Written out
# rather than matched by pattern: adding one is a decision about what MCF
# conveys, and `doc/vendored.md` is where the reasoning goes.
COMPATIBLE = {
    "MIT OR Apache-2.0",
    "Apache-2.0 OR MIT",
    "Apache-2.0 OR ISC OR MIT",
    "Apache-2.0 OR ISC OR MIT-0",
    "Apache-2.0 AND ISC",
    "ISC",
    "MIT",
    "BSD-3-Clause",
    # The root certificate set is data rather than code, and its licence is a
    # data licence: permissive, no conditions on redistribution beyond keeping
    # the notice.
    "CDLA-Permissive-2.0",
    # Stubs that no target compiles, kept for their manifests.
    "MIT OR Apache-2.0 OR LGPL-2.1-or-later",
    "Apache-2.0 WITH LLVM-exception OR Apache-2.0 OR MIT",
}

vendored = pathlib.Path("vendor")
unknown, undeclared, no_text = [], [], []
compiled = 0

for crate in sorted(vendored.iterdir()):
    if not crate.is_dir():
        continue
    manifest = (crate / "Cargo.toml").read_text()
    declared = re.search(r'^license = "([^"]+)"', manifest, re.M)
    stub = "Stubbed by scripts/vendor.sh" in (crate / "src" / "lib.rs").read_text() \
        if (crate / "src" / "lib.rs").exists() else False
    if not stub:
        compiled += 1

    if declared is None:
        undeclared.append(crate.name)
        continue
    terms = declared.group(1)
    if terms not in COMPATIBLE:
        unknown.append(f"{crate.name}: {terms}")

    carried = [
        child.name
        for child in crate.iterdir()
        if child.name.upper().startswith(("LICENSE", "LICENCE", "COPYING", "NOTICE"))
    ]
    if not carried and not stub:
        no_text.append(f"{crate.name}: declares {terms} and carries no copy of it")

print(f"  {compiled} crates compiled, {len(list(vendored.iterdir())) - compiled} stubbed")

if no_text:
    print("  declared but not carried (A21 — recorded in doc/vendored.md):")
    for one in no_text:
        print(f"    {one}")

if undeclared or unknown:
    print("  TERMS MCF HAS NOT DECIDED IT CAN SHIP:", file=sys.stderr)
    for one in undeclared:
        print(f"    {one}: declares no licence at all", file=sys.stderr)
    for one in unknown:
        print(f"    {one}", file=sys.stderr)
    sys.exit(1)

print("  every declaration is in the set MCF has decided it can ship under GPL-3.0-only")
TERMS
