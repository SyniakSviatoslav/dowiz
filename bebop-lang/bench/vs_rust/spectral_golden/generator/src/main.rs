use dowiz_core::csr::Csr;
use dowiz_core::spectral;

fn fx32(v: f64) -> i64 {
    // .bp consumer format: i64 fixed-point, scale 2^32, saturating
    let s = v * 4294967296.0;
    if s >= 9.2233720368547758e18 { i64::MAX } else if s <= -9.2233720368547758e18 { i64::MIN } else { s as i64 }
}

fn csr_from(n: usize, edges: &[(usize, usize, f64)]) -> Csr {
    // symmetric: both directions
    let mut e: Vec<(usize, usize, f64)> = Vec::new();
    for &(s, d, w) in edges {
        e.push((s, d, w));
        e.push((d, s, w));
    }
    Csr::from_edges(n, &e)
}

fn dump(name: &str, csr: &Csr, k: usize, iters: usize) {
    let (vecs, vals) = spectral::topk_symmetric(csr, k, iters);
    println!("== {} n={} nnz={} k={} iters={}", name, csr.nrows(), csr.nnz(), k, iters);
    println!("vals_fp32: {}", vals.iter().map(|&v| fx32(v).to_string()).collect::<Vec<_>>().join(" "));
    println!("vals_bits: {}", vals.iter().map(|v| format!("{:016x}", v.to_bits())).collect::<Vec<_>>().join(" "));
    for (i, v) in vecs.iter().enumerate() {
        println!("vec{}_fp32: {}", i, v.iter().map(|&x| fx32(x).to_string()).collect::<Vec<_>>().join(" "));
        println!("vec{}_bits: {}", i, v.iter().map(|x| format!("{:016x}", x.to_bits())).collect::<Vec<_>>().join(" "));
    }
}

fn dump_eigh(name: &str, a: &[Vec<f64>]) {
    let (basis, vals) = spectral::eigh(a);
    println!("== eigh {} n={}", name, a.len());
    println!("evals_fp32: {}", vals.iter().map(|&v| fx32(v).to_string()).collect::<Vec<_>>().join(" "));
    println!("evals_bits: {}", vals.iter().map(|v| format!("{:016x}", v.to_bits())).collect::<Vec<_>>().join(" "));
    for (i, v) in basis.iter().enumerate() {
        println!("evec{}_fp32: {}", i, v.iter().map(|&x| fx32(x).to_string()).collect::<Vec<_>>().join(" "));
    }
}

fn dense(n: usize, edges: &[(usize, usize, f64)]) -> Vec<Vec<f64>> {
    let mut a = vec![vec![0.0f64; n]; n];
    for &(s, d, w) in edges {
        a[s][d] = w;
        a[d][s] = w;
    }
    a
}

