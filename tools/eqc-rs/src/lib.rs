//! eqc — the equation compiler. ONE source-of-truth math expression -> Rust code.
//!
//! WHY THIS EXISTS (the "translation gap", closed).
//! The most error-prone moment in a math-first kernel is the hand-translation of a
//! formula into code: operator precedence, a dropped term, a wrong sign, a float
//! where an integer belongs. `eqc` removes the hand step: you build the equation
//! once (as an `Expr` tree — Rust operator overloading gives near-math syntax), and
//! `eqc` EMITS a *second*, independent Rust source string from it. What ships is
//! ordinary, hand-inspectable Rust that is committed to the repo and compiled
//! normally.
//!
//! NOT a runtime transpiler / proxy / middleware (cf. bebop2/ARCHITECTURE.md's
//! standing directive against those). `eqc` runs at AUTHORING time, offline. Its
//! output has ZERO runtime indirection — the generated `fn` is exactly what a
//! careful human would write, minus the transcription bugs. The equation is the
//! source; the emitted Rust is a derived artifact, like an object file from source.
//!
//! THE ONE FEATURE THAT MATTERS: dual emission from a single equation —
//!   1. `emit_f64_rust`     — the float variant (dynamics path).
//!   2. `emit_fixed_rust`   — the fixed-point (integer-scaled, Q-format) variant:
//!      bitwise-identical on every CPU/wasm target, the "crystalline" money-grade
//!      path. Refuses (honestly) on nodes it cannot represent exactly (sqrt/sin/
//!      cos/exp, non-integer or negative powers).
//!   3. `emit_proof_program` — a self-contained Rust program whose `main` ASSERTS
//!      f64 ≈ fixed ≈ a reference value computed by directly evaluating the SAME
//!      `Expr` tree (a tree-walking interpreter, a code path independent of the
//!      string-emitting codegen below). If codegen is wrong the asserts fail and
//!      `rustc && run` exits non-zero. The proof is the artifact, per the
//!      Mandatory Proof Rule.
//!
//! Fixed-point model (Q-format): a real value v is carried as the integer
//! I = round(v * 2^SHIFT). Add: I1+I2. Mul: (I1*I2)/2^SHIFT (i128 intermediate,
//! truncating toward zero — deterministic). Pow(*, n>=0): repeated Mul. Const c:
//! round(c*2^SHIFT). This subset (+-, *, integer powers, constants) covers exactly
//! the money-law / polynomial organs that WANT determinism; sqrt/sin/cos/exp are
//! dynamics and stay f64.
//!
//! Supersedes `tools/eqc/eqc.py` (retired): that version depended on Python +
//! SymPy, a live contradiction of the repo's own "core logic is Rust, never
//! Python" rule. This crate reimplements exactly the subset `eqc.py` exercised —
//! it is not a general computer-algebra system (no simplify/diff/integrate),
//! because `eqc.py` never used those either; equations are authored as `Expr`
//! trees directly rather than parsed from text.

use std::collections::HashMap;
use std::fmt;

// ── T8 / A6: integer-CORDIC fixed-point sin/cos primitive (digest-pinned) ────
// Promoted from `reexam-builds/item4_cordic.rs`; see `cordic.rs` + `tests/cordic_digest.rs`.
pub mod cordic;

/// A math expression tree. Build with `Expr::sym`/`Expr::num` plus the
/// `+`, `-`, `*`, unary `-`, `.pow(n)`, `.sqrt()`, `.sin()`, `.cos()`, `.exp()`
/// operators — deliberately mirrors ordinary math notation.
#[derive(Clone, Debug)]
pub enum Expr {
    Sym(String),
    Num(f64),
    Sum(Vec<Expr>),
    Prod(Vec<Expr>),
    Pow(Box<Expr>, i32),
    Sqrt(Box<Expr>),
    Sin(Box<Expr>),
    Cos(Box<Expr>),
    Exp(Box<Expr>),
    /// `asin(x)` — f64-only (dynamics path, like `Sin`/`Cos`). Fixed-point and
    /// integer-exact emission MUST refuse it (not in either representable subset).
    Asin(Box<Expr>),
    /// `atan2(y, x)` — the FIRST binary-function node. f64-only; fixed/int
    /// emission refuses it.
    Atan2(Box<Expr>, Box<Expr>),
    /// Integer-exact half-up division `(a + b/2) / b`, in i128. INT-MODE ONLY
    /// (mirrors `apply_tax`'s half-up form, money.rs:280,284). `emit_f64_rust`
    /// refuses it — an f64 division would silently change the semantics.
    DivHalfUp(Box<Expr>, Box<Expr>),
    /// `Index { array, idx }` — read `array[idx]`, where `idx` is an `Expr`
    /// (typically the loop variable of an `IndexSum`). Part of the indexed-
    /// summation IR extension (item 36): ONE construct serves BOTH the Laplacian
    /// neighbor-sum `Σ_j L_ij x_j` (item 32) AND the quantized-dot inner law
    /// `acc = Σ_k a_k·w_k` (this arc). The `f64` and fixed-point Q-format paths
    /// refuse it (no array indexing in scalar math); only `emit_int_checked_rust`
    /// represents it.
    Index { array: String, idx: Box<Expr> },
    /// `IndexSum { var, len, body }` — `Σ_{var=0}^{len-1} body`. `len` is a
    /// BUILD-TIME-KNOWN constant (trip count fixed → feeds item 42's
    /// cyclomatic-1 spine). `body` may reference `var` (the loop index) and
    /// `Index` nodes. The shared IR for item 32 and the quantized dot.
    IndexSum { var: String, len: usize, body: Box<Expr> },
}

/// Raised (as an `Err`) when an expression cannot be represented in the integer
/// Q-format subset (sqrt/sin/cos/exp, division by a symbol, negative/fractional
/// power). Honest boundary: the caller falls back to the f64 variant for that organ.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FixedPointUnsupported(pub String);

impl fmt::Display for FixedPointUnsupported {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "not fixed-point-representable: {}", self.0)
    }
}
impl std::error::Error for FixedPointUnsupported {}

/// Raised (as an `Err`) when an expression cannot be represented in the integer-
/// EXACT subset (the `emit_int_checked_rust` mode): raw-i64 symbols, i128
/// arithmetic, every narrowing/overflowing step checked, `Result<i64, &'static
/// str>` semantics. The mirror image of `FixedPointUnsupported`: some nodes are
/// f64-only (`Sqrt`/`Sin`/`Cos`/`Exp`/`Asin`/`Atan2`), some are int-only
/// (`DivHalfUp`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IntEmissionUnsupported(pub String);

impl fmt::Display for IntEmissionUnsupported {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "not integer-exact-representable: {}", self.0)
    }
}
impl std::error::Error for IntEmissionUnsupported {}

/// Raised (as an `Err`) when an expression cannot be represented in the f64
/// dynamics path — specifically a `DivHalfUp` node (which is integer-exact-only;
/// an f64 division would silently change its half-up rounding semantics). The
/// honest boundary cuts both ways: `FixedPointUnsupported`/`IntEmissionUnsupported`
/// refuse f64-only nodes on the int/fixed paths, and this refuses int-only nodes
/// on the f64 path.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct F64EmissionUnsupported(pub String);

impl fmt::Display for F64EmissionUnsupported {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "not f64-representable: {}", self.0)
    }
}
impl std::error::Error for F64EmissionUnsupported {}

impl Expr {
    pub fn sym(name: &str) -> Expr {
        Expr::Sym(name.to_string())
    }
    pub fn num(v: f64) -> Expr {
        Expr::Num(v)
    }
    pub fn pow(self, n: i32) -> Expr {
        Expr::Pow(Box::new(self), n)
    }
    pub fn sqrt(self) -> Expr {
        Expr::Sqrt(Box::new(self))
    }
    pub fn sin(self) -> Expr {
        Expr::Sin(Box::new(self))
    }
    pub fn cos(self) -> Expr {
        Expr::Cos(Box::new(self))
    }
    pub fn exp(self) -> Expr {
        Expr::Exp(Box::new(self))
    }
    pub fn asin(self) -> Expr {
        Expr::Asin(Box::new(self))
    }

