//! Show what eqc-rs emits. Run: `cargo run --bin eqc-demo` (in tools/eqc-rs/).
//! Rust-native replacement for the retired `tools/eqc/demo.py`.

use eqc_rs::{Equation, Expr};

fn main() {
    println!("# -- tax = sub * rate  (money organ: the last f64 that touches money) --\n");
    let (sub, rate) = (Expr::sym("sub"), Expr::sym("rate"));
    let tax = Equation::new("tax", &["sub", "rate"], sub * rate);
    println!("{}\n", tax.emit_f64_rust());
    println!("{}\n", tax.emit_fixed_rust().expect("tax is fully fixed-point-representable"));

    println!("# -- hyp = sqrt(a^2 + b^2)  (dynamics: transcendental -> fixed-point refused) --\n");
    let (a, b) = (Expr::sym("a"), Expr::sym("b"));
    let hyp = Equation::new("hyp", &["a", "b"], (a.clone().pow(2) + b.clone().pow(2)).sqrt());
    println!("{}\n", hyp.emit_f64_rust());
    match hyp.emit_fixed_rust() {
        Ok(_) => unreachable!("sqrt must refuse fixed-point emission"),
        Err(e) => println!("// emit_fixed_rust() -> FixedPointUnsupported: {e}"),
    }
}