fn main() {
    hdc_goldens();
    // P4 path graph
    let p4 = csr_from(4, &[(0, 1, 1.0), (1, 2, 1.0), (2, 3, 1.0)]);
    dump("P4_path", &p4, 2, 32);
    dump_eigh("P4_path", &dense(4, &[(0, 1, 1.0), (1, 2, 1.0), (2, 3, 1.0)]));

    // C3 triangle
    let c3 = csr_from(3, &[(0, 1, 1.0), (1, 2, 1.0), (2, 0, 1.0)]);
    dump("C3_triangle", &c3, 3, 32);
    dump_eigh("C3_triangle", &dense(3, &[(0, 1, 1.0), (1, 2, 1.0), (2, 0, 1.0)]));

    // S5 star
    let s5 = csr_from(5, &[(0, 1, 1.0), (0, 2, 1.0), (0, 3, 1.0), (0, 4, 1.0)]);
    dump("S5_star", &s5, 2, 32);

    // C4 cycle
    let c4 = csr_from(4, &[(0, 1, 1.0), (1, 2, 1.0), (2, 3, 1.0), (3, 0, 1.0)]);
    dump("C4_cycle", &c4, 2, 32);

    // weighted K4
    let k4 = csr_from(4, &[(0, 1, 2.0), (0, 2, 3.0), (0, 3, 1.0), (1, 2, 1.0), (1, 3, 4.0), (2, 3, 2.0)]);
    dump("K4_weighted", &k4, 3, 32);
    dump_eigh("K4_weighted", &dense(4, &[(0, 1, 2.0), (0, 2, 3.0), (0, 3, 1.0), (1, 2, 1.0), (1, 3, 4.0), (2, 3, 2.0)]));

    // 6-node two-cliques bridge (bipartition-relevant for Fiedler)
    let b6 = csr_from(6, &[(0, 1, 1.0), (0, 2, 1.0), (1, 2, 1.0), (3, 4, 1.0), (3, 5, 1.0), (4, 5, 1.0), (2, 3, 0.5)]);
    dump("B6_bridge", &b6, 3, 32);
    dump_eigh("B6_bridge", &dense(6, &[(0, 1, 1.0), (0, 2, 1.0), (1, 2, 1.0), (3, 4, 1.0), (3, 5, 1.0), (4, 5, 1.0), (2, 3, 0.5)]));

    // Householder-vs-Faddeev parity: charpoly/eigenvalues vs eigh on small symmetric
    for (name, a) in [
        ("SM2", vec![vec![2.0, 1.0], vec![1.0, 3.0]]),
        ("SM3", vec![vec![4.0, 1.0, 0.0], vec![1.0, 3.0, 1.0], vec![0.0, 1.0, 2.0]]),
    ] {
        let cp = spectral::charpoly(&a);
        println!("== charpoly {} coeffs_fp32: {}", name, cp.iter().map(|&v| fx32(v).to_string()).collect::<Vec<_>>().join(" "));
        dump_eigh(name, &a);
    }
}

fn hv_words_dump(name: &str, hv: &dowiz_core::hypervector::Hypervector) {
    let w = hv.as_words();
    println!("hv {}: {}", name, w.iter().map(|x| format!("{:016x}", x)).collect::<Vec<_>>().join(" "));
}

fn hdc_goldens() {
    use dowiz_core::hypervector::Hypervector as Hv;
    println!("════ HDC GOLDENS (D=1024, splitmix64 code) ════");
    // code(seed) for a fixed seed set
    for seed in [0u64, 1, 2, 42, 0xDEADBEEF, 1234567890123456789] {
        hv_words_dump(&format!("code({})", seed), &Hv::code(seed));
    }
    // bind: a ⊗ b and self-inverse check a⊗b⊗b == a
    let a = Hv::code(42);
    let b = Hv::code(0xDEADBEEF);
    let ab = a.bind(&b);
    hv_words_dump("bind(42,0xDEADBEEF)", &ab);
    hv_words_dump("bind_inv(42,0xDEADBEEF)", &ab.bind(&b));
    println!("bind_inv_ok: {}", ab.bind(&b) == a);
    // bundle: majority with ties->0 (a,b,c)
    let c = Hv::code(7);
    let bundled = Hv::bundle([&a, &b, &c]);
    hv_words_dump("bundle(42,0xDEADBEEF,7)", &bundled);
    println!("sim(a,bundled) bits: {:016x}", a.similarity(&bundled).to_bits());
    println!("sim(a,b) bits: {:016x}", a.similarity(&b).to_bits());
    // permute: rotation by 1, 64, 1023
    for sh in [1usize, 64, 255, 1023] {
        hv_words_dump(&format!("perm(42,{})", sh), &a.permute(sh));
    }
    println!("perm0_ok: {}", a.permute(0) == a);
    println!("perm1024_ok: {}", a.permute(1024) == a);
    println!("hamming(a,b): {}", a.hamming(&b));
    println!("popcount(code(42)): {}", a.popcount());
    // spectral-role pattern: splitmix-HV ⊗ spectral-role HV (Ф3 law) reference
    let role0 = Hv::code(0xA1);
    let item = Hv::code(11);
    hv_words_dump("role_A1", &role0);
    hv_words_dump("item11_xor_roleA1", &item.bind(&role0));
}
