import pathlib
from screens import Screen, check, L, R, W

# ─────────────────────────── MONITOR ───────────────────────────
def monitor(diag=False):
    s = Screen("MCF", "Monitor")
    s.rows.pop(1); s.rows.insert(1, s.row(_menu(diag), emit=False))
    s.blank()
    s.row([("  ", None), (L("PROCESSOR", 17), "a"), (R("LOAD", 8), "l"), (R("TEMP", 9), "l"),
           (R("POWER", 8), "l"), (R("CLOCK", 11), "l"), (R("CORES", 9), "l")])
    cpu = ("96 %", "71 °C", "5.12 GHz") if diag else ("38 %", "47 °C", "5.51 GHz")
    s.row([("  ", None), (L("CPU", 17), "v"), (R(cpu[0], 8), "v"),
           (R(cpu[1], 9), "w" if diag else "v"), (R("—", 8), "d"), (R(cpu[2], 11), "v"),
           (R("32", 9), "v")])
    g = ("2 %", "31 °C", "19 W") if diag else ("1 %", "29 °C", "18 W")
    s.row([("  ", None), (L("GPU  RTX 5080", 17), "v"), (R(g[0], 8), "v"), (R(g[1], 9), "v"),
           (R(g[2], 8), "v"), (R("2.61 GHz", 11), "v"), (R("10 752", 9), "v")])
    s.blank()
    s.row([("  ", None), (L("MEMORY", 17), "a"), (R("USED", 10), "l"), (R("TOTAL", 10), "l"),
           (R("FREE", 10), "l"), ("   ", None), (L("LARGEST CONTEXT", 16), "l")])
    used, free = ("28.4 GB", "63.5 GB") if diag else ("17.2 GB", "74.7 GB")
    s.row([("  ", None), (L("System", 17), "v"), (R(used, 10), "v"), (R("91.9 GB", 10), "v"),
           (R(free, 10), "v"), ("   ", None), (L("32 768 tokens", 16), "v")])
    s.row([("  ", None), (L("Graphics", 17), "v"), (R("1.9 GB", 10), "v"), (R("16.3 GB", 10), "v"),
           (R("14.4 GB", 10), "v"), ("   ", None), (L("32 768 tokens", 16), "v")])
    s.blank()
    s.row([("  ", None), (L("STORAGE", 17), "a"), (R("READ", 10), "l"), (R("WRITE", 10), "l"),
           (R("TEMP", 10), "l")])
    s.row([("  ", None), (L("nvme0", 17), "v"), (R("0.0 MB/s" if diag else "12.4 MB/s", 10), "v"),
           (R("0.0 MB/s", 10), "v"), (R("34 °C", 10), "v")])
    s.rule()
    if not diag:
        s.row([("  ", None), ("SERVING", "a"), ("  ", None), ("example-8b-q4", "v")])
        s.blank()
        s.row([("  ", None), (L("ENGINE", 20), "l"), (L("DEVICE", 14), "l"), (R("CONTEXT", 9), "l"),
               (R("SPEED", 13), "l"), (R("UPTIME", 11), "l")])
        s.row([("  ", None), (L("llama.cpp · CUDA", 20), "v"), (L("RTX 5080", 14), "v"),
               (R("32 768", 9), "v"), (R("147 tok/s", 13), "v"), (R("04:12:37", 11), "g")])
        s.blank()
        s.row([("  ", None), (L("served", 10), "l"), (L("1 284 requests", 20), "v"),
               (L("tokens out", 12), "l"), (L("184 902", 12), "v"), (L("last 1.4 s ago", 15), "d")])
    else:
        s.row([("  ", None), ("DIAGNOSING", "a"), ("  ", None), ("example-8b-q4", "v")])
        s.blank()
        s.row([("  ", None), (L("TEST", 31), "l"), (L("DEVICE", 9), "l"), (R("DEPTH", 8), "l"),
               (R("DONE", 8), "l"), (R("STEP", 11), "l")])
        s.row([("  ", None), (L("Generation speed vs depth", 31), "v"), (L("CPU", 9), "v"),
               (R("4 096", 8), "v"), (R("64 %", 8), "v"), (R("2 of 5", 11), "v")])
        s.blank()
        s.row([("  ", None), (L("elapsed", 10), "l"), (L("02:41", 12), "v"),
               (L("finishes in", 13), "l"), ("1 min 10 s – 2 min 50 s", "a")])
        s.row([("  ", None), (L("done", 10), "l"), ("✓", "g"),
               (" GPU · generation speed — 147 tok/s at depth 512", "d")])
    return s.finish()

