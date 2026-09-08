#!/usr/bin/env bash
#
# Builds the vendored source tree MCF ships (B-322, §XVI, B15).
#
# **Why the tree is in the repository at all.** Every build in this project runs
# `--offline --locked`: the from-scratch conformance container has no network,
# the reproducibility check builds two clean checkouts, and §3.12 makes what was
# built a condition of every measurement. A dependency resolved at build time
# from a registry is a dependency nobody pinned.
#
# **Why the tree is not what `cargo vendor` produces.** `cargo vendor` writes
# every crate in the lock graph, for every platform anybody's `Cargo.toml`
# mentions — 95 MiB, of which about 80 is Windows import libraries and a C
# cryptography provider that no target MCF builds ever compiles. Deleting them
# does not work: cargo resolves the whole graph before it compiles any of it
# (an earlier finding). Stubbing them does: a crate keeps
# its manifest, its licence files and an empty `lib.rs`, and its checksum file
# lists no files. The result is 13 MiB and both targets still build offline.
#
# **What is stubbed is decided by cargo, not by a list here.** The crates kept
# whole are exactly the ones `cargo tree` reports for the targets MCF builds.
# A list would go stale the first time a dependency changed a feature.
#
# Run it after changing a dependency, and commit what it produces. It is
# deterministic: the same lockfile and the same targets produce the same tree.
#
# Exit status: 0 when the tree was built and both targets compile against it,
# 1 when they do not, 2 when the check could not be made.

set -o errexit -o nounset -o pipefail

readonly EXIT_FAILED=1
readonly EXIT_CANNOT_CHECK=2

# The targets MCF builds. The musl one is a second artifact rather than a
# replacement (D29), and it is what `check-from-scratch.sh` puts in a container
# with nothing in it — so a crate that cannot build for it is a crate that costs
# MCF a claim it already makes (F9.5).
readonly TARGETS=(x86_64-unknown-linux-gnu x86_64-unknown-linux-musl)

root=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)
cd "$root"

command -v cargo >/dev/null 2>&1 || {
    printf 'cannot check: cargo is not on the path\n' >&2
    exit "$EXIT_CANNOT_CHECK"
}
command -v python3 >/dev/null 2>&1 || {
    printf 'cannot check: python3 is not on the path, and the stubbing needs it\n' >&2
    exit "$EXIT_CANNOT_CHECK"
}

printf '=== fetching the tree cargo would use\n'
rm -rf vendor
cargo vendor --versioned-dirs vendor >/dev/null

printf '=== stubbing what no target here compiles\n'
compiled=$(mktemp)
trap 'rm -f "$compiled"' EXIT
for target in "${TARGETS[@]}"; do
    cargo tree --target "$target" --edges normal,build --prefix none --no-dedupe 2>/dev/null \
        | awk '{ print $1 }' >>"$compiled"
done
sort -u -o "$compiled" "$compiled"

python3 - "$compiled" <<'STUB'
import json
import pathlib
import shutil
import sys

compiled = {name.strip() for name in pathlib.Path(sys.argv[1]).read_text().splitlines() if name.strip()}
vendored = pathlib.Path("vendor")
kept, stubbed = [], []

for crate in sorted(vendored.iterdir()):
    if not crate.is_dir():
        continue
    # `--versioned-dirs` names a directory `name-1.2.3`; the crate is the part
    # before the last dash.
    name = crate.name.rsplit("-", 1)[0]
    if name in compiled:
        kept.append(crate.name)
        continue

    checksum = json.loads((crate / ".cargo-checksum.json").read_text())
    for child in list(crate.iterdir()):
        if child.name in ("Cargo.toml", ".cargo-checksum.json"):
            continue
        # Licences stay. The crate is not compiled and not linked, and it is
        # still redistributed in this repository — a licence file removed to
        # save a kilobyte is somebody's terms thrown away (D28).
        if child.name.upper().startswith(("LICENSE", "LICENCE", "COPYING", "NOTICE")):
            continue
        shutil.rmtree(child) if child.is_dir() else child.unlink()

    source = crate / "src"
    source.mkdir(exist_ok=True)
    (source / "lib.rs").write_text(
        "// Stubbed by scripts/vendor.sh: no target MCF builds compiles this crate.\n"
        "// It is in the lock graph, so cargo requires it to be present; it is not\n"
        "// in any build, so what is present is a manifest, a licence and nothing.\n"
    )
    # No files listed means no file is checksummed, which is what makes the
    # stub acceptable to cargo. The package's own checksum is kept, so the
    # lockfile still identifies exactly which release this was.
    (crate / ".cargo-checksum.json").write_text(
        json.dumps({"files": {}, "package": checksum.get("package")})
    )
    stubbed.append(crate.name)

print(f"  compiled and kept whole: {len(kept)}")
print(f"  stubbed: {len(stubbed)}")
for name in stubbed:
    print(f"    {name}")
STUB

printf '=== the tree\n'
du -sh vendor | awk '{ printf "  %s\n", $0 }'
carried=$(find vendor -name '*.c' -o -name '*.S' -o -name '*.cc' | wc -l)
printf '  C or assembly source files: %s\n' "$carried"

printf '=== both targets, offline and locked\n'
for target in "${TARGETS[@]}"; do
    if cargo build --offline --locked --release --target "$target" >/dev/null 2>&1; then
        printf '  %s: builds\n' "$target"
    else
        printf '  %s: DOES NOT BUILD against the vendored tree\n' "$target" >&2
        exit "$EXIT_FAILED"
    fi
done

printf 'vendor: the tree is built and both targets compile against it\n'
