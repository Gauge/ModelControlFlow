"""The memory hierarchy of this machine, measured rather than looked up.

The fall-off is attention re-reading the whole KV cache once per generated
token. So the machine's read bandwidth AT THE SIZE THE CACHE HAPPENS TO BE is
the thing that sets the slope — and bandwidth is not one number. It is a curve
with steps in it where the working set stops fitting a level.

lscpu will report the cache sizes. That is a declaration, not a measurement
(A21): what matters is the bandwidth actually achieved, which depends on how
many threads are reading, whether they share a core complex, and what else the
machine is doing. This measures it.

Threads matter twice over. The engine reads its cache across many threads, so
a single-threaded figure understates it; and this part has two core complexes
with a separate L3 each, so a working set that fits one CCD's L3 does not
necessarily stay there when the threads are spread across both.
"""

import sys
import pathlib
import time
from concurrent.futures import ThreadPoolExecutor

import numpy as np

sys.path.insert(0, str(pathlib.Path(__file__).parent))
from sampler import snapshot  # noqa: E402

SIZES_MB = [0.25, 0.5, 1, 2, 4, 8, 16, 24, 32, 48, 64, 96, 128, 256, 512, 1024]
THREADS = [1, 4, 8, 16]


# Below this per-thread slice the loop is bound by Python's own overhead
# rather than by memory, and the cell measures the harness instead of the
# machine. Found by measuring: 16 threads over 0.25 MB reported 7.3 GB/s,
# which is slower than DRAM and therefore cannot be a cache reading.
MIN_SLICE = 1 << 20


def measure(size_bytes, threads, seconds=0.30):
    """Read bandwidth for one working-set size at one thread count.

    Each thread reads its OWN slice, so the aggregate working set is the size
    asked for and the threads are not all hammering one cache line."""
    n = max(size_bytes // 8, 1024)
    per = n // threads
    if per * 8 < MIN_SLICE:
        return None
    array = np.ones(n)
    slices = [array[i * per:(i + 1) * per] for i in range(threads)]
    for s in slices:                       # first touch, outside the timing
        float(s.sum())

    def work(s):
        total, reps = 0.0, 0
        started = time.perf_counter()
        while time.perf_counter() - started < seconds:
            total += float(s.sum())
            reps += 1
        return reps, time.perf_counter() - started, total

    best = 0.0
    for _ in range(3):
        with ThreadPoolExecutor(max_workers=threads) as pool:
            started = time.perf_counter()
            results = list(pool.map(work, slices))
        elapsed = time.perf_counter() - started
        moved = sum(r[0] for r in results) * per * 8
        best = max(best, moved / elapsed)
    return best


def main():
    before = snapshot()
    print("Read bandwidth against working-set size, GB/s")
    print("=" * 68)
    print("\n  Each thread reads its own slice; the size is the total.\n")
    print(f"    {'working set':>13}" + "".join(f"{str(t) + ' thr':>11}" for t in THREADS))
    table = {}
    for mb in SIZES_MB:
        line = f"    {mb:>9.2f} MB "
        for t in THREADS:
            got = measure(int(mb * 1024 * 1024), t)
            if got is None:
                line += f"{'-':>11}"
                continue
            table[(mb, t)] = got / 1e9
            line += f"{got / 1e9:>11.1f}"
        print(line)
    after = snapshot()

    print("\n  Single thread, where the steps are — one thread sees ONE core")
    print("  complex's L3, so its step is the per-CCD size:")
    prev1 = None
    for mb in SIZES_MB:
        if (mb, 1) not in table:
            continue
        v1 = table[(mb, 1)]
        if prev1 is not None and v1 < prev1 * 0.80:
            print(f"    a fall of {(1 - v1 / prev1) * 100:.0f}% between "
                  f"{prev1_mb:g} MB and {mb:g} MB")
        prev1, prev1_mb = v1, mb

    print("\n  Where the steps are (16 threads), as a ratio to the step before:")
    prev = None
    for mb in SIZES_MB:
        if (mb, 16) not in table:
            continue
        v = table[(mb, 16)]
        if prev is not None and v < prev * 0.72:
            print(f"    a fall of {(1 - v / prev) * 100:.0f}% between "
                  f"{prev_mb:g} MB and {mb:g} MB")
        prev, prev_mb = v, mb

    dram = min(v for (m, t), v in table.items() if t == 16 and m >= 512)
    # in-cache is read single-threaded: it is the only column whose small
    # working sets are memory-bound rather than harness-bound
    cache = max(v for (m, t), v in table.items() if t == 1 and m <= 24)
    print(f"\n  in-cache  {cache:.1f} GB/s   (1 thread; see MIN_SLICE)")
    print(f"  from DRAM {dram:.1f} GB/s      <- the number that sets the fall-off")
    print(f"\n  die {before.get('temp_Tccd1')} -> {after.get('temp_Tccd1')} C, "
          f"clock {before.get('freq_max_mhz'):.0f} -> {after.get('freq_max_mhz'):.0f} MHz")
    with open("/tmp/falloff/bandwidth.tsv", "w") as handle:
        handle.write("megabytes\tthreads\tgigabytes_per_second\n")
        for (mb, t), v in sorted(table.items()):
            handle.write(f"{mb}\t{t}\t{v:.3f}\n")
    print("  written to /tmp/falloff/bandwidth.tsv")


if __name__ == "__main__":
    main()
