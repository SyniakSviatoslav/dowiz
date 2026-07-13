#!/usr/bin/env python3
"""Falsifiable red->green proof for markov_attractor.analyze().

Run: python3 tools/loop-signals/test_markov_attractor.py
Each case prints computed metrics so the separation margin is visible; asserts
fail (non-zero exit) if the detector ever mislabels a stream.
"""
import sys
from markov_attractor import analyze, H_LO, ESCAPE_LO

FAILS = []


def check(name, states, expect):
    r = analyze(states)
    got = r["verdict"]
    ok = got == expect
    m = (f"H={r.get('entropy_rate_bits','-')} escape={r.get('escape_mass','-')} "
         f"drift={r.get('drift','-')} slem={r.get('slem','-')} period={r.get('period','-')}")
    print(f"[{'PASS' if ok else 'FAIL'}] {name:<36} expect={expect:<17} got={got:<17} {m}")
    if not ok:
        FAILS.append((name, expect, got, r))
    return r


def lcg_walk(alphabet, n, seed=1):
    """Deterministic pseudo-random walk (no RNG import; reproducible)."""
    x = seed
    out = []
    for _ in range(n):
        x = (1103515245 * x + 12345) & 0x7FFFFFFF
        out.append(alphabet[x % len(alphabet)])
    return out


# 1) HEALTHY rhythm: edit -> green run. Low entropy BUT high escape.
#    The case naive "alternation == stuck" logic gets WRONG.
check("healthy rhythm (edit<->run_ok)", ["edit", "run_ok"] * 8, "HEALTHY")

# 2) HEALTHY varied with regular progress.
check("healthy varied+progress",
      ["edit", "run_fail", "edit", "run_ok", "edit", "run_ok", "run_fail",
       "edit", "run_ok", "edit", "run_ok"], "HEALTHY")

# 3) RED — LIMIT CYCLE: edit <-> run_fail thrash.
check("limit cycle (edit<->run_fail)", ["edit", "run_fail"] * 8, "LIMIT_CYCLE")

# 4) RED — LIMIT CYCLE: deterministic 3-cycle, no green.
check("limit cycle (3-cycle, no green)",
      ["edit", "run_fail", "edit_fail"] * 5, "LIMIT_CYCLE")

# 5) RED — STRANGE-ATTRACTOR: high-entropy churn over failure states, no green.
check("strange-attractor (churn, no green)",
      lcg_walk(["edit", "edit_fail", "run_fail"], 40), "STRANGE_ATTRACTOR")

# 6) Cold start -> quiet (fail-open, no false alarm).
check("cold start (short window)", ["edit", "run_fail", "edit"], "HEALTHY")

# 7) RED — UN-BLINDING: agent fails, does a benign successful read, fails again.
#    OLD alphabet counted the read as `run_ok` (progress) -> escape inflated ->
#    HEALTHY (blind). NEW: benign read = `probe` (neutral) -> caught as a cycle.
r7 = check("un-blinded: edit->fail->probe cycle",
           ["edit", "run_fail", "probe"] * 5, "LIMIT_CYCLE")
assert r7["escape_mass"] == 0, "probe must NOT count as escape/progress"

# 8) RED — spectral does INDEPENDENT work: a bipartite "star" oscillation where
#    `edit` branches 3 ways (entropy 0.79 > H_LO, so the entropy path stays SILENT),
#    yet the graph is bipartite -> a real eigenvalue at -1. Only the spectral period
#    signal classifies this trap. This is the CODE-GAP case (complex/-1 eigenvalue).
star = ["edit", "run_fail", "edit", "edit_fail", "edit", "probe"] * 4
r8 = check("spectral-only: bipartite star (H>H_LO)", star, "LIMIT_CYCLE")
assert r8["entropy_rate_bits"] > H_LO, "star must exceed entropy threshold (entropy path silent)"
assert r8["period"] is True, "spectral period signal (lambda~=-1) must fire"
assert r8["escape_mass"] == 0, "no green run -> genuinely trapped"

# 9) REGRESSION (live false-positive): the detector fired STRANGE_ATTRACTOR on its
#    own author during task WRAP-UP — all edit+probe, zero failures, zero test runs.
#    Quiet non-test work is NOT a trap: a trap requires evidence of struggle.
r9 = check("wrap-up bookkeeping (no failures)",
           ["probe", "edit", "probe", "edit", "edit", "edit", "edit", "edit"], "HEALTHY")
assert r9["has_failure"] is False, "this window has no failure states"

# 10) A longer edit/probe bookkeeping run -> still HEALTHY (no struggle signal).
check("bookkeeping run (edit/probe only)",
      ["edit", "probe", "edit", "probe", "edit", "edit", "probe", "edit", "edit"], "HEALTHY")

# --- spectral separation proof --------------------------------------------
lc = analyze(["edit", "run_fail"] * 8)
sa = analyze(lcg_walk(["edit", "edit_fail", "run_fail"], 40))
print(f"\nseparation: limit-cycle H={lc['entropy_rate_bits']} <= {H_LO} < strange H={sa['entropy_rate_bits']}")
print(f"            escape<= {ESCAPE_LO}: lc={lc['escape_mass']} sa={sa['escape_mass']}")
print(f"            spectral: limit-cycle period={lc['period']} slem={lc['slem']} | "
      f"strange period={sa['period']} slem={sa['slem']}")
assert lc["entropy_rate_bits"] <= H_LO < sa["entropy_rate_bits"], "entropy must separate the traps"
assert lc["escape_mass"] <= ESCAPE_LO and sa["escape_mass"] <= ESCAPE_LO, "traps must have ~0 escape"
assert lc["period"] is True, "a clean 2-cycle must show the spectral period signal"
assert lc["slem"] >= 0.95, "a 2-cycle must have |lambda_2| ~ 1 (poorly mixing)"
assert sa["slem"] < lc["slem"], "churn must mix faster (smaller |lambda_2|) than a clean cycle"

if FAILS:
    print(f"\n{len(FAILS)} FAILURE(S)")
    sys.exit(1)
print("\nALL GREEN")
