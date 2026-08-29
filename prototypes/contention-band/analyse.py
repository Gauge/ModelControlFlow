"""The band, read out of the record at a fixed number of pairs.

Interval width depends on how many pairs a run took, and the stopping
condition takes as many as it needs — so comparing widths across load levels
without controlling the count would be comparing the stopping condition with
itself. Every run here is cut to the same first `PAIRS` pairs, in interleaving
order, and the interval recomputed from those.

The criterion is the operator's, 2026-08-28: the quiet runs give a baseline
width and its own run-to-run spread, and the band is the highest load at which
width is still indistinguishable from that baseline. No threshold is chosen
here; the baseline's own variability is the threshold.
"""
import json, sys
from math import comb

PAIRS = 6
CORES = 32


def spread(d):
    n = len(d)
    d = sorted(d)
    best = None
    for k in range(1, n // 2 + 1):
        below = sum(comb(n, i) for i in range(0, k))
        cov = (2**n - 2 * below) * 10**6 // 2**n
        if cov >= 950_000:
            best = (k, cov)
    if not best:
        return None
    k, cov = best
    lo, hi = d[k - 1], d[n - k]
    if (lo <= 0 <= hi) or (lo >= 0 >= hi):
        return 0, max(abs(lo), abs(hi)), cov
    return min(abs(lo), abs(hi)), max(abs(lo), abs(hi)), cov


def runs(path, since):
    for line in open(path):
        e = json.loads(line)
        if e.get("kind") != "comparison":
            continue
        if e["recorded_at_utc_nanos"] < since:
            continue
        b = e["body"]
        m = b.get("machine") or {}
        pairs = b["pairs"][:PAIRS]
        if len(pairs) < PAIRS:
            continue
        d = [
            (p["right_ns"] - p["left_ns"]) * 10**6 // min(p["left_ns"], p["right_ns"])
            for p in pairs
            if p["left_ns"] and p["right_ns"]
        ]
        if len(d) < PAIRS:
            continue
        s = spread(d)
        if not s:
            continue
        lo, hi, cov = s
        yield {
            "competing": m.get("competing_before_thousandths", 0) / 1000,
            "width": (hi - lo) / 10_000,
            "at": e["recorded_at"][:19],
        }


if __name__ == "__main__":
    since = int(sys.argv[1])
    found = list(runs("/home/gauge/.local/share/mcf/record.jsonl", since))
    print(f"{len(found)} run(s), each cut to the first {PAIRS} pairs\n")
    print(f"{'competing':>10} {'% machine':>10} {'width (pts)':>12}   when")
    for r in sorted(found, key=lambda r: r["competing"]):
        print(
            f"{r['competing']:>8.2f}c {r['competing'] / CORES * 100:>9.0f}% "
            f"{r['width']:>12.1f}   {r['at']}"
        )
