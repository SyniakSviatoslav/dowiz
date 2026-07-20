"""Proof harness for eqc (Mandatory Proof Rule).

This is not a "should work" check: it EMITS Rust from equations, compiles it with
the real `rustc`, and RUNS it. The generated program's `main` asserts the emitted
f64 AND fixed-point code equal the SymPy-evaluated reference at sample points. If
codegen is wrong, an assert fails, the process exits non-zero, and this harness
fails. It also proves the honest fixed-point boundary (transcendentals refuse).

Run:  <python-with-sympy> tools/eqc/test_eqc.py
"""

import subprocess
import sys
import tempfile
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))

import sympy as sp  # noqa: E402
from eqc import Equation, FixedPointUnsupported  # noqa: E402

RUSTC = "rustc"


def _compile_and_run(rust_src: str, tag: str) -> str:
    with tempfile.TemporaryDirectory() as d:
        src = Path(d) / f"{tag}.rs"
        binp = Path(d) / tag
        src.write_text(rust_src)
        cc = subprocess.run(
            [RUSTC, "-O", "-C", "lto=fat", "--edition", "2021", "-o", str(binp), str(src)],
            capture_output=True, text=True,
        )
        if cc.returncode != 0:
            raise AssertionError(f"[{tag}] rustc FAILED:\n{cc.stderr}\n--- source ---\n{rust_src}")
        run = subprocess.run([str(binp)], capture_output=True, text=True)
        if run.returncode != 0:
            raise AssertionError(f"[{tag}] generated program FAILED its own asserts:\n{run.stderr}")
        return run.stdout.strip()


def main() -> int:
    x, a, b, c, sub, rate = sp.symbols("x a b c sub rate", real=True)
    passed = []

    # 1. Quadratic — fully fixed-point-representable (±, ·, integer power, consts).
    quad = Equation("quad", ["a", "b", "c", "x"], a * x**2 + b * x + c)
    out = _compile_and_run(
        quad.emit_proof_program(
            [{"a": 2.0, "b": -3.0, "c": 1.5, "x": 3.5},
             {"a": 0.5, "b": 4.0, "c": -2.0, "x": -1.25}],
            eps=1e-4,
        ),
        "quad",
    )
    assert "eqc proof OK" in out and "fixed=True" in out, out
    passed.append(f"quad → f64+fixed proven ({out})")

    # 2. Tax law `tax = sub * rate` — the money-adjacent fixed-point candidate
    #    (lane-1 identified `tax_rate: f64` as the last float touching money).
    tax = Equation("tax", ["sub", "rate"], sub * rate)
    out = _compile_and_run(
        tax.emit_proof_program(
            [{"sub": 1234.56, "rate": 0.2}, {"sub": 99.99, "rate": 0.075}],
            eps=1e-3,
        ),
        "tax",
    )
    assert "eqc proof OK" in out and "fixed=True" in out, out
    passed.append(f"tax  → f64+fixed proven ({out})")

    # 3. Haversine-shaped (sqrt) — MUST refuse fixed-point (honest boundary),
    #    while the f64 variant still emits correctly.
    hav = Equation("hyp", ["a", "b"], sp.sqrt(a**2 + b**2))
    try:
        hav.emit_fixed_rust()
        raise AssertionError("expected FixedPointUnsupported for sqrt, got none")
    except FixedPointUnsupported:
        pass
    f64_src = hav.emit_f64_rust()
    assert ".sqrt()" in f64_src, f64_src
    # the f64 variant still compiles + runs
    out = _compile_and_run(
        hav.emit_proof_program([{"a": 3.0, "b": 4.0}], eps=1e-9),  # → 5.0
        "hyp",
    )
    assert "fixed=False" in out, out
    passed.append(f"hyp  → f64 proven, fixed correctly REFUSED ({out})")

    print("\n=== EQC PROOF RESULTS ===")
    for p in passed:
        print("  ✓ " + p)
    print(f"\nALL EQC PROOFS PASSED ({len(passed)}/3 equations, rustc-compiled + executed)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
