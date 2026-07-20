"""Show what eqc emits. Run with a sympy-equipped python:
    <python> tools/eqc/demo.py
"""
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import sympy as sp  # noqa: E402
from eqc import Equation, FixedPointUnsupported  # noqa: E402

sub, rate = sp.symbols("sub rate", real=True)
a, b = sp.symbols("a b", real=True)

print("# ── tax = sub * rate  (money organ: the last f64 that touches money) ──\n")
tax = Equation("tax", ["sub", "rate"], sub * rate)
print(tax.emit_f64_rust(), "\n")
print(tax.emit_fixed_rust(), "\n")

print("# ── hyp = sqrt(a^2 + b^2)  (dynamics: transcendental → fixed-point refused) ──\n")
hyp = Equation("hyp", ["a", "b"], sp.sqrt(a**2 + b**2))
print(hyp.emit_f64_rust(), "\n")
try:
    hyp.emit_fixed_rust()
except FixedPointUnsupported as e:
    print(f"// emit_fixed_rust() → FixedPointUnsupported: {e}")