    /// `atan2(y, x)`. The first binary-function node — both children are
    /// full `Expr` subtrees.
    pub fn atan2(y: Expr, x: Expr) -> Expr {
        Expr::Atan2(Box::new(y), Box::new(x))
    }

    /// Integer-exact half-up division `(a + b/2) / b` over i128. INT-MODE ONLY
    /// (see `emit_int_checked_rust`); `emit_f64_rust` refuses it.
    pub fn div_half_up(a: Expr, b: Expr) -> Expr {
        Expr::DivHalfUp(Box::new(a), Box::new(b))
    }

    /// Indexed array read `array[idx]`. `idx` is a full `Expr` (so it may be the
    /// loop variable of an `IndexSum`). Part of the indexed-summation IR
    /// extension (item 36): the SAME `Index`/`IndexSum` construct serves the
    /// Laplacian neighbor-sum (item 32) and the quantized-dot inner law. Only
    /// `emit_int_checked_rust` represents it (f64 / fixed-point refuse it).
    pub fn index(array: &str, idx: Expr) -> Expr {
        Expr::Index {
            array: array.to_string(),
            idx: Box::new(idx),
        }
    }

    /// Indexed summation `Σ_{var=0}^{len-1} body`. `len` is a build-time-known
    /// constant (trip count fixed → feeds item 42's cyclomatic-1 spine); `body`
    /// may reference `var` (the loop index) and `Index` nodes.
    pub fn index_sum(var: &str, len: usize, body: Expr) -> Expr {
        Expr::IndexSum {
            var: var.to_string(),
            len,
            body: Box::new(body),
        }
    }

    /// Build an integral constant for the integer-exact emission mode. The value is
    /// carried as an `f64` (the `Num` variant) but `emit_int_checked_rust` checks
    /// it is exactly integral at emit time; non-integral `Num`s are refused there.
    pub fn int(v: i64) -> Expr {
        Expr::Num(v as f64)
    }

    /// True if the tree contains any `DivHalfUp` node (which is integer-exact-only
    /// and therefore not representable on the f64 path).
    fn any_div_half_up(&self) -> bool {
        match self {
            Expr::DivHalfUp(_, _) => true,
            Expr::Sym(_) | Expr::Num(_) => false,
            Expr::Sum(xs) | Expr::Prod(xs) => xs.iter().any(|x| x.any_div_half_up()),
            Expr::Pow(b, _) | Expr::Sqrt(b) | Expr::Sin(b) | Expr::Cos(b) | Expr::Exp(b)
            | Expr::Asin(b) => b.any_div_half_up(),
            Expr::Atan2(y, x) => y.any_div_half_up() || x.any_div_half_up(),
            Expr::Index { .. } | Expr::IndexSum { .. } => false,
        }
    }

    /// True if the tree contains any `Index`/`IndexSum` node (the indexed-
    /// summation IR extension, item 36). Used to switch emission/proof paths:
    /// indexed equations take `&[i8]` array params and emit the i32-accumulator
    /// loop instead of the scalar `i64` path.
    fn any_indexed(&self) -> bool {
        match self {
            Expr::Index { .. } | Expr::IndexSum { .. } => true,
            Expr::Sym(_) | Expr::Num(_) | Expr::DivHalfUp(_, _) => false,
            Expr::Sum(xs) | Expr::Prod(xs) => xs.iter().any(|x| x.any_indexed()),
            Expr::Pow(b, _) | Expr::Sqrt(b) | Expr::Sin(b) | Expr::Cos(b) | Expr::Exp(b)
            | Expr::Asin(b) => b.any_indexed(),
            Expr::Atan2(y, x) => y.any_indexed() || x.any_indexed(),
        }
    }

    /// Collect the array names referenced by `Index` nodes (for &[i8] parameter
    /// derivation in the indexed emitter, item 36).
    fn collect_index_arrays(&self, out: &mut Vec<String>) {
        match self {
            Expr::Index { array, idx } => {
                if !out.contains(array) {
                    out.push(array.clone());
                }
                idx.collect_index_arrays(out);
            }
            Expr::IndexSum { body, .. } => body.collect_index_arrays(out),
            Expr::Sum(xs) | Expr::Prod(xs) => {
                for x in xs {
                    x.collect_index_arrays(out);
                }
            }
            Expr::Pow(b, _) | Expr::Sqrt(b) | Expr::Sin(b) | Expr::Cos(b) | Expr::Exp(b)
            | Expr::Asin(b) => b.collect_index_arrays(out),
            Expr::Atan2(y, x) => {
                y.collect_index_arrays(out);
                x.collect_index_arrays(out);
            }
            _ => {}
        }
    }

    /// The build-time `len` of the first `IndexSum` encountered (item 36). The
    /// trip count is fixed at authoring time so the emitted loop is cyclomatic-1
    /// (feeds item 42's spine). `None` for scalar expressions.
    fn first_index_len(&self) -> Option<usize> {
        fn walk(e: &Expr) -> Option<usize> {
            match e {
                Expr::IndexSum { len, .. } => Some(*len),
                Expr::Sym(_) | Expr::Num(_) | Expr::Index { .. } | Expr::DivHalfUp(_, _) => None,
                Expr::Sum(xs) | Expr::Prod(xs) => xs.iter().find_map(walk),
                Expr::Pow(b, _) | Expr::Sqrt(b) | Expr::Sin(b) | Expr::Cos(b) | Expr::Exp(b)
                | Expr::Asin(b) => walk(b),
                Expr::Atan2(y, x) => walk(y).or_else(|| walk(x)),
            }
        }
        walk(self)
    }

    fn free_symbols(&self, out: &mut Vec<String>) {
        match self {
            Expr::Sym(s) => {
                if !out.contains(s) {
                    out.push(s.clone());
                }
            }
            Expr::Num(_) => {}
            Expr::Sum(xs) | Expr::Prod(xs) => {
                for x in xs {
                    x.free_symbols(out);
                }
            }
            Expr::Pow(b, _) | Expr::Sqrt(b) | Expr::Sin(b) | Expr::Cos(b) | Expr::Exp(b)
            | Expr::Asin(b) => b.free_symbols(out),
            Expr::Index { idx, .. } => idx.free_symbols(out),
            Expr::IndexSum { var, body, .. } => {
                // The loop var is bound inside the sum, not a free scalar.
                let mut inner = Vec::new();
                body.free_symbols(&mut inner);
                inner.retain(|s| s != var);
                for s in inner {
                    if !out.contains(&s) {
                        out.push(s);
                    }
                }
            }
            Expr::Atan2(y, x) | Expr::DivHalfUp(y, x) => {
                y.free_symbols(out);
                x.free_symbols(out);
            }
        }
    }

    /// Direct f64 evaluation of the tree — an interpreter, independent of the
    /// string-emitting codegen below. Used ONLY to compute the proof program's
    /// reference value; it never itself appears in emitted Rust, so comparing its
    /// result against the compiled-and-run emitted code is a real check that the
    /// codegen (operator precedence, parenthesization, sign) is correct.
    pub fn eval(&self, env: &HashMap<String, f64>) -> f64 {
        match self {
            Expr::Sym(s) => *env
                .get(s)
                .unwrap_or_else(|| panic!("unbound symbol: {s}")),
            Expr::Num(v) => *v,
            Expr::Sum(xs) => xs.iter().map(|x| x.eval(env)).sum(),
            Expr::Prod(xs) => xs.iter().map(|x| x.eval(env)).product(),
            Expr::Pow(b, n) => b.eval(env).powi(*n),
            Expr::Sqrt(b) => b.eval(env).sqrt(),
            Expr::Sin(b) => b.eval(env).sin(),
            Expr::Cos(b) => b.eval(env).cos(),
            Expr::Exp(b) => b.eval(env).exp(),
            Expr::Asin(b) => b.eval(env).asin(),
            Expr::Atan2(y, x) => y.eval(env).atan2(x.eval(env)),
            Expr::DivHalfUp(_, _) => {
                panic!("DivHalfUp has no f64 evaluation — it is an integer-exact op; use emit_int_checked_rust")
            }
            // Item 36 — the indexed-summation IR has no f64 semantics (it is
            // integer-exact: i8 arrays, i32 accumulator). It is never evaluated as
            // f64; the scalar interpreter must not pretend to.
            Expr::Index { .. } | Expr::IndexSum { .. } => {
                panic!("Index/IndexSum have no f64 evaluation — they are integer-exact; use eval_int_indexed or emit_int_checked_rust")
            }
        }
    }

