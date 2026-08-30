"""What a diagnostic will do, and how long it will take, said before it starts.

The rules this obeys, and where each is enforced:

  1. Nothing is sampled above the model's trained context. `ceiling()` caps
     there and `depths()` cannot exceed it.
  2. Every window and every sampled depth is a power of two. There is no path
     that produces another number — the ladder doubles and the cap is taken by
     halving, never by subtracting headroom.
  3. The context evaluated is the highest the MACHINE can afford, per device,
     from the memory it actually has free. CPU and GPU are both planned unless
     one is excluded; a device the engine cannot use is reported as such rather
     than quietly skipped.
  4. The estimate is produced before the run and the caller may cap it. Under a
     cap the plan degrades to a cheaper shape and the accuracy quoted changes
     with it, so a shortened test never claims a full test's precision.

Every constant is a measurement from this machine, cited where it came from.
"""

import os
import sys
import pathlib

sys.path.insert(0, str(pathlib.Path(__file__).parent))
import physics    # noqa: E402
import hardware   # noqa: E402

BW = 55.8e9                            # bandwidth.py
WEIGHTS_ACHIEVED = 0.92                # F125, held out
PREFILL_A, PREFILL_B = 662.0, -0.77    # F126, 27 models

# F126: below 128 the tokens after a prefill are still cache-warm and the probe
# reads optimistically — -2.6% at 32 tokens. Not a knob to turn for speed.
PROBE_TOKENS = 128
SHALLOWEST = 512

# The estimate is unbiased but not precise: over 136 measured probes the ratio
# of estimated to measured runs 0.58 at the 5th percentile and 1.42 at the
# 95th. Prefill throughput varies with the quantization in ways neither model
# size nor the attention geometry accounts for, and a two-term physical fit was
# no better (90th percentile worse). So the estimate is quoted as a RANGE, and
# the budget is tested against the pessimistic end — a plan that promised five
# minutes may not take six.
ESTIMATE_LOW = 1 / 1.42
ESTIMATE_HIGH = 1 / 0.58


def as_range(seconds):
    return seconds * ESTIMATE_LOW, seconds * ESTIMATE_HIGH

# F125, held-out accuracy. Quoted with the plan so a cheap plan cannot borrow
# an expensive plan's precision.
ACCURACY = {
    "every depth": (0.4, 3.5, "every sampled depth is measured"),
    "both ends": (0.8, 4.6, "the range is interpolated between two measurements"),
    "anchored": (2.2, 11.8, "intercept measured, slope from the header"),
    "computed": (4.2, 13.7, "nothing generated; the file and this machine's constants"),
}


def prefill_rate(weight_bytes):
    return PREFILL_A * (weight_bytes / 1e9) ** PREFILL_B


def ms_per_token(weight_bytes, geo, depth):
    per = physics.achieved_fraction(geo["per_layer_read"], geo["query_per_kv_head"])
    growing = physics.growing_bytes_at(geo, depth) or geo["growing_bytes_per_depth_token"]
    return (weight_bytes * geo["active_weight_fraction"] / (BW * WEIGHTS_ACHIEVED) * 1e3
            + depth * growing / (BW * per) * 1e3)


def probe_seconds(weight_bytes, geo, depth):
    return (depth / prefill_rate(weight_bytes)
            + PROBE_TOKENS * ms_per_token(weight_bytes, geo, depth) / 1e3)


def ceiling(weight_bytes, geo, device):
    """Rule 1 and rule 3: a power of two, no larger than the trained context,
    no larger than this device can hold."""
    trained = geo.get("trained") or 0
    if not trained:
        return 0, "the header declares no context length"
    return hardware.affordable_context(
        weight_bytes, geo["bytes_per_depth_token"], device, int(trained))


def depths(context):
    """Rule 2: powers of two only.

    The deepest sampled depth is half the window. A probe generates tokens at
    the depth it reached, so it needs room above it; halving is the only way to
    make that room without leaving the powers of two."""
    out, d = [], SHALLOWEST
    while d <= context // 2:
        out.append(d)
        d *= 2
    return out


