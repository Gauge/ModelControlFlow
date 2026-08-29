"""What the matrix says: the pin, the line, and the floor.

Three questions, in the order they decide things.

1. DOES THE PIN CHANGE THE ANSWER? For each model, how many tokens came back
   with the pin off against the length that was asked for. A model that stops
   early is not doing the work the run says it did, and two models that stop at
   different places are not doing the same work as each other.

2. IS TIME LINEAR IN LENGTH, AND WHAT ARE THE TWO HALVES? A least-squares line
   through the pinned readings gives a slope — the marginal cost of one token —
   and an intercept — everything that happens once per request. The intercept is
   the part that does not belong in a rate, and reporting total time over tokens
   folds it in.

3. WHAT IS THE FLOOR? The spread of repeats within a cell, as a percentage of
   its own median, is the smallest difference this machine could resolve at that
   length. A comparison asking about a difference smaller than its own floor is
   asking a question the machine cannot answer.

No threshold is chosen here. Every number is read out of the readings, and the
recommendation at the end follows from them rather than from taste.
"""
import collections
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
            row["length"] = int(row["length"])
            row["nanoseconds"] = int(row["nanoseconds"])
            row["produced"] = int(row["produced"])
            rows.append(row)
    if not rows:
        print("no readings")
        return 2

    models = sorted({row["model"] for row in rows})

    print("\n1. THE PIN: what came back against what was asked for")
    print(f"   {'model':36} {'pin':4} {'asked':>6} {'produced (median)':>18}")
    for model in models:
        for pinned in ("off", "on"):
            for length in sorted({r["length"] for r in rows}):
                cell = [r for r in rows if r["model"] == model
                        and r["pinned"] == pinned and r["length"] == length]
                if not cell:
                    continue
                produced = statistics.median(r["produced"] for r in cell)
                mark = "" if abs(produced - length) <= 1 else "   <-- not what was asked"
                print(f"   {model[:36]:36} {pinned:4} {length:>6} {int(produced):>18}{mark}")

    print("\n2. THE LINE: time = intercept + slope x tokens, from the pinned readings")
    print(f"   {'model':36} {'intercept':>12} {'per token':>12} {'rate':>14}")
    slopes = {}
    for model in models:
        cells = []
        for length in sorted({r["length"] for r in rows}):
            cell = [r["nanoseconds"] for r in rows
                    if r["model"] == model and r["pinned"] == "on" and r["length"] == length]
            if cell:
                cells.append((length, statistics.median(cell)))
        if len(cells) < 2:
            continue
        n = len(cells)
        mean_x = sum(x for x, _ in cells) / n
        mean_y = sum(y for _, y in cells) / n
        denominator = sum((x - mean_x) ** 2 for x, _ in cells)
        slope = sum((x - mean_x) * (y - mean_y) for x, y in cells) / denominator
        intercept = mean_y - slope * mean_x
        slopes[model] = (slope, intercept)
        rate = 1e9 / slope if slope > 0 else float("inf")
        print(f"   {model[:36]:36} {intercept/1e6:>9.1f} ms {slope/1e6:>9.3f} ms "
              f"{rate:>9.1f} tok/s")

    print("\n   the same readings as a rate computed the WRONG way (total / tokens):")
    print(f"   {'model':36} {'length':>7} {'apparent rate':>15}")
    for model in models:
        for length in sorted({r["length"] for r in rows}):
            cell = [r["nanoseconds"] for r in rows
                    if r["model"] == model and r["pinned"] == "on" and r["length"] == length]
            if not cell:
                continue
            apparent = length * 1e9 / statistics.median(cell)
            print(f"   {model[:36]:36} {length:>7} {apparent:>11.1f} tok/s")

    print("\n3. THE FLOOR: repeat-to-repeat spread within one cell")
    print(f"   {'model':36} {'length':>7} {'median':>10} {'spread':>10}")
    floors = collections.defaultdict(list)
    for model in models:
        for length in sorted({r["length"] for r in rows}):
            cell = sorted(r["nanoseconds"] for r in rows
                          if r["model"] == model and r["pinned"] == "on" and r["length"] == length)
            if len(cell) < 3:
                continue
            median = statistics.median(cell)
            spread = (cell[-1] - cell[0]) * 100 / median
            floors[model].append(spread)
            print(f"   {model[:36]:36} {length:>7} {median/1e6:>7.1f} ms {spread:>9.1f}%")

    print("\n   WHAT FOLLOWS")
    for model, (slope, intercept) in slopes.items():
        at32 = intercept + slope * 32
        share = intercept * 100 / at32 if at32 else 0
        print(f"   {model[:44]}")
        print(f"     at a 32-token run, {share:.0f}% of the time is fixed overhead, not generation")
        one_token = slope * 100 / at32 if at32 else 0
        print(f"     one token of mismatch between two arms is worth {one_token:.2f}% of the run")
    if floors:
        worst = max(statistics.median(v) for v in floors.values())
        print(f"   the machine's own repeat spread is around {worst:.1f}%, which is the floor "
              f"under any difference it can resolve")
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1] if len(sys.argv) > 1 else "readings.tsv"))