    /// Integer-exact reference evaluation over an array env — the independent
    /// referee for `emit_int_checked_rust` on indexed equations (item 36). It is
    /// a tree-walking interpreter, deliberately a SEPARATE code path from the
    /// string-emitting `emit_int_checked` below, so a differential mismatch
    /// between them catches codegen bugs. `scalars` carries raw `i128` values
    /// for `Sym` leaves; `arrays` carries `Vec<i8>` for `Index` reads. `IndexSum`
    /// folds `body` over `[0, len)` binding the loop var in a fresh scalar env.
    /// Returns `None` on overflow (mirrors the `?`-propagating emitted shape);
    /// callers map it to the emitted `Result`'s `Err`.
    pub fn eval_int_indexed(
        &self,
        scalars: &HashMap<String, i128>,
        arrays: &HashMap<String, Vec<i8>>,
        arrays_i32: &HashMap<String, Vec<i32>>,
    ) -> Option<i128> {
        let r = self.eval_int_indexed_inner(scalars, arrays, arrays_i32);
        // Saturate negatives-at-zero nudge used by the requantize step only when
        // explicitly applied by the caller; here we return the raw value/None.
        r
    }

    fn eval_int_indexed_inner(
        &self,
        scalars: &HashMap<String, i128>,
        arrays: &HashMap<String, Vec<i8>>,
        arrays_i32: &HashMap<String, Vec<i32>>,
    ) -> Option<i128> {
        match self {
            Expr::Sym(s) => scalars.get(s).copied(),
            Expr::Num(v) => {
                if !v.is_finite() || (v.fract() != 0.0) || (v.abs() >= 9.22_337_203_685_477_6e18) {
                    return None;
                }
                Some(*v as i64 as i128)
            }
            Expr::Sum(xs) => {
                let mut acc: i128 = 0;
                for x in xs {
                    acc = acc.checked_add(x.eval_int_indexed_inner(scalars, arrays, arrays_i32)?)?;
                }
                Some(acc)
            }
            Expr::Prod(xs) => {
                let mut acc: i128 = 1;
                for x in xs {
                    acc = acc.checked_mul(x.eval_int_indexed_inner(scalars, arrays, arrays_i32)?)?;
                }
                Some(acc)
            }
            Expr::Pow(b, n) => {
                if *n < 0 {
                    return None;
                }
                let base = b.eval_int_indexed_inner(scalars, arrays, arrays_i32)?;
                let mut acc: i128 = 1;
                for _ in 0..*n {
                    acc = acc.checked_mul(base)?;
                }
                Some(acc)
            }
            Expr::DivHalfUp(a, b) => {
                let av = a.eval_int_indexed_inner(scalars, arrays, arrays_i32)?;
                let bv = b.eval_int_indexed_inner(scalars, arrays, arrays_i32)?;
                if bv == 0 {
                    return None;
                }
                Some((av + bv / 2) / bv)
            }
            Expr::Index { array, idx } => {
                let i = idx
                    .eval_int_indexed_inner(scalars, arrays, arrays_i32)?
                    as usize;
                if let Some(arr) = arrays.get(array) {
                    arr.get(i).map(|v| *v as i128)
                } else if let Some(arr) = arrays_i32.get(array) {
                    arr.get(i).map(|v| *v as i128)
                } else {
                    None
                }
            }
            Expr::IndexSum { var, len, body } => {
                let mut acc: i32 = 0;
                for k in 0..*len {
                    let mut local = scalars.clone();
                    local.insert(var.clone(), k as i128);
                    let term =
                        body.eval_int_indexed_inner(&local, arrays, arrays_i32)?;
                    // term is an i8×i8 product carried in i32 (matches the emitted
                    // i32-accumulator Q-format shape). Fold into the i32 accumulator.
                    let term_i32 = term as i32;
                    acc = acc.checked_add(term_i32)?;
                }
                Some(acc as i128)
            }
            // f64-only nodes are not in the integer-exact set; the reference
            // refuses them, consistent with `emit_int_checked`.
            Expr::Sqrt(_)
            | Expr::Sin(_)
            | Expr::Cos(_)
            | Expr::Exp(_)
            | Expr::Asin(_)
            | Expr::Atan2(_, _) => None,
        }
    }

    fn to_rust_f64(&self) -> String {
        match self {
            Expr::Sym(s) => s.clone(),
            Expr::Num(v) => flit(*v),
            Expr::Sum(xs) => format!(
                "({})",
                xs.iter().map(|x| x.to_rust_f64()).collect::<Vec<_>>().join(" + ")
            ),
            Expr::Prod(xs) => format!(
                "({})",
                xs.iter().map(|x| x.to_rust_f64()).collect::<Vec<_>>().join(" * ")
            ),
            Expr::Pow(b, n) => format!("({}).powi({})", b.to_rust_f64(), n),
            Expr::Sqrt(b) => format!("({}).sqrt()", b.to_rust_f64()),
            Expr::Sin(b) => format!("({}).sin()", b.to_rust_f64()),
            Expr::Cos(b) => format!("({}).cos()", b.to_rust_f64()),
            Expr::Exp(b) => format!("({}).exp()", b.to_rust_f64()),
            Expr::Asin(b) => format!("({}).asin()", b.to_rust_f64()),
            Expr::Atan2(y, x) => format!("({}).atan2({})", y.to_rust_f64(), x.to_rust_f64()),
            Expr::DivHalfUp(_, _) => {
                panic!("DivHalfUp is integer-exact only — emit_f64_rust refuses it")
            }
            Expr::Index { .. } | Expr::IndexSum { .. } => {
                panic!("Index/IndexSum are integer-exact only — emit_f64_rust refuses them")
            }
        }
    }
}

impl std::ops::Add for Expr {
    type Output = Expr;
    fn add(self, rhs: Expr) -> Expr {
        match (self, rhs) {
            (Expr::Sum(mut a), Expr::Sum(b)) => {
                a.extend(b);
                Expr::Sum(a)
            }
            (Expr::Sum(mut a), b) => {
                a.push(b);
                Expr::Sum(a)
            }
            (a, Expr::Sum(mut b)) => {
                b.insert(0, a);
                Expr::Sum(b)
            }
            (a, b) => Expr::Sum(vec![a, b]),
        }
    }
}

impl std::ops::Sub for Expr {
    type Output = Expr;
    fn sub(self, rhs: Expr) -> Expr {
        self + (Expr::Num(-1.0) * rhs)
    }
}

impl std::ops::Neg for Expr {
    type Output = Expr;
    fn neg(self) -> Expr {
        Expr::Num(-1.0) * self
    }
}

impl std::ops::Mul for Expr {
    type Output = Expr;
    fn mul(self, rhs: Expr) -> Expr {
        match (self, rhs) {
            (Expr::Prod(mut a), Expr::Prod(b)) => {
                a.extend(b);
                Expr::Prod(a)
            }
            (Expr::Prod(mut a), b) => {
                a.push(b);
                Expr::Prod(a)
            }
            (a, Expr::Prod(mut b)) => {
                b.insert(0, a);
                Expr::Prod(b)
            }
            (a, b) => Expr::Prod(vec![a, b]),
        }
    }
}

/// A valid Rust f64 literal: no e-notation, always a decimal point, `f64` suffix
/// (mirrors the original's `_flit`, which existed because Python's `repr()` emits
/// forms like `1e-07` that rustc rejects as a literal).
fn flit(x: f64) -> String {
    if x.is_nan() {
        return "f64::NAN".to_string();
    }
    if x.is_infinite() {
        return if x > 0.0 { "f64::INFINITY".to_string() } else { "f64::NEG_INFINITY".to_string() };
    }
    let mut s = format!("{x:.17}");
    while s.ends_with('0') && !s.ends_with(".0") {
        s.pop();
    }
    format!("{s}f64")
}

