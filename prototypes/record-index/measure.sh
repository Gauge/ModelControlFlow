#!/bin/sh
# What the record costs as it grows, and what an embedded SQL engine would cost
# to ship (B-300, B-042, D6, D20, §XVI).
#
# D20 settled the shape — an append-only journal with a **derived, rebuildable**
# index over it — and D6, written earlier, named SQLite as the record. Both
# cannot be the first thing MCF reaches for: one of them is the record and the
# other is the index. Which is which is a question with numbers behind it, and
# this is where they are taken rather than argued.
#
# Three questions, each answered by measurement:
#
#   1. How does a replay scale? MCF replays its journal at every daemon start,
#      so the cost of the whole history is paid on every open (B-030).
#   2. What would SQLite buy? A query over a large record, without a replay.
#   3. What would SQLite cost to *ship*? §XVI's constraint is vendoring, and
#      B-183's from-scratch container runs a static musl artifact with nothing
#      installed. A C dependency is measured against that, not against taste.
#
# Everything is built OUTSIDE this repository, and nothing here vendors
# anything. Question 1 is MCF's own code and is measured by MCF's own suite:
#
#   cargo test -p mcf-record --release --test scale -- --ignored --nocapture
#
#   ./prototypes/record-index/measure.sh [scratch directory]
#
# Reports a finding.

set -eu

scratch="${1:-${TMPDIR:-/tmp}}/mcf-record-index"
rows=1000000
amalgamation=https://sqlite.org/2026/sqlite-amalgamation-3510200.zip

say() { printf '%s\n' "$*"; }
rule() { printf '\n=== %s\n' "$*"; }

mkdir -p "$scratch"
say "scratch: $scratch"

rule "what SQLite would buy: a query over $rows rows, without a replay"
if command -v sqlite3 >/dev/null 2>&1; then
    db="$scratch/record.db"
    rm -f "$db"
    say "sqlite3 $(sqlite3 --version | cut -d' ' -f1)"
    start=$(date +%s%N)
    sqlite3 "$db" <<SQL
PRAGMA journal_mode=WAL;
PRAGMA synchronous=NORMAL;
CREATE TABLE entry (id TEXT PRIMARY KEY, kind TEXT NOT NULL, at INTEGER NOT NULL, body TEXT NOT NULL);
CREATE INDEX entry_kind_at ON entry (kind, at);
WITH RECURSIVE counter(n) AS (SELECT 1 UNION ALL SELECT n + 1 FROM counter WHERE n < $rows)
INSERT INTO entry SELECT 'e' || n, CASE n % 7 WHEN 0 THEN 'artifact.acquired' ELSE 'trials' END, n,
  '{"repository":"unsloth/Qwen3-8B-GGUF","file":"Qwen3-8B-Q4_K_M.gguf","bytes":4920000000}' FROM counter;
SQL
    built=$(( ( $(date +%s%N) - start ) / 1000000 ))
    say "  built in ${built} ms; $(du -k "$db" | cut -f1) KiB on disk (plus WAL)"
    for query in "SELECT count(*) FROM entry WHERE kind = 'artifact.acquired'" \
                 "SELECT id FROM entry WHERE kind = 'artifact.acquired' ORDER BY at DESC LIMIT 20"; do
        start=$(date +%s%N)
        out=$(sqlite3 "$db" "$query" | tail -1)
        took=$(( ( $(date +%s%N) - start ) / 1000000 ))
        say "  ${took} ms (process included): $query -> $out"
    done
else
    say "  cannot measure: no sqlite3 on this machine"
fi

rule "what SQLite would cost to ship"
zip="$scratch/amalgamation.zip"
if [ ! -f "$zip" ]; then
    if command -v curl >/dev/null 2>&1 && curl -fsS -o "$zip" "$amalgamation"; then
        say "fetched $amalgamation"
    else
        say "  cannot measure: the amalgamation could not be fetched (offline is the ordinary case, D33)"
    fi
fi
if [ -f "$zip" ] && command -v unzip >/dev/null 2>&1; then
    rm -rf "$scratch/src" && mkdir -p "$scratch/src"
    unzip -qo "$zip" -d "$scratch/src"
    source_c=$(find "$scratch/src" -name sqlite3.c | head -1)
    say "  source: $(wc -l <"$source_c") lines, $(( $(wc -c <"$source_c") / 1024 )) KiB in one file"
    say "  terms:  $(grep -m1 -i 'public domain' "$source_c" | sed 's/^[* ]*//' || echo 'not stated in the first lines')"
    if command -v cc >/dev/null 2>&1; then
        start=$(date +%s%N)
        cc -O2 -c -o "$scratch/sqlite3.o" "$source_c" \
            -DSQLITE_OMIT_LOAD_EXTENSION -DSQLITE_THREADSAFE=1 2>"$scratch/cc.log" || say "  the host compile failed; see $scratch/cc.log"
        took=$(( ( $(date +%s%N) - start ) / 1000000 ))
        [ -f "$scratch/sqlite3.o" ] && say "  host object: $(( $(wc -c <"$scratch/sqlite3.o") / 1024 )) KiB, compiled in ${took} ms by $(cc --version | head -1)"
    fi
    # The one that decides it: B-183 runs a *static musl* artifact in a
    # container with no toolchain. A C dependency needs a C cross compiler at
    # build time, which is exactly what F12 found missing for the engines.
    if command -v musl-gcc >/dev/null 2>&1 || command -v x86_64-linux-musl-gcc >/dev/null 2>&1; then
        say "  musl: a C cross compiler is present on this machine"
    else
        say "  musl: NO C cross compiler here — the same wall F12 found for both engine candidates"
    fi
fi

rule "for comparison: what MCF's own record costs"
say "  run the internal half with:"
say "    cargo test -p mcf-record --release --test scale -- --ignored --nocapture"
