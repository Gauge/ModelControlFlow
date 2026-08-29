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

# How many layers actually attend to the WHOLE context, where an architecture
# interleaves sliding-window layers with full ones. A sliding-window layer's
# read stops growing once the window is full, so it contributes nothing to the
# slope past that depth -- only the full-attention layers do.
#
# The interleave is not in the header. It is a property of the architecture
# that the engine hard-codes, so it is named here per architecture rather than
# guessed, and an architecture with a sliding window but no entry is reported
# as such instead of being predicted wrongly (A7).
#
# gemma3: one full-attention layer in every six. Found because gemma-3-270m
# was the only model in the corpus whose measured slope missed the prediction
# by more than 3x -- and it missed it by close to 6x, in the direction of
# being FLATTER than predicted.
FULL_ATTENTION_EVERY = {"gemma3": 6}

# Architectures whose cost per token does NOT grow with depth, because they
# keep no per-token cache to re-read. A recurrent or state-space model carries
# a fixed-size state, so its fall-off is flat and the bandwidth argument does
# not apply — predicting a slope for one would be confidently wrong rather
# than merely imprecise.
NO_GROWING_CACHE = {"mamba", "mamba2", "rwkv", "rwkv6", "rwkv7", "falcon_mamba", "jamba"}

# Architectures that keep a cache, but not one this arithmetic describes.
# Multi-head latent attention stores a compressed latent per token instead of
# K and V per head, so layers x kv_heads x (key + value) is the wrong product.
CACHE_NOT_DESCRIBED = {
    "deepseek2": "multi-head latent attention: the cache holds a compressed "
                 "latent, not K and V per head",
}

# The default cache element in llama.cpp is f16. A run using --cache-type-k or
# --cache-type-v changes it, which is why this is a stated condition of a
# prediction rather than a constant buried in a product (A6).
DEFAULT_CACHE_BYTES = 2


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


WANTED = [".block_count", ".attention.head_count", ".attention.key_length",
          ".attention.value_length", ".embedding_length", ".context_length",
          ".attention.sliding_window", ".expert_count", ".expert_used_count"]