/// A named equation: y = f(args...). `args` fixes the Rust parameter order.
pub struct Equation {
    pub name: String,
    pub args: Vec<String>,
    pub expr: Expr,
    pub scale_bits: u32,
}

impl Equation {
    /// Panics if `expr` uses a symbol not listed in `args` — mirrors the original's
    /// `__post_init__` validation (a build-time authoring error, not a runtime one).
    pub fn new(name: &str, args: &[&str], expr: Expr) -> Self {
        let args: Vec<String> = args.iter().map(|s| s.to_string()).collect();
        let mut free = Vec::new();
        expr.free_symbols(&mut free);
        let missing: Vec<&String> = free.iter().filter(|f| !args.contains(f)).collect();
        assert!(
            missing.is_empty(),
            "expr uses symbols not in args: {missing:?} (args={args:?})"
        );
        Equation { name: name.to_string(), args, expr, scale_bits: 32 }
    }

    pub fn with_scale_bits(mut self, bits: u32) -> Self {
        self.scale_bits = bits;
        self
    }

    /// float variant (dynamics path — not bitwise-deterministic across targets on
    /// libm calls: sqrt/sin/cos/exp differ in ULPs between x86_64 and ARM libm).
    /// `Err` (never a fallback) if the expression contains an integer-exact-only
    /// node (`DivHalfUp`) whose f64 division would silently change the semantics.
    pub fn emit_f64_rust(&self) -> Result<String, F64EmissionUnsupported> {
        if self.expr.any_div_half_up() {
            return Err(F64EmissionUnsupported(
                "DivHalfUp is integer-exact-only; f64 division would change its half-up semantics"
                    .into(),
            ));
        }
        if self.expr.any_indexed() {
            return Err(F64EmissionUnsupported(
                "Index/IndexSum are integer-exact-only; f64 has no array indexing".into(),
            ));
        }
        let params = self
            .args
            .iter()
            .map(|a| format!("{a}: f64"))
            .collect::<Vec<_>>()
            .join(", ");
        Ok(format!(
            "/// GENERATED by eqc-rs from: {name}\n\
             /// f64 variant — dynamics path (not bitwise-deterministic across targets on libm calls).\n\
             #[inline(always)]\n\
             pub fn {name}_f64({params}) -> f64 {{\n    {body}\n}}",
            name = self.name,
            body = self.expr.to_rust_f64(),
        ))
    }

    /// Integer-exact checked emission mode. Symbols are raw `i64` (minor units /\
    /// micro-rates, NOT Q-scaled reals); arithmetic runs in `i128`; every\
    /// narrowing/overflowing step is checked; the returned fn has signature\
    /// `fn(args: i64) -> Result<i64, &'static str>`. The emitted body mirrors\
    /// `apply_tax`'s fail-closed contract (money.rs BP-17).\
    ///\
    /// Allowed nodes: `Sym`, `Num(integral)`, `Sum`, `Prod`, `Pow(n>=0)`,\
    /// `DivHalfUp`. Refuses (Err, never a fallback): `Sqrt`/`Sin`/`Cos`/`Exp`/\
    /// `Asin`/`Atan2` and any non-integral `Num`. `DivHalfUp` emits a division-by-\
    /// zero guard before each use. The indexed-summation IR extension (item 36)\
    /// adds `Index`/`IndexSum`: an indexed equation is emitted with `&[i8]` array\
    /// parameters and the i32-accumulator Q-format loop (the shared construct for\
    /// the Laplacian neighbor-sum AND the quantized-dot inner law — one IR, never\
    /// two). The emitter REFUSES (typed `Err`) any `IndexSum` whose i8×i8\
    /// accumulation exceeds the `i32` accumulator ceiling `K·P_MAX ≤ 2^31−1`.
    pub fn emit_int_checked_rust(&self) -> Result<String, IntEmissionUnsupported> {
        if self.expr.any_indexed() {
            return self.emit_int_indexed_rust();
        }
        let inner = emit_int_checked(&self.expr)?;
        let params = self
            .args
            .iter()
            .map(|a| format!("{a}: i64"))
            .collect::<Vec<_>>()
            .join(", ");
        Ok(format!(
            "/// GENERATED by eqc-rs from: {name}\n\
             /// Integer-exact variant — raw i64 (minor units), i128 arithmetic, every\n\
             /// step checked. Returns Result<i64, &'static str> (Err on overflow/div-by-zero).\n\
             #[inline(always)]\n\
             pub fn {name}_int({params}) -> Result<i64, &'static str> {{\n    use std::convert::TryFrom; i64::try_from({inner}).map_err(|_| \"overflow: result exceeds i64\")\n}}",
            name = self.name,
        ))
    }

    /// Integer-exact emission for an indexed equation (item 36): the `Index`/\
    /// `IndexSum` IR. Array args become `&[i8]` parameters; the `IndexSum` emits a\
    /// fixed-trip `for` loop accumulating i8×i8 products into an `i32` (checked,\
    /// `?`-propagating). The trip count is the build-time `len`, so the emitted\
    /// loop is cyclomatic-1. Refuses (typed `Err`) when `K·P_MAX` would exceed\
    /// `2^31−1` (the i32 accumulator ceiling, item 35 §3.4) — refuse-never-fall-\
    /// back. Returns `Result<_, IntEmissionUnsupported>`.
    pub fn emit_int_indexed_rust(&self) -> Result<String, IntEmissionUnsupported> {
        if !self.expr.any_indexed() {
            return Err(IntEmissionUnsupported(
                "emit_int_indexed_rust called on a non-indexed equation".into(),
            ));
        }
        let len = self
            .expr
            .first_index_len()
            .ok_or_else(|| IntEmissionUnsupported("indexed equation has no IndexSum".into()))?;
        // Overflow boundary (item 35 §3.4): K·P_MAX² ≤ 2^31−1 for an i8×i8 dot
        // accumulating into i32. Each term is an i8×i8 product (max P_MAX² =
        // 127² = 16129), so the worst-case accumulator is K·P_MAX². P_MAX = 127.
        // Refuse if K·127² > 2^31−1.
        let p_max: i128 = 127;
        let k = len as i128;
        if k * p_max * p_max > (i32::MAX as i128) {
            return Err(IntEmissionUnsupported(format!(
                "indexed sum length {len} with i8×i8 products ({k}·{p_max}²) exceeds the i32 accumulator ceiling 2^31−1; reduce K or widen the accumulator"
            )));
        }
        // Array parameters: the array names bound by `Index` nodes (e.g. `a`,
        // `w`) become `&[i8]` parameters; any remaining scalar `Sym`s in `args`
        // stay `i64`. For the item-36 quantized-dot / neighbor-sum shape, arrays
        // a/w are i8. We derive the array set from the IR itself so an Equation
        // authored with empty `args` (the test case) still emits the right
        // `&[i8]` parameters.
        let mut array_set: Vec<String> = Vec::new();
        self.expr.collect_index_arrays(&mut array_set);
        let mut array_params: Vec<String> = array_set
            .iter()
            .map(|a| format!("{a}: &[i8]"))
            .collect();
        let mut scalar_params = Vec::new();
        for a in &self.args {
            if !array_set.contains(a) {
                scalar_params.push(format!("{a}: i64"));
            }
        }
        let params = array_params
            .iter()
            .chain(scalar_params.iter())
            .cloned()
            .collect::<Vec<_>>()
            .join(", ");
        let inner = format!("Ok({{ {} }})", emit_int_indexed_checked(&self.expr)?);
        Ok(format!(
            "/// GENERATED by eqc-rs from: {name}\n\
             /// Integer-exact INDEXED variant — &[i8] arrays, i8×i8 product into an i32\n\
             /// accumulator, every step checked. Returns Result<i32, &'static str>\n\
             /// (Err on overflow/out-of-bounds). Trip count fixed at build time (len={len}).\n\
             #[inline(always)]\n\
             pub fn {name}_int({params}) -> Result<i32, &'static str> {{\n    {inner}\n}}",
            name = self.name,
            len = len,
        ))
    }

