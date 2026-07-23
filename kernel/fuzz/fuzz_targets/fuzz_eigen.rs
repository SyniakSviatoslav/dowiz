#![no_main]
use libfuzzer_sys::fuzz_target;
use dowiz_kernel::eigen::Eigen;

fuzz_target!(|data: &[u8]| {
    if data.len() < 16 { return; }
    let n = (data.len() / 8).min(32);
    let mut v = Vec::with_capacity(n);
    for i in 0..n {
        let bytes: [u8; 8] = data[i*8..(i+1)*8].try_into().unwrap();
        let val = f64::from_le_bytes(bytes);
        if val.is_finite() {
            v.push(val);
        }
    }
    if v.is_empty() { return; }
    let lambda = v.iter().map(|x| x * x).sum::<f64>().sqrt();
    let e = Eigen::new(lambda, v.clone());
    assert_eq!(e.lambda, lambda);
    assert_eq!(e.vector.len(), v.len());
    let sim = e.cosine_sim(&e);
    assert!(sim.is_finite(), "Self-cosine sim must be finite for Eigen");
});
