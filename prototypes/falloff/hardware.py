"""What this machine can afford, per device, before anything is run.

A diagnostic that picks its context window from the model alone will ask for a
window the machine cannot hold, and find out by being killed. The ceiling is a
property of the pair — this model, this device — and it is arithmetic, not an
experiment: the weights are a file size and the cache is a size per token that
the header states.

Every device is considered, not only the one MCF happens to use, because
"context this machine can afford" has a different answer on each and the
operator asked for both.
"""

import glob
import os
import re
import subprocess

# Room for the engine's own buffers: compute graph, logits, scratch. Measured
# rather than guessed — a Qwen3-0.6B at -c 512 held 187 MB resident against
# 359 MB at -c 8192, so the fixed part is a few hundred megabytes and the rest
# is cache. Kept generous so a plan never proposes a window that is refused.
OVERHEAD_BYTES = 512 << 20
HEADROOM = 0.85          # never plan to fill a device to the brim


def system_memory():
    """Available rather than total: what is free for a new process now."""
    with open("/proc/meminfo") as handle:
        fields = dict(
            (k.strip(), int(re.sub(r"[^0-9]", "", v)) * 1024)
            for k, v in (line.split(":", 1) for line in handle)
        )
    return {
        "name": "CPU (system memory)",
        "kind": "cpu",
        "total": fields.get("MemTotal", 0),
        "available": fields.get("MemAvailable", 0),
    }


def gpus():
    """Every GPU the machine reports, with the memory it actually has.

    nvidia-smi where it answers; the kernel's own amdgpu counters otherwise, so
    a machine with no vendor tool installed still gets a real number."""
    found = []
    try:
        out = subprocess.run(
            ["nvidia-smi", "--query-gpu=name,memory.total,memory.free",
             "--format=csv,noheader,nounits"],
            capture_output=True, text=True, timeout=15,
        )
        for line in out.stdout.strip().splitlines():
            parts = [p.strip() for p in line.split(",")]
            if len(parts) >= 3:
                found.append({"name": parts[0], "kind": "gpu",
                              "total": int(parts[1]) * 1024 * 1024,
                              "available": int(parts[2]) * 1024 * 1024})
    except (OSError, subprocess.SubprocessError, ValueError):
        pass
    for path in sorted(glob.glob("/sys/class/drm/card*/device/mem_info_vram_total")):
        try:
            with open(path) as handle:
                total = int(handle.read().strip())
            used_path = path.replace("_total", "_used")
            used = 0
            if os.path.exists(used_path):
                with open(used_path) as handle:
                    used = int(handle.read().strip())
            name = "AMD GPU"
            vendor = os.path.join(os.path.dirname(path), "vendor")
            if os.path.exists(vendor):
                with open(vendor) as handle:
                    name = {"0x1002": "AMD GPU", "0x10de": "NVIDIA GPU"}.get(
                        handle.read().strip(), "GPU")
            found.append({"name": f"{name} ({os.path.basename(os.path.dirname(os.path.dirname(path)))})",
                          "kind": "gpu", "total": total, "available": total - used})
        except (OSError, ValueError):
            continue
    # nvidia-smi and the kernel may both describe the same card; prefer the
    # named one and drop a duplicate of the same size.
    seen, unique = set(), []
    for d in found:
        key = round(d["total"] / (64 << 20))
        if key in seen:
            continue
        seen.add(key)
        unique.append(d)
    return unique


def engine_devices(engine):
    """What the ENGINE can actually use, which is not what the machine has.

    A build without a GPU backend reports no devices however many cards are
    installed, and a plan that offered to test one would be proposing something
    the binary cannot do (A21: declared is not verified)."""
    try:
        out = subprocess.run([engine, "--list-devices"], capture_output=True,
                             text=True, timeout=30)
    except (OSError, subprocess.SubprocessError):
        return []
    names = []
    for line in out.stdout.splitlines():
        line = line.strip()
        if not line or line.lower().startswith("available devices"):
            continue
        if line.lower() == "(none)":
            return []
        names.append(line)
    return names


def affordable_context(weight_bytes, kv_bytes_per_token, device, trained,
                       floor=1024):
    """The largest POWER OF TWO context this device can hold for this model.

    Rule 2 is why the search walks powers of two rather than solving for the
    exact fit: a window that is not a power of two is not a window this
    diagnostic may target. Rule 1 is the cap at the trained context — a model
    is never asked about a window it was not trained for."""
    budget = device["available"] * HEADROOM - OVERHEAD_BYTES - weight_bytes
    if budget <= 0:
        return 0, "the weights alone do not fit"
    context = floor
    best = 0
    while context <= trained:
        if context * kv_bytes_per_token <= budget:
            best = context
        else:
            break
        context *= 2
    if best == 0:
        return 0, f"not even {floor} tokens of cache fit"
    why = ("capped by the model's trained context" if best * 2 > trained
           else "capped by this device's memory")
    return best, why


def report(engine=None):
    devices = [system_memory()] + gpus()
    usable = engine_devices(engine) if engine else []
    lines = []
    for d in devices:
        lines.append(f"  {d['name']:<34}{d['total']/1e9:>7.1f} GB total"
                     f"{d['available']/1e9:>8.1f} GB free")
    return devices, usable, lines


if __name__ == "__main__":
    import sys
    engine = sys.argv[1] if len(sys.argv) > 1 else None
    devices, usable, lines = report(engine)
    print("Devices this machine has:")
    print("\n".join(lines))
    if engine:
        print(f"\nDevices the provisioned engine can use: "
              f"{', '.join(usable) if usable else 'NONE — this build has no GPU backend'}")
        if not usable and any(d["kind"] == "gpu" for d in devices):
            print("  So a GPU arm cannot be run, however many cards are present.")
            print("  That is a fact about the build, not about the machine (A21).")
