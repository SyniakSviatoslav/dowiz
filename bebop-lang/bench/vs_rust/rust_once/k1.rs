// K1 twin of kernels/k1.bp: run ONCE, print the fold (process wall-clock twin).
// black_box on the inputs keeps the loop alive under -O (else it folds to a constant).
fn main() {
    let mut s: i64 = std::hint::black_box(0);
    let mut i: i64 = std::hint::black_box(1_000_000);
    while i > 0 { s = std::hint::black_box(s + i); i -= 1; }
    println!("{}", s);
}