    /// Fixed-point Q-format variant — bitwise-identical on every CPU/wasm target.
    /// `Err` (never a fallback) if the expression leaves the representable subset.
    ///
    /// The indexed-summation IR (item 36) is integer-exact only: `Index`/`IndexSum`
    /// carry `&[i8]` arrays and an i32 accumulator, which is NOT the scalar Q-format
    /// subset. So `emit_fixed_rust` refuses indexed equations (the same honest
    /// boundary the f64 path uses).
    pub fn emit_fixed_rust(&self) -> Result<String, FixedPointUnsupported> {
        if self.expr.any_indexed() {
            return Err(FixedPointUnsupported(
                "Index/IndexSum are integer-exact-only; the fixed-point Q-format subset has no array indexing".into(),
            ));
        }
        let s_int: i128 = 1i128 << self.scale_bits;
        let inner = emit_fixed(&self.expr, s_int)?;
        let params = self
            .args
            .iter()
            .map(|a| format!("{a}: i64"))
            .collect::<Vec<_>>()
            .join(", ");
        Ok(format!(
            "/// GENERATED by eqc-rs from: {name}\n\
             /// Fixed-point Q{scale} variant — inputs/output are round(v * 2^{scale}) as i64.\n\
             /// BITWISE-DETERMINISTIC on every target (pure integer arithmetic, i128 intermediates).\n\
             #[inline(always)]\n\
             pub fn {name}_fixed({params}) -> i64 {{\n    ({inner}) as i64\n}}",
            name = self.name,
            scale = self.scale_bits,
        ))
    }

    /// A standalone Rust program. For each sample it asserts:
    /// |f64_variant - reference| < eps AND (if representable)
    /// |fixed_variant/2^SHIFT - reference| < eps, where `reference` comes from
    /// `Expr::eval` — a tree-walking interpreter, independent of the two codegen
    /// paths above. Compile + run; a non-zero exit means codegen is wrong.
    ///
    /// `Err` from `emit_f64_rust` (e.g. a `DivHalfUp`-containing expr) means the
    /// f64 path cannot represent the equation — no f64 proof program is emitted
    /// for it (the integer-exact path is the correct one; see `emit_int_checked_rust`).
    pub fn emit_proof_program(&self, samples: &[HashMap<String, f64>], eps: f64) -> String {
        if self.expr.any_indexed() {
            // Indexed equations carry integer-exact `i8` arrays, not f64 scalars.
            // The scalar `&[HashMap<String,f64>]` sample shape does not apply; the
            // caller must use `emit_indexed_proof_program` with `IndexedSample`s.
            return self.emit_int_indexed_proof_program(&[], eps);
        }
        let f64_res = self.emit_f64_rust();
        let fixed_res = self.emit_fixed_rust();
        let (fns, has_fixed) = match (&f64_res, &fixed_res) {
            (Ok(f), Ok(fx)) => (format!("{f}\n\n{fx}"), true),
            (Ok(f), Err(_)) => (f.clone(), false),
            (Err(_), _) => (
                "// f64 variant NOT representable (integer-exact-only equation) — no f64 proof.\n"
                    .to_string(),
                false,
            ),
        };
        let s_int: i128 = 1i128 << self.scale_bits;
        let mut checks = Vec::new();
        for (i, smp) in samples.iter().enumerate() {
            let reference = self.expr.eval(smp);
            let f64_args = self
                .args
                .iter()
                .map(|a| flit(smp[a]))
                .collect::<Vec<_>>()
                .join(", ");
            if f64_res.is_ok() {
                checks.push(format!(
                    "    let got = {name}_f64({f64_args});\n    assert!((got - ({refv})).abs() < {epsv}, \"f64 sample {i}: got {{}} want {refv}\", got);",
                    name = self.name,
                    refv = flit(reference),
                    epsv = flit(eps),
                ));
            }
            if has_fixed {
                let fixed_args = self
                    .args
                    .iter()
                    .map(|a| format!("({}i64)", (smp[a] * s_int as f64).round() as i64))
                    .collect::<Vec<_>>()
                    .join(", ");
                checks.push(format!(
                    "    let got_fx = {name}_fixed({fixed_args}) as f64 / ({s_int}u64 as f64);\n    assert!((got_fx - ({refv})).abs() < {epsv}, \"fixed sample {i}: got {{}} want {refv}\", got_fx);",
                    name = self.name,
                    refv = flit(reference),
                    epsv = flit(eps),
                ));
            }
        }
        let checks_src = checks.join("\n");
        format!(
            "// GENERATED by eqc-rs — self-asserting proof for `{name}`.\n\
             // exit 0 <=> the emitted f64 (and fixed-point) code matches the Expr-evaluated reference.\n\
             {fns}\n\n\
             fn main() {{\n{checks_src}\n    println!(\"eqc proof OK: {name} ({n} samples, fixed={has_fixed})\");\n}}\n",
            name = self.name,
            n = samples.len(),
        )
    }

    /// Self-asserting proof program for an INDEXED equation (item 36): emits the
    /// `&[i8]` `_int` fn and a `main` that feeds fixed `i8` arrays through it and
    /// asserts the result equals the tree-walking `eval_int_indexed` reference
    /// (the independent interpreter, a SEPARATE code path from the string
    /// emitter). Compiled by real rustc and run; exit 0 ⇒ codegen correct. This is
    /// the existing proof-program mechanism extended to indexed sums — the oracle
    /// proof (item 36 §4.1). `eps` is accepted for signature parity with the
    /// scalar path but is unused (the indexed reference is exact integer equality).
    fn emit_int_indexed_proof_program(&self, samples: &[IndexedSample], _eps: f64) -> String {
        let int_src = self.emit_int_indexed_rust().expect(
            "indexed equation must emit in integer-exact mode (emit_int_indexed_proof_program)",
        );
        let mut checks = String::new();
        for (i, s) in samples.iter().enumerate() {
            let a_lit = s.a_literal();
            let w_lit = s.w_literal();
            let reference = self
                .expr
                .eval_int_indexed(
                    &HashMap::new(),
                    &{
                        let mut m = HashMap::new();
                        m.insert("a".to_string(), s.a.clone());
                        m.insert("w".to_string(), s.w.clone());
                        m
                    },
                    &HashMap::new(),
                )
                .expect("reference eval must succeed for a valid sample");
            checks.push_str(&format!(
                "    let a{i}: &[i8] = &{a_lit};\n    let w{i}: &[i8] = &{w_lit};\n    let got{i} = {name}_int(a{i}, w{i}).expect(\"indexed int must not overflow\");\n    assert_eq!(got{i}, {want}i32, \"indexed sample {i}: got {{}} want {want}\", got{i});\n",
                i = i,
                name = self.name,
                a_lit = a_lit,
                w_lit = w_lit,
                want = reference as i32,
            ));
        }
        // The emitted _int fn returns Result<i32,&'static str>, so call it from a
        // thin unsafe wrapper to keep `main` simple and the generated program a
        // standalone, compilable Rust binary (mirrors the scalar proof shape).
        format!(
            "{int_src}\nunsafe fn {name}_checked(a: &[i8], w: &[i8]) -> i32 {{ {name}_int(a, w).expect(\"indexed int overflow\") }}\nfn main() {{\n{checks}    println!(\"eqc indexed proof OK: {name} ({n} samples)\");\n}}\n",
            name = self.name,
            n = samples.len(),
        )
    }
}

