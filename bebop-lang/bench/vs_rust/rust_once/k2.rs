// K2 twin of kernels/k2.bp: fib(25) once.
fn fib(n: i64) -> i64 { if n < 2 { n } else { fib(n - 1) + fib(n - 2) } }
fn main() { println!("{}", fib(std::hint::black_box(25))); }
