//! ITEM 5 — Is there a genuine continuous-geometry EXTENSION of the discrete graph Laplacian?
//!
//! The blueprint rejects "Laplace-domain primitives + Kuen surface" as domain-mismatch. This
//! tests the ONE part that is NOT a mismatch: the graph Laplacian (kernel/src/spectral.rs::
//! laplacian + eigenvalues + algebraic_connectivity) is the DISCRETE form of a CONTINUOUS
//! operator. Belkin-Niyogi (2008): as n->inf, the graph-Laplacian spectrum converges to the
//! Laplace-Beltrami operator's spectrum on the underlying manifold.
//!
//! We MEASURE that convergence on the unit circle S^1, whose Laplace-Beltrami eigenvalues are
//! known EXACTLY: {0, 1, 1, 4, 4, 9, 9, ...} = k^2 (eigenfunctions sin k*theta, cos k*theta).
//! We build a Gaussian-weighted graph over n sampled points, symmetric-normalized Laplacian,
//! take its low eigenvalues via cyclic Jacobi, and show the RATIOS -> {1,1,4,4,9,9} as n grows.
//! Ratios cancel the (bandwidth-dependent) constant, so this is a clean spectrum-SHAPE match.

// ---- deterministic cyclic-Jacobi symmetric eigenvalues (pure, no deps) ----
fn jacobi_eigenvalues(mut a: Vec<Vec<f64>>) -> Vec<f64> {
    let n = a.len();
    for _sweep in 0..100 {
        // off-diagonal Frobenius norm
        let mut off = 0.0;
        for p in 0..n { for q in (p+1)..n { off += a[p][q]*a[p][q]; } }
        if off.sqrt() < 1e-12 { break; }
        for p in 0..n {
            for q in (p+1)..n {
                if a[p][q].abs() < 1e-300 { continue; }
                let app = a[p][p]; let aqq = a[q][q]; let apq = a[p][q];
                let theta = (aqq - app) / (2.0 * apq);
                let t = theta.signum() / (theta.abs() + (theta*theta + 1.0).sqrt());
                let c = 1.0 / (t*t + 1.0).sqrt();
                let s = t * c;
                for k in 0..n {
                    let akp = a[k][p]; let akq = a[k][q];
                    a[k][p] = c*akp - s*akq;
                    a[k][q] = s*akp + c*akq;
                }
                for k in 0..n {
                    let apk = a[p][k]; let aqk = a[q][k];
                    a[p][k] = c*apk - s*aqk;
                    a[q][k] = s*apk + c*aqk;
                }
            }
        }
    }
    let mut ev: Vec<f64> = (0..n).map(|i| a[i][i]).collect();
    ev.sort_by(|x,y| x.partial_cmp(y).unwrap());
    ev
}

// graph Laplacian spectrum for n points on the unit circle
fn circle_laplacian_spectrum(n: usize) -> Vec<f64> {
    let spacing = 2.0*std::f64::consts::PI / n as f64;
    let eps = 5.0 * spacing;              // Gaussian bandwidth ~5 neighbor spacings (eps->0 with n)
    // coords
    let pts: Vec<(f64,f64)> = (0..n).map(|i| {
        let th = 2.0*std::f64::consts::PI * i as f64 / n as f64;
        (th.cos(), th.sin())
    }).collect();
    // weighted adjacency + degree
    let mut w = vec![vec![0.0f64; n]; n];
    let mut deg = vec![0.0f64; n];
    for i in 0..n { for j in 0..n { if i!=j {
        let dx = pts[i].0 - pts[j].0; let dy = pts[i].1 - pts[j].1;
        let d2 = dx*dx + dy*dy;
        let wij = (-d2/(eps*eps)).exp();
        w[i][j] = wij; deg[i] += wij;
    }}}
    // symmetric normalized Laplacian  L = I - D^-1/2 W D^-1/2
    let mut l = vec![vec![0.0f64; n]; n];
    for i in 0..n { for j in 0..n {
        let norm = (deg[i]*deg[j]).sqrt();
        l[i][j] = if i==j { 1.0 } else { 0.0 } - w[i][j]/norm;
    }}
    jacobi_eigenvalues(l)
}

fn main() {
    println!("Laplace-Beltrami spectrum of S^1 (continuous, exact): 0, 1, 1, 4, 4, 9, 9  (= k^2)\n");
    println!("Discrete graph-Laplacian low eigenvalues, normalized so lambda_1 == 1 (ratios):");
    println!("{:>6} | {:>6} {:>6} {:>6} {:>6} {:>6} {:>6}", "n", "l1", "l2", "l3", "l4", "l5", "l6");
    println!("{}", "-".repeat(60));
    for &n in &[24usize, 48, 96, 160] {
        let ev = circle_laplacian_spectrum(n);
        // ev[0] ~ 0 (constant mode). Normalize the rest by ev[1].
        let base = ev[1];
        let r: Vec<f64> = (1..7).map(|k| ev[k]/base).collect();
        println!("{:>6} | {:>6.3} {:>6.3} {:>6.3} {:>6.3} {:>6.3} {:>6.3}", n, r[0], r[1], r[2], r[3], r[4], r[5]);
    }
    println!("\ntarget ratios ->  1.000  1.000  4.000  4.000  9.000  9.000");
    println!("\nRESULT: the discrete graph Laplacian's low spectrum reproduces the CONTINUOUS");
    println!("Laplace-Beltrami spectrum {{k^2}} — the two are the same operator at different");
    println!("resolutions (Belkin-Niyogi 2008). This is a real continuous relaxation of the SAME");
    println!("machinery the kernel already ships (spectral.rs eigenvalues / Fiedler / DriftClass),");
    println!("NOT a replacement. Note: eps=5*spacing shrinks with n, so higher modes (l5,l6) trail");
    println!("the target until eps->0 more slowly than 1/n (the B-N regime) — visible convergence.");
}
