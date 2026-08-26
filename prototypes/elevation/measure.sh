#!/bin/sh
# What actually needs elevation on this machine (DEC-039, §7.39, §6.32, §XVII).
#
# §6.32 requires MCF's privileged surface be an enumerable, auditable, short
# list, and §7.39 records that the list does not exist. It cannot be written
# from documentation: what a platform lets an ordinary user do differs by
# kernel, by distribution, by hardware and by how the machine was set up. So it
# is asked.
#
# **Nothing here changes the machine.** Every check either reads, or tests
# permission the way a program would — opening a file for writing and closing
# it, or performing an operation on this probe's *own* process, which is
# reversible by the process exiting. Nothing is written to `sysfs` or `procfs`,
# no other process is touched, and no device mode is changed. A measurement that
# had to reconfigure a shared machine to find out what it could reconfigure
# would be exactly the ambient authority §6.32 exists to refuse.
#
# The operations checked are the ones MCF has a stated reason to want:
#
#   * §6.39's ladder — governor, priority, core pinning, memory locking;
#   * §3.25's environment control and A27's obligation to put it back;
#   * D8's exclusive laboratory, which is where an accelerator mode would matter;
#   * PR5's contention snapshot, which needs per-process attribution;
#   * a cold-start measurement, which wants a cold page cache.
#
#   ./prototypes/elevation/measure.sh
#
# Reports to doc/findings.md F15.

set -eu

say() { printf '%s\n' "$*"; }
rule() { printf '\n=== %s\n' "$*"; }
row() { printf '  %-46s %s\n' "$1" "$2"; }

# Reports whether a path exists and whether this user could write it, without
# writing anything. `test -w` asks the kernel the same question `open(O_WRONLY)`
# would and has no side effect.
writable() {
    if [ ! -e "$1" ]; then
        printf 'not present on this machine'
    elif [ -w "$1" ]; then
        printf 'PERMITTED as this user'
    else
        printf 'needs elevation'
    fi
}

say "as uid $(id -u) ($(id -un)), on $(uname -sr)"
say "groups: $(id -Gn)"

rule "§6.39's ladder: the environment MCF would ask a machine to stand aside with"

governor=/sys/devices/system/cpu/cpu0/cpufreq/scaling_governor
row "CPU frequency governor" "$(writable "$governor")"
[ -r "$governor" ] && row "  (currently)" "$(cat "$governor")"

row "Intel p-state turbo (no_turbo)" "$(writable /sys/devices/system/cpu/intel_pstate/no_turbo)"
row "SMT control" "$(writable /sys/devices/system/cpu/smt/control)"
row "per-core online/offline (cpu1)" "$(writable /sys/devices/system/cpu/cpu1/online)"

# Priority: performed on this probe's own child, which exits immediately. A
# negative nice is the classic thing that needs CAP_SYS_NICE — and a shell that
# *reports* success is not proof, so the child says what its priority actually
# became.
got=$(nice -n -5 sh -c 'ps -o ni= -p $$' 2>/dev/null | tr -d ' ' || printf '')
if [ "${got:-0}" = "-5" ]; then
    row "raise this process's priority (nice -5)" "PERMITTED as this user"
elif [ -n "${got:-}" ]; then
    row "raise this process's priority (nice -5)" "refused, ran at nice $got"
else
    row "raise this process's priority (nice -5)" "needs elevation"
fi
if command -v chrt >/dev/null 2>&1; then
    if chrt -f 1 true 2>/dev/null; then
        row "real-time scheduling (SCHED_FIFO)" "PERMITTED as this user"
    else
        row "real-time scheduling (SCHED_FIFO)" "needs elevation"
    fi
else
    row "real-time scheduling (SCHED_FIFO)" "cannot check: no chrt"
fi

if command -v taskset >/dev/null 2>&1; then
    if taskset -c 0 true 2>/dev/null; then
        row "pin this process to a core" "PERMITTED as this user"
    else
        row "pin this process to a core" "needs elevation"
    fi
else
    row "pin this process to a core" "cannot check: no taskset"
fi

locked=$(ulimit -l 2>/dev/null || printf 'unknown')
row "memory this user may lock (RLIMIT_MEMLOCK)" "$locked KiB"

rule "the measurement conditions MCF would like to control"

row "drop the page cache (cold start)" "$(writable /proc/sys/vm/drop_caches)"
row "IRQ affinity (irq 0)" "$(writable /proc/irq/0/smp_affinity)"
paranoid=$(cat /proc/sys/kernel/perf_event_paranoid 2>/dev/null || printf 'not present')
row "perf_event_paranoid" "$paranoid"
case "$paranoid" in
    0|-1) row "  hardware counters for this user" "PERMITTED as this user" ;;
    1) row "  hardware counters for this user" "own processes only" ;;
    2) row "  hardware counters for this user" "user-space events only" ;;
    *) row "  hardware counters for this user" "needs elevation" ;;