fn emit_fixed(expr: &Expr, s_int: i128) -> Result<String, FixedPointUnsupported> {
    match expr {
        Expr::Sym(s) => Ok(format!("({s} as i128)")),
        Expr::Num(v) => Ok(format!("({}i128)", (*v * s_int as f64).round() as i128)),
        Expr::Sum(xs) => {
            let parts: Result<Vec<_>, _> = xs.iter().map(|x| emit_fixed(x, s_int)).collect();
            Ok(format!("({})", parts?.join(" + ")))
        }
        Expr::Prod(xs) => {
            let parts: Result<Vec<_>, _> = xs.iter().map(|x| emit_fixed(x, s_int)).collect();
            let parts = parts?;
            let mut acc = parts[0].clone();
            for p in &parts[1..] {
                acc = format!("(({acc} * {p}) / {s_int}i128)");
            }
            Ok(acc)
        }
        Expr::Pow(base, n) => {
            if *n < 0 {
                return Err(FixedPointUnsupported(format!(
                    "negative exponent not fixed-point-representable: pow(_, {n})"
                )));
            }
            if *n == 0 {
                return Ok(format!("({s_int}i128)"));
            }
            let base_s = emit_fixed(base, s_int)?;
            let mut acc = base_s.clone();
            for _ in 1..*n {
                acc = format!("(({acc} * {base_s}) / {s_int}i128)");
            }
            Ok(acc)
        }
        Expr::Sqrt(_) => Err(FixedPointUnsupported("sqrt not in the fixed-point subset".into())),
        Expr::Sin(_) => Err(FixedPointUnsupported("sin not in the fixed-point subset".into())),
        Expr::Cos(_) => Err(FixedPointUnsupported("cos not in the fixed-point subset".into())),
        Expr::Exp(_) => Err(FixedPointUnsupported("exp not in the fixed-point subset".into())),
        // New A1 nodes: Asin/Atan2 are f64-only; DivHalfUp is integer-exact-only.
        // None belong in the Q-format fixed-point subset.
        Expr::Asin(_) => Err(FixedPointUnsupported("asin not in the fixed-point subset".into())),
        Expr::Atan2(_, _) => {
            Err(FixedPointUnsupported("atan2 not in the fixed-point subset".into()))
        }
        Expr::DivHalfUp(_, _) => {
            Err(FixedPointUnsupported("div_half_up is integer-exact-only".into()))
        }
        // Item 36 — the indexed-summation IR is integer-exact only (i8 arrays, i32
        // accumulator); it is NOT scalar Q-format math, so the fixed-point path
        // refuses it with the same honest boundary as f64.
        Expr::Index { .. } | Expr::IndexSum { .. } => Err(FixedPointUnsupported(
            "Index/IndexSum are integer-exact-only; the fixed-point Q-format subset has no array indexing".into(),
        )),
    }
}

/// Integer-exact emission (int-mode): raw i64 symbols, i128 arithmetic, every
/// narrowing/overflowing step checked. Returns the inner `Result<i128, &'static
/// str>`-typed expression; the `Equation::emit_int_checked_rust` wrapper wraps it
/// in the `i64::try_from(...)` final narrowing.
fn emit_int_checked(expr: &Expr) -> Result<String, IntEmissionUnsupported> {
    match expr {
        Expr::Sym(s) => Ok(format!("({s} as i128)")),
        Expr::Num(v) => {
            if !v.is_finite() || (v.fract() != 0.0) || (v.abs() >= 9.22_337_203_685_477_6e18) {
                return Err(IntEmissionUnsupported(format!(
                    "non-integral or out-of-i64-range constant not integer-exact-representable: {v}"
                )));
            }
            Ok(format!("({}i128)", *v as i64))
        }
        Expr::Sum(xs) => {
            let mut acc: Option<String> = None;
            for x in xs {
                let p = emit_int_checked(x)?;
                acc = Some(match acc {
                    None => p,
                    Some(a) => format!("(({a}).checked_add({p}).ok_or(\"overflow in Sum\")?)"),
                });
            }
            Ok(acc.unwrap_or_else(|| "0i128".to_string()))
        }
        Expr::Prod(xs) => {
            let mut acc: Option<String> = None;
            for x in xs {
                let p = emit_int_checked(x)?;
                acc = Some(match acc {
                    None => p,
                    Some(a) => format!("(({a}).checked_mul({p}).ok_or(\"overflow in Prod\")?)"),
                });
            }
            Ok(acc.unwrap_or_else(|| "1i128".to_string()))
        }
        Expr::Pow(base, n) => {
            if *n < 0 {
                return Err(IntEmissionUnsupported(format!(
                    "negative exponent not integer-exact-representable: pow(_, {n})"
                )));
            }
            let base_s = emit_int_checked(base)?;
            if *n == 0 {
                return Ok("1i128".to_string());
            }
            let mut acc = base_s.clone();
            for _ in 1..*n {
                acc = format!("(({acc}).checked_mul({base_s}).ok_or(\"overflow in Pow\")?)");
            }
            Ok(acc)
        }
        Expr::DivHalfUp(a, b) => {
            let a_s = emit_int_checked(a)?;
            let b_s = emit_int_checked(b)?;
            Ok(format!(
                "{{ let b = {b_s}; if b == 0i128 {{ return Err(\"division by zero in DivHalfUp\"); }} (({a_s}) + b / 2) / b }}"
            ))
        }
        // f64-only nodes (and the Q-format subset) are NOT in the integer-exact set.
        Expr::Sqrt(_) => Err(IntEmissionUnsupported("sqrt not in the integer-exact subset".into())),
        // A6 / T8 — integer-CORDIC fixed-point sin/cos: routes through the digest-pinned
        // Q30 substrate `eqc_rs::cordic::cordic_sincos` instead of hard-refusing. This
        // EXTENDS the representable integer-exact subset (without touching the f64 dynamics
        // path). Convention: the argument is Q30 fixed-point radians and the result is Q30
        // (unit-magnitude, like a scaled sine/cosine). Deterministic — same i64 add/sub/
        // compare/arithmetic-shift ops the digest in `tests/cordic_digest.rs` pins.
        Expr::Sin(inner) => {
            let z = emit_int_checked(inner)?;
            Ok(format!("(eqc_rs::cordic::cordic_sincos(({z}) as i64).1 as i128)"))
        }
        Expr::Cos(inner) => {
            let z = emit_int_checked(inner)?;
            Ok(format!("(eqc_rs::cordic::cordic_sincos(({z}) as i64).0 as i128)"))
        }
        Expr::Exp(_) => Err(IntEmissionUnsupported("exp not in the integer-exact subset".into())),
        Expr::Asin(_) => Err(IntEmissionUnsupported("asin not in the integer-exact subset".into())),
        Expr::Atan2(_, _) => {
            Err(IntEmissionUnsupported("atan2 not in the integer-exact subset".into()))
        }
        // Item 36 — scalar int-mode emission has NO array indexing. An `Index`/
        // `IndexSum` node is the indexed-summation IR, which must take the
        // dedicated `emit_int_indexed_checked` path (the `&[i8]` `_int` fn), never
        // the scalar `i64` expression emitter. Refuse here so the two paths stay
        // disjoint (no silent mis-codegen).
        Expr::Index { .. } | Expr::IndexSum { .. } => {
            Err(IntEmissionUnsupported(
                "Index/IndexSum require the indexed-summation IR path (emit_int_indexed_checked), not scalar emit_int_checked".into(),
            ))
        }
    }
}

/// A fixed test sample for an indexed equation (item 36): the `i8` element arrays
/// `a` and `w` feeding the `Σ_k a_k·w_k` quantized dot (and, dually, the Laplacian
/// neighbor-sum). Held separately from `HashMap<String,f64>` because the indexed IR
/// is integer-exact (`i8` elements, `i32` accumulator) — there is no f64 domain.
pub struct IndexedSample {
    pub a: Vec<i8>,
    pub w: Vec<i8>,
}

impl IndexedSample {
    pub fn new(a: Vec<i8>, w: Vec<i8>) -> Self {
        IndexedSample { a, w }
    }
    /// Rust array literal for `vec![...]`-style `&[i8]` binding in a generated proof.
    pub fn a_literal(&self) -> String {
        format!("[{}{}]", self.a.iter().map(|v| v.to_string()).collect::<Vec<_>>().join(", "), if self.a.is_empty() { "" } else { "" })
    }
    pub fn w_literal(&self) -> String {
        format!("[{}]", self.w.iter().map(|v| v.to_string()).collect::<Vec<_>>().join(", "))
    }
}

