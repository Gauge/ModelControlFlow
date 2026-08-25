#!/bin/sh
# What a network costs MCF, measured rather than argued (F9, B-021).
#
# MCF has no network. §III requires models enter this machine, the hub speaks
# HTTPS, and nothing in MCF's tree can open a TLS session — so the acquisition
# path stops at a boundary no decision has crossed. This script is the evidence
# for crossing it: what the hub actually does, and what each way of talking to
# it costs.
#
# It is a shell script rather than a crate because what it measures is the
# *build*: how many third-party crates a shape drags in, how much source that
# is, and — the question §XVI settles — what the resulting binary demands of a
# machine. A crate cannot measure its own dependency tree without having one.
#
# Needs a network and a writable scratch directory. Everything it builds is
# built OUTSIDE this repository, in a directory it names: measuring a candidate
# is not admitting one, and nothing here vendors anything into MCF's tree.
#
#   ./prototypes/transport-cost/measure.sh [scratch directory]
#
# Reports to doc/findings.md F9. Re-run it before citing those numbers on
# another machine, or after a year: crate trees grow.

set -eu

scratch="${1:-${TMPDIR:-/tmp}/mcf-transport-cost}"
repository="unsloth/Qwen3-0.6B-GGUF"
weights="Qwen3-0.6B-Q4_K_M.gguf"

say() { printf '%s\n' "$*"; }
rule() { printf '\n=== %s\n' "$*"; }

rm -rf "$scratch"
mkdir -p "$scratch"
say "measuring in $scratch"

# ---------------------------------------------------------------- the hub
# What the acquisition path is actually talking to. Every answer here is a
# requirement on whatever MCF writes, and each was a guess before it was run.
rule "what the hub does"
headers="$scratch/headers.txt"
if curl -sS --http1.1 -L -D "$headers" -o /dev/null -r 0-15 \
    "https://huggingface.co/$repository/resolve/main/$weights"; then
    say "HTTP/1.1: accepted (so nothing here needs HTTP/2)"
    grep -iE '^HTTP/|^location:|^accept-ranges:|^content-range:|^x-linked-etag:|^x-linked-size:|^x-repo-commit:' \
        "$headers" | sed 's/^/  /'
else
    say "the hub could not be reached; the rest of this measures the build only"
fi

# ------------------------------------------------------------- the shapes
# Each shape is a crate that does the smallest thing MCF needs, built in
# isolation. What is compared is the tree it drags in and what the binary it
# produces demands of a machine.
# measure <name> <one cargo-add argument list per line, on stdin>
measure() {
    name="$1"
    rule "$name"
    ( cd "$scratch" && cargo new --quiet --bin "$name" ) || return 0
    while IFS= read -r spec; do
        [ -n "$spec" ] || continue
        # shellcheck disable=SC2086 -- the spec is a deliberate word list
        ( cd "$scratch/$name" && cargo add --quiet $spec >/dev/null 2>&1 ) || {
            say "  could not add: $spec"
            return 0
        }
    done
    crates=$(grep -c '^name = ' "$scratch/$name/Cargo.lock" || echo 0)
    ( cd "$scratch/$name" && cargo vendor --quiet vendored >/dev/null 2>&1 ) || true
    tree="$scratch/$name/vendored"
    [ -d "$tree" ] || { say "  $crates crates (nothing vendored)"; return 0; }

    total=$(du -sb "$tree" | cut -f1)
    # A Linux build compiles none of the Windows import libraries, and they are
    # most of what a vendored tree weighs. Reporting the whole number would
    # overstate the cost by a factor of five, which is how a real objection
    # gets dismissed for the wrong reason.
    portable=$(du -sb "$tree"/* | grep -v '/windows' | awk '{s+=$1} END {print s+0}')
    source=$(find "$tree" -name '*.rs' -printf '%s\n' | awk '{s+=$1} END {print s+0}')
    say "  crates: $crates"
    say "  vendored tree: $(( total / 1048576 )) MiB, of which $(( portable / 1048576 )) MiB is not Windows"
    say "  rust source: $(( source / 1048576 )) MiB"
    say "  licences declared:"
    grep -h '^license' "$tree"/*/Cargo.toml 2>/dev/null | sed 's/license *= *//' \
        | sort | uniq -c | sort -rn | sed 's/^/    /'
}

measure "client-and-tls" <<'ADD'
ureq
ADD

# The same, without the client: what MCF would vendor if it wrote the HTTP
# itself. The difference between the two is what a client crate costs, and it
# is the smaller half of the question.
measure "tls-only" <<'ADD'
rustls --no-default-features --features ring,std,tls12
webpki-roots
ADD

# --------------------------------------------------------- what it demands
# §XVI and vendored.md §3a: the artifact may need libc, libgcc_s and the
# loader, and nothing else. This is the half of the question a licence check
# cannot answer, and the shape that fails it fails here rather than on a user's
# machine.
rule "what each artifact demands of a machine"
demand() {
    name="$1"
    binary="$scratch/$name/target/release/$name"
    [ -f "$binary" ] || return 0
    say "  $name:"
    readelf -d "$binary" | grep NEEDED | sed 's/.*\[/    /; s/\]//'
}

( cd "$scratch/client-and-tls" && cat > src/main.rs <<'RS'
fn main() {
    match ureq::get("https://huggingface.co/api/models/unsloth/Qwen3-0.6B-GGUF")
        .call()
        .and_then(|mut answer| answer.body_mut().read_to_string().map_err(Into::into))
    {
        Ok(text) => println!("the hub answered with {} bytes", text.len()),
        Err(error) => println!("the hub did not answer: {error}"),
    }
}
RS
cargo build --release --quiet >/dev/null 2>&1 ) || true
demand "client-and-tls"

# The control: a shape that uses the machine's own TLS. It is the cheapest to
# vendor and it is refused, and this is where that shows up rather than in an
# argument.
( cd "$scratch" && cargo new --quiet --bin system-tls >/dev/null 2>&1 ) || true
( cd "$scratch/system-tls" && cargo add --quiet native-tls >/dev/null 2>&1 && cat > src/main.rs <<'RS'
fn main() {
    let connector = native_tls::TlsConnector::new().expect("a connector");
    let socket = std::net::TcpStream::connect("huggingface.co:443").expect("a socket");
    let _session = connector.connect("huggingface.co", socket).expect("a session");
    println!("connected using the machine's own TLS");
}
RS
cargo build --release --quiet >/dev/null 2>&1 ) || true
demand "system-tls"

rule "done"
say "the numbers above belong in doc/findings.md F9 with the date they were taken"
