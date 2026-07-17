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
            Expr::Pow(b, _) | Expr::Sqrt(b) | Expr::Sin(b) | Expr::Cos(b) | Expr::Exp(b) => {
                b.free_symbols(out)
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
    pub fn emit_f64_rust(&self) -> String {
        let params = self
            .args
            .iter()
            .map(|a| format!("{a}: f64"))
            .collect::<Vec<_>>()
            .join(", ");
        format!(
            "/// GENERATED by eqc-rs from: {name}\n\
             /// f64 variant — dynamics path (not bitwise-deterministic across targets on libm calls).\n\
             #[inline(always)]\n\
             pub fn {name}_f64({params}) -> f64 {{\n    {body}\n}}",
            name = self.name,
            body = self.expr.to_rust_f64(),
        )
    }

    /// Fixed-point Q-format variant — bitwise-identical on every CPU/wasm target.
    /// `Err` (never a fallback) if the expression leaves the representable subset.
    pub fn emit_fixed_rust(&self) -> Result<String, FixedPointUnsupported> {
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
    pub fn emit_proof_program(&self, samples: &[HashMap<String, f64>], eps: f64) -> String {
        let f64_src = self.emit_f64_rust();
        let fixed_res = self.emit_fixed_rust();
        let (fns, has_fixed) = match &fixed_res {
            Ok(fx) => (format!("{f64_src}\n\n{fx}"), true),
            Err(_) => (f64_src, false),
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
            checks.push(format!(
                "    let got = {name}_f64({f64_args});\n    assert!((got - ({refv})).abs() < {epsv}, \"f64 sample {i}: got {{}} want {refv}\", got);",
                name = self.name,
                refv = flit(reference),
                epsv = flit(eps),
            ));
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
    }
}

#[cfg(test)]
mod unit_tests {
    use super::*;

    #[test]
    fn quad_emits_expected_shape() {
        let (a, b, c, x) = (Expr::sym("a"), Expr::sym("b"), Expr::sym("c"), Expr::sym("x"));
        let quad = Equation::new("quad", &["a", "b", "c", "x"], a * x.clone().pow(2) + b * x + c);
        let f64_src = quad.emit_f64_rust();
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
        assert!(hyp.emit_f64_rust().contains(".sqrt()"));
    }

    #[test]
    #[should_panic(expected = "expr uses symbols not in args")]
    fn rejects_symbol_not_in_args() {
        let (a, b) = (Expr::sym("a"), Expr::sym("b"));
        Equation::new("bad", &["a"], a + b);
    }
}
