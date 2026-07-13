#!/usr/bin/env python3
"""markov_attractor.py — Markov-chain attractor detector for the self-improvement loop.

WHAT / WHY
  loop-detector.sh senses "stuck" by counting CONSECUTIVE failures on ONE identical
  signature (N=3) — a 0th-order detector, blind to (a) LIMIT CYCLES across >=2
  signatures (edit->run_fail->edit->run_fail) and (b) the "STRANGE-ATTRACTOR" analog
  — busy, high-entropy churn that never reaches a progress state. Both are the
  dominant failure modes of long agent runs.

  This models the recent tool-outcome stream as a first-order Markov chain
  (Jurafsky & Martin, SLP3 Appendix A: states Q, transition matrix A, stationary pi)
  and derives DETERMINISTIC signals from A. ADVISORY (loop-detector.sh keeps
  deciding); zero-dep stdlib only; fail-open (bad/short input -> HEALTHY).

FULL / NOT-BLIND (this revision)
  Two blindnesses closed:
  1. SPECTRAL. Entropy alone can smooth a noisy-but-real oscillation. We now compute
     the actual eigenvalues of A via Faddeev-LeVerrier (characteristic polynomial)
     + Durand-Kerner (complex roots). This is exactly the general non-symmetric
     eigensolver docs/design/hydraulic-loop-v2/MATH-RESEARCH-CONSPECT.md flags as a
     CODE GAP ("symmetric Jacobi sweep misreports COMPLEX DMD eigenvalues, 2-cycle
     mu~=-1"). It yields SLEM = |lambda_2| (mixing rate; ->1 = trapped/near-reducible)
     and a PERIOD signal (an eigenvalue near the unit circle pointing away from +1 =
     a real limit cycle, incl. mu~=-1 for period-2).
  2. PROGRESS vs PROBE. A successful `ls`/`cat`/`grep`/`git status` is NOT progress.
     Counting every green Bash as escape hid churn that merely reads a lot. The shell
     hook now emits `run_ok` ONLY for verify/progress commands (test/build/commit/...)
     and `probe` for benign successful reads (neutral, non-escape).

HONEST MATH NOTE
  A finite Markov chain has NO strange attractor in the fractal/chaos sense (needs
  continuous chaotic dynamics + sensitive dependence). Rigorously it has: a recurrent
  class == the attractor; a periodic class == a LIMIT CYCLE; an entropy rate == how
  chaotic the wandering is. "strange-attractor" below is a deliberate metaphor for one
  DETECTABLE regime (bounded, non-low-entropy, non-progressing). We detect that; we
  claim no fractality.

STATE ALPHABET (derived upstream in loop-detector.sh from TOOL x FAILBIT x cmd)
  edit      : Edit/Write/MultiEdit ok                        (neutral,        V=0)
  edit_fail : Edit/Write/MultiEdit failed                    (non-progress,   V=-1)
  run_ok    : Bash ok AND verify/progress cmd (test/build..) (PROGRESS/escape,V=+1)
  probe     : Bash ok, benign read (ls/cat/grep/git status)  (neutral,        V=0)
  run_fail  : Bash failed (test/build red)                   (non-progress,   V=-1)
  Unknown tokens tolerated: V defaults to 0, never an escape state.
"""
from __future__ import annotations
import cmath
import json
import math
import sys

# --- state semantics -------------------------------------------------------
ESCAPE_STATES = frozenset({"run_ok"})
POTENTIAL = {"run_ok": 1.0, "edit": 0.0, "probe": 0.0, "edit_fail": -1.0, "run_fail": -1.0}

# --- thresholds (each justified; the test harness proves the margins) ------
MIN_EVENTS = 8       # short window -> stay quiet (cold start)
H_LO = 0.5           # bits/step; rows >~75% deterministic => "cyclic", not exploring
ESCAPE_LO = 0.05     # <5% long-run time in a progress state => effectively no escape
DRIFT_LO = 0.0       # non-positive expected one-step progress (Foster-Lyapunov)
DAMPING = 0.02       # PageRank teleport -> irreducible+aperiodic => unique pi
POWER_ITERS = 300    # fixed (deterministic; damped chain contracts fast for small N)
PERIOD_MAG = 0.85    # eigenvalue this close to the unit circle counts as persistent
PERIOD_ARG = 0.6     # rad (~34deg) away from 0 => oscillatory mode, not the trivial +1
DK_ITERS = 200       # Durand-Kerner iterations (small polynomials converge fast)


