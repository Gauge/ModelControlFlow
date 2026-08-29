"""Reads the fall-off readings and answers the three questions separately."""

import sys
import pathlib
from collections import defaultdict

import numpy as np

sys.path.insert(0, str(pathlib.Path(__file__).parent))
import curves  # noqa: E402


def load(path):
    rows = []
    with open(path) as handle:
        header = handle.readline()
        for line in handle:
            f = line.rstrip("\n").split("\t")
            if len(f) < 9 or f[0] == "warmup":
                continue
            lo, hi, ns = int(f[5]), int(f[6]), int(f[7])
            n = hi - lo + 1
            if n <= 0:
                continue
            rows.append(
                {
                    "model": f[0], "allocation": int(f[1]), "repeat": int(f[2]),
                    "arrival": f[3], "depth": (lo + hi) / 2,
                    "ms": ns / 1e6 / n, "prefill_ns": int(f[8]),
                }
            )
    return rows


def arm_order(rows):
    rows = [r for r in rows if r["arrival"] == "generate"]
    if not rows:
        return
    print("\n" + "=" * 72)
    print("ORDER — is it the allocation, or was it the running order?")
    print("=" * 72)
    print("\n  The context ladder walked allocations upward and recorded no")
    print("  temperature per reading, so an allocation effect and a drift over")
    print("  the hour looked the same. Here each pass visits every allocation.\n")
    for model in sorted({r["model"] for r in rows}):
        mine = [r for r in rows if r["model"] == model]
        allocs = sorted({r["allocation"] for r in mine})
        passes = sorted({r["repeat"] for r in mine})
        depths = sorted({r["depth"] for r in mine})[:4]
        print(f"  {model}")
        print("    ms/token, by ALLOCATION (pooled over passes)")
        print("      depth  " + "".join(f"{('c=' + str(a)):>11}" for a in allocs))
        spread_alloc = []
        for d in depths:
            line = f"      {int(d):<7}"
            vals = []
            for a in allocs:
                v = [r["ms"] for r in mine if r["depth"] == d and r["allocation"] == a]
                if v:
                    vals.append(np.median(v))
                    line += f"{np.median(v):>11.3f}"
                else:
                    line += f"{'—':>11}"
            if len(vals) > 1:
                spread_alloc.append((max(vals) - min(vals)) / min(vals) * 100)
            print(line)
        print("    ms/token, by PASS (pooled over allocations)")
        print("      depth  " + "".join(f"{('pass ' + str(p)):>11}" for p in passes))
        spread_pass = []
        for d in depths:
            line = f"      {int(d):<7}"
            vals = []
            for p in passes:
                v = [r["ms"] for r in mine if r["depth"] == d and r["repeat"] == p]
                if v:
                    vals.append(np.median(v))
                    line += f"{np.median(v):>11.3f}"
                else:
                    line += f"{'—':>11}"
            if len(vals) > 1:
                spread_pass.append((max(vals) - min(vals)) / min(vals) * 100)
            print(line)
        a_m = np.mean(spread_alloc) if spread_alloc else float("nan")
        p_m = np.mean(spread_pass) if spread_pass else float("nan")
        print(f"    spread across allocations {a_m:5.1f}%   across passes {p_m:5.1f}%")
        verdict = (
            "the allocation is doing the work" if a_m > 2 * p_m
            else "the pass is doing as much as the allocation — not an allocation effect"
            if p_m >= a_m else "neither dominates; the effect is small either way"
        )
        print(f"    -> {verdict}\n")


