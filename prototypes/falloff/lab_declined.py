"""Does measuring the intercept rescue the models the prediction had to decline?

F124 declined every model whose weights fit near L3: they are re-read from
cache rather than memory, so an intercept computed at DRAM speed over-predicts
them, by a factor of two for Llama-160M.

But that failure is entirely in the INTERCEPT. The slope is the cache growing
with depth, and a growing cache leaves L3 whatever the weights do. So a method
that measures the intercept and computes only the slope should work on exactly
the models the fully-computed curve could not touch — the probe absorbs the
cache residency without needing to model it.

If that holds, the declined band stops being a hole in the tool.
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
    every = dataset.curves()
    rows = lab_holdout.observations([c for c in every if c["dram_bound"]])
    constants = lab_holdout.fit(rows)
    print("The declined band, measured two ways")
    print("=" * 74)
    print("\n  'computed' takes the intercept from the file, as F124 did.")
    print("  'anchored' takes it from one shallow probe and computes only the")
    print("  slope. Both use the same header arithmetic for the rise.\n")
    print(f"  {'':<38}{'computed':>18}{'anchored':>18}")
    print(f"  {'model':<38}{'median  worst':>18}{'median  worst':>18}")
    groups = {True: {"c": [], "a": []}, False: {"c": [], "a": []}}
    for c in sorted(every, key=lambda x: (x["dram_bound"], x["model"])):
        geo, d, y = c["geometry"], c["depths"], c["ms"]
        if len(d) < 3:
            continue
        per = (constants["top"] * geo["per_layer_read"]
               / (geo["per_layer_read"] + constants["half"])
               / (1.0 + constants["sharing"] * (max(geo["query_per_kv_head"], 1.0) - 1.0)))
        a_file = c["bytes"] * geo["active_weight_fraction"] / (BW * 1e9 * constants["weights"]) * 1e3
        unit = 1.0 / (BW * 1e9 * per) * 1e3
        g0 = physics.growing_bytes_at(geo, d[0])
        comp, anch = [], []
        for depth, measured in zip(d[1:], y[1:]):
            g = physics.growing_bytes_at(geo, depth)
            if g is None or g0 is None:
                continue
            comp.append(abs((a_file + g * unit * depth) - measured) / measured * 100)
            anch.append(abs((y[0] + (g * depth - g0 * d[0]) * unit) - measured) / measured * 100)
        if not comp:
            continue
        groups[c["dram_bound"]]["c"] += comp
        groups[c["dram_bound"]]["a"] += anch
        mark = "" if c["dram_bound"] else "  <- was declined"
        print(f"  {c['model'][:37]:<38}{np.median(comp):>10.1f}%{max(comp):>7.1f}%"
              f"{np.median(anch):>10.1f}%{max(anch):>7.1f}%{mark}")
    print()
    for bound, label in ((True, "weights reach DRAM"), (False, "weights near L3 (declined)")):
        g = groups[bound]
        if not g["c"]:
            continue
        print(f"  {label:<30} computed median {np.median(g['c']):5.1f}% worst {max(g['c']):5.1f}%"
              f"   |  anchored median {np.median(g['a']):5.1f}% worst {max(g['a']):5.1f}%")


if __name__ == "__main__":
    main()
