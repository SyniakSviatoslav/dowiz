// K4 twin of kernels/k4.bp: wrapping arith chain, 2M iterations once.
fn main() {
    let mut v: i64 = std::hint::black_box(1);
    let mut i: i64 = std::hint::black_box(2_000_000);
    while i > 0 { v = std::hint::black_box(v.wrapping_add(i.wrapping_mul(7)).wrapping_mul(3).wrapping_sub(11)); i -= 1; }
    println!("{}", v);
}
