"""Candidate shapes for the fall-off, and the discipline for choosing between them.

The quantity throughout is **milliseconds per token at depth d** — the marginal
cost of the next token when d tokens already sit in the cache. Its reciprocal is
the rate. Working in ms/token rather than tok/s is not cosmetic: the theory is
additive in cost (a fixed part plus an attention part), and a sum is what a
straight line can be fitted to. A rate is the reciprocal of a sum and is
straight in nothing.

No timing originates here. Every function consumes readings taken by a
prototype on real hardware and returns arithmetic (A11).
"""

import numpy as np
from scipy.optimize import curve_fit

# ── the candidates ───────────────────────────────────────────────────────────
# Each is cost(depth, *params) in ms/token, with the reason it is a candidate.


def linear(d, a, b):
    """a + b·d — what the theory says.

    Attention at depth d reads d keys and values, so the marginal cost should
    rise linearly, on top of a fixed per-token cost a (projections and MLP,
    which do not know how deep they are)."""
    return a + b * d


def quadratic(d, a, b, c):
    """a + b·d + c·d² — a line that is allowed to bend either way.

    Not motivated by theory. It is here as the cheapest way to detect that the
    truth is *not* a line, and which way it departs."""
    return a + b * d + c * d * d


def power(d, a, b, k):
    """a + b·d^k — a line when k = 1, flattening below, steepening above.

    k is the interesting number: it says whether the attention term grows as
    fast as the token count, which is the thing the theory asserts."""
    return a + b * np.power(np.maximum(d, 1e-9), k)


def saturating(d, a, b, k):
    """a + b·d/(1 + d/k) — attention cost that flattens as it becomes
    bandwidth-bound.

    Motivated: once the cache no longer fits a level of the memory hierarchy,
    the machine is reading from the next level at a fixed rate, and the *per
    token* increment stops growing. k is the depth where that turns over."""
    return a + b * d / (1.0 + d / np.maximum(k, 1e-9))


def knee(d, a, b1, b2, k):
    """Two straight segments joined at depth k.

    The cache hierarchy has edges, not curves. If the fall-off has a corner in
    it, no smooth form will sit on it and this one will."""
    d = np.asarray(d, dtype=float)
    return np.where(d <= k, a + b1 * d, a + b1 * k + b2 * (d - k))


FORMS = {
    "linear": (linear, 2),
    "quadratic": (quadratic, 3),
    "power": (power, 3),
    "saturating": (saturating, 3),
    "knee": (knee, 4),
}


def _guess(name, d, y):
    """A starting point near the data, so the fit does not wander."""
    lo, hi = float(np.min(d)), float(np.max(d))
    span = max(hi - lo, 1.0)
    a0 = float(np.min(y))
    b0 = max((float(np.max(y)) - a0) / span, 1e-12)
    return {
        "linear": ([a0, b0], (-np.inf, np.inf)),
        "quadratic": ([a0, b0, 0.0], (-np.inf, np.inf)),
        "power": ([a0, b0, 1.0], ([0, 0, 0.1], [np.inf, np.inf, 3.0])),
        "saturating": ([a0, b0, hi], ([0, 0, 1.0], [np.inf, np.inf, np.inf])),
        "knee": ([a0, b0, b0, (lo + hi) / 2], ([0, 0, 0, lo], [np.inf, np.inf, np.inf, hi])),
    }[name]


def fit(name, d, y):
    """Fits one form. Returns (params, covariance) or None where it will not sit.

    A form that cannot be fitted is not a failure to hide (A2): the caller is
    told None and reports the form as unfitted rather than scoring it."""
    d = np.asarray(d, float)
    y = np.asarray(y, float)
    func, n = FORMS[name]
    if len(d) < n:
        return None
    p0, bounds = _guess(name, d, y)
    try:
        params, cov = curve_fit(func, d, y, p0=p0, bounds=bounds, maxfev=20000)
    except (RuntimeError, ValueError):
        return None
    return params, cov


def predict(name, params, d):
    return FORMS[name][0](np.asarray(d, float), *params)


def interval(name, params, cov, d, residual_sd, sigmas=2.0):
    """A prediction interval, by pushing the parameter covariance through the
    form and adding the scatter of the readings themselves.

    Stated so it can be *checked*: the synthetic lab measures how often the
    truth actually lands inside, which is the only thing that makes an interval
    worth printing (A6)."""
    func = FORMS[name][0]
    d = np.atleast_1d(np.asarray(d, float))
    params = np.asarray(params, float)
    grad = np.empty((len(d), len(params)))
    for i in range(len(params)):
        step = max(abs(params[i]) * 1e-6, 1e-12)
        up, dn = params.copy(), params.copy()
        up[i] += step
        dn[i] -= step
        grad[:, i] = (func(d, *up) - func(d, *dn)) / (2 * step)
    if cov is None or not np.all(np.isfinite(cov)):
        var = np.zeros(len(d))
    else:
        var = np.einsum("ij,jk,ik->i", grad, cov, grad)
        var = np.maximum(var, 0.0)
    return sigmas * np.sqrt(var + residual_sd**2)


def extrapolation_trial(d, y, cut, forms=None):
    """The protocol, and the whole point of this file.

    Fit using only readings at depth <= cut; predict every reading deeper than
    cut; report the error at each. This is what a cheap diagnostic actually
    does, so it is what must be scored — never the fit's agreement with the
    points it was fitted to, which measures nothing."""
    d = np.asarray(d, float)
    y = np.asarray(y, float)
    train, test = d <= cut, d > cut
    out = {}
    if train.sum() < 2 or test.sum() == 0:
        return out
    for name in (forms or FORMS):
        got = fit(name, d[train], y[train])
        if got is None:
            continue
        params, cov = got
        fitted = predict(name, params, d[train])
        dof = max(train.sum() - FORMS[name][1], 1)
        residual_sd = float(np.sqrt(np.sum((y[train] - fitted) ** 2) / dof))
        pred = predict(name, params, d[test])
        err = (pred - y[test]) / y[test] * 100.0
        half = interval(name, params, cov, d[test], residual_sd)
        out[name] = {
            "params": params,
            "depths": d[test],
            "predicted": pred,
            "measured": y[test],
            "error_pct": err,
            "worst_pct": float(np.max(np.abs(err))),
            "reach": float(np.max(d[test]) / cut),
            "covered": bool(np.all(np.abs(pred - y[test]) <= half)),
            "half_width": half,
        }
    return out
