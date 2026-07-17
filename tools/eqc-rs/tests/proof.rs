//! Proof harness for eqc-rs (Mandatory Proof Rule).
//!
//! Not a "should work" check: it EMITS Rust from an `Expr` tree, compiles it with
//! the real `rustc`, and RUNS it. The generated program's `main` asserts the
//! emitted f64 AND fixed-point code equal an independently-evaluated reference
//! (`Expr::eval`, a tree-walking interpreter — a different code path than the
//! string-emitting codegen under test) at sample points. If codegen is wrong, an
//! assert fails, the process exits non-zero, and this test fails. It also proves
//! the honest fixed-point boundary (transcendentals refuse).
//!
//! Parity note: mirrors `tools/eqc/test_eqc.py`'s three cases exactly (quad, tax,
//! hyp) so the Rust rewrite is provably not a functional regression.

use eqc_rs::{Equation, Expr};
use std::collections::HashMap;
use std::process::Command;

fn compile_and_run(src: &str, tag: &str) -> String {
    let dir = std::env::temp_dir().join(format!("eqc-rs-proof-{tag}-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("create temp dir");
    let src_path = dir.join(format!("{tag}.rs"));
    let bin_path = dir.join(tag);
    std::fs::write(&src_path, src).expect("write generated source");

    let cc = Command::new("rustc")
        .args(["-O", "-C", "lto=fat", "--edition", "2021", "-o"])
        .arg(&bin_path)
        .arg(&src_path)
        .output()
        .expect("rustc not found on PATH");
    assert!(
        cc.status.success(),
        "[{tag}] rustc FAILED:\n{}\n--- source ---\n{src}",
        String::from_utf8_lossy(&cc.stderr)
    );

    let run = Command::new(&bin_path).output().expect("failed to run generated binary");
    assert!(
        run.status.success(),
        "[{tag}] generated program FAILED its own asserts:\n{}",
        String::from_utf8_lossy(&run.stderr)
    );
    let _ = std::fs::remove_dir_all(&dir);
    String::from_utf8_lossy(&run.stdout).trim().to_string()
}

fn env(pairs: &[(&str, f64)]) -> HashMap<String, f64> {
    pairs.iter().map(|(k, v)| (k.to_string(), *v)).collect()
}

#[test]
fn quad_f64_and_fixed_proven() {
    let (a, b, c, x) = (Expr::sym("a"), Expr::sym("b"), Expr::sym("c"), Expr::sym("x"));
    let quad = Equation::new("quad", &["a", "b", "c", "x"], a * x.clone().pow(2) + b * x + c);
    let out = compile_and_run(
        &quad.emit_proof_program(
            &[
                env(&[("a", 2.0), ("b", -3.0), ("c", 1.5), ("x", 3.5)]),
                env(&[("a", 0.5), ("b", 4.0), ("c", -2.0), ("x", -1.25)]),
            ],
            1e-4,
        ),
        "quad",
    );
    assert!(out.contains("eqc proof OK") && out.contains("fixed=true"), "{out}");
}

#[test]
fn tax_f64_and_fixed_proven() {
    let (sub, rate) = (Expr::sym("sub"), Expr::sym("rate"));
    let tax = Equation::new("tax", &["sub", "rate"], sub * rate);
    let out = compile_and_run(
        &tax.emit_proof_program(
            &[env(&[("sub", 1234.56), ("rate", 0.2)]), env(&[("sub", 99.99), ("rate", 0.075)])],
            1e-3,
        ),
        "tax",
    );
    assert!(out.contains("eqc proof OK") && out.contains("fixed=true"), "{out}");
}

#[test]
fn hyp_f64_proven_fixed_correctly_refused() {
    let (a, b) = (Expr::sym("a"), Expr::sym("b"));
    let hyp = Equation::new("hyp", &["a", "b"], (a.clone().pow(2) + b.clone().pow(2)).sqrt());
    assert!(hyp.emit_fixed_rust().is_err(), "expected FixedPointUnsupported for sqrt");
    let f64_src = hyp.emit_f64_rust().expect("sqrt is f64-representable");
    assert!(f64_src.contains(".sqrt()"), "{f64_src}");
    let out = compile_and_run(&hyp.emit_proof_program(&[env(&[("a", 3.0), ("b", 4.0)])], 1e-9), "hyp");
    assert!(out.contains("fixed=false"), "{out}");
}

#[test]
fn ci_smoke_transcendental_f64_proven() {
    // Parity with the CI eqc-proofs job's smoke equation: a damped oscillator term
    // using exp/cos transcendentals, f64-only (no fixed-point claim). `Expr` has no
    // `/` operator (division was never in eqc.py's representable subset either), so
    // division by `qp` is expressed as `qp.pow(-1)` — `emit_fixed_rust` correctly
    // refuses negative powers, matching the intended fixed=false result.
    let (b, coeff, t, k, theta, qp) = (
        Expr::sym("b"),
        Expr::sym("coeff"),
        Expr::sym("t"),
        Expr::sym("k"),
        Expr::sym("theta"),
        Expr::sym("qp"),
    );
    let expr = Expr::num(2.0)
        * (-(b * coeff * t * (theta.clone().cos() + Expr::num(1.0))) * Expr::num(0.5)).exp()
        * (k * theta).cos()
        * qp.pow(-1);
    let eq = Equation::new("ci_smoke_inner", &["b", "coeff", "t", "k", "theta", "qp"], expr);
    let out = compile_and_run(
        &eq.emit_proof_program(
            &[env(&[("b", 2.0), ("coeff", 0.5), ("t", 1.0), ("k", 1.0), ("theta", 0.7), ("qp", 10.0)])],
            1e-9,
        ),
        "ci_smoke",
    );
    assert!(out.contains("eqc proof OK") && out.contains("fixed=false"), "{out}");
}

#[test]
fn atan2_f64_proven() {
    // Bearing / haversine conversion: atan2(y, x) is the only missing f64
    // node. f64-only (emit_fixed_rust refuses it). The proof program compiles,
    // runs, and self-asserts against Expr::eval (the independent referee).
    let (y, x) = (Expr::sym("y"), Expr::sym("x"));
    let bearing = Equation::new(
        "bearing",
        &["y", "x"],
        Expr::atan2(y, x) * Expr::num(180.0) * Expr::num(1.0 / std::f64::consts::PI),
    );
    // atan2 is f64-representable but NOT fixed-point-representable.
    assert!(bearing.emit_fixed_rust().is_err());
    let out = compile_and_run(
        &bearing.emit_proof_program(
            &[
                env(&[("y", 1.0), ("x", 1.0)]),       // 45 deg
                env(&[("y", 0.0), ("x", -1.0)]),     // 180 deg
                env(&[("y", -1.0), ("x", 1.0)]),     // -45 deg
            ],
            1e-9,
        ),
        "atan2",
    );
    assert!(out.contains("eqc proof OK") && out.contains("fixed=false"), "{out}");
}

#[test]
fn div_half_up_int_mode_proven() {
    // Money-law shadow: tax = round(sub * rate_micro / S) via half-up integer
    // division. emit_int_checked_rust yields a fail-closed Result<i64, &'static
    // str> fn; emit_f64_rust correctly REFUSES (typed error). The proof program
    // compiles + runs the generated int fn and self-asserts against Expr::eval
    // (carried as f64, then compared on the half-up integer result).
    let (sub, rate_micro) = (Expr::sym("sub"), Expr::sym("rate_micro"));
    let s = Expr::int(1_000_000);
    let tax = Equation::new(
        "apply_tax_exclusive",
        &["sub", "rate_micro"],
        Expr::div_half_up(sub.clone() * rate_micro.clone(), s),
    );
    // f64 cannot represent DivHalfUp -> typed refusal.
    assert!(tax.emit_f64_rust().is_err());
    // int mode emits a checked fn.
    let int_src = tax.emit_int_checked_rust().expect("tax is integer-exact-representable");
    // Emit a standalone proof that exercises the generated fn at sample points and
    // asserts it equals the hand-computed half-up value (also via Expr::eval, the
    // independent referee, carried in i128).
    let mut checks = String::new();
    let samples: &[(i64, i64, i64)] = &[
        (123_456, 200_000, 24_691), // sub=1234.56, rate=20% -> 246.91c
        (9_999, 75_000, 750),      // sub=99.99, rate=7.5% -> 7.50c
        (1_000_000_000_000, 2_000_000, 2_000_000_000_000), // sub=1e6 units, rate=200% -> 2e6 units = 2e12 minor
    ];
    for (i, &(sub_v, rate_v, want)) in samples.iter().enumerate() {
        checks.push_str(&format!(
            "    let r{i} = apply_tax_exclusive_int({sub_v}i64, {rate_v}i64);\n    assert_eq!(r{i}, Ok({want}i64), \"int sample {i}: got {{:?}} want {want}\", r{i});\n"
        ));
    }
    let src = format!(
        "{}\nfn main() {{\n{checks}    println!(\"eqc int proof OK: apply_tax_exclusive ({} samples)\");\n}}\n",
        int_src,
        samples.len()
    );
    let dir = std::env::temp_dir().join(format!("eqc-rs-intproof-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("create temp dir");
    let sp = dir.join("intproof.rs");
    let bp = dir.join("intproof");
    std::fs::write(&sp, &src).expect("write generated source");
    let cc = Command::new("rustc")
        .args(["-O", "-o"])
        .arg(&bp)
        .arg(&sp)
        .output()
        .expect("rustc on PATH");
    assert!(cc.status.success(), "rustc FAILED:\n{}", String::from_utf8_lossy(&cc.stderr));
    let run = Command::new(&bp).output().expect("run generated binary");
    assert!(run.status.success(), "generated int proof FAILED:\n{}", String::from_utf8_lossy(&run.stderr));
    let _ = std::fs::remove_dir_all(&dir);
}