def describe(kv, cache_bytes=DEFAULT_CACHE_BYTES):
    """The arithmetic, on a header already read.

    Separated from the file so it can be exercised against headers that do not
    exist on this machine — which is the only way to find out what the
    predictor does with an architecture nobody here has (D26).

    Always returns a dict. `verdict` is one of:

      described          the slope follows from the geometry
      no-growing-cache   there is no cache to re-read; the slope is ~0
      not-described      a cache this arithmetic does not model
      header-incomplete  the header does not say enough (A7)
    """
    architecture = next((k.split(".")[0] for k in kv), None)
    if architecture in NO_GROWING_CACHE:
        return {"verdict": "no-growing-cache", "architecture": architecture,
                "why": "carries a fixed-size state, not a per-token cache",
                "growing_bytes_per_depth_token": 0}
    if architecture in CACHE_NOT_DESCRIBED:
        return {"verdict": "not-described", "architecture": architecture,
                "why": CACHE_NOT_DESCRIBED[architecture],
                "growing_bytes_per_depth_token": None}

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
        missing = [n for n, v in (("layers", layers), ("kv heads", kv_heads),
                                  ("key length", key_len), ("value length", val_len))
                   if not v]
        return {"verdict": "header-incomplete", "architecture": architecture,
                "why": "the header does not state " + ", ".join(missing),
                "growing_bytes_per_depth_token": None}
    # A mixture of experts holds many feed-forward blocks and reads only some
    # of them per token, so the file size overstates what a token actually
    # moves. TinyMixtral uses 2 of 4 and its intercept was over-predicted by
    # 1.79x -- close to the 2x the counts imply.
    #
    # Scaling the WHOLE file by used/total assumes the experts dominate it.
    # That holds for the mixtures here and is stated rather than hidden: a
    # dense model has no expert count and is scaled by 1.
    experts = pick(".expert_count")
    experts_used = pick(".expert_used_count")
    active_fraction = 1.0
    if experts and experts_used and experts > 0:
        active_fraction = float(experts_used) / float(experts)

    window = pick(".attention.sliding_window")
    architecture = next((k.split(".")[0] for k in kv), None)
    # Layers whose cost keeps growing with depth. Without a sliding window
    # that is all of them.
    growing = layers
    window_note = None
    if window:
        every = FULL_ATTENTION_EVERY.get(architecture)
        if every:
            growing = max(layers // every, 1)
            window_note = f"window {int(window)}, 1 full layer in {every}"
        else:
            window_note = f"window {int(window)}, interleave unknown"
            growing = None            # not predictable; say so rather than guess

    per_layer_read = kv_heads * (key_len + val_len) * cache_bytes
    return {
        "verdict": "described" if growing is not None else "not-described",
        "architecture": architecture,
        "why": window_note if growing is None else None,
        "cache_bytes": cache_bytes,
        "experts": experts, "experts_used": experts_used,
        "active_weight_fraction": active_fraction,
        "layers": layers, "kv_heads": kv_heads,
        "growing_layers": growing, "window_note": window_note,
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
        # what the WHOLE cache costs to hold, all layers
        "bytes_per_depth_token": int(layers * per_layer_read),
        "trained": trained,
        # what actually grows with depth, and so sets the slope
        "growing_bytes_per_depth_token": (
            int(growing * per_layer_read) if growing is not None else None
        ),
    }


def geometry(path, cache_bytes=DEFAULT_CACHE_BYTES):
    """describe(), for a file on this machine. Always returns a dict; read
    `verdict` before reading anything else."""
    return describe(gguf_metadata(path, WANTED), cache_bytes)


def predicted_slope_ms(bytes_per_depth_token, dram_gbs):
    """Milliseconds per token, per token of depth."""
    return bytes_per_depth_token / (dram_gbs * 1e9) * 1e3


def predicted_intercept_ms(file_bytes, active_fraction, dram_gbs):
    """The depth-independent part: the weights, read once per generated token.

    Counted rather than assumed. At depth 4096, 128 extra tokens of Qwen3-0.6B
    moved 153.9 GB measured by L3 miss; the KV cache accounts for 60.1 GB and
    the weights re-read once per token for 81.8 GB, which together come to
    142.0 GB -- a ratio of 1.08. A token reads the whole model AND the whole
    cache, so both halves of the curve are in the file."""
    return file_bytes * active_fraction / (dram_gbs * 1e9) * 1e3


def curve(file_bytes, geo, dram_gbs, achieved=1.0):
    """The whole curve from the file: ms/token as a function of depth.

    `achieved` is the fraction of peak bandwidth the engine reaches -- one
    number for this machine, measured, not shipped as a constant."""
    if geo.get("verdict") != "described":
        return None
    a = predicted_intercept_ms(file_bytes, geo["active_weight_fraction"], dram_gbs * achieved)
    b = predicted_slope_ms(geo["growing_bytes_per_depth_token"], dram_gbs * achieved)
    return lambda depth: a + b * depth


def main():
    dram = float(sys.argv[1]) if len(sys.argv) > 1 else 55.8
    print(f"Fall-off predicted from the header, at {dram:.1f} GB/s measured DRAM")
    print("=" * 94)
    print("\n  'stays in L3 to' is the depth where the cache outgrows one core")
    print("  complex's 32 MiB. Below it the slope is NOT bandwidth-bound and")
    print("  this prediction does not apply.\n")
    print(f"  {'model':<40}{'KV B/tok':>10}{'in L3 to':>10}{'slope':>12}{'per-layer':>11}")
    print(f"  {'':<40}{'':>10}{'depth':>10}{'ms/tok/tok':>12}{'read':>11}")
    found, declined = [], []
    for path in sorted(glob.glob(os.path.expanduser("~/.local/share/mcf/**/*.gguf"), recursive=True)):
        name = os.path.basename(path)
        # ggml-vocab-*.gguf are tokenizer fixtures shipped with llama.cpp, not
        # models: they carry a vocabulary and no weights, and their attention
        # fields describe nothing that can be run.
        if name.startswith("ggml-vocab-"):
            continue
        geo = geometry(path)
        if geo["verdict"] != "described":
            print(f"  {name[:39]:<40}{geo['verdict']:>20}  {geo.get('why') or ''}"[:110])
            declined.append((name, geo["verdict"]))
            continue
        per = geo["bytes_per_depth_token"]
        slope = predicted_slope_ms(geo["growing_bytes_per_depth_token"], dram)
        found.append((name, geo, slope))
        note = "" if geo["per_layer_read"] >= 2048 else "  latency-bound"
        if geo["window_note"]:
            note += f"  [{geo['window_note']}]"
        print(f"  {name[:39]:<40}{per / 1024:>9.0f}K{L3_PER_CCD // per:>10,}"
              f"{slope:>12.6f}{geo['per_layer_read']:>9.0f}B{note}")
    print(f"\n  {len(found)} described, {len(declined)} declined with a reason")
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
