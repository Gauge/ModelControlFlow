"""Is the rate constant, and if not, where does it go.

Three readings out of one ladder.

1. THE HEADLINE RATE at each rung — tokens divided by the whole wall time. This
   is what a fixed-duration test reports, and it climbs with the rung whatever
   the model does, because the per-request fixed cost is being spread over more
   tokens. It is here to be compared against the next one.

2. THE MARGINAL RATE between consecutive rungs — the tokens added, over the time
   added. The fixed cost cancels in the subtraction, so this is the rate AT that
   context depth. If generation were constant-rate these would all be equal.

3. WHAT THE MACHINE WAS DOING. The die temperature either side of each run and
   the load average. A rate that fell while the die climbed is a condition, not
   a property of the model (§3.4), and saying which is why both are recorded.

The conclusion at the end is derived: the marginal rates are compared with their
own repeat-to-repeat spread, so "the rate decays" is only said where the decay
is larger than the noise the same machine produced measuring the same thing.
"""
import statistics
import sys


def main(path: str) -> int:
    rows = []
    with open(path) as handle:
        header = handle.readline().rstrip("\n").split("\t")
        for line in handle:
            values = line.rstrip("\n").split("\t")
            if len(values) != len(header):
                continue
            row = dict(zip(header, values))
            for key in ("rung", "nanoseconds", "produced", "die_before_mc", "die_after_mc",
                        "allocation"):
                row[key] = int(row[key])
            rows.append(row)
    if not rows:
        print("no readings")
        return 2

    models = sorted({row["model"] for row in rows})
    rungs = sorted({row["rung"] for row in rows})
    allocations = sorted({row["allocation"] for row in rows})
    # The allocation is a condition of every figure below, and two runs at
    # different allocations are two experiments (the operator, F118).
    print(f"\n   context allocated: {', '.join(str(a) for a in allocations)}")
    if len(allocations) > 1:
        print("   MORE THAN ONE — the figures below mix allocations and are not comparable")

    print("\n1. THE HEADLINE RATE — tokens over the whole run, which is what a")
    print("   fixed-duration test reports")
    print(f"   {'model':34} " + "".join(f"{r:>10}" for r in rungs))
    for model in models:
        cells = []
        for rung in rungs:
            times = [r["nanoseconds"] for r in rows if r["model"] == model and r["rung"] == rung]
            got = [r["produced"] for r in rows if r["model"] == model and r["rung"] == rung]
            if not times:
                cells.append("     —")
                continue
            rate = statistics.median(got) * 1e9 / statistics.median(times)
            cells.append(f"{rate:>9.1f}")
        print(f"   {model[:34]:34} " + "".join(cells))

    print("\n2. THE MARGINAL RATE — tokens added over time added, which is the")
    print("   rate at that depth of context")
    print(f"   {'model':34} " + "".join(f"{a}→{b:<6}" for a, b in zip(rungs, rungs[1:])))
    marginals = {}
    for model in models:
        medians = {}
        for rung in rungs:
            times = [r["nanoseconds"] for r in rows if r["model"] == model and r["rung"] == rung]
            got = [r["produced"] for r in rows if r["model"] == model and r["rung"] == rung]
            if times:
                medians[rung] = (statistics.median(times), statistics.median(got))
        cells, values = [], []
        for lower, upper in zip(rungs, rungs[1:]):
            if lower not in medians or upper not in medians:
                cells.append("      —")
                continue
            dt = medians[upper][0] - medians[lower][0]
            dn = medians[upper][1] - medians[lower][1]
            rate = dn * 1e9 / dt if dt > 0 else float("inf")
            values.append(rate)
            cells.append(f"{rate:>9.1f}")
        marginals[model] = values
        print(f"   {model[:34]:34} " + "".join(cells))

    print("\n3. THE MACHINE, either side of the deepest rung")
    deepest = rungs[-1]
    for model in models:
        cell = [r for r in rows if r["model"] == model and r["rung"] == deepest]
        if not cell:
            continue
        before = statistics.median(r["die_before_mc"] for r in cell) / 1000
        after = statistics.median(r["die_after_mc"] for r in cell) / 1000
        print(f"   {model[:34]:34} die {before:.1f} °C → {after:.1f} °C   "
              f"load {cell[0]['load_before']} → {cell[-1]['load_after']}")

    print("\n   WHAT FOLLOWS")
    for model in models:
        got = [r["produced"] for r in rows if r["model"] == model]
        asked = [r["rung"] for r in rows if r["model"] == model]
        pinned = all(g >= a - 1 for g, a in zip(got, asked))
        print(f"   {model[:44]}")
        print(f"     the pin held at every rung: {'yes' if pinned else 'NO'}")
        values = marginals.get(model) or []
        if len(values) >= 2:
            first, last = values[0], values[-1]
            spread = []
            for rung in rungs:
                times = sorted(r["nanoseconds"] for r in rows
                               if r["model"] == model and r["rung"] == rung)
                if len(times) >= 3 and statistics.median(times):
                    spread.append((times[-1] - times[0]) * 100 / statistics.median(times))
            noise = statistics.median(spread) if spread else 0
            change = (last - first) * 100 / first if first else 0
            verdict = ("a decay larger than this machine's own noise"
                       if abs(change) > noise else
                       "flat to within the machine's own noise")
            print(f"     marginal rate {first:.1f} → {last:.1f} tok/s "
                  f"({change:+.1f}% across the ladder, noise {noise:.1f}%)")
            print(f"     {verdict}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1] if len(sys.argv) > 1 else "readings.tsv"))
