"""The laboratory for the estimator: curves whose true shape is known.

The prototypes measure the machine. This measures *the method* — and it can,
because here the answer is set in advance and the estimator is graded against
it. It needs no window, no engine and no model, and it produces no timing: the
durations in it are constructed, never observed (A11), which is exactly why
they may be compared with a truth.

Three questions, none of which real readings can answer, because with real
readings nobody knows the true curve:

  1. How deep must a diagnostic measure to predict N times deeper, honestly,
     at the noise real hardware actually has?
  2. When the truth bends somewhere past where we stopped looking, how wrong
     does the projection get — and does the estimator have any way to know?
  3. When the estimator states an interval, does the truth land inside it as
     often as the interval claims? An interval that has never been scored is
     decoration.
"""

import sys
import pathlib

import numpy as np

sys.path.insert(0, str(pathlib.Path(__file__).parent))
import curves  # noqa: E402

# The depths a banded run reports: midpoints of doubling bands.
DEPTHS = np.array([64, 192, 384, 768, 1536, 3072, 6144, 12288], dtype=float)

# Noise as *measured* on this machine rather than assumed: the spread between
# repeats was about 0.8% (Qwen3) and 2.8% (SmolLM2) in the sustained ladder.
NOISE = {"quiet (0.8%)": 0.008, "ordinary (2.8%)": 0.028}


def truth_linear(d):
    """Attention exactly as the theory has it."""
    return 13.2 + 0.0024 * d


def truth_saturating(d):
    """Cost that flattens once the cache stops fitting a level of the
    hierarchy. The real Qwen3 readings lean this way."""
    return 13.2 + 0.0043 * d / (1 + d / 4000.0)


def truth_knee_up(d):
    """The adversary: flat and well-behaved everywhere a cheap diagnostic
    looks, then twice as steep past 4096. Nothing in the shallow readings
    announces it."""
    return np.where(d <= 4096, 13.2 + 0.0024 * d, 13.2 + 0.0024 * 4096 + 0.0060 * (d - 4096))


TRUTHS = {
    "linear": truth_linear,
    "saturating": truth_saturating,
    "knee beyond reach": truth_knee_up,
}

CUTS = [512, 1024, 2048, 4096]
TRIALS = 400


def run():
    rng = np.random.default_rng(20260829)
    print("Every cell: fit only on depths <= cut, then predict every deeper")
    print("band. The number is the error at the DEEPEST band (12288), as a")
    print("percentage of the truth — median, and 90th percentile over")
    print(f"{TRIALS} noisy repeats.\n")

    coverage = {}
    for noise_name, sd in NOISE.items():
        print(f"── noise {noise_name} {'─' * 46}")
        for truth_name, truth in TRUTHS.items():
            clean = truth(DEPTHS)
            print(f"\n  truth: {truth_name}")
            header = "    " + f"{'fitted form':<12}" + "".join(f"{'cut ' + str(c):>16}" for c in CUTS)
            print(header)
            for form in curves.FORMS:
                row = f"    {form:<12}"
                for cut in CUTS:
                    errs, hits = [], []
                    for _ in range(TRIALS):
                        y = clean * (1 + rng.normal(0, sd, len(DEPTHS)))
                        got = curves.extrapolation_trial(DEPTHS, y, cut, forms=[form])
                        if form not in got:
                            continue
                        r = got[form]
                        deepest = int(np.argmax(r["depths"]))
                        # graded against the TRUTH, not the noisy reading
                        t = truth(r["depths"][deepest])
                        errs.append(abs(r["predicted"][deepest] - t) / t * 100)
                        hits.append(abs(r["predicted"][deepest] - t) <= r["half_width"][deepest])
                    if not errs:
                        row += f"{'—':>16}"
                        continue
                    med, p90 = np.median(errs), np.percentile(errs, 90)
                    row += f"{med:>7.1f} /{p90:>6.1f}%"
                    coverage[(noise_name, truth_name, form, cut)] = float(np.mean(hits))
                print(row)
        print()

    print("\n── are the stated intervals honest? ─────────────────────────────")
    print("  A 2-sigma interval claims to contain the truth ~95% of the time.")
    print("  Measured coverage, pooled over cuts and noise levels:\n")
    print(f"    {'fitted form':<12}" + "".join(f"{t:>22}" for t in TRUTHS))
    for form in curves.FORMS:
        row = f"    {form:<12}"
        for truth_name in TRUTHS:
            vals = [v for k, v in coverage.items() if k[1] == truth_name and k[2] == form]
            row += f"{(np.mean(vals) * 100 if vals else float('nan')):>21.0f}%"
        print(row)

    print("\n── what this settles ────────────────────────────────────────────")
    best = {}
    for truth_name in TRUTHS:
        for form in curves.FORMS:
            vals = [
                v
                for k, v in coverage.items()
                if k[1] == truth_name and k[2] == form
            ]
            best.setdefault(truth_name, {})[form] = np.mean(vals) if vals else 0.0
    print("  Read the 'knee beyond reach' column: it is the case where the")
    print("  truth does something past the fitted range that the fitted range")
    print("  cannot show. Any form scoring well there is scoring by luck, and")
    print("  any interval with high coverage there would be an interval that")
    print("  knows what it cannot know.")


if __name__ == "__main__":
    run()
