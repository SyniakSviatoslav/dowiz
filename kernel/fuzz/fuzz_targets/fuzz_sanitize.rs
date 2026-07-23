#![no_main]
use libfuzzer_sys::fuzz_target;
use dowiz_kernel::sanitize_f64;

fuzz_target!(|data: &[u8]| {
    if data.len() < 8 { return; }
    let bytes: [u8; 8] = data[..8].try_into().unwrap();
    let val = f64::from_le_bytes(bytes);
    let result = sanitize_f64(val);
    assert!(result.is_finite(), "sanitize_f64 must return finite value");
    if val.is_nan() {
        assert_eq!(result, 0.0);
    }
    if val == f64::INFINITY {
        assert!(result.is_finite());
    }
    if val == f64::NEG_INFINITY {
        assert!(result.is_finite());
    }
});
