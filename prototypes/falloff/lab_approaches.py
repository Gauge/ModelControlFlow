"""Every way there is to say how fast a model generates, scored against each other.

The predictive curve works. That is not the same as it being the best available
answer, and "more accurate" is not the only axis — a method that is twice as
good and takes an hour is not better for someone deciding whether to run a
model. So each approach is scored on both, against the same readings.

The approaches, in order of what they cost:

  scalar rate     one generation, tokens per second, quoted everywhere. What
                  MCF does today and what almost every published figure is.
  predicted       nothing generated at all: the curve computed from the file
                  and this machine's constants, fitted WITHOUT this model's
                  architecture (lab_holdout).
  anchored        one shallow probe for the intercept, the slope from the
                  header. The measurement pins where the curve starts; the
                  arithmetic supplies how it rises.
  two point       probe the shallowest and deepest depth, fit the straight
                  line the physics says it is. No header used.
  every point     probe every depth. The most anyone can do, and the ceiling
                  on what measurement alone achieves.

Scored on held-out depths wherever an approach fits to some: a method is not
allowed to be graded on a point it was given. No timing originates here (A11).
"""

import sys
import pathlib

import numpy as np

sys.path.insert(0, str(pathlib.Path(__file__).parent))
import physics  # noqa: E402
import dataset  # noqa: E402
import lab_holdout  # noqa: E402

BW = 55.8


def probe_cost_seconds(curve, depth):
    """What one prefilled probe at this depth costs: the prefill, plus the 128
    tokens generated to measure the rate. Taken from the readings themselves."""
    i = int(np.argmin(np.abs(curve["depths"] - depth)))
    return float(curve["ms"][i] * 128 / 1e3 + curve["depths"][i] / 1500.0)


def score(curve, predicted_at, fitted_depths):
    """Absolute percentage error at every depth the method was NOT given."""
    errs = []
    for depth, measured in zip(curve["depths"], curve["ms"]):
        if any(abs(depth - f) < 1 for f in fitted_depths):
            continue
        got = predicted_at(depth)
        if got is None:
            continue
        errs.append(abs(got - measured) / measured * 100)
    return errs


def main():
    curves = [c for c in dataset.curves() if c["dram_bound"] and len(c["depths"]) >= 4]
    rows = lab_holdout.observations(curves)
    by_model = {r["model"]: r for r in rows}

    results = {name: {"err": [], "cost": []} for name in
               ("scalar rate", "predicted", "anchored", "two point", "every point")}

    for c in curves:
        if c["model"] not in by_model:
            continue
        row = by_model[c["model"]]
        geo, d, y = c["geometry"], c["depths"], c["ms"]

        # constants fitted WITHOUT this model's architecture
        train = [r for r in rows if r["architecture"] != c["architecture"]]
        if len({r["architecture"] for r in train}) < 2:
            continue
        constants = lab_holdout.fit(train)

        # --- scalar rate: one shallow generation, quoted at every depth
        results["scalar rate"]["err"] += score(c, lambda _d: y[0], [d[0]])
        results["scalar rate"]["cost"].append(probe_cost_seconds(c, d[0]))

        # --- predicted: nothing generated
        per = (constants["top"] * row["read"] / (row["read"] + constants["half"])
               / (1.0 + constants["sharing"] * (max(row["sharing"], 1.0) - 1.0)))
        a_pred = c["bytes"] * geo["active_weight_fraction"] / (BW * 1e9 * constants["weights"]) * 1e3

        def predicted(depth):
            g = physics.growing_bytes_at(geo, depth)
            return None if g is None else a_pred + g / (BW * 1e9 * per) * 1e3 * depth

        results["predicted"]["err"] += score(c, predicted, [])
        results["predicted"]["cost"].append(0.0)

        # --- anchored: one shallow probe fixes the intercept, header gives the rise
        def anchored(depth, d0=d[0], y0=y[0]):
            g0 = physics.growing_bytes_at(geo, d0)
            g = physics.growing_bytes_at(geo, depth)
            if g is None or g0 is None:
                return None
            slope = 1.0 / (BW * 1e9 * per) * 1e3
            return y0 + (g * depth - g0 * d0) * slope

        results["anchored"]["err"] += score(c, anchored, [d[0]])
        results["anchored"]["cost"].append(probe_cost_seconds(c, d[0]))

        # --- two point: shallowest and deepest, straight line, no header
        slope2 = (y[-1] - y[0]) / (d[-1] - d[0])
        results["two point"]["err"] += score(
            c, lambda depth: y[0] + slope2 * (depth - d[0]), [d[0], d[-1]])
        results["two point"]["cost"].append(
            probe_cost_seconds(c, d[0]) + probe_cost_seconds(c, d[-1]))

        # --- every point: fit all, scored on all (in-sample; the ceiling)
        sl, ic = np.polyfit(d, y, 1)
        results["every point"]["err"] += [
            abs((ic + sl * dd - mm) / mm * 100) for dd, mm in zip(d, y)]
        results["every point"]["cost"].append(
            sum(probe_cost_seconds(c, dd) for dd in d))

    print("Five ways to say how fast a model generates")
    print("=" * 78)
    print(f"\n  {len(curves)} DRAM-bound models, "
          f"{len({c['architecture'] for c in curves})} architectures.")
    print("  Scored only at depths the method was not given. Cost is per model.\n")
    print(f"  {'approach':<14}{'median':>9}{'90th':>8}{'worst':>8}"
          f"{'cost':>10}{'what it needs':>26}")
    needs = {
        "scalar rate": "one generation",
        "predicted": "the file only",
        "anchored": "one probe + header",
        "two point": "two probes",
        "every point": "a probe per depth",
    }
    for name, got in results.items():
        e = np.array(got["err"])
        if not len(e):
            continue
        print(f"  {name:<14}{np.median(e):>8.1f}%{np.percentile(e, 90):>7.1f}%"
              f"{max(e):>7.1f}%{np.mean(got['cost']):>9.0f}s{needs[name]:>26}")
    print("\n  'every point' is scored in-sample and is the ceiling on measurement;")
    print("  every other row is scored on depths it never saw.")


if __name__ == "__main__":
    main()
