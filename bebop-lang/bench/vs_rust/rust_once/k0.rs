// K0: process-startup floor (prints 0, no work).
fn main() { println!("{}", std::hint::black_box(0)); }
