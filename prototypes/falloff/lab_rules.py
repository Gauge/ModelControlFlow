"""The operator's four sampling rules, checked against every model here.

Written as a lab rather than a comment because B16 prefers a rule a machine can
check: a rule stated in prose is a rule that drifts the first time the planner
is edited. Each check names the rule it enforces and fails by name.

No timing originates here (A11): the planner's estimates are arithmetic on
constants measured elsewhere.
"""

import glob
import os
import sys
import pathlib

sys.path.insert(0, str(pathlib.Path(__file__).parent))
import physics   # noqa: E402
import hardware  # noqa: E402
import plan      # noqa: E402

BUDGETS = [30, 60, 120, 300, 900, None]


def power_of_two(n):
    return n > 0 and (n & (n - 1)) == 0


def main():
    paths = [p for p in sorted(glob.glob(
        os.path.expanduser("~/.local/share/mcf/**/*.gguf"), recursive=True))
        if not os.path.basename(p).startswith("ggml-vocab-")]
    devices = [hardware.system_memory()] + hardware.gpus()
    failures, checked, planned = [], 0, 0

    for path in paths:
        geo = physics.geometry(path)
        if geo.get("verdict") != "described":
            continue
        trained = geo.get("trained") or 0
        for device in devices:
            for budget in BUDGETS:
                got = plan.for_device(path, geo, device, budget)
                checked += 1
                name = f"{os.path.basename(path)} on {device['name']} at {budget}"
                window = got["context"]

                # RULE 1 — nothing is sampled above the model's trained context
                if trained and window > trained:
                    failures.append(f"[1] {name}: window {window} > trained {trained}")
                for d in got["probes"]:
                    if trained and d >= trained:
                        failures.append(f"[1] {name}: depth {d} >= trained {trained}")

                # RULE 2 — every window and depth is a power of two
                if window and not power_of_two(window):
                    failures.append(f"[2] {name}: window {window} is not a power of two")
                for d in got["probes"]:
                    if not power_of_two(d):
                        failures.append(f"[2] {name}: depth {d} is not a power of two")
                    if d >= window:
                        failures.append(f"[2] {name}: depth {d} leaves no room in {window}")

                # RULE 3 — the window is the most the DEVICE can afford
                if window:
                    planned += 1
                    kv = geo["bytes_per_depth_token"]
                    need = (os.path.getsize(path) + window * kv
                            + hardware.OVERHEAD_BYTES)
                    if need > device["available"] * hardware.HEADROOM:
                        failures.append(f"[3] {name}: window {window} needs "
                                        f"{need/1e9:.1f} GB, device has "
                                        f"{device['available']/1e9:.1f} GB")
                    bigger = window * 2
                    fits = (os.path.getsize(path) + bigger * kv
                            + hardware.OVERHEAD_BYTES) <= device["available"] * hardware.HEADROOM
                    if fits and (not trained or bigger <= trained):
                        failures.append(f"[3] {name}: window {window} but {bigger} "
                                        f"would have fitted and is within the trained context")

                # RULE 4 — an estimate exists, is a range, and honours the budget
                if got["probes"]:
                    low, high = plan.as_range(got["seconds"])
                    if not (low < got["seconds"] < high):
                        failures.append(f"[4] {name}: estimate is not a range")
                    if budget is not None and high > budget:
                        failures.append(f"[4] {name}: worst case {high:.0f}s "
                                        f"exceeds the {budget}s budget")

    print("The four sampling rules, checked over every model, device and budget")
    print("=" * 74)
    print(f"\n  {len(paths)} model files, {len(devices)} devices, "
          f"{len(BUDGETS)} budgets -> {checked} plans, {planned} with a window\n")
    print("  1. nothing sampled above the model's trained context")
    print("  2. every window and depth is a power of two, and a depth leaves")
    print("     room inside its own window")
    print("  3. the window is the largest the device can afford, and no larger")
    print("     window would have fitted")
    print("  4. an estimate is produced as a range, and its slow end fits the")
    print("     budget the caller set\n")
    if failures:
        print(f"  {len(failures)} VIOLATION(S):")
        for f in failures[:25]:
            print(f"    {f}")
        return 1
    print("  no violations.")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
