"""The fall-off predicted from the file and the machine, before generating.

Every generated token makes attention re-read the whole KV cache. How big that
cache is per token of depth is fixed by the architecture and written in the
GGUF header; how fast it can be read is a property of this machine, measured
by bandwidth.py. Divide one by the other and the slope of the fall-off follows
without generating anything:

    ms per token at depth d  =  a  +  d x (KV bytes per token) / bandwidth
                                ^                ^                   ^
                                |                |                   |
                       must be measured    read from the file    measured once
                       (one short run)         for free           per machine

That is the whole argument for a fast diagnostic: of the two numbers that
describe a model's speed, only ONE of them needs the model to be run, and it
is the one that a short shallow generation gives.

The argument has a precondition, and the precondition is checkable in advance:
it holds only where the cache is big enough to be coming from DRAM. While the
cache still fits in L3 the read is not bandwidth-bound and the slope is set by
something else. This machine's L3 is 32 MiB per core complex and 64 MiB across
both, measured as a 36% and a 53% step in bandwidth.py — so a model whose
cache is small stays in cache to a great depth, and its slope must be measured
rather than predicted. Which regime a model is in is arithmetic on its header.
"""

import struct
import sys
import glob
import os

L3_PER_CCD = 32 * 1024 * 1024
L3_TOTAL = 64 * 1024 * 1024


def gguf_metadata(path, want):
    """Reads the header key/values. Stops at the tensor data — the point is to
    do this for nothing, so nothing large is read."""
    with open(path, "rb") as handle:
        if handle.read(4) != b"GGUF":
            return {}
        struct.unpack("<I", handle.read(4))
        struct.unpack("<Q", handle.read(8))
        n_kv, = struct.unpack("<Q", handle.read(8))

        def text():
            length, = struct.unpack("<Q", handle.read(8))
            return handle.read(length).decode("utf-8", "replace")

        def value(kind):
            simple = {0: "<B", 1: "<b", 2: "<H", 3: "<h", 4: "<I", 5: "<i",
                      6: "<f", 7: "<?", 10: "<Q", 11: "<q", 12: "<d"}
            if kind in simple:
                fmt = simple[kind]
                return struct.unpack(fmt, handle.read(struct.calcsize(fmt)))[0]
            if kind == 8:
                return text()
            if kind == 9:
                inner, = struct.unpack("<I", handle.read(4))
                count, = struct.unpack("<Q", handle.read(8))
                return [value(inner) for _ in range(count)]
            raise ValueError(kind)

        out = {}
        for _ in range(n_kv):
            key = text()
            kind, = struct.unpack("<I", handle.read(4))
            got = value(kind)
            if any(w in key for w in want):
                out[key] = got
        return out


def geometry(path):
    """Layers, KV heads and head dimension, whatever the architecture calls
    them. Returns None where the header does not say (A7: unknown is unknown,
    not a guess)."""
    kv = gguf_metadata(path, [".block_count", ".attention.head_count",
                              ".attention.key_length", ".attention.value_length",
                              ".embedding_length", ".context_length"])
    def pick(suffix):
        v = next((v for k, v in kv.items() if k.endswith(suffix)), None)
        # Some headers store a per-layer array rather than one number. Where
        # the layers differ, the cache is the sum, so the mean stands in for
        # the per-layer figure and the caller multiplies by the layer count.
        if isinstance(v, list):
            return (sum(v) / len(v)) if v else None
        return v

    layers = pick(".block_count")
    heads = pick(".attention.head_count")
    kv_heads = pick(".attention.head_count_kv")
    key_len = pick(".attention.key_length")
    val_len = pick(".attention.value_length")
    width = pick(".embedding_length")
    trained = pick(".context_length")
    if kv_heads is None:
        kv_heads = heads
    if key_len is None and width and heads:
        key_len = width // heads          # the usual convention where unstated
    if val_len is None:
        val_len = key_len
    if not all((layers, kv_heads, key_len, val_len)):
        return None
    per_layer_read = kv_heads * (key_len + val_len) * 2
    return {
        "layers": layers, "kv_heads": kv_heads,
        # How much of ONE layer's cache is read contiguously per token. A
        # bandwidth argument needs this to be big enough to stream; where it
        # is small the read is bound by latency instead and the prediction
        # under-states the slope. Qwen3 reads 4 KiB a layer, SmolLM2 768 B.
        "per_layer_read": per_layer_read,
        "key_len": key_len, "val_len": val_len,
        "trained": trained,
        # K and V, two bytes each: llama.cpp's default cache is f16. A run
        # using --cache-type-k/-v changes this, which is why it is a stated
        # condition and not a constant.
        "bytes_per_depth_token": int(layers * per_layer_read),
    }


