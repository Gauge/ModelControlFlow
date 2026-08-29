"""What the machine was doing while a timing was taken.

MCF has been recording one number beside its timings — a die temperature — and
inferring everything else. This samples the rest of what this machine will
give up without privilege, so a reading can say what the conditions were
rather than assuming them.

Nothing here is privileged. What needs root is listed in WHAT_IS_LOCKED and is
not silently skipped (A2, A7): a caller is told the reading is unavailable and
why, rather than being handed a zero.

The instrument is only worth having if it does not disturb what it measures,
which is not something to assert. `overhead()` measures it, and the lab test
beside this file compares a timed workload with the sampler running and with
it stopped.
"""

import os
import glob
import threading
import time

WHAT_IS_LOCKED = {
    "package energy (RAPL)": "/sys/class/powercap/intel-rapl:0/energy_uj is root-only; "
    "would give joules per generation, and so joules per token",
    "DRAM controller counters": "amd_umc_0/1 need the perf binary and "
    "perf_event_paranoid <= 0; would count bytes moved rather than infer them",
    "L3 counters": "amd_l3 likewise; would show the cache crossing directly",
}


def _read_int(path):
    try:
        with open(path) as handle:
            return int(handle.read().strip())
    except (OSError, ValueError):
        return None


def _cpu_freqs():
    """Every core's current clock, in kHz. The governor here is `performance`,
    which does not mean the clock is constant: boost still moves with power and
    temperature, and a drift in it looks exactly like a drift in the thing
    being timed."""
    out = []
    for path in sorted(glob.glob("/sys/devices/system/cpu/cpu*/cpufreq/scaling_cur_freq")):
        value = _read_int(path)
        if value is not None:
            out.append(value)
    return out


def _temps():
    """k10temp: Tctl is the control temperature, Tccd1 and Tccd2 the two core
    complexes. Which CCD the work landed on matters — each has its own L3."""
    out = {}
    for label_path in glob.glob("/sys/class/hwmon/hwmon*/temp*_label"):
        try:
            with open(label_path) as handle:
                label = handle.read().strip()
        except OSError:
            continue
        if label.startswith(("Tctl", "Tccd")):
            value = _read_int(label_path.replace("_label", "_input"))
            if value is not None:
                out[label] = value / 1000.0
    return out


def _proc(pid):
    """Resident memory and storage traffic for one process. read_bytes is what
    actually came off the device — a warm run should show none, and a cold one
    should show the weights."""
    out = {}
    try:
        with open(f"/proc/{pid}/status") as handle:
            for line in handle:
                if line.startswith(("VmRSS:", "VmHWM:")):
                    out[line.split(":")[0]] = int(line.split()[1]) * 1024
    except OSError:
        pass
    try:
        with open(f"/proc/{pid}/io") as handle:
            for line in handle:
                key, _, value = line.partition(":")
                if key in ("read_bytes", "write_bytes", "rchar"):
                    out[key] = int(value)
    except OSError:
        pass
    try:
        with open(f"/proc/{pid}/stat") as handle:
            fields = handle.read().split()
        out["minflt"], out["majflt"] = int(fields[9]), int(fields[11])
        out["utime_ticks"], out["stime_ticks"] = int(fields[13]), int(fields[14])
    except (OSError, IndexError, ValueError):
        pass
    return out


def snapshot(pid=None):
    freqs = _cpu_freqs()
    shot = {
        "wall": time.time(),
        "freq_mean_mhz": (sum(freqs) / len(freqs) / 1000.0) if freqs else None,
        "freq_max_mhz": (max(freqs) / 1000.0) if freqs else None,
        "loadavg": os.getloadavg()[0],
    }
    shot.update({f"temp_{k}": v for k, v in _temps().items()})
    if pid:
        shot.update({f"proc_{k}": v for k, v in _proc(pid).items()})
    return shot


class Sampler(threading.Thread):
    """Samples in the background while something else is timed.

    Deliberately a thread rather than a subprocess: the cost of a sample is a
    few sysfs reads, and a process would cost more to start than to run."""

    def __init__(self, pid=None, interval=0.25):
        super().__init__(daemon=True)
        self.pid, self.interval = pid, interval
        self.samples = []
        # NOT _stop: threading.Thread uses that name internally, and
        # shadowing it breaks join() in a way that only shows under load.
        self._halt = threading.Event()

    def run(self):
        while not self._halt.is_set():
            self.samples.append(snapshot(self.pid))
            self._halt.wait(self.interval)

    def stop(self):
        self._halt.set()
        self.join(timeout=2.0)
        return self.samples

    def summary(self):
        """What is worth putting beside a timing: the range each condition
        moved over, not a single value that hides the drift (A6)."""
        if not self.samples:
            return {}
        out = {"n_samples": len(self.samples)}
        keys = {k for s in self.samples for k in s if k != "wall"}
        for key in sorted(keys):
            vals = [s[key] for s in self.samples if s.get(key) is not None]
            if not vals:
                continue
            if key.startswith("proc_") and key.split("_")[-1] in (
                "bytes", "minflt", "majflt", "ticks", "rchar",
            ) or key in (
                "proc_read_bytes", "proc_write_bytes", "proc_rchar",
                "proc_minflt", "proc_majflt", "proc_utime_ticks", "proc_stime_ticks",
            ):
                out[key + "_delta"] = vals[-1] - vals[0]   # counters: the change
            else:
                out[key + "_first"], out[key + "_last"] = vals[0], vals[-1]
                out[key + "_min"], out[key + "_max"] = min(vals), max(vals)
        return out


def overhead(interval=0.25, seconds=2.0, pid=None):
    """How long one sample takes, and what fraction of a second that is."""
    started = time.perf_counter()
    n = 0
    while time.perf_counter() - started < seconds:
        snapshot(pid)
        n += 1
    each = (time.perf_counter() - started) / n
    return {
        "seconds_per_sample": each,
        "samples_taken": n,
        "duty_at_interval": each / interval,
    }


if __name__ == "__main__":
    import json
    print(json.dumps(overhead(pid=os.getpid()), indent=2))
    print(json.dumps(snapshot(os.getpid()), indent=2, default=str))
    print("\nlocked without privilege:")
    for what, why in WHAT_IS_LOCKED.items():
        print(f"  {what}: {why}")
