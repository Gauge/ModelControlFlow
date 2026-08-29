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
import http.client
import os
import pathlib
import sys
import time
import urllib.request

sys.path.insert(0, str(pathlib.Path(__file__).parent))
from sampler import Sampler  # noqa: E402

# The engine's pid, so the conditions recorded are the conditions the engine
# was under and not this script's. run.sh knows it; nothing else does.
SERVER_PID = int(os.environ.get("MCF_FALLOFF_SERVER_PID", "0")) or None

# Columns appended to every reading. Named here so the header and the rows
# cannot drift apart.
CONDITIONS = [
    "freq_max_mhz_min", "freq_max_mhz_max", "temp_Tccd1_first", "temp_Tccd1_last",
    "proc_VmRSS_last", "proc_read_bytes_delta", "proc_majflt_delta", "n_samples",
]

# One more column after the conditions: why a reading was refused, empty where
# it was not.
TRAILING = ["refused_because"]


def conditions(summary):
    return "\t".join(str(summary.get(k, "unknown")) for k in CONDITIONS)


class Refused(Exception):
    """The engine declined this request. What it said is the reading (A2, A9):
    a depth a model cannot reach is a fact about the model, and recording it as
    a gap in the table loses it."""

    def __init__(self, why):
        super().__init__(why)
        self.why = why


def _stream(port, body):
    """Sends one request; returns (first_token_seconds, gaps, final_json, conditions).

    The sampler runs for exactly the span of the request, so what it reports is
    what held while this reading was taken rather than a session average."""
    request = urllib.request.Request(
        f"http://127.0.0.1:{port}/completion",
        data=body.encode(),
        headers={"Content-Type": "application/json"},
    )
    gaps, final = [], {}
    watcher = Sampler(pid=SERVER_PID, interval=0.25)
    watcher.start()
    started = time.perf_counter()
    first = None
    try:
        response = urllib.request.urlopen(request)
    except urllib.error.HTTPError as error:
        watcher.stop()
        detail = error.read().decode("utf-8", "replace")[:200].replace("\n", " ")
        raise Refused(f"HTTP {error.code}: {detail}") from error
    except (urllib.error.URLError, http.client.HTTPException, OSError) as error:
        watcher.stop()
        raise Refused(f"{type(error).__name__}: {error}") from error
    with response:
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
    return first, gaps, final, watcher.stop() and watcher.summary()


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


def record_refusal(out, model, allocation, repeat, mode, depth, why):
    """A refusal is a row, not an absence. Five models produced nothing at all
    in the first corpus sweep because their trained context was smaller than
    the depth asked for, and the run swallowed it — which is the shape of
    defect that makes a tool look as though it works everywhere."""
    sys.stderr.write(f"    REFUSED at {depth}: {why}\n")
    with open(out, "a") as handle:
        handle.write(
            f"{model}\t{allocation}\t{repeat}\t{mode}\trefused\t{depth}\t{depth}\t0\t0"
            f"\t{conditions({})}\t{why}\n"
        )


def main():
    mode, port = sys.argv[1], int(sys.argv[2])
    model, allocation, repeat, out = sys.argv[-4:]

    if mode == "generate":
        depth = int(sys.argv[3])
        try:
            first, gaps, final, held = _stream(port, _body("A", depth))
        except Refused as refusal:
            record_refusal(out, model, allocation, repeat, "generate", depth, refusal.why)
            return
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
                    f"{model}\t{allocation}\t{repeat}\tgenerate\t{kind}\t{lo}\t{hi}\t{ns}\t0"
                    f"\t{conditions(held)}\t\n"
                )
        return

    if mode == "prefill":
        want, tokens = int(sys.argv[3]), int(sys.argv[4])
        _ = tokens
        # " the" is one token for every vocabulary MCF has looked at; the count
        # the engine reports is what is recorded, not this estimate.
        prompt = "A" + " the" * max(want - 1, 0)
        try:
            first, gaps, final, held = _stream(port, _body(prompt, tokens))
        except Refused as refusal:
            record_refusal(out, model, allocation, repeat, "prefill", want, refusal.why)
            return
        seeded = final.get("tokens_evaluated", -1)
        if not gaps:
            record_refusal(out, model, allocation, repeat, "prefill", want,
                           "the engine returned no tokens")
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
                f"{seeded + len(gaps)}\t{int(total * 1e9)}\t{int(first * 1e9)}"
                f"\t{conditions(held)}\t\n"
            )
        return

    raise SystemExit(f"unknown mode {mode}")


if __name__ == "__main__":
    main()
