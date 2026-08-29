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
        print(f"    {'depth':>7} {'generated':>12} {'prefilled':>12} {'differ':>9}   {'prefill cost':>13}")
        for d in sorted({r["depth"] for r in pre if r["model"] == model}):
            p = [r for r in pre if r["model"] == model and r["depth"] == d]
            # the generated band whose midpoint is nearest this depth
            g = [r for r in gen if r["model"] == model]
            if not g or not p:
                continue
            near = min({r["depth"] for r in g}, key=lambda x: abs(x - d))
            if abs(near - d) > 0.35 * d:
                continue
            gv = np.median([r["ms"] for r in g if r["depth"] == near])
            pv = np.median([r["ms"] for r in p])
            pf = np.median([r["prefill_ns"] for r in p]) / 1e6
            print(f"    {int(d):>7} {gv:>12.3f} {pv:>12.3f} {(pv - gv) / gv * 100:>+8.1f}%   {pf:>10.0f} ms")
        print()


def arm_corpus(rows):
    """Fit the shallow readings, predict the deep anchor, and let the spread of
    that error across models be the diagnostic's stated accuracy."""
    by_model = defaultdict(list)
    for r in rows:
        by_model[r["model"]].append(r)
    scored = []
    for model, mine in by_model.items():
        deep = [r for r in mine if r["arrival"] == "prefill" and r["depth"] > 5000]
        shallow = [r for r in mine if r["arrival"] == "generate" and r["depth"] <= 4096]
        if not deep or len(shallow) < 4:
            continue
        pts = defaultdict(list)
        for r in shallow:
            pts[r["depth"]].append(r["ms"])
        d = np.array(sorted(pts))
        y = np.array([np.median(pts[k]) for k in d])
        target_d = float(np.median([r["depth"] for r in deep]))
        target_y = float(np.median([r["ms"] for r in deep]))
        row = {"model": model, "reach": target_d / d.max()}
        for form in ("linear", "saturating"):
            got = curves.fit(form, d, y)
            if got is None:
                continue
            pred = float(curves.predict(form, got[0], [target_d])[0])
            row[form] = (pred - target_y) / target_y * 100
        scored.append(row)
    if not scored:
        return
    print("\n" + "=" * 72)
    print("CORPUS — how wrong is a shallow fit, on models it has not seen?")
    print("=" * 72)
    print(f"\n  Fit on depths <= 4096, predict the prefilled anchor deeper.\n")
    print(f"    {'model':<44}{'linear':>10}{'saturating':>12}")
    for r in sorted(scored, key=lambda x: abs(x.get("linear", 0)), reverse=True):
        lin = f"{r['linear']:+.1f}%" if "linear" in r else "—"
        sat = f"{r['saturating']:+.1f}%" if "saturating" in r else "—"
        print(f"    {r['model'][:43]:<44}{lin:>10}{sat:>12}")
    for form in ("linear", "saturating"):
        errs = [abs(r[form]) for r in scored if form in r]
        if errs:
            print(
                f"\n  {form}: median {np.median(errs):.1f}%, "
                f"90th percentile {np.percentile(errs, 90):.1f}%, worst {max(errs):.1f}%  "
                f"(n={len(errs)} models)"
            )
    print("\n  That 90th percentile is the number a diagnostic may print beside")
    print("  a projected rate. It is measured, not derived — the lab shows a")
    print("  derived interval does not cover.")


if __name__ == "__main__":
    data = load(sys.argv[1] if len(sys.argv) > 1 else "/tmp/falloff/readings.tsv")
    print(f"{len(data)} readings")
    arm_order(data)
    arm_arrival(data)
    arm_corpus(data)
