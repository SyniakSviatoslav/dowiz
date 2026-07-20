# eqc — the equation compiler

**One source-of-truth math expression → committed, hand-inspectable Rust.**
Closes the "translation gap": the single most bug-prone step in a math-first
kernel is a human transcribing a formula into code (precedence, a dropped term,
a wrong sign, a float where an integer belongs). `eqc` removes the human step —
you author the equation once as a SymPy expression, and `eqc` emits the Rust.

## What it is NOT

Not a runtime transpiler / proxy / middleware. `eqc` runs at **authoring time**,
offline. Its output is ordinary Rust that you commit and compile normally — zero
runtime indirection. The equation is the source; the `.rs` is a derived artifact,
like an object file from source. (This respects the standing directive against
runtime transpilers in `bebop2/ARCHITECTURE.md` — build-time codegen is not that.)

## The one feature that matters: dual emission

From a single equation, `eqc` emits **both** variants plus the proof they agree:

| Method | Output |
|---|---|
| `emit_f64_rust()` | float variant (dynamics; via SymPy's own Rust printer) |
| `emit_fixed_rust()` | **fixed-point Q-format** (integer-scaled) variant — bitwise-identical on every CPU/wasm target. Refuses (honestly) on transcendentals it cannot represent exactly. |
| `emit_proof_program()` | a self-contained Rust program whose `main` **asserts** f64 ≈ fixed ≈ the SymPy reference at sample points |

Fixed-point model (Q-format): a real `v` is carried as `I = round(v · 2^SHIFT)`.
`Add: I₁+I₂`; `Mul: (I₁·I₂)/2^SHIFT` (i128 intermediate, deterministic);
`Pow(·,n≥0): repeated Mul`; `const c: round(c·2^SHIFT)`. This subset (`±·`, integer
powers, constants) is exactly the money-law / polynomial organs that *want*
determinism; `sqrt/sin/exp` are dynamics and stay f64.

## Example (real output)

```rust
// eqc: tax = sub * rate   (the money organ)
#[inline(always)]
pub fn tax_f64(sub: f64, rate: f64) -> f64 { rate*sub }

#[inline(always)]
pub fn tax_fixed(sub: i64, rate: i64) -> i64 {   // Q32; bitwise-deterministic
    ((((rate as i128) * (sub as i128)) / 4294967296i128)) as i64
}
```

## Proof (Mandatory Proof Rule)

`test_eqc.py` does not "should-work" check — it EMITS Rust, compiles it with the
real `rustc`, RUNS it, and the generated program self-asserts against SymPy. If
codegen is wrong an assert fails and the process exits non-zero.

```
$ /path/to/python-with-sympy tools/eqc/test_eqc.py
  ✓ quad → f64+fixed proven (eqc proof OK: quad (2 samples, fixed=True))
  ✓ tax  → f64+fixed proven (eqc proof OK: tax (2 samples, fixed=True))
  ✓ hyp  → f64 proven, fixed correctly REFUSED (eqc proof OK: hyp (1 samples, fixed=False))
ALL EQC PROOFS PASSED (3/3 equations, rustc-compiled + executed)
```

## Usage

```python
import sympy as sp
from eqc import Equation
sub, rate = sp.symbols("sub rate", real=True)
tax = Equation("tax", ["sub", "rate"], sub * rate)
print(tax.emit_fixed_rust())     # → the crystalline integer variant
```

## Requirements

`sympy` (`pip install sympy`). `rustc` on PATH for the proof harness. See
`requirements.txt`.

## Roadmap (see `docs/design/math-first-architecture-blueprint.md`)

- Q-format **rounding modes** (half-up like `money.rs`, not just truncation).
- **Overflow guard**: emit `checked_mul`/domain asserts for the fixed path.
- **Fixed-point transcendentals** via CORDIC/LUT (extend the refused set).
- **wasm-bindgen wrapper** emission + the parity `#[test]` beside each organ.
- A machine-readable **equation IR** (`organ.eq.py` specs) so every kernel math
  organ is generated + parity-tested from one source of truth.