def arm_arrival(rows):
    gen = [r for r in rows if r["arrival"] == "generate"]
    pre = [r for r in rows if r["arrival"] == "prefill"]
    if not pre:
        return
    print("\n" + "=" * 72)
    print("ARRIVAL — does a prefilled depth cost what a generated one costs?")
    print("=" * 72)
    print("\n  If it does, the deep part of the curve stops being expensive:")
    print("  a measured deep point replaces an extrapolated one.\n")
    for model in sorted({r["model"] for r in pre}):
        print(f"  {model}")
        print(f"    {'depth':>7} {'generated':>12} {'prefilled':>12} {'differ':>9}"
              f"   {'prefill':>9} {'vs generating':>14}")
        for d in sorted({r["depth"] for r in pre if r["model"] == model}):
            p = [r for r in pre if r["model"] == model and r["depth"] == d]
            # the generated band whose midpoint is nearest this depth
            g = [r for r in gen if r["model"] == model]
            if not g or not p:
                continue
            # The generated readings are band midpoints and land nowhere near
            # the prefilled depths. Comparing against the nearest band compares
            # two different depths and manufactures a disagreement, so the
            # generated curve is interpolated at the prefilled depth instead.
            gd = np.array(sorted({r["depth"] for r in g}))
            gy = np.array([np.median([r["ms"] for r in g if r["depth"] == x]) for x in gd])
            if d < gd.min() or d > gd.max():
                continue
            gv = float(np.interp(d, gd, gy))
            pv = np.median([r["ms"] for r in p])
            pf = np.median([r["prefill_ns"] for r in p]) / 1e6
            # what it would have cost to GENERATE to this depth instead:
            # every token below d, each at its own cost
            fill = float(np.trapz(np.interp(np.arange(0, d), gd, gy), dx=1.0))
            print(f"    {int(d):>7} {gv:>12.3f} {pv:>12.3f} {(pv - gv) / gv * 100:>+8.1f}%"
                  f"   {pf:>7.0f}ms {fill / pf:>12.0f}x")
        print()


def arm_corpus(rows, dram_gbs=55.8):
    """The measured curve for every model, against the one computed from its
    header — and what the whole diagnostic cost."""
    import physics
    import glob
    import os

    paths = {os.path.basename(q): q for q in
             glob.glob(os.path.expanduser("~/.local/share/mcf/**/*.gguf"), recursive=True)}
    pre = [r for r in rows if r["arrival"] == "prefill"]
    if not pre:
        return
    print("\n" + "=" * 78)
    print("CORPUS — the curve measured at every depth, against the header")
    print("=" * 78)
    print("\n  The slope is fitted only to depths past the L3 crossing, because")
    print("  that is where a bandwidth argument applies at all. 'streams' is")
    print("  whether each layer's read is big enough to be bound by bandwidth")
    print("  rather than latency -- known from the header, before running.\n")
    print(f"    {'model':<34}{'measured':>11}{'predicted':>11}{'ratio':>7}{'streams':>9}{'cost':>8}")
    scored = []
    for model in sorted({r["model"] for r in pre}):
        mine = [r for r in pre if r["model"] == model]
        geo = physics.geometry(paths[model]) if model in paths else None
        if geo is None:
            continue
        crossing = physics.L3_PER_CCD / geo["bytes_per_depth_token"]
        pts = defaultdict(list)
        for r in mine:
            pts[r["depth"]].append(r["ms"])
        d = np.array(sorted(pts))
        y = np.array([np.median(pts[k]) for k in d])
        deep = d > crossing
        if deep.sum() < 2:
            continue
        # a straight line through the bandwidth-bound part
        slope = np.polyfit(d[deep], y[deep], 1)[0]
        want = physics.predicted_slope_ms(geo["bytes_per_depth_token"], dram_gbs)
        streams = geo["per_layer_read"] >= 2048
        seconds = sum(r["prefill_ns"] for r in mine) / 1e9 + sum(
            r["ms"] * 128 for r in mine) / 1e3
        scored.append((model, slope, want, want / slope if slope else float("nan"), streams))
        print(f"    {model[:33]:<34}{slope:>11.6f}{want:>11.6f}"
              f"{want / slope if slope else float('nan'):>7.2f}"
              f"{('yes' if streams else 'no'):>9}{seconds:>7.0f}s")
    if not scored:
        return
    for label, want_streaming in (("streams (bandwidth-bound)", True),
                                  ("does not stream (latency-bound)", False)):
        ratios = [r[3] for r in scored if r[4] is want_streaming]
        if not ratios:
            continue
        print(f"\n  {label}: n={len(ratios)}, ratio median {np.median(ratios):.2f}, "
              f"range {min(ratios):.2f}-{max(ratios):.2f}")
    good = [r[3] for r in scored if r[4]]
    if good:
        print(f"\n  For the streaming models the header predicts the slope to within")
        print(f"  {(1 - min(good)) * 100:.0f}-{(1 - max(good)) * 100:.0f}% once scaled by the fraction of peak")
        print(f"  bandwidth the engine achieves -- a single constant for this machine.")


if __name__ == "__main__":
    data = load(sys.argv[1] if len(sys.argv) > 1 else "/tmp/falloff/readings.tsv")
    print(f"{len(data)} readings")
    arm_order(data)
    arm_arrival(data)
    arm_corpus(data)
