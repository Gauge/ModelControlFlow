#!/usr/bin/env bash
#
# The fault signal can fail (B-193, A19).
#
# `mcf_core::hardware::storage::children_major_faults` is what tells the budget
# tier that a measurement went to a device for bytes rather than measuring MCF
# (findings.md F5). A signal that cannot fail is not a signal — F3 records MCF
# shipping one that silently could not — so this is the experiment that shows
# this one moving.
#
# It spawns one binary many times, twice: once warm, and once with the file's
# pages evicted from the cache before each spawn. Warm must take **no** major
# faults; evicted must take some. If the second half produces none, the
# filesystem did not honour the eviction hint and the check says so rather than
# reporting a signal it did not see (exit 2).
#
# `posix_fadvise` is why python3 is here: the standard library MCF is written
# against does not expose it, and reaching for the C ABI to write a check would
# be a worse trade than reaching for a program every machine that builds MCF
# already has.
#
# Exit status: 0 when the signal moved as it should, 1 when it did not, 2 when
# the experiment could not be run.

set -o errexit -o nounset -o pipefail

readonly EXIT_DID_NOT_MOVE=1
readonly EXIT_CANNOT_CHECK=2

root=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)

fail_cannot_check() {
    printf 'cannot check: %s\n' "$1" >&2
    exit "$EXIT_CANNOT_CHECK"
}

command -v python3 >/dev/null 2>&1 || fail_cannot_check "python3 is not on PATH"

subject="$root/target/debug/mcf"
[ -x "$subject" ] || subject="$root/target/release/mcf"
[ -x "$subject" ] || fail_cannot_check "no mcf binary is built; run cargo build first"

# Candidate directories to run the probe from. A tmpfs cannot demonstrate this
# — there is no device behind it to fault from — so a disk-backed directory is
# what the experiment needs, and the first one that works is the one reported.
candidates=("${XDG_CACHE_HOME:-$HOME/.cache}" "$root/target" "${TMPDIR:-/tmp}")

python3 - "$subject" "${candidates[@]}" <<'PY'
import os, shutil, subprocess, sys

subject, *candidates = sys.argv[1:]

def children_major_faults():
    with open("/proc/self/stat", encoding="ascii") as handle:
        stat = handle.read()
    # The kernel's field 17, counting from `state` after the last ')'.
    return int(stat[stat.rindex(")") + 2:].split()[10])

def spawn(path, times, evict):
    before = children_major_faults()
    for _ in range(times):
        if evict:
            handle = os.open(path, os.O_RDONLY)
            try:
                os.posix_fadvise(handle, 0, 0, os.POSIX_FADV_DONTNEED)
            finally:
                os.close(handle)
        subprocess.run([path, "--version"],
                       stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL, check=False)
    return children_major_faults() - before

TIMES = 30
for directory in candidates:
    probe = os.path.join(directory, "mcf-fault-signal-probe")
    try:
        os.makedirs(directory, exist_ok=True)
        shutil.copyfile(subject, probe)
        os.chmod(probe, 0o755)
        subprocess.run(["sync"], check=False)
    except OSError as error:
        print(f"  {directory}: not usable ({error})")
        continue

    try:
        warm = spawn(probe, TIMES, evict=False)
        evicted = spawn(probe, TIMES, evict=True)
    finally:
        try:
            os.remove(probe)
        except OSError:
            pass

    print(f"  {directory}: {TIMES} spawns warm took {warm} major faults, "
          f"{TIMES} with the pages evicted took {evicted}")

    if warm != 0:
        print("\na warm, resident binary took major faults, so this signal cannot tell a")
        print("cold artifact from an ordinary one here", file=sys.stderr)
        raise SystemExit(1)
    if evicted > 0:
        print(f"\nthe signal moved: 0 warm, {evicted} evicted, over the same {TIMES} spawns")
        print("of the same file. A reading that took any of these is a reading of the")
        print("device (B-193, findings.md F5).")
        raise SystemExit(0)
    print("  (this filesystem did not honour the eviction hint; trying the next)")

print("\nno candidate directory could evict a file's pages, so the signal was not",
      file=sys.stderr)
print("observed moving here. It is not thereby known to work (A19).", file=sys.stderr)
raise SystemExit(2)
PY