# --- small dense linear algebra (N is tiny: |alphabet| <= 5) ---------------
def _matmul(A, B, n):
    C = [[0.0] * n for _ in range(n)]
    for i in range(n):
        Ai, Ci = A[i], C[i]
        for k in range(n):
            aik = Ai[k]
            if aik == 0.0:
                continue
            Bk = B[k]
            for j in range(n):
                Ci[j] += aik * Bk[j]
    return C


def _trace(A, n):
    return sum(A[i][i] for i in range(n))


def _row_normalize(counts, n):
    a = [[0.0] * n for _ in range(n)]
    for i in range(n):
        s = sum(counts[i])
        if s > 0:
            for j in range(n):
                a[i][j] = counts[i][j] / s
        else:
            for j in range(n):
                a[i][j] = 1.0 / n  # unseen source (only the last state) -> uniform
    return a


def _stationary(a, n):
    """Left eigenvector pi (pi = pi A) of the DAMPED chain via power iteration."""
    d = DAMPING
    pi = [1.0 / n] * n
    for _ in range(POWER_ITERS):
        nxt = [0.0] * n
        for i in range(n):
            pii = pi[i]
            if pii == 0.0:
                continue
            for j in range(n):
                nxt[j] += pii * ((1.0 - d) * a[i][j] + d / n)
        s = sum(nxt) or 1.0
        pi = [v / s for v in nxt]
    return pi


def _entropy_rate(a, pi, n):
    """H = -sum_i pi_i sum_j A_ij log2 A_ij (bits/step). 0 for a deterministic cycle."""
    h = 0.0
    for i in range(n):
        row_h = 0.0
        for j in range(n):
            p = a[i][j]
            if p > 0.0:
                row_h -= p * math.log2(p)
        h += pi[i] * row_h
    return h


def _drift(a, pi, idx, n):
    """Expected one-step change in progress potential V (Foster-Lyapunov drift)."""
    inv = {v: k for k, v in idx.items()}
    mu = 0.0
    for i in range(n):
        vi = POTENTIAL.get(inv[i], 0.0)
        step = sum(a[i][j] * (POTENTIAL.get(inv[j], 0.0) - vi) for j in range(n))
        mu += pi[i] * step
    return mu


def _charpoly(a, n):
    """Faddeev-LeVerrier -> monic char-poly coeffs, highest degree first: [1, ...]."""
    ident = [[1.0 if i == j else 0.0 for j in range(n)] for i in range(n)]
    c = [0.0] * (n + 1)
    c[n] = 1.0
    m = [row[:] for row in ident]                       # M_1 = I
    c[n - 1] = -_trace(_matmul(a, m, n), n)
    for k in range(2, n + 1):
        am = _matmul(a, m, n)                           # A M_{k-1}
        add = c[n - k + 1]
        m = [[am[i][j] + (add if i == j else 0.0) for j in range(n)] for i in range(n)]
        c[n - k] = -_trace(_matmul(a, m, n), n) / k
    return [c[n - i] for i in range(n + 1)]


def _roots(coeffs):
    """Durand-Kerner: all (complex) roots of a monic polynomial (highest-first)."""
    deg = len(coeffs) - 1
    if deg <= 0:
        return []
    if deg == 1:
        return [complex(-coeffs[1])]
    p = [complex(x) for x in coeffs]

    def peval(x):
        r = 0j
        for co in p:
            r = r * x + co
        return r

    roots = [(0.4 + 0.9j) ** k for k in range(deg)]     # deterministic spread, no RNG
    for _ in range(DK_ITERS):
        maxd = 0.0
        for i in range(deg):
            xi = roots[i]
            denom = 1 + 0j
            for j in range(deg):
                if j != i:
                    denom *= (xi - roots[j])
            if denom == 0:
                continue
            delta = peval(xi) / denom
            roots[i] = xi - delta
            ad = abs(delta)
            if ad > maxd:
                maxd = ad
        if maxd < 1e-12:
            break
    return roots