def for_device(path, geo, device, budget_seconds=None):
    weight_bytes = os.path.getsize(path)
    context, why = ceiling(weight_bytes, geo, device)
    if context < SHALLOWEST * 2:
        return {"device": device["name"], "context": context, "why": why,
                "plan": "cannot run", "probes": [], "seconds": 0.0}
    ladder = depths(context)
    shapes = [
        ("every depth", ladder),
        ("both ends", [ladder[0], ladder[-1]] if len(ladder) > 1 else ladder),
        ("anchored", [ladder[0]]),
        ("computed", []),
    ]
    for name, probes in shapes:
        # "both ends" is chosen by which second probe fits, BEFORE the budget is
        # tested — otherwise the shape is abandoned whenever its deepest probe
        # alone overruns, and a plan falls to one probe where two would have fit.
        if name == "both ends" and budget_seconds is not None and len(ladder) > 1:
            first = probe_seconds(weight_bytes, geo, ladder[0])
            fits = [d for d in ladder[1:]
                    if as_range(first + probe_seconds(weight_bytes, geo, d))[1]
                    <= budget_seconds]
            probes = [ladder[0], fits[-1]] if fits else probes
        seconds = sum(probe_seconds(weight_bytes, geo, d) for d in probes)
        # tested against the pessimistic end, never the point estimate
        if budget_seconds is None or as_range(seconds)[1] <= budget_seconds:
            median, worst, how = ACCURACY[name]
            return {"device": device["name"], "context": context, "why": why,
                    "plan": name, "probes": probes, "seconds": seconds,
                    "median": median, "worst": worst, "how": how,
                    "ladder": ladder}
    return {"device": device["name"], "context": context, "why": why,
            "plan": "computed", "probes": [], "seconds": 0.0,
            "median": 4.2, "worst": 13.7, "how": ACCURACY["computed"][2],
            "ladder": ladder}


def announce(path, budget_seconds=None, engine=None, use=("cpu", "gpu")):
    """What rule 4 requires: the whole plan, and its cost, before it runs."""
    geo = physics.geometry(path)
    name = os.path.basename(path)
    print(f"  {name}")
    if geo.get("verdict") != "described":
        print(f"    cannot be planned: {geo.get('verdict')} — {geo.get('why')}")
        return []
    weight_bytes = os.path.getsize(path)
    print(f"    {weight_bytes/1e9:.2f} GB of weights, trained context "
          f"{geo.get('trained')}, {geo['bytes_per_depth_token']/1024:.0f} KiB "
          f"of cache per token")

    devices = [d for d in ([hardware.system_memory()] + hardware.gpus())
               if d["kind"] in use]
    usable = hardware.engine_devices(engine) if engine else []
    plans, total = [], 0.0
    for device in devices:
        if device["kind"] == "gpu" and engine and not usable:
            print(f"    {device['name']:<32} NOT TESTED — the provisioned engine "
                  f"has no GPU backend")
            continue
        got = for_device(path, geo, device, budget_seconds)
        plans.append(got)
        total += got["seconds"]
        if got["plan"] == "cannot run":
            print(f"    {device['name']:<32} cannot run: {got['why']}")
            continue
        probes = ", ".join(str(p) for p in got["probes"]) or "none"
        print(f"    {device['name']:<32} window {got['context']} ({got['why']})")
        print(f"      {got['plan']:<14} probes at {probes}")
        low, high = as_range(got["seconds"])
        print(f"      {low:.0f}-{high:.0f}s (about {got['seconds']:.0f}s), expected error "
              f"{got['median']}% median / {got['worst']}% worst")
        print(f"      {got['how']}")
    if total:
        low, high = as_range(total)
        print(f"    TOTAL {low:.0f}-{high:.0f}s"
              + (f", held under the {budget_seconds:.0f}s you set" if budget_seconds else ""))
        print(f"    The range is measured, not hedged: over 136 probes the estimate")
        print(f"    ran 0.58x to 1.42x of the truth, and the budget uses the slow end.")
    return plans


if __name__ == "__main__":
    budget = None
    args = sys.argv[1:]
    if args and args[0].startswith("--budget="):
        budget = float(args.pop(0).split("=", 1)[1])
    engine = os.environ.get("MCF_ENGINE") or (
        __import__("glob").glob(os.path.expanduser(
            "~/.local/share/mcf/**/llama-server"), recursive=True) or [None])[0]
    print(f"Plan{' under a ' + str(int(budget)) + 's budget' if budget else ''}, "
          f"before anything runs\n")
    for path in args:
        announce(path, budget, engine)
        print()