def predicted_slope_ms(bytes_per_depth_token, dram_gbs):
    """Milliseconds per token, per token of depth."""
    return bytes_per_depth_token / (dram_gbs * 1e9) * 1e3


def main():
    dram = float(sys.argv[1]) if len(sys.argv) > 1 else 55.8
    print(f"Fall-off predicted from the header, at {dram:.1f} GB/s measured DRAM")
    print("=" * 94)
    print("\n  'stays in L3 to' is the depth where the cache outgrows one core")
    print("  complex's 32 MiB. Below it the slope is NOT bandwidth-bound and")
    print("  this prediction does not apply.\n")
    print(f"  {'model':<40}{'KV B/tok':>10}{'in L3 to':>10}{'slope':>12}{'per-layer':>11}")
    print(f"  {'':<40}{'':>10}{'depth':>10}{'ms/tok/tok':>12}{'read':>11}")
    found = []
    for path in sorted(glob.glob(os.path.expanduser("~/.local/share/mcf/**/*.gguf"), recursive=True)):
        name = os.path.basename(path)
        # ggml-vocab-*.gguf are tokenizer fixtures shipped with llama.cpp, not
        # models: they carry a vocabulary and no weights, and their attention
        # fields describe nothing that can be run.
        if name.startswith("ggml-vocab-"):
            continue
        geo = geometry(path)
        if geo is None:
            print(f"  {name[:39]:<40}{'header does not say':>42}")
            continue
        per = geo["bytes_per_depth_token"]
        slope = predicted_slope_ms(per, dram)
        found.append((name, geo, slope))
        note = "" if geo["per_layer_read"] >= 2048 else "  latency-bound"
        print(f"  {name[:39]:<40}{per / 1024:>9.0f}K{L3_PER_CCD // per:>10,}"
              f"{slope:>12.6f}{geo['per_layer_read']:>9.0f}B{note}")
    print(f"\n  {len(found)} model(s) with usable geometry")
    print("\n  A prediction from bandwidth needs the per-layer read to be")
    print("  large enough to stream. Marked rows read under 2 KiB a layer and")
    print("  are bound by latency instead, where this under-states the slope.")

    print("\n  Against what was measured:")
    print(f"    {'model':<34}{'predicted':>11}{'measured':>11}{'ratio':>8}")
    for name, want, got in MEASURED:
        row = next((f for f in found if f[0] == name), None)
        if row is None:
            continue
        print(f"    {name[:33]:<34}{row[2]:>11.6f}{got:>11.6f}{row[2] / got:>8.2f}")
    print("\n  A ratio near one means the fall-off IS the memory bus and can be")
    print("  had from the header. The shortfall is the fraction of peak the")
    print("  engine actually achieves, which is one number per machine.")
    return found


# Slopes fitted from readings already taken, in ms per token per token of
# depth: context-ladder warm bands at allocations past the plateau.
MEASURED = [
    ("Qwen3-0.6B-Q8_0.gguf", None, 0.002398),
    ("SmolLM2-135M-Instruct-Q4_0.gguf", None, 0.001122),
]


if __name__ == "__main__":
    main()
