#!/usr/bin/env python3
"""How many tokens does one probe actually need?

Every cost projection rests on this. A probe prefills to a depth and then times
some generated tokens; the tokens are the expensive part for a large model, and
128 of them was a number chosen when the first probe was written, not measured.

If 32 give the same answer with acceptable spread, every large-model estimate
falls by a factor of four and a 70B model comes inside five minutes. If they do
not, the projections are wrong and should be corrected rather than believed.

Run under an exclusive window. Produces readings; the arithmetic is elsewhere.
"""

import json
import subprocess
import statistics as st
import sys
import time
import urllib.request

PORT = 8085
LENGTHS = [int(x) for x in __import__('os').environ.get('LENGTHS','8,16,32,64,128,256').split(',')]
REPEATS = int(__import__('os').environ.get('REPEATS','7'))
DEPTH = 2048


def wait(port, seconds=120):
    for _ in range(seconds * 5):
        try:
            with urllib.request.urlopen(f"http://127.0.0.1:{port}/health", timeout=1) as r:
                if b'"ok"' in r.read():
                    return True
        except OSError:
            pass
        time.sleep(0.2)
    return False


def probe(port, depth, tokens):
    body = json.dumps({
        "prompt": "A" + " the" * max(depth - 1, 0), "n_predict": tokens,
        "stream": True, "cache_prompt": False, "ignore_eos": True,
        "temperature": 0, "seed": 0,
    }).encode()
    req = urllib.request.Request(f"http://127.0.0.1:{port}/completion", data=body,
                                 headers={"Content-Type": "application/json"})
    gaps, first, previous = [], None, None
    started = time.perf_counter()
    with urllib.request.urlopen(req) as response:
        for raw in response:
            line = raw.decode("utf-8", "replace").strip()
            if not line.startswith("data: "):
                continue
            now = time.perf_counter()
            if first is None:
                first = now - started
            else:
                gaps.append(now - previous)
            previous = now
    return (sum(gaps) / len(gaps) * 1e3) if gaps else None


def main(models):
    server = subprocess.run(["bash", "-c",
                             "command -v llama-server || find ~/.local/share/mcf "
                             "-name llama-server -type f | head -1"],
                            capture_output=True, text=True).stdout.strip()
    print(f"depth {DEPTH}, {REPEATS} repeats at each length\n")
    for path in models:
        name = path.split("/")[-1]
        proc = subprocess.Popen([server, "-m", path, "--host", "127.0.0.1", "--port", str(PORT),
                                 "-c", "4096", "-ngl", "0", "--no-warmup"],
                                stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
        try:
            if not wait(PORT):
                print(f"  {name}: no server"); continue
            probe(PORT, 256, 8)
            print(f"  {name}")
            print(f"    {'tokens':>7}{'ms/token':>11}{'spread':>9}{'vs 256':>9}{'cost':>9}")
            got = {}
            for n in LENGTHS:
                runs = [probe(PORT, DEPTH, n) for _ in range(REPEATS)]
                runs = [r for r in runs if r]
                if not runs:
                    continue
                got[n] = st.median(runs)
                spread = (max(runs) - min(runs)) / st.median(runs) * 100
                ref = got.get(max(got))
                print(f"    {n:>7}{st.median(runs):>11.3f}{spread:>8.1f}%"
                      f"{(got[n] / got[LENGTHS[-1]] - 1) * 100 if LENGTHS[-1] in got else 0:>+8.1f}%"
                      f"{st.median(runs) * n / 1e3:>8.1f}s")
            if LENGTHS[-1] in got:
                print(f"    against {LENGTHS[-1]} tokens as the reference:")
                for n in LENGTHS[:-1]:
                    if n in got:
                        print(f"      {n:>4} tokens: {(got[n]/got[LENGTHS[-1]]-1)*100:+6.1f}%")
        finally:
            proc.terminate(); proc.wait()


if __name__ == "__main__":
    main(sys.argv[1:])
