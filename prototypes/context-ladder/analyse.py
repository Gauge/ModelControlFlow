"""The performance picture: rate against depth, at every allocation.

Read out of the readings, never chosen.

1. THE PICTURE. For each model, a grid: allocation down the side, context depth
   across the top, and the rate in that band. Reading a row shows what depth
   costs at a fixed allocation; reading a column shows whether the allocation
   itself matters at a fixed depth — which is the question F118 could not answer
   because it never varied the allocation.

2. THE FIXED COST, WARM. The time to the first token with the model already
   resident. This is what a request costs before it has generated anything, and
   it is the honest intercept for the regime MCF actually serves in.

3. THE COLD START, which is a different question and is kept apart from every
   rate above. A20 and F117: a cold reading is not a speed and is not compared
   with one. What it answers is what a start costs, and whether allocating more
   context makes starting slower.

Nothing here is a figure MCF may publish: one machine, one engine, one
afternoon.
"""
import statistics
import sys


def read(path: str) -> list[dict]:
    rows = []
    try:
        handle = open(path)
    except OSError:
        return rows
    with handle:
        header = handle.readline().rstrip("\n").split("\t")
        for line in handle:
            values = line.rstrip("\n").split("\t")
            if len(values) != len(header):
                continue
            row = dict(zip(header, values))
            for key in ("allocation", "repeat", "nanoseconds", "from_token", "to_token"):
                if key in row:
                    row[key] = int(row[key])
            rows.append(row)
    return rows


def main(argv: list[str]) -> int:
    warm = read(argv[1] if len(argv) > 1 else "warm.tsv")
    cold = read(argv[2] if len(argv) > 2 else "cold.tsv")
    if not warm:
        print("no warm readings")
        return 2

    models = sorted({row["model"] for row in warm})
    bands = sorted({(r["from_token"], r["to_token"]) for r in warm if r["kind"] == "band"})
    labels = [f"{a - 1}–{b}" for a, b in bands]

    print("\n1. THE PICTURE — tokens per second in each band of context depth")
    for model in models:
        print(f"\n   {model}")
        allocations = sorted({r["allocation"] for r in warm if r["model"] == model})
        width = max((len(l) for l in labels), default=8) + 2
        print("   " + "allocated".ljust(12) + "".join(l.rjust(width) for l in labels))
        for allocation in allocations:
            cells = []
            for lower, upper in bands:
                got = [r["nanoseconds"] for r in warm
                       if r["model"] == model and r["allocation"] == allocation
                       and r["kind"] == "band" and r["from_token"] == lower
                       and r["to_token"] == upper]
                if not got:
                    cells.append("—".rjust(width))
                    continue
                rate = (upper - lower + 1) * 1e9 / statistics.median(got)
                cells.append(f"{rate:.1f}".rjust(width))
            print("   " + str(allocation).ljust(12) + "".join(cells))

    print("\n2. THE FIXED COST, WARM — time to the first token, model resident")
    print(f"   {'model':34} {'allocated':>10} {'first token':>14}")
    for model in models:
        for allocation in sorted({r["allocation"] for r in warm if r["model"] == model}):
            got = [r["nanoseconds"] for r in warm
                   if r["model"] == model and r["allocation"] == allocation
                   and r["kind"] == "first_token"]
            if got:
                print(f"   {model[:34]:34} {allocation:>10} "
                      f"{statistics.median(got)/1e6:>11.1f} ms")

    if cold:
        print("\n3. THE COLD START — not a speed, and not compared with one")
        print(f"   {'model':34} {'allocated':>10} {'start + one token':>20}")
        for model in sorted({r["model"] for r in cold}):
            for allocation in sorted({r["allocation"] for r in cold if r["model"] == model}):
                got = [r["nanoseconds"] for r in cold
                       if r["model"] == model and r["allocation"] == allocation]
                if got:
                    print(f"   {model[:34]:34} {allocation:>10} "
                          f"{statistics.median(got)/1e6:>17.1f} ms")

    print("\n   WHAT FOLLOWS")
    for model in models:
        allocations = sorted({r["allocation"] for r in warm if r["model"] == model})
        if not bands:
            continue
        first_band = bands[0]
        column = []
        for allocation in allocations:
            got = [r["nanoseconds"] for r in warm
                   if r["model"] == model and r["allocation"] == allocation
                   and r["kind"] == "band" and (r["from_token"], r["to_token"]) == first_band]
            if got:
                column.append((allocation,
                               (first_band[1] - first_band[0] + 1) * 1e9 / statistics.median(got)))
        print(f"   {model[:44]}")
        if len(column) >= 2:
            low, high = column[0][1], column[-1][1]
            change = (high - low) * 100 / low if low else 0
            print(f"     at the shallowest depth, allocating {column[0][0]} gives {low:.1f} tok/s "
                  f"and allocating {column[-1][0]} gives {high:.1f} ({change:+.1f}%)")
            print("     — which is whether the allocation alone costs speed, at equal depth")
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv))
