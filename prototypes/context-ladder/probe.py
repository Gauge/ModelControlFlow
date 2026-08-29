"""One streaming generation, with every token's arrival timed (B-398, B-399, F118).

**Why one generation rather than a ladder of them.** The rungs of a performance
picture — the rate between 128 and 256 tokens, between 256 and 512, and so on —
can be built two ways. Running a separate generation to each depth costs about
twice the tokens and forces the rate at a depth to be *differenced* out of two
independently noisy runs. Streaming one generation to the deepest rung and
timing each token as it arrives gives every band from one continuous run: the
same clock, the same cache, the same process, so a band's rate carries only the
noise inside a single run.

It also gives the one figure a ladder of whole-run timings cannot: the time to
the **first** token, which is the fixed cost of a request with everything after
it removed.

What is written is one row per band, plus the first-token row, so the analysis
never sees a raw timestamp and cannot accidentally publish one.
"""
import http.client
import json
import sys
import time


def generate(port: int, tokens: int, prompt: str = "A") -> tuple[float, list[float]]:
    """Streams a generation, returning the first token's delay and every gap after it."""
    body = json.dumps({
        "prompt": prompt,
        "n_predict": tokens,
        "stream": True,
        # A fresh key/value state per request: what stays warm is the model in
        # memory, which is what "warm" means here. Reusing the cache would make
        # the second request measure the first one's leftovers.
        "cache_prompt": False,
        # The pin (F117): without it a model that emits its end-of-turn token
        # ends the run early and the depth axis becomes fiction.
        "ignore_eos": True,
        "temperature": 0,
        "seed": 0,
    })
    connection = http.client.HTTPConnection("127.0.0.1", port, timeout=3600)
    started = time.monotonic()
    connection.request("POST", "/completion", body=body,
                       headers={"Content-Type": "application/json"})
    response = connection.getresponse()
    first: float | None = None
    gaps: list[float] = []
    previous = started
    while True:
        line = response.readline()
        if not line:
            break
        text = line.decode("utf-8", "replace").strip()
        if not text.startswith("data:"):
            continue
        now = time.monotonic()
        try:
            piece = json.loads(text[5:].strip())
        except json.JSONDecodeError:
            continue
        if piece.get("stop"):
            break
        if first is None:
            first = now - started
        else:
            gaps.append(now - previous)
        previous = now
    connection.close()
    return (first if first is not None else 0.0), gaps


def main(argv: list[str]) -> int:
    if len(argv) < 6:
        print("usage: probe.py <port> <tokens> <model> <allocation> <repeat> [out]",
              file=sys.stderr)
        return 2
    port, tokens = int(argv[1]), int(argv[2])
    model, allocation, repeat = argv[3], int(argv[4]), int(argv[5])
    out = argv[6] if len(argv) > 6 else "readings.tsv"

    first, gaps = generate(port, tokens)
    produced = len(gaps) + (1 if first else 0)

    with open(out, "a") as handle:
        # The fixed cost, as its own row: everything that happens once.
        handle.write(f"{model}\t{allocation}\t{repeat}\tfirst_token\t0\t1\t"
                     f"{int(first * 1e9)}\n")
        # Then one row per band, in powers of two, each carrying the tokens it
        # covers and the time they took.
        band, index = 128, 0
        while index < len(gaps):
            upper = min(band, len(gaps))
            span = gaps[index:upper]
            if span:
                handle.write(f"{model}\t{allocation}\t{repeat}\tband\t{index + 1}\t"
                             f"{upper}\t{int(sum(span) * 1e9)}\n")
            index = upper
            band *= 2
            if band > len(gaps) and index < len(gaps):
                band = len(gaps)
    print(f"    produced {produced} of {tokens}, first token {first * 1000:.1f} ms")
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv))
