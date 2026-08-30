"""Is the curve reliable on an architecture it has never seen?

The accuracy reported in F124 — median 3.6%, worst 14.7% — was measured against
the same readings the constants were fitted to. That is in-sample error, and it
is the number a model always flatters itself with. The question a diagnostic
actually faces is different: a user brings a model of a family nobody here has
measured, and the constants were fitted without it.

So: hold out one architecture entirely, refit on the rest, predict the held-out
one. That is the number worth quoting.

No timing originates here (A11). Every duration is a reading taken by a
prototype on real hardware; this file only does arithmetic on them.
"""

import sys
import pathlib

import numpy as np
from scipy.optimize import curve_fit

sys.path.insert(0, str(pathlib.Path(__file__).parent))
import physics  # noqa: E402
import dataset  # noqa: E402

BW = 55.8  # GB/s, measured on this machine by bandwidth.py


def observations(curves):
    """One row per model: what the achieved fraction actually was, and the two
    header quantities that are supposed to predict it."""
    out = []
    for c in curves:
        geo = c["geometry"]
        window = geo.get("sliding_window")
        d, y = c["depths"], c["ms"]
        keep = (d > window) if (window and (d > window).sum() >= 2) else (
            (d <= window) if window else np.ones(len(d), bool))
        if keep.sum() < 2:
            continue
        slope, intercept = np.polyfit(d[keep], y[keep], 1)
        growing = physics.growing_bytes_at(geo, float(np.median(d[keep])))
        if growing is None or slope <= 0 or intercept <= 0:
            continue
        out.append({
            "model": c["model"], "architecture": c["architecture"],
            "read": float(geo["per_layer_read"]),
            "sharing": float(geo["query_per_kv_head"]),
            "achieved": growing / 1e-3 / slope / (BW * 1e9),
            "weight_achieved": (c["bytes"] * geo["active_weight_fraction"]
                                / 1e-3 / intercept / (BW * 1e9)),
            "dram_bound": c["dram_bound"],
            "curve": c,
        })
    return out


def fit(rows):
    """The four constants, from a set of observations."""
    read = np.array([r["read"] for r in rows])
    share = np.array([r["sharing"] for r in rows])
    achieved = np.array([r["achieved"] for r in rows])

    def form(x, top, half, k):
        r, q = x
        return top * r / (r + half) / (1.0 + k * (np.maximum(q, 1.0) - 1.0))

    try:
        (top, half, k), _ = curve_fit(form, (read, share), achieved,
                                      p0=[1.0, 500, 0.05], maxfev=60000)
    except RuntimeError:
        return None
    weights = [r["weight_achieved"] for r in rows if r["dram_bound"]]
    return {"top": top, "half": half, "sharing": k,
            "weights": float(np.median(weights)) if weights else 0.90}


def predict(row, constants):
    """The whole curve for one model, under a given set of constants."""
    c = row["curve"]
    geo = c["geometry"]
    per = (constants["top"] * row["read"] / (row["read"] + constants["half"])
           / (1.0 + constants["sharing"] * (max(row["sharing"], 1.0) - 1.0)))
    a = c["bytes"] * geo["active_weight_fraction"] / (BW * 1e9 * constants["weights"]) * 1e3
    errs = []
    for depth, measured in zip(c["depths"], c["ms"]):
        growing = physics.growing_bytes_at(geo, depth)
        if growing is None:
            continue
        got = a + growing / (BW * 1e9 * per) * 1e3 * depth
        errs.append((got - measured) / measured * 100)
    return errs


def main():
    curves = [c for c in dataset.curves() if c["dram_bound"]]
    rows = observations(curves)
    print("Held out one architecture at a time, constants refitted without it")
    print("=" * 76)
    print(f"\n  {len(rows)} DRAM-bound models, "
          f"{len({r['architecture'] for r in rows})} architectures\n")

    everything = fit(rows)
    in_sample = np.abs(np.concatenate([predict(r, everything) for r in rows]))
    print(f"  {'architecture':<14}{'models':>7}{'held-out error':>28}{'in-sample':>12}")
    print(f"  {'':<14}{'':>7}{'median   90th    worst':>28}")
    held = []
    for arch in sorted({r["architecture"] for r in rows}):
        train = [r for r in rows if r["architecture"] != arch]
        test = [r for r in rows if r["architecture"] == arch]
        if len({r["architecture"] for r in train}) < 2:
            continue
        constants = fit(train)
        if constants is None:
            continue
        errs = np.abs(np.concatenate([predict(r, constants) for r in test]))
        mine = np.abs(np.concatenate([predict(r, everything) for r in test]))
        held.append(errs)
        print(f"  {arch:<14}{len(test):>7}{np.median(errs):>10.1f}%"
              f"{np.percentile(errs, 90):>8.1f}%{max(errs):>8.1f}%"
              f"{np.median(mine):>11.1f}%")

    allheld = np.concatenate(held)
    print(f"\n  {'ALL HELD OUT':<14}{len(rows):>7}{np.median(allheld):>10.1f}%"
          f"{np.percentile(allheld, 90):>8.1f}%{max(allheld):>8.1f}%"
          f"{np.median(in_sample):>11.1f}%")
    print(f"\n  in-sample says {np.median(in_sample):.1f}%; held out it is "
          f"{np.median(allheld):.1f}%.")
    print(f"  The gap between those two numbers is what fitting and scoring on")
    print(f"  the same readings was hiding.")
    print("\n  Constants fitted on everything, for reference:")
    print(f"    achieved = {everything['top']:.3f} x read/(read + {everything['half']:.0f})"
          f" / (1 + {everything['sharing']:.4f}(q/kv - 1)),  weights {everything['weights']:.2f}")
    return allheld


if __name__ == "__main__":
    main()
