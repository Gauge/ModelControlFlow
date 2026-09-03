#!/usr/bin/env bash
#
# Builds the Flatpak bundle (B-448): one file, `dist/ModelControlFlow-<version>.flatpak`,
# that installs on another machine with `flatpak install --user`. It is not
# published anywhere; DEC-032 (how MCF is distributed) is still open, and this
# is the artifact that lets the question be asked with something in hand.
#
# **Two ways to run it.** With `flatpak-builder` on the host, it uses that.
# Without — this machine has neither flatpak nor flatpak-builder installed —
# it runs the same steps inside a Fedora container under rootless podman,
# which needs `--privileged`: flatpak-builder sandboxes each module with bwrap,
# and a user namespace inside a user namespace is only allowed there. The
# container's flatpak state (the two 24.08 runtimes, ~1.5 GB) and the
# builder's cache persist under `~/.cache/mcf-flatpak`, so a second build does
# not download them again.
#
# **What the bundle carries** is written in the manifest,
# `packaging/flatpak/io.github.gauge.ModelControlFlow.yml`.
#
# Exit status: 0 with the bundle written and its sha256 beside it; 1 when a
# step failed; 2 when nothing on this machine could build it.

set -o errexit -o nounset -o pipefail

here=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
app=io.github.gauge.ModelControlFlow
manifest=packaging/flatpak/$app.yml
version=$(sed -n 's/^version = "\(.*\)"/\1/p' "$here/Cargo.toml" | head -1)
bundle=dist/ModelControlFlow-$version.flatpak
runtime_branch=24.08
# The Fedora image every engine is built in (component.rs), by digest.
image=registry.fedoraproject.org/fedora@sha256:5a4a491c33973b8173e6134d6f00e77f27cebef581c9b34420b2b6183a6398df

# The steps themselves, run wherever flatpak-builder is. `$1` is the source
# tree, `$2` the state directory that persists between builds.
build() {
    local tree=$1 state=$2
    cd "$tree"
    flatpak --user remote-add --if-not-exists flathub \
        https://dl.flathub.org/repo/flathub.flatpakrepo
    flatpak --user install --noninteractive --or-update flathub \
        "org.freedesktop.Platform//$runtime_branch" "org.freedesktop.Sdk//$runtime_branch"
    flatpak-builder --user --force-clean --ccache \
        --state-dir="$state/builder" --repo="$state/repo" \
        "$state/build" "$manifest"
    mkdir -p dist
    flatpak build-bundle "$state/repo" "$bundle" "$app"
    (cd dist && sha256sum "$(basename "$bundle")" > "$(basename "$bundle").sha256")
    ls -l "$bundle"
}

if [ "${1:-}" = "--inside" ]; then
    build "$2" "$3"
    exit 0
fi

if command -v flatpak-builder >/dev/null 2>&1; then
    build "$here" "$here/.flatpak-builder-state"
    exit 0
fi

if ! command -v podman >/dev/null 2>&1; then
    echo "flatpak.sh: neither flatpak-builder nor podman is on this machine; nothing here can build the bundle" >&2
    exit 2
fi

cache=${MCF_FLATPAK_CACHE:-$HOME/.cache/mcf-flatpak}
mkdir -p "$cache/home" "$cache/state"
echo "flatpak.sh: flatpak-builder is not installed; building inside $image under podman"
# The repository is mounted read-only; the builder copies the tree it needs
# (the manifest's `dir` source) and writes only into the state directory and
# `dist/`, which are mounted separately so that nothing else in the checkout
# is touched.
mkdir -p "$here/dist"
podman run --rm --privileged \
    --security-opt label=disable \
    -v "$here:/work:ro" \
    -v "$here/dist:/work/dist" \
    -v "$cache/home:/root:Z" \
    -v "$cache/state:/state:Z" \
    -w /work \
    "$image" \
    bash -c '
        set -euo pipefail
        if ! command -v flatpak-builder >/dev/null; then
            dnf -q install -y flatpak flatpak-builder ccache >/dev/null
        fi
        flatpak --version; flatpak-builder --version
        # The `dir` source copies from a path the sandbox can read; /work is
        # read-only, which is fine for reading, but the builder wants its
        # working copy elsewhere.
        exec scripts/flatpak.sh --inside /work /state
    '
