"""What a diagnostic should do, given a model and a time budget.

Every constant here was measured on this machine and is named where it came
from. The point is that the plan is chosen from the model's own size rather
than fixed: a 135M model can afford everything, a 70B model can afford one
probe, and the difference is two orders of magnitude.
"""

import sys
import pathlib

sys.path.insert(0, str(pathlib.Path(__file__).parent))
import physics  # noqa: E402

BW = 55.8e9                 # bandwidth.py, this machine
WEIGHTS_ACHIEVED = 0.92     # F125's held-out fit
PREFILL_A, PREFILL_B = 662.0, -0.77   # measured over 27 models

# 128 generated tokens per probe. NOT a free knob: the tokens right after a
# prefill still hit a warm cache, so a short probe reads optimistically. The
# bias is -2.6% at 32 tokens and -0.6% at 128, on both a 1.7B and an 8B model,
# against a 512-token reference. Below 128 the bias eats the whole error budget
# the method is trying to hold.
PROBE_TOKENS = 128


def prefill_rate(bytes_):
    return PREFILL_A * (bytes_ / 1e9) ** PREFILL_B


def ms_per_token(bytes_, geo, depth):
    per = physics.achieved_fraction(geo["per_layer_read"], geo["query_per_kv_head"])
    growing = physics.growing_bytes_at(geo, depth) or 0
    return (bytes_ * geo["active_weight_fraction"] / (BW * WEIGHTS_ACHIEVED) * 1e3
            + depth * growing / (BW * per) * 1e3)


def probe_seconds(bytes_, geo, depth, tokens=PROBE_TOKENS):
    return depth / prefill_rate(bytes_) + tokens * ms_per_token(bytes_, geo, depth) / 1e3


def choose(bytes_, geo, budget_seconds=300, depths=(512, 1024, 2048, 4096, 8192, 16384)):
    """The most accurate plan that fits the budget.

    Accuracy figures are the held-out measurements from F125, not estimates.
    """
    usable = [d for d in depths if not geo.get("trained") or d < geo["trained"] - 256]
    full = sum(probe_seconds(bytes_, geo, d) for d in usable)
    if full <= budget_seconds:
        return {"plan": "every depth", "probes": usable, "seconds": full,
                "median": 0.4, "worst": 3.5,
                "why": "interpolates everywhere; nothing is extrapolated"}
    # Two probes, the second as deep as the budget reaches. For a large model
    # the deep probe is dominated by PREFILL rather than by the tokens timed,
    # so taking the deepest depth outright throws the plan away when a
    # shallower second probe would still have fitted.
    shallow = usable[0]
    first = probe_seconds(bytes_, geo, shallow)
    reachable = [d for d in usable[1:]
                 if first + probe_seconds(bytes_, geo, d) <= budget_seconds]
    if reachable:
        deep = reachable[-1]
        return {"plan": "both ends", "probes": [shallow, deep],
                "seconds": first + probe_seconds(bytes_, geo, deep),
                "median": 0.8, "worst": 4.6,
                "why": f"interpolated to {deep}; past that the header's slope "
                       f"carries it, which does not decay with depth"}
    one = probe_seconds(bytes_, geo, shallow)
    if one <= budget_seconds:
        return {"plan": "anchored", "probes": [shallow], "seconds": one,
                "median": 2.2, "worst": 11.8,
                "why": "intercept measured, slope from the header; does not "
                       "decay with depth because that slope carries no noise"}
    return {"plan": "computed", "probes": [], "seconds": 0.0,
            "median": 4.2, "worst": 13.7,
            "why": "nothing generated; held-out accuracy on unseen architectures"}


if __name__ == "__main__":
    print(f"Plans under a {int(sys.argv[1]) if len(sys.argv) > 1 else 300}-second budget\n")
    budget = int(sys.argv[1]) if len(sys.argv) > 1 else 300
    print(f"  {'model':<12}{'GB':>6}  {'plan':<13}{'probes':>6}{'deepest':>9}{'takes':>9}"
          f"{'median':>9}{'worst':>8}")
    for name, gb, kv, layers, kvh, hd in [
        ("0.6B Q8", 0.64, 114688, 28, 8, 128), ("1.7B Q8", 1.83, 114688, 28, 8, 128),
        ("3B Q4", 1.85, 30720, 30, 2, 128), ("8B Q4", 5.03, 147456, 36, 8, 128),
        ("14B Q4", 8.5, 196608, 40, 8, 128), ("32B Q4", 19.0, 262144, 64, 8, 128),
        ("70B Q4", 40.0, 327680, 80, 8, 128),
    ]:
        geo = {"per_layer_read": kvh * hd * 2 * 2, "query_per_kv_head": 4.0,
               "active_weight_fraction": 1.0, "growing_bytes_per_depth_token": kv,
               "growing_bytes_past_window": kv, "sliding_window": None,
               "trained": 32768}
        got = choose(gb * 1e9, geo, budget)
        took = f"{got['seconds']:.0f}s" if got["seconds"] < 120 else f"{got['seconds']/60:.1f}m"
        deepest = max(got["probes"]) if got["probes"] else 0
        print(f"  {name:<12}{gb:>6.1f}  {got['plan']:<13}{len(got['probes']):>6}"
              f"{deepest:>9}{took:>9}{got['median']:>8.1f}%{got['worst']:>7.1f}%")