esac

rule "the counters D11 and §3.4 want to read"

energy=$(ls /sys/class/powercap/intel-rapl:0/energy_uj /sys/class/powercap/*/energy_uj 2>/dev/null | head -1 || true)
if [ -n "${energy:-}" ]; then
    if head -c 1 "$energy" >/dev/null 2>&1; then
        row "processor energy (RAPL, $(basename "$(dirname "$energy")"))" "READABLE as this user"
    else
        row "processor energy (RAPL, $(basename "$(dirname "$energy")"))" "present, needs elevation"
    fi
else
    row "processor energy (RAPL)" "not present on this machine"
fi

thermal=$(ls /sys/class/thermal/thermal_zone*/temp 2>/dev/null | head -1 || true)
if [ -n "${thermal:-}" ]; then
    if head -c 1 "$thermal" >/dev/null 2>&1; then
        row "processor temperature (thermal zone)" "READABLE as this user"
    else
        row "processor temperature (thermal zone)" "present, needs elevation"
    fi
else
    row "processor temperature (thermal zone)" "not present on this machine"
fi

hwmon=$(ls /sys/class/hwmon/hwmon*/temp1_input 2>/dev/null | head -1 || true)
if [ -n "${hwmon:-}" ] && head -c 1 "$hwmon" >/dev/null 2>&1; then
    row "temperatures via hwmon" "READABLE as this user"
fi

rule "PR5's contention snapshot: what can be attributed without privilege"

others=$(ps -eo user= 2>/dev/null | sort -u | wc -l)
row "processes visible to this user (owners seen)" "$others"
mine=$(ps -u "$(id -un)" -o pid= 2>/dev/null | wc -l)
row "processes of this user" "$mine"
# hidepid on /proc would hide other users' processes entirely.
row "/proc mount options" "$(awk '$2 == "/proc" { print $4 }' /proc/self/mounts | head -1)"

rule "the accelerator (D8's exclusive laboratory)"

if command -v nvidia-smi >/dev/null 2>&1; then
    row "nvidia-smi" "present"
    mode=$(nvidia-smi --query-gpu=compute_mode --format=csv,noheader 2>/dev/null | head -1 || printf 'unreadable')
    row "  compute mode (read)" "${mode:-unreadable}"
    persist=$(nvidia-smi --query-gpu=persistence_mode --format=csv,noheader 2>/dev/null | head -1 || printf 'unreadable')
    row "  persistence mode (read)" "${persist:-unreadable}"
    # NOT changed here: `nvidia-smi -c` and `-pm` are documented root-only, and
    # setting either on somebody's shared machine is not a measurement.
    row "  changing either" "root, per the vendor's own tool (not attempted)"
    row "  per-process occupancy" "$(nvidia-smi --query-compute-apps=pid --format=csv,noheader >/dev/null 2>&1 && printf 'PERMITTED as this user' || printf 'unreadable as this user')"
else
    row "nvidia-smi" "not present on this machine"
fi
for card in /sys/class/drm/card*/device/power_dpm_force_performance_level; do
    [ -e "$card" ] || continue
    row "AMD performance level ($(basename "$(dirname "$(dirname "$card")")"))" "$(writable "$card")"
done

rule "cgroups: the other way to ask for resources"

cgroup=$(awk -F: '$1 == "0" { print $3 }' /proc/self/cgroup 2>/dev/null | head -1)
row "this process's cgroup" "${cgroup:-none}"
if [ -n "${cgroup:-}" ] && [ -d "/sys/fs/cgroup$cgroup" ]; then
    row "  cpuset.cpus writable here" "$(writable "/sys/fs/cgroup$cgroup/cpuset.cpus")"
    row "  memory.high writable here" "$(writable "/sys/fs/cgroup$cgroup/memory.high")"
    controllers=$(cat "/sys/fs/cgroup$cgroup/cgroup.controllers" 2>/dev/null || printf 'unreadable')
    row "  controllers delegated" "$controllers"
fi

rule "what a helper would have to be given"

row "sudo present" "$(command -v sudo >/dev/null 2>&1 && printf yes || printf no)"
row "polkit present" "$(command -v pkexec >/dev/null 2>&1 && printf yes || printf no)"
row "setcap present" "$(command -v setcap >/dev/null 2>&1 && printf yes || printf no)"
say ""
say "Nothing above was changed. Every row is a read or a permission test."
