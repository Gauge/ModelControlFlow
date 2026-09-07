#!/usr/bin/env bash
#
# Installs MCF for the person running this (B-585): the release binary on the
# path, and the launcher and icon that put the window in the desktop's
# application search. `~/.local` by default, which needs no privilege and is
# what the desktop already looks in; `--prefix` puts it elsewhere.
#
# **What it does not do.** Build. The binary installed is the one in
# `target/release`, and an official build is made the way build.md §3 says,
# with the source revision set and every tier run against it; this
# script refuses when there is no release binary rather than building one
# without those. It does not touch the daemon: a window started from the
# launcher starts the daemon itself when none is up.
#
# Exit status: 0 with everything in place; 1 when a step failed; 2 when there
# is nothing to install.

set -o errexit -o nounset -o pipefail

here=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
app=io.github.gauge.ModelControlFlow
prefix=$HOME/.local

while [ $# -gt 0 ]; do
    case $1 in
        --prefix)
            prefix=$2
            shift 2
            ;;
        *)
            echo "usage: scripts/install.sh [--prefix <dir>]" >&2
            exit 1
            ;;
    esac
done

binary=$here/target/release/mcf
if [ ! -x "$binary" ]; then
    echo "install: no release binary at $binary" >&2
    echo "  build one first, with its revision set (build.md §3):" >&2
    echo "    MCF_BUILD_COMMIT=\$(git rev-parse HEAD) cargo build --locked --release" >&2
    exit 2
fi

bin=$prefix/bin
applications=$prefix/share/applications
icons=$prefix/share/icons/hicolor/scalable/apps

install -Dm755 "$binary" "$bin/mcf"
install -Dm644 "$here/packaging/flatpak/$app.svg" "$icons/$app.svg"
# The launcher is the Flatpak's, with the installed binary as what it runs:
# a desktop's launcher does not read the shell's path, so the path is whole.
sed -e "s#^Exec=.*#Exec=$bin/mcf desk#" -e "s#^Name=.*#Name=Model Control Flow#" \
    "$here/packaging/flatpak/$app.desktop" > "$applications/$app.desktop.tmp"
install -Dm644 "$applications/$app.desktop.tmp" "$applications/$app.desktop"
rm -f "$applications/$app.desktop.tmp"

# The desktop's caches, where the tools to refresh them are installed;
# without them the desktop notices on its own, a little later.
if command -v update-desktop-database > /dev/null 2>&1; then
    update-desktop-database "$applications" 2> /dev/null || true
fi
if command -v gtk-update-icon-cache > /dev/null 2>&1; then
    gtk-update-icon-cache -q -t "$prefix/share/icons/hicolor" 2> /dev/null || true
fi

echo "installed $("$bin/mcf" --version)"
echo "  binary    $bin/mcf"
echo "  launcher  $applications/$app.desktop  (search for \"Model Control Flow\")"
echo "  icon      $icons/$app.svg"
case ":$PATH:" in
    *":$bin:"*) ;;
    *) echo "  $bin is not on this shell's path; the launcher does not need it, a terminal does" ;;
esac
