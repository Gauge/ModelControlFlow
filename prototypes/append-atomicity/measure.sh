#!/bin/sh
# Does an append tear? (DEC-037, F13)
#
# MCF's journal opens with `O_APPEND` and writes one whole line per entry, and
# since the daemon exists more than one process writes to it. Before deciding
# who may write (§7.37), it was worth knowing what the platform gives with no
# coordination at all.
#
# This is that measurement, kept so it can be repeated somewhere else: the
# answer is a property of a filesystem, and the one that matters is the one the
# operator's record is on. A network share is the classic place where `O_APPEND`
# stops meaning what it says, and nobody here has one to try.
#
#   ./prototypes/append-atomicity/measure.sh [directory to write in]
#
# Reports a finding.

set -eu

where="${1:-${TMPDIR:-/tmp}}"
workers=8
each=2000

command -v rustc >/dev/null 2>&1 || { printf 'cannot check: no rustc\n' >&2; exit 2; }

probe=$(mktemp -d "${TMPDIR:-/tmp}/mcf-append-XXXXXX")
trap 'rm -rf "$probe"' EXIT
rustc -O -o "$probe/probe" "$(dirname "$0")/probe.rs" 2>/dev/null

printf 'writing in %s (%s)\n' "$where" "$(df -T "$where" 2>/dev/null | awk 'NR==2 { print $2 }')"
for size in 400 8192 131072; do
    file="$probe/append-$size"
    : >"$file"
    marks="a b c d e f g h"
    for mark in $marks; do
        "$probe/probe" "$file" "$mark" "$each" "$size" &
    done
    wait

    lines=$(wc -l <"$file")
    # A line is torn if the filler does not start with the mark that wrote it.
    mixed=$(awk '{ if (length($3) > 0 && substr($3, 1, 1) != $1) print }' "$file" | wc -l)
    short=$(awk -v want="$size" 'length($0) < want { count++ } END { print count + 0 }' "$file")
    printf '  %7s bytes a line: %s lines (expected %s), %s interleaved, %s short\n' \
        "$size" "$lines" "$((workers * each))" "$mixed" "$short"
done