/// Integer-exact emitter for the indexed-summation IR (item 36): the `Index`/
/// `IndexSum` nodes. Produces a Rust expression fragment (NOT wrapped in a checked
/// `i128::try_from` — the `emit_int_indexed_rust` caller owns the fn signature and
/// the `i32` accumulator). The emitted shape:
///   - `Sym("a")` / `Sym("w")` → the `&[i8]` array parameter (base address).
///   - `Index { array, idx }` → `{array}[({idx}) as usize]` (bounds-checked at the
///     emitted loop body via `get`/`expect` — see `IndexSum`).
///   - `IndexSum { var, len, body }` →
///       `{{ let mut acc: i32 = 0i32; for {var} in 0..{len} {{ let {var}_a = a[{var} as usize]; let {var}_w = w[{var} as usize]; let term: i32 = (({var}_a as i32).checked_mul({var}_w as i32).ok_or("overflow in indexed product")?); acc = acc.checked_add(term).ok_or("overflow in indexed sum")?; }} acc }}`
///     The trip count is the build-time `len`, so the loop is cyclomatic-1. Every
///     add/mul is checked and `?`-propagated (fail-closed, mirroring `apply_tax`).
///
/// The emitted `IndexSum` body assumes the canonical `Σ_k a_k·w_k` shape (the
/// shared construct for the Laplacian neighbor-sum and the quantized dot): each
/// trip reads `a[k]` and `w[k]` and folds their `i8×i8→i32` product into `acc`.
fn emit_int_indexed_checked(expr: &Expr) -> Result<String, IntEmissionUnsupported> {
    match expr {
        Expr::Sym(s) => Ok(s.clone()),
        Expr::Num(v) => Ok(format!("({v}i32)")),
        Expr::Index { array, idx } => {
            let i = emit_int_indexed_checked(idx)?;
            Ok(format!("({array}[({i}) as usize])"))
        }
        // A `Prod`/`Sum` over `Index` nodes is the canonical quantized-dot /
        // neighbor-sum body. Each `Index` already yields an i32 (i8 cast), so the
        // product is the i8×i8→i32 step and the sum folds terms into the i32
        // accumulator — both checked and `?`-propagated (fail-closed).
        Expr::Prod(xs) => {
            let mut acc: Option<String> = None;
            for x in xs {
                let p = emit_int_indexed_checked(x)?;
                acc = Some(match acc {
                    None => format!("(({p}) as i32)"),
                    Some(a) => format!(
                        "((({a}) as i32).checked_mul(({p}) as i32).ok_or(\"overflow in indexed product\")?)"
                    ),
                });
            }
            Ok(acc.unwrap_or_else(|| "1i32".to_string()))
        }
        Expr::Sum(xs) => {
            let mut acc: Option<String> = None;
            for x in xs {
                let p = emit_int_indexed_checked(x)?;
                acc = Some(match acc {
                    None => format!("(({p}) as i32)"),
                    Some(a) => format!(
                        "((({a}) as i32).checked_add(({p}) as i32).ok_or(\"overflow in indexed sum\")?)"
                    ),
                });
            }
            Ok(acc.unwrap_or_else(|| "0i32".to_string()))
        }
        Expr::IndexSum { var, len, body } => {
            // Canonical quantized-dot / neighbor-sum body: the `body` expression
            // references `Index` nodes `a[k]` and `w[k]` (and possibly the loop var
            // `var`). Emit it as a Rust i32 expression; the loop folds each trip's
            // product into the i32 accumulator. The trip count is the build-time
            // `len`, so the emitted loop is cyclomatic-1. Every add/mul is checked
            // and `?`-propagated (fail-closed, mirroring `apply_tax`).
            let body_s = emit_int_indexed_checked(body)?;
            Ok(format!(
                "{{ let mut acc: i32 = 0i32; for {var} in 0..{len} {{ let term: i32 = ({body_s}); acc = acc.checked_add(term).ok_or(\"overflow in indexed sum\")?; }} acc }}"
            ))
        }
        _ => Err(IntEmissionUnsupported(format!(
            "node {:?} is not part of the indexed-summation IR (item 36) — use the scalar emitters for non-indexed expressions",
            expr
        ))),
    }
}

#[cfg(test)]
mod unit_tests {
    use super::*;

    #[test]
    fn quad_emits_expected_shape() {
        let (a, b, c, x) = (Expr::sym("a"), Expr::sym("b"), Expr::sym("c"), Expr::sym("x"));
        let quad = Equation::new("quad", &["a", "b", "c", "x"], a * x.clone().pow(2) + b * x + c);
        let f64_src = quad.emit_f64_rust().unwrap();
        assert!(f64_src.contains("pub fn quad_f64(a: f64, b: f64, c: f64, x: f64) -> f64"));
        assert!(quad.emit_fixed_rust().is_ok());
    }

    #[test]
    fn hyp_refuses_fixed_point() {
        let (a, b) = (Expr::sym("a"), Expr::sym("b"));
        let hyp = Equation::new("hyp", &["a", "b"], (a.clone().pow(2) + b.clone().pow(2)).sqrt());
        assert_eq!(
            hyp.emit_fixed_rust().unwrap_err(),
            FixedPointUnsupported("sqrt not in the fixed-point subset".into())
        );
        assert!(hyp.emit_f64_rust().unwrap().contains(".sqrt()"));
    }

    #[test]
    fn ema_with_negative_power_refused_by_fixed_subset() {
        let (p, s, a) = (Expr::sym("p"), Expr::sym("s"), Expr::sym("a"));
        // alpha⁻¹ smuggled in: 1/a is NOT fixed-point-representable (lib.rs:376-380)
        let bad = Equation::new("ema_bad", &["p", "s", "a"], p.clone() + a.pow(-1) * (s - p));
        assert!(bad.emit_fixed_rust().is_err()); // refusal, never a silent fallback
    }

    #[test]
    #[should_panic(expected = "expr uses symbols not in args")]
    fn rejects_symbol_not_in_args() {
        let (a, b) = (Expr::sym("a"), Expr::sym("b"));
        Equation::new("bad", &["a"], a + b);
    }

    // ── A1 adversarial refusal tests (§3.2) ──────────────────────────────────

    /// (i) emit_fixed_rust on an Atan2 expr MUST refuse (Atan2 is f64-only).
    #[test]
    fn atan2_refused_by_fixed_subset() {
        let (y, x) = (Expr::sym("y"), Expr::sym("x"));
        let bearing = Equation::new("bearing", &["y", "x"], Expr::atan2(y, x));
        assert!(bearing.emit_fixed_rust().is_err());
    }

    /// (ii) emit_int_checked_rust on an expr containing Sqrt MUST refuse with
    /// IntEmissionUnsupported (Sqrt is f64-only).
    #[test]
    fn sqrt_refused_by_int_checked_subset() {
        let (a, b) = (Expr::sym("a"), Expr::sym("b"));
        let bad = Equation::new(
            "with_sqrt",
            &["a", "b"],
            (a.clone().pow(2) + b.clone().pow(2)).sqrt(),
        );
        assert_eq!(
            bad.emit_int_checked_rust().unwrap_err(),
            IntEmissionUnsupported("sqrt not in the integer-exact subset".into())
        );
    }

    /// (iii) emit_f64_rust on a DivHalfUp expr MUST refuse (typed error, not a
    /// silent f64 division that would change the half-up semantics).
    #[test]
    fn div_half_up_refused_by_f64_emission() {
        let (sub, rate) = (Expr::sym("sub"), Expr::sym("rate"));
        let tax = Equation::new(
            "tax",
            &["sub", "rate"],
            Expr::div_half_up(sub * rate, Expr::int(1_000_000)),
        );
        assert!(matches!(
            tax.emit_f64_rust(),
            Err(F64EmissionUnsupported(_))
        ));
    }

