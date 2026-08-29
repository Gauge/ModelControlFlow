"""Does the instrument disturb what it measures?

An instrument that changes the reading is worse than no instrument, because
the reading still looks like a reading. This is the control: the same
deterministic workload, timed with the sampler running and with it stopped,
many times, interleaved so that a drift over the run cannot land on one arm.

It produces no claim about MCF's speed — the workload is arithmetic chosen for
being repeatable, not for resembling a model (A11, A18). What is being graded
is the difference between two arms, and a difference is not a performance
number.
"""

import statistics as st
import sys
import pathlib
import time

import numpy as np

sys.path.insert(0, str(pathlib.Path(__file__).parent))
from sampler import Sampler, overhead  # noqa: E402

REPEATS = 40


def workload():
    """Memory-bound, like the thing we actually time: an array far larger than
    L3, summed. If the sampler's sysfs reads were going to evict anything or
    steal a core, this is where it would show."""
    a = workload.array
    return float(a.sum())


workload.array = np.ones(64 * 1024 * 1024 // 8)  # 64 MB, past a 32 MiB CCD L3


def timed():
    started = time.perf_counter()
    workload()
    return time.perf_counter() - started


def main():
    print("Sampler perturbation control")
    print("=" * 60)
    o = overhead()
    print(f"\n  one sample costs {o['seconds_per_sample'] * 1e6:.0f} us")
    print(f"  at a 0.25 s interval that is {o['duty_at_interval'] * 100:.2f}% of a core\n")

    workload()  # first touch, so neither arm pays for the page faults
    off, on = [], []
    for i in range(REPEATS):
        # interleaved, alternating which arm goes first, so that neither the
        # order within a pair nor a drift across pairs favours an arm
        if i % 2 == 0:
            off.append(timed())
            s = Sampler(interval=0.01); s.start(); on.append(timed()); s.stop()
        else:
            s = Sampler(interval=0.01); s.start(); on.append(timed()); s.stop()
            off.append(timed())

    for name, vals in (("sampler off", off), ("sampler on ", on)):
        print(f"  {name}  median {st.median(vals) * 1e3:7.3f} ms   "
              f"min {min(vals) * 1e3:7.3f}   max {max(vals) * 1e3:7.3f}   n={len(vals)}")

    d = (st.median(on) - st.median(off)) / st.median(off) * 100
    # the spread of the quiet arm is the yardstick: a difference smaller than
    # the noise of doing nothing is not a difference
    noise = (st.pstdev(off) / st.median(off)) * 100
    print(f"\n  difference {d:+.2f}%   |   spread of the undisturbed arm {noise:.2f}%")
    print(f"  sampling at 0.01 s here — FORTY times more often than in use.\n")
    if abs(d) <= noise:
        print("  -> the instrument is not visible above the noise of the thing")
        print("     it measures, at forty times its working rate.")
    else:
        print("  -> VISIBLE. The sampler is changing the reading and must be")
        print("     slowed, moved off the timed core, or dropped.")


if __name__ == "__main__":
    main()
