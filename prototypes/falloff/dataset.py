"""Every measured curve on this machine, with the geometry that should predict it.

Shared by the labs so that they are all scored against the same readings, and
so that a change to how a curve is extracted cannot quietly differ between one
lab and the next.
"""

import glob
import os
import sys
import pathlib
from collections import defaultdict

import numpy as np

sys.path.insert(0, str(pathlib.Path(__file__).parent))
import physics  # noqa: E402
import analyse  # noqa: E402

READINGS = [
    "readings-all.tsv", "readings-arch1.tsv", "readings.tsv",
    "readings-corpus.tsv", "readings-arrival.tsv",
]


def load_all(directory="/tmp/falloff"):
    """Every reading, from wherever the sweeps left them."""
    rows = []
    for name in READINGS:
        path = os.path.join(directory, name)
        if os.path.exists(path):
            try:
                rows += analyse.load(path)
            except (OSError, ValueError):
                continue
    return rows


def curves(directory="/tmp/falloff", minimum_points=3):
    """One entry per model: its measured depths and costs, and its header.

    Only prefilled band readings, because those are the ones taken at a stated
    depth (F120), and only models whose header the arithmetic describes."""
    paths = {os.path.basename(p): p
             for p in glob.glob(os.path.expanduser("~/.local/share/mcf/**/*.gguf"),
                                recursive=True)}
    rows = load_all(directory)
    out = []
    for model in sorted({r["model"] for r in rows}):
        mine = [r for r in rows
                if r["model"] == model and r["kind"] == "band" and r["arrival"] == "prefill"]
        if model not in paths or not mine:
            continue
        geo = physics.geometry(paths[model])
        if geo.get("verdict") != "described":
            continue
        seen = defaultdict(list)
        for r in mine:
            seen[r["depth"]].append(r["ms"])
        depths = np.array(sorted(seen), dtype=float)
        if len(depths) < minimum_points:
            continue
        cost = np.array([float(np.median(seen[d])) for d in depths])
        spread = np.array([
            (max(seen[d]) - min(seen[d])) / np.median(seen[d]) * 100 for d in depths
        ])
        out.append({
            "model": model,
            "path": paths[model],
            "architecture": geo["architecture"],
            "bytes": os.path.getsize(paths[model]),
            "geometry": geo,
            "depths": depths,
            "ms": cost,
            "spread_pct": spread,
            "dram_bound": physics.weights_are_dram_bound(
                os.path.getsize(paths[model]), geo["active_weight_fraction"]
            ),
        })
    return out


if __name__ == "__main__":
    data = curves()
    archs = defaultdict(int)
    for c in data:
        archs[c["architecture"]] += 1
    print(f"{len(data)} measured curves, {len(archs)} architectures")
    for a, n in sorted(archs.items(), key=lambda t: -t[1]):
        print(f"  {a:<12} {n:>2} model(s)")
    pts = sum(len(c["depths"]) for c in data)
    noise = np.concatenate([c["spread_pct"] for c in data])
    print(f"{pts} depth points; repeat spread median {np.median(noise):.1f}%, "
          f"90th {np.percentile(noise, 90):.1f}%")
    print(f"{sum(c['dram_bound'] for c in data)} of {len(data)} are DRAM-bound")