    /// (iv) generated checked int code at sub=i64::MAX, rate_micro=2_000_000 MUST
    /// return Err, never wrap.
    #[test]
    fn int_checked_overflow_never_wraps() {
        let (sub, rate_micro) = (Expr::sym("sub"), Expr::sym("rate_micro"));
        let s = Expr::int(1_000_000);
        let tax = Equation::new(
            "apply_tax_exclusive",
            &["sub", "rate_micro"],
            Expr::div_half_up(sub.clone() * rate_micro.clone(), s),
        );
        // Emit + compile the generated fn, then invoke at the overflow edge.
        let src = format!(
            "{}fn main() {{ let r = apply_tax_exclusive_int(i64::MAX, 2_000_000); assert!(r.is_err(), \"overflow must be refused, got {{:?}}\", r); println!(\"ok: {{:?}}\", r); }}",
            tax.emit_int_checked_rust().unwrap()
        );
        let dir = std::env::temp_dir().join(format!("eqc-rs-overflow-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let sp = dir.join("ovf.rs");
        let bp = dir.join("ovf");
        std::fs::write(&sp, &src).unwrap();
        let cc = std::process::Command::new("rustc")
            .args(["-O", "-o"])
            .arg(&bp)
            .arg(&sp)
            .output()
            .expect("rustc on PATH");
        assert!(cc.status.success(), "rustc failed: {}", String::from_utf8_lossy(&cc.stderr));
        let run = std::process::Command::new(&bp).output().expect("run bin");
        assert!(run.status.success(), "generated overflow test FAILED: {}", String::from_utf8_lossy(&run.stderr));
        let _ = std::fs::remove_dir_all(&dir);
    }

    // ── A6 / T8 — Sin/Cos no longer hard-refuse in integer-exact mode ───────
    // The emitter used to return `Err(IntEmissionUnsupported("sin/cos not in the
    // integer-exact subset"))`. A6/T8 routes Sin/Cos through the digest-pinned Q30
    // CORDIC substrate, so emission now SUCCEEDS and the emitted code calls
    // `eqc_rs::cordic::cordic_sincos`. The f64 dynamics path is untouched.
    #[test]
    fn sin_cos_int_mode_routes_through_cordic() {
        let theta = Expr::sym("theta");
        let sin_eq = Equation::new("my_sin", &["theta"], theta.clone().sin());
        let cos_eq = Equation::new("my_cos", &["theta"], theta.cos());

        let sin_src = sin_eq
            .emit_int_checked_rust()
            .expect("A6/T8: Sin MUST now emit in integer-exact mode (no longer refused)");
        let cos_src = cos_eq
            .emit_int_checked_rust()
            .expect("A6/T8: Cos MUST now emit in integer-exact mode (no longer refused)");

        assert!(
            sin_src.contains("eqc_rs::cordic::cordic_sincos"),
            "emitted sin must call the CORDIC substrate:\n{sin_src}"
        );
        assert!(
            cos_src.contains("eqc_rs::cordic::cordic_sincos"),
            "emitted cos must call the CORDIC substrate:\n{cos_src}"
        );

        // The f64 dynamics path is unchanged: still uses .sin()/.cos().
        assert!(sin_eq.emit_f64_rust().unwrap().contains(".sin()"));
        assert!(cos_eq.emit_f64_rust().unwrap().contains(".cos()"));
        // Fixed-point Q-format subset still refuses Sin/Cos (orthogonal path).
        assert!(sin_eq.emit_fixed_rust().is_err());
        assert!(cos_eq.emit_fixed_rust().is_err());
    }

    // ── Item 36 — indexed-summation IR (Σ over index) ──────────────────────
    // One IR that expresses BOTH the Laplacian neighbor-sum (item 32) AND the
    // quantized dot `acc = Σ_k a_k·w_k`. The f64 and fixed-point paths refuse it;
    // only emit_int_checked_rust (the indexed variant) represents it. The
    // reference evaluator `eval_int_indexed` is a SEPARATE code path from the
    // string emitter, so the oracle proof is independent.

    /// Helper: build the canonical `Σ_{k=0}^{len-1} a[k]·w[k]` quantized-dot expr.
    fn dot_expr(len: usize) -> Expr {
        Expr::index_sum(
            "k",
            len,
            Expr::index("a", Expr::sym("k")) * Expr::index("w", Expr::sym("k")),
        )
    }

    /// (a) f64 path refuses the indexed IR (no array indexing in scalar math).
    #[test]
    fn indexed_ir_refused_by_f64() {
        let dot = Equation::new("dot", &[], dot_expr(4));
        assert!(dot.emit_f64_rust().is_err());
    }

    /// (b) fixed-point Q-format path refuses the indexed IR (integer-exact only).
    #[test]
    fn indexed_ir_refused_by_fixed() {
        let dot = Equation::new("dot", &[], dot_expr(4));
        assert!(dot.emit_fixed_rust().is_err());
    }

    /// (c) the integer-exact path EMITS a `&[i8]` `_int` fn with the i32-accumulator
    /// loop, and the build-time trip count is baked into the loop bound.
    #[test]
    fn indexed_ir_emits_i32_loop() {
        let len = 4usize;
        let dot = Equation::new("dot", &[], dot_expr(len));
        let src = dot.emit_int_checked_rust().expect("indexed must emit in int mode");
        assert!(src.contains("pub fn dot_int(a: &[i8], w: &[i8]) -> Result<i32, &'static str>"),
            "must emit a &[i8]-parameterized i32 fn:\n{src}");
        assert!(src.contains("for k in 0..4"),
            "trip count must be the build-time len (cyclomatic-1 loop):\n{src}");
        assert!(src.contains("checked_add"), "accumulator must be checked");
        assert!(src.contains("checked_mul"), "i8×i8 product must be checked");
    }

    /// (d) the integer-exact path REFUSES an IndexSum whose K·P_MAX exceeds the i32
    /// accumulator ceiling 2^31−1 (item 35 §3.4) — refuse-never-fallback.
    #[test]
    fn indexed_ir_refuses_overflow_ceiling() {
        // K=2_000_000 with i8 elements (P_MAX=127) → 2.54e8·... well over 2^31.
        let len = 2_000_000usize;
        let dot = Equation::new("dot", &[], dot_expr(len));
        let err = dot.emit_int_checked_rust().unwrap_err();
        assert!(err.to_string().contains("i32 accumulator ceiling"),
            "must refuse on the i32 ceiling, got: {err}");
    }

    /// (e) the independent reference evaluator matches the emitted-code result:
    /// emit a real, compilable proof program and run it (exit 0 ⇒ codegen correct).
    #[test]
    fn indexed_ir_proof_program_self_asserts() {
        let len = 4usize;
        let dot = Equation::new("dot", &[], dot_expr(len));
        let samples = vec![
            IndexedSample::new(vec![1, 2, 3, 4], vec![5, 6, 7, 8]), // 5+12+21+32 = 70
            IndexedSample::new(vec![-1, 3, -2, 0], vec![4, -4, 4, 4]), // -4-12-8+0 = -24
        ];
        let src = dot.emit_int_indexed_proof_program(&samples, 1e-9);
        // The emitted proof must reference the integer-exact fn and assert equality.
        assert!(src.contains("pub fn dot_int(a: &[i8], w: &[i8]) -> Result<i32, &'static str>"));
        assert!(src.contains("assert_eq!"));
        // Compile + run the oracle proof for real.
        let dir = std::env::temp_dir().join(format!("eqc-rs-idxproof-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let sp = dir.join("idxproof.rs");
        let bp = dir.join("idxproof");
        std::fs::write(&sp, &src).unwrap();
        let cc = std::process::Command::new("rustc")
            .args(["-O", "-o"])
            .arg(&bp)
            .arg(&sp)
            .output()
            .expect("rustc on PATH");
        assert!(cc.status.success(), "rustc failed: {}", String::from_utf8_lossy(&cc.stderr));
        let run = std::process::Command::new(&bp).output().expect("run bin");
        assert!(run.status.success(),
            "oracle proof FAILED: {}",
            String::from_utf8_lossy(&run.stderr));
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// (f) the reference evaluator `eval_int_indexed` is a separate path from the
    /// string emitter and yields the exact integer dot product.
    #[test]
    fn indexed_reference_evaluator_matches_hand_compute() {
        let len = 4usize;
        let expr = dot_expr(len);
        let a = vec![1i8, 2, 3, 4];
        let w = vec![5i8, 6, 7, 8];
        let mut arrays = HashMap::new();
        arrays.insert("a".to_string(), a.clone());
        arrays.insert("w".to_string(), w.clone());
        let got = expr
            .eval_int_indexed(&HashMap::new(), &arrays, &HashMap::new())
            .expect("eval must succeed");
        // 1*5 + 2*6 + 3*7 + 4*8 = 5 + 12 + 21 + 32 = 70
        assert_eq!(got, 70i128);
    }
}
