use dowiz_kernel::householder;
use dowiz_kernel::spectral;
use std::time::Instant;

fn faddeev(a: &[Vec<f64>]) -> Vec<spectral::Complex> {
    let c = spectral::charpoly(a);
    spectral::roots(&c)
}
fn ns<F: FnMut()>(mut f: F, it: u32) -> f64 {
    let t = Instant::now();
    for _ in 0..it {
        f();
    }
    t.elapsed().as_nanos() as f64 / it as f64
}
fn main() {
    for n in [8usize, 16, 32] {
        let mut a = vec![vec![0.0; n]; n];
        let mut s = 12345u64;
        for i in 0..n {
            for j in 0..n {
                s = s.wrapping_mul(6364136223846793005).wrapping_add(1);
                a[i][j] = ((s >> 33) as f64 / 2147483648.0) - 1.0;
            }
        }
        let mut buf = vec![0.0; n * n];
        for i in 0..n {
            for j in 0..n {
                buf[i * n + j] = a[i][j];
            }
        }
        let hh = ns(
            || {
                let mut b = buf.clone();
                let _ = householder::eigenvalues_contig(&mut b, n);
            },
            200,
        );
        let fd = ns(|| {
            let _ = faddeev(&a);
        }, 200);
        println!(
            "n={:>3}  householder={:>10.1} ns   faddeev={:>10.1} ns   speedup={:.2}x",
            n,
            hh,
            fd,
            fd / hh
        );
    }
}
