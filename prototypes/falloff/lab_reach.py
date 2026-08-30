"""How far past the depths it measured can a method be believed?

Two probes beat everything affordable, but the probes were shallow and the
question a user has is often about a depth nobody measured — "how slow will
this get at thirty-two thousand tokens". So this asks the only thing that
matters for that: fit on the SHALLOW depths only, then predict the deep ones,
and report the error against how far beyond the fitted range the answer is.

The earlier synthetic laboratory showed that extrapolating a fitted curve is
where estimation goes badly wrong — a two-sigma interval covered the truth
between 4% and 79% of the time. But that lab fitted an UNKNOWN form. The
physics since established the form: cost is a straight line in depth, because
attention re-reads a cache that grows linearly. Fitting a line you already know
to be a line is a different act from choosing a curve, and this measures the
difference.

No timing originates here (A11).
"""

import sys
import pathlib

import numpy as np

sys.path.insert(0, str(pathlib.Path(__file__).parent))
import physics  # noqa: E402
import dataset  # noqa: E402
import lab_holdout  # noqa: E402

BW = 55.8


def main():
    curves = [c for c in dataset.curves() if c["dram_bound"] and len(c["depths"]) >= 5]
    rows = lab_holdout.observations(curves)
    by_model = {r["model"]: r for r in rows}
    buckets = {}

    for c in curves:
        if c["model"] not in by_model:
            continue
        row, geo = by_model[c["model"]], c["geometry"]
        d, y = c["depths"], c["ms"]
        train = [r for r in rows if r["architecture"] != c["architecture"]]
        if len({r["architecture"] for r in train}) < 2:
            continue
        constants = lab_holdout.fit(train)
        per = (constants["top"] * row["read"] / (row["read"] + constants["half"])
               / (1.0 + constants["sharing"] * (max(row["sharing"], 1.0) - 1.0)))
        a_pred = c["bytes"] * geo["active_weight_fraction"] / (BW * 1e9 * constants["weights"]) * 1e3

        # the two shallowest depths, and nothing else
        cut = d[1]
        slope = (y[1] - y[0]) / (d[1] - d[0])

        for depth, measured in zip(d[2:], y[2:]):
            reach = depth / cut
            two = y[0] + slope * (depth - d[0])
            g = physics.growing_bytes_at(geo, depth)
            pred = None if g is None else a_pred + g / (BW * 1e9 * per) * 1e3 * depth
            # anchored: the intercept from the shallow probe, the slope from
            # the header. Its slope carries no measurement noise, so it should
            # not degrade with distance the way a fitted slope does.
            g0 = physics.growing_bytes_at(geo, d[0])
            anchored = None
            if g is not None and g0 is not None:
                unit = 1.0 / (BW * 1e9 * per) * 1e3
                anchored = y[0] + (g * depth - g0 * d[0]) * unit

            key = ("2-4x" if reach < 4 else "4-8x" if reach < 8 else "8x+")
            b = buckets.setdefault(key, {"two": [], "pred": [], "anch": []})
            b["two"].append(abs(two - measured) / measured * 100)
            if pred is not None:
                b["pred"].append(abs(pred - measured) / measured * 100)
            if anchored is not None:
                b["anch"].append(abs(anchored - measured) / measured * 100)

    print("Extrapolating past the depths that were measured")
    print("=" * 72)
    print(f"\n  Two probes at the two shallowest depths, then asked about depths")
    print(f"  further out. 'predicted' uses no probe at all and is shown beside")
    print(f"  it, because a method that does not extrapolate cannot get worse.\n")
    print(f"  {'beyond the fit':<16}{'n':>4}{'two probes':>18}"
          f"{'anchored (1 probe)':>22}{'predicted':>18}")
    print(f"  {'':<16}{'':>4}{'median  worst':>18}{'median  worst':>22}{'median  worst':>18}")
    for key in ("2-4x", "4-8x", "8x+"):
        if key not in buckets:
            continue
        b = buckets[key]
        t = np.array(b["two"])
        p = np.array(b["pred"]) if b["pred"] else np.array([np.nan])
        a = np.array(b["anch"]) if b["anch"] else np.array([np.nan])
        print(f"  {key:<16}{len(t):>4}{np.median(t):>10.1f}%{max(t):>7.1f}%"
              f"{np.nanmedian(a):>14.1f}%{np.nanmax(a):>7.1f}%"
              f"{np.nanmedian(p):>10.1f}%{np.nanmax(p):>7.1f}%")

    for name, key in (("two probes", "two"), ("anchored", "anch"), ("predicted", "pred")):
        e = np.concatenate([np.array(b[key]) for b in buckets.values() if b[key]])
        print(f"\n  {name:<12} over every extrapolated point: "
              f"median {np.median(e):.1f}%, 90th {np.percentile(e, 90):.1f}%, worst {max(e):.1f}%")
    print("\n  Compare the synthetic laboratory, which fitted an unknown form and")
    print("  was ~41% wrong past its range with no way to tell. Knowing the form")
    print("  is a straight line is what changed — the physics bought the SHAPE,")
    print("  not the numbers.")


if __name__ == "__main__":
    main()
