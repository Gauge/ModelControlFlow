"""Two ways to arrive at a given depth, so the cheap one can be checked.

To measure the cost of a token at depth D, the cache must hold D tokens. The
context ladder got there the obvious way: it generated D tokens and timed them.
That is quadratic in D — the tokens near the end are the slow ones, and they
are also the ones being waited for.

There is a second way. A prompt is *prefilled*, and prefill is batched: the
engine processes the prompt in parallel rather than one token at a time. So a
prompt of D tokens should reach the same depth far more cheaply, after which a
short generation measures the rate there directly.

Whether the two agree is an empirical question and the reason this file has two
modes. They are only interchangeable if what matters about the cache is how
much is in it and not how it got there.

  generate D          generate D tokens, timing each; report per-band rates
  prefill  D N        prompt of about D tokens, then N generated; report the
                      rate at the depth the engine says it reached
"""

import json
import sys
import time
import urllib.request


def _stream(port, body):
    """Sends one request; returns (first_token_seconds, gaps, final_json)."""
    request = urllib.request.Request(
        f"http://127.0.0.1:{port}/completion",
        data=body.encode(),
        headers={"Content-Type": "application/json"},
    )
    gaps, final = [], {}
    started = time.perf_counter()
    first = None
    with urllib.request.urlopen(request) as response:
        previous = started
        for raw in response:
            line = raw.decode("utf-8", "replace").strip()
            if not line.startswith("data: "):
                continue
            payload = json.loads(line[6:])
            now = time.perf_counter()
            if first is None:
                first = now - started
            else:
                gaps.append(now - previous)
            previous = now
            if payload.get("stop"):
                final = payload
    return first, gaps, final


def _body(prompt, tokens):
    return json.dumps(
        {
            "prompt": prompt,
            "n_predict": tokens,
            "stream": True,
            "cache_prompt": False,  # every request pays its own prefill
            "ignore_eos": True,  # the pin: the length is the length asked for
            "temperature": 0,
            "seed": 0,
        }
    )


def main():
    mode, port = sys.argv[1], int(sys.argv[2])
    model, allocation, repeat, out = sys.argv[-4:]

    if mode == "generate":
        depth = int(sys.argv[3])
        first, gaps, final = _stream(port, _body("A", depth))
        rows = []
        band, start = 128, 0
        while start < len(gaps):
            end = min(start + band, len(gaps))
            if end > start:
                total = sum(gaps[start:end])
                rows.append(("band", start + 1, end, int(total * 1e9)))
            start = end
            band = min(band * 2, 4096) if start >= band else band
        produced = final.get("tokens_predicted", len(gaps) + 1)
        sys.stderr.write(f"    produced {produced} of {depth}, first token {first * 1e3:.1f} ms\n")
        with open(out, "a") as handle:
            for kind, lo, hi, ns in rows:
                handle.write(
                    f"{model}\t{allocation}\t{repeat}\tgenerate\t{kind}\t{lo}\t{hi}\t{ns}\t0\n"
                )
        return

    if mode == "prefill":
        want, tokens = int(sys.argv[3]), int(sys.argv[4])
        # " the" is one token for every vocabulary MCF has looked at; the count
        # the engine reports is what is recorded, not this estimate.
        prompt = "A" + " the" * max(want - 1, 0)
        first, gaps, final = _stream(port, _body(prompt, tokens))
        seeded = final.get("tokens_evaluated", -1)
        if not gaps:
            sys.stderr.write(f"    prefill {want}: no tokens generated\n")
            return
        total = sum(gaps)
        per = total / len(gaps) * 1e3
        sys.stderr.write(
            f"    prefilled {seeded} (asked {want}), {len(gaps) + 1} generated, "
            f"{per:.3f} ms/token, prefill {first * 1e3:.0f} ms\n"
        )
        with open(out, "a") as handle:
            # from/to are the true depth the engine reported, so a prefilled
            # reading and a generated one are comparable on the same axis.
            handle.write(
                f"{model}\t{allocation}\t{repeat}\tprefill\tband\t{seeded}\t"
                f"{seeded + len(gaps)}\t{int(total * 1e9)}\t{int(first * 1e9)}\n"
            )
        return

    raise SystemExit(f"unknown mode {mode}")


if __name__ == "__main__":
    main()
