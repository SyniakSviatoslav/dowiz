//! Cross-module parity pins for the Householder eigensolver after its
//! extraction to no_std `dowiz-core`. The `householder` module previously
//! pinned its `eigenvalues_contig` spectrum against the legacy Faddeev-LeVerrier
//! path (`spectral::charpoly` + `spectral::roots`) inside its own test module.
//! Once `householder` moved into `dowiz-core` (which cannot depend on
//! `spectral`), those pins had to move here, where BOTH solvers are in scope:
//!
//!   `householder::eigenvalues_contig` (QR, complex) vs
//!   `spectral::roots(&spectral::charpoly(a))` (Faddeev, complex)
//!
//! must agree on every hand-oracle + asymmetric/mixed matrix, sorted by
//! (re, im). This is the same "householder ⇄ spectral" parity pin as before —
//! relocated, not weakened.

use dowiz_kernel::householder::eigenvalues_contig;
use dowiz_kernel::spectral::{charpoly, roots, Complex};

fn cclose(a: Complex, b: Complex, tol: f64) -> bool {
    (a.re - b.re).abs() < tol && (a.im - b.im).abs() < tol
}

/// Householder (QR) spectrum vs legacy Faddeev-LeVerrier, within `tol`.
fn parity(a: &[Vec<f64>], tol: f64) {
    let n = a.len();
    let mut buf = vec![0.0f64; n * n];
    for i in 0..n {
        for j in 0..n {
            buf[i * n + j] = a[i][j];
        }
    }
    let got = eigenvalues_contig(&mut buf, n);
    let want = roots(&charpoly(a));
    assert_eq!(got.len(), want.len(), "spectrum length mismatch");
    // sort both by (re, im) for comparison
    let mut g = got.clone();
    g.sort_by(|x, y| x.re.total_cmp(&y.re).then(x.im.total_cmp(&y.im)));
    let mut w = want.clone();
    w.sort_by(|x, y| x.re.total_cmp(&y.re).then(x.im.total_cmp(&y.im)));
    for (i, (x, y)) in g.iter().zip(w.iter()).enumerate() {
        assert!(
            cclose(*x, *y, tol),
            "eig[{i}] mismatch: householder {x:?} vs faddeev {y:?}"
        );
    }
}

#[test]
fn hand_two_cycle_is_plus_minus_one() {
    let c = vec![vec![0.0, 1.0], vec![1.0, 0.0]];
    parity(&c, 1e-9);
}

#[test]
fn hand_diagonal_known_spectrum() {
    let d = vec![
        vec![2.0, 0.0, 0.0],
        vec![0.0, 5.0, 0.0],
        vec![0.0, 0.0, -3.0],
    ];
    parity(&d, 1e-9);
}

#[test]
fn hand_path_p3_laplacian_spectrum() {
    // P₃ Laplacian = [[1,-1,0],[-1,2,-1],[0,-1,1]] → spectrum {0,1,3}.
    let l = vec![
        vec![1.0, -1.0, 0.0],
        vec![-1.0, 2.0, -1.0],
        vec![0.0, -1.0, 1.0],
    ];
    parity(&l, 1e-9);
}

#[test]
fn parity_general_3x3_asymmetric() {
    let a = vec![
        vec![1.0, 2.0, 3.0],
        vec![0.0, 4.0, 5.0],
        vec![0.0, 0.0, 6.0], // already upper-triangular, eigs 1,4,6
    ];
    parity(&a, 1e-9);
}

#[test]
fn parity_general_4x4_mixed() {
    let a = vec![
        vec![0.0, 1.0, 0.0, 0.0],
        vec![1.0, 0.0, 1.0, 0.0],
        vec![0.0, 1.0, 0.0, 1.0],
        vec![0.0, 0.0, 1.0, 0.0],
    ];
    parity(&a, 1e-9);
}
