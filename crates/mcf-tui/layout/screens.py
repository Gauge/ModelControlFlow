"""Every console screen, at exactly 80 x 24 — the size every terminal has."""
import html

COLS, ROWS = 80, 24
W = COLS - 2                      # inner width, between the outer frame

def esc(t, c): return html.escape(t) if c is None else f'<span class="{c}">{html.escape(t)}</span>'
def L(t, n): return t.ljust(n)
def R(t, n): return t.rjust(n)

class Screen:
    def __init__(self, title, here):
        self.rows = []
        pad = "─" * (W - len(title) - 3)
        self.rows.append(f'<span class="f">┌─ </span><span class="a">{html.escape(title)}</span><span class="f"> {pad}┐</span>')
        self.menu(here)
        self.rows.append(f'<span class="f">╞{"═"*W}╡</span>')

    def menu(self, here, dot=None, word=None, dot_cls="g"):
        items = ["Monitor", "Host", "Diagnostics", "Models", "Settings", "Exit"]
        segs = [(" ", None)]
        for name in items:
            if name == here:
                segs.append((f" {name} ", "sel"))
            else:
                segs.append((f" {name} ", "l"))
            segs.append((" ", None))
        if dot:
            tail = [(dot, dot_cls), (" ", None), (word, "v"), (" ", None)]
            used = sum(len(t) for t, _ in segs) + sum(len(t) for t, _ in tail)
            segs.append((" " * (W - used), None))
            segs += tail
        self.rows.append(self.row(segs, emit=False))

    def row(self, segs, emit=True):
        plain = "".join(t for t, _ in segs)
        if len(plain) > W:
            raise SystemExit(f"row {len(plain)}>{W}: {plain!r}")
        out = f'<span class="f">│</span>{"".join(esc(t,c) for t,c in segs)}{" "*(W-len(plain))}<span class="f">│</span>'
        if emit: self.rows.append(out)
        return out

    def blank(self): self.row([])
    def rule(self): self.rows.append(f'<span class="f">├{"─"*W}┤</span>')

    def finish(self):
        while len(self.rows) < ROWS - 1:
            self.blank()
        self.rows.append(f'<span class="f">└{"─"*W}┘</span>')
        if len(self.rows) != ROWS:
            raise SystemExit(f"{len(self.rows)} rows, wanted {ROWS}")
        return '<div class="screen"><pre>\n' + "\n".join(self.rows) + '\n</pre></div>'

def check(fragment):
    """Every rendered line is exactly COLS characters once markup is stripped."""
    import re
    body = fragment.split("<pre>\n", 1)[1].rsplit("\n</pre>", 1)[0]
    for i, ln in enumerate(body.split("\n"), 1):
        plain = html.unescape(re.sub(r"<[^>]+>", "", ln))
        if len(plain) != COLS:
            raise SystemExit(f"line {i} is {len(plain)} cols, wanted {COLS}: {plain!r}")
    return len(body.split("\n"))
