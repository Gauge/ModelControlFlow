"""What the predictor does with architectures this machine does not have.

The corpus here is four architectures, and every model the slope arithmetic
predicted well was a Qwen3. A tool that measures one family and reports a
number for anything is not a measuring tool, so the question is not whether
the arithmetic is right about Qwen3 — it is what it does when handed something
else.

That cannot be answered by measuring, because the models are not here. It can
be answered by construction: headers are just key/value pairs, so a header for
an architecture nobody here has can be written down and put through the same
function the real ones go through. This is a laboratory in the sense D26 means
— what is simulated is the world being observed, never MCF's own response to
it. No timing is produced or consumed (A11).

The property under test is NOT that the predictor is right about everything.
It is that **the predictor is never confidently wrong**: every header either
gets a prediction the arithmetic actually describes, or a refusal that names
what is missing.
"""

import sys
import pathlib

sys.path.insert(0, str(pathlib.Path(__file__).parent))
import physics  # noqa: E402

# Each case: a constructed header, the verdict it must get, and why it exists.
CASES = [
    (
        "qwen3, ordinary multi-head with GQA",
        {"qwen3.block_count": 28, "qwen3.attention.head_count": 16,
         "qwen3.attention.head_count_kv": 8, "qwen3.attention.key_length": 128,
         "qwen3.attention.value_length": 128, "qwen3.context_length": 40960},
        "described",
        "the case the arithmetic was written for; it must still work",
    ),
    (
        "multi-query attention, one KV head",
        {"falcon.block_count": 32, "falcon.attention.head_count": 71,
         "falcon.attention.head_count_kv": 1, "falcon.attention.key_length": 64,
         "falcon.attention.value_length": 64},
        "described",
        "MQA shares one KV head across many query heads. The product handles "
        "it, but nothing here has ever tested that it does",
    ),
    (
        "head length implied by width, not stated",
        {"llama.block_count": 30, "llama.attention.head_count": 9,
         "llama.attention.head_count_kv": 3, "llama.embedding_length": 576},
        "described",
        "most llama headers omit key_length; it is width / head count",
    ),
    (
        "mamba: a state-space model",
        {"mamba.block_count": 24, "mamba.embedding_length": 768},
        "no-growing-cache",
        "carries a fixed-size state. Its fall-off is FLAT, and predicting a "
        "slope from a cache that does not exist is the worst failure available",
    ),
    (
        "rwkv: recurrent",
        {"rwkv6.block_count": 24, "rwkv6.embedding_length": 1024},
        "no-growing-cache",
        "same shape of mistake as mamba, different family",
    ),
    (
        "deepseek2: multi-head latent attention",
        {"deepseek2.block_count": 30, "deepseek2.attention.head_count": 16,
         "deepseek2.attention.head_count_kv": 16,
         "deepseek2.attention.key_length": 192,
         "deepseek2.attention.value_length": 128},
        "not-described",
        "stores a compressed latent per token, so layers x heads x (k+v) is "
        "the wrong product. It would compute a plausible, wrong number",
    ),
    (
        "gemma3: sliding window, interleave known",
        {"gemma3.block_count": 18, "gemma3.attention.head_count": 4,
         "gemma3.attention.head_count_kv": 1, "gemma3.attention.key_length": 256,
         "gemma3.attention.value_length": 256,
         "gemma3.attention.sliding_window": 512},
        "described",
        "measured: only one layer in six grows with depth",
    ),
    (
        "an unknown architecture with a sliding window",
        {"newarch.block_count": 40, "newarch.attention.head_count": 32,
         "newarch.attention.head_count_kv": 8, "newarch.attention.key_length": 128,
         "newarch.attention.value_length": 128,
         "newarch.attention.sliding_window": 4096},
        "not-described",
        "the window is in the header but the interleave is hard-coded in the "
        "engine. Guessing 'all layers' overstates the slope several times over",
    ),
    (
        "a header missing its attention fields",
        {"llama.block_count": 30},
        "header-incomplete",
        "must name what is missing rather than substitute a default",
    ),
    (
        "an empty header",
        {},
        "header-incomplete",
        "the degenerate case",
    ),
]


def main():
    print("What the predictor does with architectures that are not here")
    print("=" * 74)
    print("\n  The property: never confidently wrong. Either a prediction the")
    print("  arithmetic describes, or a refusal naming what is missing.\n")
    failures = 0
    for name, header, expected, why in CASES:
        got = physics.describe(header)
        verdict = got.get("verdict")
        ok = verdict == expected
        failures += not ok
        mark = "ok  " if ok else "FAIL"
        print(f"  [{mark}] {name}")
        print(f"         expected {expected}, got {verdict}")
        if verdict == "described":
            per = got["growing_bytes_per_depth_token"]
            print(f"         {per / 1024:.1f} KiB per token of depth, "
                  f"{got['growing_layers']} of {got['layers']} layers grow")
        elif got.get("why"):
            print(f"         says: {got['why']}")
        print(f"         why this case: {why}")
        print()

    print("=" * 74)
    if failures:
        print(f"  {failures} of {len(CASES)} cases wrong.")
        return 1
    print(f"  {len(CASES)} cases, none confidently wrong.")
    print("\n  What this does NOT show: that the predicted slopes are accurate")
    print("  for the families not measured here. It shows only that a family")
    print("  the arithmetic cannot describe is refused instead of guessed at.")
    print("  Accuracy for MQA and for implied head length is still unmeasured,")
    print("  and needs one model of each on this machine.")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
