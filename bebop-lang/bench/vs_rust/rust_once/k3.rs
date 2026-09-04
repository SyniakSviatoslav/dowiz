// K3 twin of kernels/k3.bp: nested 300x300 once.
fn main() {
    let n: i64 = std::hint::black_box(300);
    let mut a: i64 = 0;
    let mut x: i64 = n;
    while x > 0 {
        let mut y: i64 = n;
        while y > 0 { a = std::hint::black_box(a + x * 2 + y * 3); y -= 1; }
        x -= 1;
    }
    println!("{}", a);
}
