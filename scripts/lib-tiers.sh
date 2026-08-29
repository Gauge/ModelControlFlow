#!/usr/bin/env bash
#
# What a tier's age is, and what makes one stale (B-185, B38).
#
# Sourced by `scripts/ci.sh`, which stamps a tier when it passes, and by
# `scripts/check-tier-ages.sh`, which refuses a release when a heavy tier's
# stamp is missing or stale. One file, because two implementations of "is this
# result still about this code" would eventually disagree, and the one that
# disagreed in the permissive direction would be the one nobody noticed.
#
# **A tier is stale when the source has changed since it ran, not when a clock
# says so.** The obvious design is a maximum age in days, and it needs a number
# nobody has: D24 states sixteen figures and none of them is how old a soak
# result may be. What can be said without inventing intent (A23) is the thing
# that actually invalidates a result — the code it was taken against is no
# longer the code in the tree. A wall-clock age is reported beside it, because
# a reader wants to know *when*, but the verdict does not rest on it.
#
# **What the digest covers is what can change a tier's outcome**: the
# workspace's manifests, the toolchain pin, every crate, the checks and the
# scripts. Documents are excluded — a rewritten paragraph in `doc/` cannot
# change what a soak run does, and treating it as if it could would make every
# tier stale after every documentation commit, which is how a staleness
# mechanism gets switched off.

# The mutation score this suite may not fall below (B-186, B20).
#
# **A hundred per cent, and the number is smaller than it sounds.** It is not a
# claim that every conceivable mutation of MCF dies; it is a claim about the
# eleven in `scripts/check-mutants.sh`, each of which breaks something a rule in
# rules.md depends on. A survivor there names a rule nothing is checking, which
# is not a percentage to be traded off — it is a gap, and B20 budgets a property
# by refusing the trade rather than by pricing it.
#
# The floor is a floor and not the whole of the check: the tier also refuses a
# score *lower than the last one recorded*, which is what B20 means by no silent
# regression and what a floor alone cannot see once the catalogue grows.
readonly MUTATION_FLOOR_PERCENT=100

# Where the stamps live. Outside `target/`, because a tier's result is about the
# source rather than about the build directory: `cargo clean` throws away
# something derived, and it should not throw away the evidence that a four-
# minute tier ran.
readonly TIER_STAMPS=".mcf-tiers"

# A digest of everything that can change a tier's outcome.
#
# `sha256sum` over a sorted file list. The sort is what makes it a digest of the
# tree rather than of a directory traversal order, which differs between
# filesystems.
tier_source_digest() {
    local root="$1"
    if ! command -v sha256sum >/dev/null 2>&1; then
        printf 'unknown\n'
        return 0
    fi
    (
        cd "$root" || exit 1
        find Cargo.toml Cargo.lock rust-toolchain.toml clippy.toml crates checks scripts \
            -type f -print0 2>/dev/null |
            LC_ALL=C sort -z |
            xargs -0 sha256sum |
            sha256sum |
            cut -d' ' -f1
    )
}

# Writes a stamp for a tier that has just passed.
#
# The stamp says what ran, when, against which source, and anything the tier
# wants to carry forward — the mutation tier's score, for instance, which is
# what B-186's floor will be compared against.
tier_stamp() {
    local root="$1" tier="$2" detail="${3:-}"
    local digest
    digest=$(tier_source_digest "$root")
    mkdir -p "$root/$TIER_STAMPS"
    {
        printf 'tier %s\n' "$tier"
        printf 'at %s\n' "$(date -u +%Y-%m-%dT%H:%M:%SZ)"
        printf 'epoch %s\n' "$(date -u +%s)"
        printf 'source %s\n' "$digest"
        # An `if` rather than `[ … ] && printf`: under `errexit` a false test at
        # the end of a function is a failing function, and a tier with nothing
        # extra to say would have failed the run that stamped it. The same
        # mistake `ci.sh` made when it reported which tiers had not run.
        if [ -n "$detail" ]; then
            printf 'detail %s\n' "$detail"
        fi
    } >"$root/$TIER_STAMPS/$tier"
}

# Reads one field out of a stamp, or nothing when there is no stamp.
tier_stamp_field() {
    local root="$1" tier="$2" field="$3"
    local file="$root/$TIER_STAMPS/$tier"
    [ -f "$file" ] || return 0
    awk -v want="$field" '$1 == want { $1 = ""; sub(/^ /, ""); print; exit }' "$file"
}

# A wall-clock age in a form a person reads, from an epoch second.
tier_age_in_words() {
    local then="$1" now
    now=$(date -u +%s)
    local seconds=$(( now - then ))
    if [ "$seconds" -lt 0 ]; then
        printf 'in the future, which means a clock moved\n'
    elif [ "$seconds" -lt 3600 ]; then
        printf '%d minutes ago\n' $(( seconds / 60 ))
    elif [ "$seconds" -lt 172800 ]; then
        printf '%d hours ago\n' $(( seconds / 3600 ))
    else
        printf '%d days ago\n' $(( seconds / 86400 ))
    fi
}

# A runtime directory of the tier's own, so that a daemon somebody left running
# does not answer for the binary the tier just built (F104, F103, §3.12).
#
# `mcf run`, `mcf bench` and `mcf probe` all look for a control socket under
# `XDG_RUNTIME_DIR` and let a listening daemon serve the work. That is correct
# for a person at a terminal — it is what the daemon is *for* — and wrong for a
# check: the daemon is another process running another binary, built from source
# that may be days old, and the account it returns is indistinguishable from the
# fresh binary's. F103 pinned *which engine* answers a tier's question;
# `--engine stand-in` is honoured by the daemon, and the daemon's stand-in is
# still not the one under test. This pins *which build* answers it.
#
# The Rust tier has done this since it was written: `crates/mcf-cli/tests/`
# gives every process a machine of its own, including its socket. The shell
# tiers handed it the operator's.
#
# **The path is short by construction.** A Unix socket path must fit in
# `sun_path`, 108 bytes on Linux, and the first attempt at this experiment put
# the directory under a scratch path whose name alone was 96: the daemon
# refused to start, which was honest, but a tier that isolates itself only where
# the path happens to be short is a tier that isolates itself on some machines
# and not others. `TMPDIR` is used when it leaves room and `/tmp` when it does
# not, and which one was used is visible in the directory the caller prints.
#
# Prints the directory. The caller exports it and removes it; it is not removed
# here because a helper that installed its own `trap` would silently replace the
# caller's.
tier_private_runtime_dir() {
    # "/mcf-tier-XXXXXX" is 16 bytes and "/mcf/control.sock" — what MCF appends
    # — is 17. Ten bytes of margin for the platform's own accounting.
    local room=$(( 108 - 16 - 17 - 10 ))
    local base=${TMPDIR:-/tmp}
    if [ "${#base}" -gt "$room" ]; then
        base=/tmp
    fi
    mktemp -d "$base/mcf-tier-XXXXXX"
}
