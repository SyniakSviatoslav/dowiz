# eqc-rs — the equation compiler (pure Rust)

**One source-of-truth math expression → committed, hand-inspectable Rust.**
Closes the "translation gap": the single most bug-prone step in a math-first
kernel is a human transcribing a formula into code (precedence, a dropped term,
a wrong sign, a float where an integer belongs). `eqc` removes the human step —
you build the equation once as an `Expr` tree (Rust operator overloading gives
near-math syntax: `a * x.clone().pow(2) + b * x + c`), and `eqc` emits a
*second*, independent Rust source string from it.

Supersedes `tools/eqc/` (Python + SymPy), retired 2026-07-17: that version was
itself a live contradiction of the repo's "core logic is Rust, never
Python/Node/JS" rule. This crate is a from-scratch, zero-dependency
reimplementation of exactly the subset `eqc.py` exercised — `+ - * pow sqrt sin
cos exp`, symbols, and numeric constants — not a general computer-algebra
system (no simplify/diff/integrate, because the Python version never used those
either). Equations are authored as `Expr` trees directly; there is no text
parser, so there is nothing to bootstrap.

## What it is NOT

Not a runtime transpiler / proxy / middleware. `eqc` runs at **authoring time**,
offline. Its output is ordinary Rust that you commit and compile normally — zero
runtime indirection. The equation is the source; the emitted `.rs` snippet is a
derived artifact, like an object file from source. (This respects the standing
directive against runtime transpilers in `bebop2/ARCHITECTURE.md` — build-time
codegen is not that.)

## The one feature that matters: dual emission

From a single `Expr`, `eqc` emits **both** variants plus the proof they agree:

| Method | Output |
|---|---|
| `emit_f64_rust()` | float variant (dynamics path) |
| `emit_fixed_rust()` | **fixed-point Q-format** (integer-scaled) variant — bitwise-identical on every CPU/wasm target. Refuses (`Err`, never silently falls back) on nodes it cannot represent exactly. |
| `emit_proof_program()` | a self-contained Rust program whose `main` **asserts** f64 ≈ fixed ≈ `Expr::eval`'s reference value at sample points |

Fixed-point model (Q-format): a real `v` is carried as `I = round(v * 2^SHIFT)`.
`Add: I1+I2`; `Mul: (I1*I2)/2^SHIFT` (i128 intermediate, deterministic);
`Pow(_, n>=0): repeated Mul`; `const c: round(c*2^SHIFT)`. This subset (`+- *`,
non-negative integer powers, constants) is exactly the money-law / polynomial
organs that *want* determinism; `sqrt/sin/cos/exp` are dynamics and stay f64.

## Example (real output)

```rust
// eqc: tax = sub * rate   (the money organ)
#[inline(always)]
pub fn tax_f64(sub: f64, rate: f64) -> f64 { (sub * rate) }

#[inline(always)]
pub fn tax_fixed(sub: i64, rate: i64) -> i64 {   // Q32; bitwise-deterministic
    ((((sub as i128) * (rate as i128)) / 4294967296i128)) as i64
}
```

## Proof (Mandatory Proof Rule)

`tests/proof.rs` does not "should-work" check — it EMITS Rust, compiles it with
the real `rustc`, RUNS it, and the generated program self-asserts against
`Expr::eval` (a tree-walking interpreter, a code path independent of the
string-emitting codegen under test). If codegen is wrong an assert fails and the
process exits non-zero.

```
$ cd tools/eqc-rs && cargo test --release
running 4 tests
test ci_smoke_transcendental_f64_proven ... ok
test hyp_f64_proven_fixed_correctly_refused ... ok
test quad_f64_and_fixed_proven ... ok
test tax_f64_and_fixed_proven ... ok
```

## Usage

```rust
use eqc_rs::{Equation, Expr};

let (sub, rate) = (Expr::sym("sub"), Expr::sym("rate"));
let tax = Equation::new("tax", &["sub", "rate"], sub * rate);
println!("{}", tax.emit_fixed_rust().unwrap()); // -> the crystalline integer variant
```

`cargo run --bin eqc-demo` prints the tax/hyp examples end-to-end (Rust
replacement for the retired `demo.py`).

## Requirements

`rustc` on PATH (for the proof harness — same requirement the Python version
had). Zero crate dependencies.

## Roadmap (see `docs/design/math-first-architecture-blueprint.md`)

- Q-format **rounding modes** (half-up like `money.rs`, not just truncation).
- **Overflow guard**: emit `checked_mul`/domain asserts for the fixed path.
- **Fixed-point transcendentals** via CORDIC/LUT (extend the refused set).
- **wasm-bindgen wrapper** emission + the parity `#[test]` beside each organ.
- A machine-readable **equation IR** so every kernel math organ is generated +
  parity-tested from one source of truth.