def _menu(diag):
    items = ["Monitor", "Host", "Diagnostics", "Models", "Settings", "Exit"]
    segs = [(" ", None)]
    for n in items:
        segs.append((f" {n} ", "sel" if n == "Monitor" else "l")); segs.append((" ", None))
    dot, word = ("◐", "Diagnosing") if diag else ("●", "Serving")
    tail = [(dot, "w" if diag else "g"), (" ", None), (word, "v"), (" ", None)]
    used = sum(len(t) for t, _ in segs) + sum(len(t) for t, _ in tail)
    return segs + [(" " * (W - used), None)] + tail

# ─────────────────────────── HOST ───────────────────────────
def host():
    LWi, RWi = 20, 53                      # inner widths of the two boxes
    left, right = [], []
    def lbox_top(t): left.append(("box", "┌─ ", t, LWi))
    def rbox_top(t): right.append(("box", "┌─ ", t, RWi))
    models = [("example-3b-q4", "1.8 GB", 0), ("example-2b-it", "1.7 GB", 0),
              ("example-8b-q4", "5.0 GB", 1), ("example-1b-q4", "0.8 GB", 0),
              ("example-135m", "0.1 GB", 0)]
    lbox_top("MODELS")
    for n, sz, here in models:
        left.append(("sel" if here else "row",
                     [("›" if here else " ", None), (L(n, 13), "v"), (R(sz, 6), "d")], LWi))
    left.append(("row", [(" ", None), ("▾ 29 more", "d")], LWi))
    left.append(("bot", LWi))
    lbox_top("ACTIONS")
    for lab, cls in [(" Host this model ", "btnp"), (" Run diagnostics ", "btn"), (" Back ", "btn")]:
        left.append(("row", [(" ", None), (lab, cls)], LWi))
    left.append(("bot", LWi))

    def pair(a, av, ac, b, bv, bc):
        right.append(("row", [(" ", None), (L(a, 11), "l"), (L(av, 13), ac),
                              (" ", None), (L(b, 9), "l"), (L(bv, 18), bc)], RWi))
    def trip(lab, gv, gc, cv, cc):
        right.append(("row", [(" ", None), (L(lab, 24), "l"), (R(gv, 12), gc),
                              (R(cv, 14), cc), ("  ", None)], RWi))
    rbox_top("example-8b-q4")
    right.append(("row", [(" ", None), (L("FILE", 25), "a"), (L("ARCHITECTURE", 27), "a")], RWi))
    pair("size", "5.02 GB", "v", "family", "qwen3-like", "v")
    pair("quantized", "Q4_K_M", "v", "params", "8.2 B", "v")
    pair("licence", "Apache-2.0", "v", "layers", "28", "v")
    pair("pulled", "3 days ago", "v", "kv heads", "8  (2 per kv)", "v")
    pair("trained ctx", "40 960", "v", "cache/tok", "112 KiB", "v")
    right.append(("mid", RWi))
    right.append(("row", [(" ", None), (L("ENGINE", 25), "a"), (L("THIS MACHINE", 27), "a")], RWi))
    pair("engine", "llama.cpp", "v", "on GPU", "5.0 of 16 GB", "g")
    pair("backend", "CUDA", "v", "on CPU", "fits", "g")
    pair("status", "ready", "g", "max ctx", "32 768", "v")
    right.append(("mid", RWi))
    right.append(("row", [(" ", None), (L("MEASURED", 24), "a"), (R("GPU", 12), "a"),
                          (R("CPU", 14), "a"), ("  ", None)], RWi))
    trip("speed at 512", "147 tok/s", "v", "11.2 tok/s", "v")
    trip("speed at 4 096", "132 tok/s", "v", "9.9 tok/s", "v")
    trip("speed at 32 768", "Unknown", "w", "Unknown", "w")
    trip("cold start", "1.9 s", "v", "4.2 s", "v")
    trip("memory ceiling", "Unknown", "w", "Unknown", "w")
    right.append(("bot", RWi))

    import html as H
    def render(item):
        kind = item[0]
        if kind == "box":
            _, lead, t, w = item
            return f'<span class="f">{lead}</span><span class="a">{H.escape(t)}</span><span class="f"> {"─"*(w-len(t)-3)}┐</span>'
        if kind == "mid": return f'<span class="f">├{"─"*item[1]}┤</span>'
        if kind == "bot": return f'<span class="f">└{"─"*item[1]}┘</span>'
        segs, w = item[1], item[2]
        plain = "".join(t for t, _ in segs)
        inner = "".join(H.escape(t) if c is None else f'<span class="{c}">{H.escape(t)}</span>' for t, c in segs) + " "*(w-len(plain))
        if kind == "sel":
            inner = f'<span class="sel">{H.escape(plain)}{" "*(w-len(plain))}</span>'
        return f'<span class="f">│</span>{inner}<span class="f">│</span>'

    s = Screen("Host a model", "Host")
    s.rows.pop(1)
    items = ["Monitor", "Host", "Diagnostics", "Models", "Settings", "Exit"]
    segs = [(" ", None)]
    for n in items:
        segs.append((f" {n} ", "sel" if n == "Host" else "l")); segs.append((" ", None))
    tail = [("●", "g"), (" ", None), ("Serving", "v"), (" ", None)]
    used = sum(len(t) for t, _ in segs) + sum(len(t) for t, _ in tail)
    s.rows.insert(1, s.row(segs + [(" "*(W-used), None)] + tail, emit=False))

    while len(left) < len(right): left.append(("pad", LWi))
    while len(right) < len(left): right.append(("pad", RWi))
    for a, b in zip(left, right):
        la = " "*(a[1]+2) if a[0] == "pad" else render(a)
        rb = " "*(b[1]+2) if b[0] == "pad" else render(b)
        plain_len = (LWi+2) + 1 + (RWi+2)
        s.rows.append(f'<span class="f">│</span>{la} {rb}{" "*(W-plain_len)}<span class="f">│</span>')
    return s.finish()

