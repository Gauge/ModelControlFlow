# The companion site

Receives measurements people chose to publish, and shows them.

## What it is not

**It is not MCF, and it is deliberately a separate workspace.** MCF cannot
send — there is no destination in it, no address to configure, and a check that
holds that absence against the tree. A contribution arrives here because a
person moved a file, which is exactly the act MCF's own terms describe.

MCF also refuses to listen on a network. This does nothing else. Keeping the two
apart is what lets both be true at once, and it is why the site has its own
lockfile: whatever it grows to need must never become a condition of a
measurement somebody takes at home.

The one edge back into MCF is `mcf-core`, which has no dependencies of its own.
It is there so the contribution format has a single definition and the producer
and the receiver cannot drift apart.

## Running it

    cargo run --release          # 127.0.0.1:8080, archive in ./contributions

    MCF_SITE_ADDRESS   default 127.0.0.1 — anything else is reachable from the network
    MCF_SITE_PORT      default 8080
    MCF_SITE_ARCHIVE   default ./contributions

In a container:

    podman build -f site/Containerfile -t mcf-site .
    podman run --rm -p 127.0.0.1:8080:8080 -v mcf-contributions:/contributions mcf-site

## Publishing to it

    mcf share --into contribution.txt

MCF prints every row before writing the file, because a person deciding about
something irreversible is entitled to read what it says. Sending it is a
separate act:

    curl -X POST --data-binary @contribution.txt http://localhost:8080/contribute

## What it holds

Contributions are kept **verbatim**. A reader is shown what was submitted rather
than what the site understood, and the site does not reimplement MCF's format —
it reads only enough to list a submission.

There is **no delete**. MCF's terms say publication cannot be undone, and this
is where that becomes true; offering a withdrawal would be offering a retraction
the terms already said does not exist.

## What is not built yet

Accounts, rate limiting and TLS. An open endpoint that stores what strangers
send is a thing that needs all three before it faces the network, and the
default address is loopback until they exist.