def _spectral(a, n):
    """SLEM (|lambda_2|) + PERIOD signal from the true eigenvalues of A."""
    if n < 2:
        return {"slem": 0.0, "period": False, "eigs": []}
    eigs = _roots(_charpoly(a, n))
    mags = sorted((abs(e) for e in eigs), reverse=True)
    slem = mags[1] if len(mags) > 1 else 0.0            # 2nd-largest modulus
    # period: an eigenvalue near the unit circle pointing away from +1 (oscillation).
    period = any(abs(e) >= PERIOD_MAG and abs(cmath.phase(e)) >= PERIOD_ARG for e in eigs)
    return {
        "slem": round(slem, 4),
        "period": period,
        "eigs": sorted(([round(e.real, 3), round(e.imag, 3)] for e in eigs),
                       key=lambda z: -(z[0] ** 2 + z[1] ** 2)),
    }


def analyze(states):
    """states: list[str]. Returns metrics + verdict. Pure function."""
    states = [s for s in states if s]
    L = len(states)
    if L < MIN_EVENTS:
        return {"verdict": "HEALTHY", "reason": "window too short", "events": L}

    alpha = sorted(set(states))
    n = len(alpha)
    idx = {s: k for k, s in enumerate(alpha)}
    counts = [[0] * n for _ in range(n)]
    for t in range(L - 1):
        counts[idx[states[t]]][idx[states[t + 1]]] += 1

    a = _row_normalize(counts, n)
    pi = _stationary(a, n)
    H = _entropy_rate(a, pi, n)
    escape = sum(pi[idx[s]] for s in alpha if s in ESCAPE_STATES)
    mu = _drift(a, pi, idx, n)
    spec = _spectral(a, n)

    # A trap requires EVIDENCE OF STRUGGLE — at least one failure in the window.
    # `escape==0` alone is just quiet non-test work (editing docs, reading files); it
    # is NOT churning. Without this guard the detector false-fires on a task's wrap-up
    # phase (all edit+probe, no run) — which is exactly how it first fired on its own
    # author. You cannot be "stuck thrashing" if nothing is failing.
    has_failure = any(s in ("run_fail", "edit_fail") for s in states)

    # verdict — severity order matters (a clean trap is also low-escape).
    #   LIMIT_CYCLE      : struggling AND (low entropy OR spectral oscillation)
    #   STRANGE_ATTRACTOR: struggling, no net progress, high entropy, no clean period
    trapped = escape <= ESCAPE_LO and has_failure
    if trapped and (H <= H_LO or spec["period"]):
        verdict = "LIMIT_CYCLE"
        reason = (f"cyclic trap: escape={escape:.3f}, H={H:.3f}, "
                  f"period={spec['period']}, slem={spec['slem']:.3f}")
    elif trapped and mu <= DRIFT_LO and H > H_LO:
        verdict = "STRANGE_ATTRACTOR"
        reason = (f"bounded churn never reaching progress: escape={escape:.3f}, "
                  f"H={H:.3f}, drift={mu:+.3f}, slem={spec['slem']:.3f}")
    elif not has_failure:
        verdict = "HEALTHY"
        reason = f"quiet work, no failures in window (escape={escape:.3f}, H={H:.3f})"
    else:
        verdict = "HEALTHY"
        reason = f"progress reachable: escape={escape:.3f}, drift={mu:+.3f}, H={H:.3f}"

    return {
        "verdict": verdict,
        "reason": reason,
        "events": L,
        "alphabet": alpha,
        "entropy_rate_bits": round(H, 4),
        "escape_mass": round(escape, 4),
        "drift": round(mu, 4),
        "has_failure": has_failure,
        "slem": spec["slem"],
        "period": spec["period"],
        "eigs": spec["eigs"],
        "stationary": {alpha[k]: round(pi[k], 4) for k in range(n)},
    }


def main():
    try:
        toks = [ln.strip() for ln in sys.stdin.read().splitlines()]
        print(json.dumps(analyze(toks)))
    except Exception:  # fail-open: never let the analyzer break the hook
        print(json.dumps({"verdict": "HEALTHY", "reason": "analyzer error"}))
    return 0


if __name__ == "__main__":
    sys.exit(main())