# ─────────────────────────── DIAGNOSTICS ───────────────────────────
def diagnostics():
    s = Screen("Diagnostics", "Diagnostics")
    s.rows.pop(1)
    items = ["Monitor", "Host", "Diagnostics", "Models", "Settings", "Exit"]
    segs = [(" ", None)]
    for n in items:
        segs.append((f" {n} ", "sel" if n == "Diagnostics" else "l")); segs.append((" ", None))
    tail = [("●", "g"), (" ", None), ("Serving", "v"), (" ", None)]
    used = sum(len(t) for t, _ in segs) + sum(len(t) for t, _ in tail)
    s.rows.insert(1, s.row(segs + [(" "*(W-used), None)] + tail, emit=False))
    s.blank()
    s.row([("   ", None), (" Quick Run ", "btnp"), ("     ", None), (" Run Selected ", "btn"),
           ("                         ", None), (" Back ", "btn")])
    s.row([("   ", None), (L("about 40 s", 16), "d"), (L("3 min 25 s – 8 min 12 s", 30), "d")])
    s.blank()
    s.row([("   ", None), (L("WHAT TO MEASURE", 30), "a")])
    s.row([("   ", None), (L("model", 18), "l"), ("( ", "d"), (L("example-8b-q4", 34), "btn"), ("▾ )", "d")])
    s.row([("   ", None), (L("devices", 18), "l"), ("[", "v"), ("x", "g"), ("] ", "v"),
           (L("CPU", 12), "v"), ("[", "v"), ("x", "g"), ("] ", "v"), ("GPU · RTX 5080", "v")])
    s.row([("   ", None), (L("context window", 18), "l"), ("( ", "d"),
           (L("largest this machine affords", 34), "btn"), ("▾ )", "d")])
    s.row([("   ", None), (L("", 18), None),
           (L("32 768 on GPU · 32 768 on CPU · model allows 40 960", 52), "d")])
    # The window implies the ladder: every power of two up to it is sampled, so
    # the depths are stated rather than asked for a second time.
    s.row([("   ", None), (L("samples", 18), "l"),
           ("512 · 1 024 · 2 048 · 4 096 · 8 192 · 16 384  ", "v"),
           ("every step", "d")])
    s.row([("   ", None), (L("repeats", 18), "l"), ("( ", "d"), (" 3 ", "btn"), ("▾ )", "d"),
           ("      ", None), (L("time budget", 13), "l"), ("( ", "d"), (" 5 minutes ", "btn"), ("▾ )", "d")])
    s.blank()
    s.row([("   ", None), (L("TESTS", 40), "a"), (R("DEVICES", 14), "l"), (R("TIME", 12), "l")])
    tests = [("x", "Generation speed against depth", "both", "about 3 min"),
             ("x", "Cold start cost", "both", "about 25 s"),
             (" ", "Memory ceiling — largest context", "both", "about 2 min"),
             (" ", "CPU and GPU agree on the output", "needs both", "about 90 s"),
             (" ", "Prompt reading speed", "both", "about 45 s")]
    for mark, name, dev, time in tests:
        on = mark == "x"
        s.row([("   ", None), ("[", "v"), (mark, "g"), ("] ", "v"),
               (L(name, 36), "v" if on else "l"), (R(dev, 14), "d"), (R(time, 12), "d")])
    s.rule()
    s.row([("   ", None), (L("selected", 10), "l"), (L("2 of 5", 10), "v"),
           (L("estimate", 10), "l"), (L("3 min 25 s – 8 min 12 s", 26), "a"),
           (L("budget", 8), "l"), ("5 min", "v")])
    return s.finish()

for name, frag in [("monitor", monitor(False)), ("monitor-diag", monitor(True)),
                   ("host", host()), ("diagnostics", diagnostics())]:
    n = check(frag)
    pathlib.Path(f"{name}.frag").write_text(frag)
    print(f"  {name:<14} {n} rows x 80 cols  ✓")
