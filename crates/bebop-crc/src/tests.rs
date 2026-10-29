//! P12: the hardware crc is the software crc (`sw.rs`), byte for byte. No fuzzer (no-fuzzers rule): every
//! length 0..4096 at every misalignment 0..16, over NAMED patterns.

use super::*;

/// The patterns, by name. Each is a pure function of the index.
fn pattern(name: &str, n: usize) -> Vec<u8> {
    (0..n)
        .map(|i| match name {
            "zeros" => 0u8,
            "ones" => 0xFF,
            "count" => i as u8,
            "affine" => (i.wrapping_mul(31).wrapping_add(7)) as u8,
            "square" => (i.wrapping_mul(i) >> 3) as u8,
            "sparse" => if i % 97 == 0 { 0x80 } else { 0 },
            _ => panic!("unnamed pattern {name}"),
        })
        .collect()
}

const PATTERNS: [&str; 6] = ["zeros", "ones", "count", "affine", "square", "sparse"];

#[test]
fn zlib_check_value_through_the_hardware_path() {
    // zlib's own check value, and a buffer long enough to take the folding path on x86_64.
    assert_eq!(!bytes(!0, b"123456789"), 0xCBF4_3926, "path {}", path());
    let long: Vec<u8> = b"123456789".iter().copied().cycle().take(9 * 64).collect();
    assert_eq!(!bytes(!0, &long), !step_bytes(!0, &long), "path {}", path());
    assert_eq!(!bytes(!0, b""), 0);
}

#[test]
fn bytes_equal_software_on_every_length_and_offset() {
    for name in PATTERNS {
        let buf = pattern(name, 4096 + 16);
        // Every length 0..=4096 at offsets 0 and 5; every offset 0..16 and three seeds up to 300
        // bytes (where the folding set-up, its 16-byte loop and the table tail all meet).
        for off in 0..16 {
            let top = if off == 0 || off == 5 { 4096 } else { 300 };
            for len in 0..=top {
                let b = &buf[off..off + len];
                let seeds: &[u32] = if len <= 300 { &[!0u32, 0, 0x1234_5678] } else { &[!0u32] };
                for &seed in seeds {
                    assert_eq!(
                        bytes(seed, b),
                        step_bytes(seed, b),
                        "{} pattern={name} off={off} len={len} seed={seed:#x}",
                        path()
                    );
                }
            }
        }
    }
}

#[test]
fn cells_and_cells_by_equal_software() {
    for name in PATTERNS {
        let raw = pattern(name, 8 * 600);
        let v: Vec<i64> = raw.chunks_exact(8).map(|c| i64::from_le_bytes(c.try_into().unwrap())).collect();
        for off in [0usize, 1, 3, 7] {
            for n in 0..=(v.len() - off) {
                let s = &v[off..off + n];
                let want = s.iter().fold(!0u32, |h, &x| step_cell(h, x));
                assert_eq!(cells(!0, s), want, "{} cells pattern={name} off={off} n={n}", path());
                assert_eq!(cells_by(!0, n, |i| v[off + i]), want, "{} cells_by pattern={name} off={off} n={n}", path());
            }
        }
        // A reader's "past the end reads 0", through the closure, as bebop-image's `crc32_in` needs.
        let n = v.len();
        let want = (0..n + 9).fold(!0u32, |h, i| step_cell(h, v.get(i).copied().unwrap_or(0)));
        assert_eq!(cells_by(!0, n + 9, |i| v.get(i).copied().unwrap_or(0)), want, "past-the-end cells read 0");
    }
}

/// Mixing the two on one register: a hardware step after a software step is the same function.
#[test]
fn hardware_and_software_steps_compose() {
    let buf = pattern("affine", 1000);
    let mid = step_bytes(!0, &buf[..333]);
    assert_eq!(bytes(mid, &buf[333..]), step_bytes(!0, &buf));
}

/// Bench (P12): the crc of a 1e6-key image's cells, software tables vs this path, in one process.
/// Printed, not asserted. `cargo test --release crc_hw_bench -- --ignored --nocapture`.
#[test]
#[ignore]
fn crc_hw_bench() {
    let n = 1usize << 21; // 16 MiB of cells
    let v: Vec<i64> = (0..n as i64).map(|i| i.wrapping_mul(0x9E37_79B9_7F4A_7C15u64 as i64)).collect();
    let reps = 5;
    let t = std::time::Instant::now();
    let mut a = 0;
    for _ in 0..reps {
        a = std::hint::black_box(&v).iter().fold(!0u32, |h, &x| step_cell(h, x));
    }
    let sw = t.elapsed().as_secs_f64() / reps as f64;
    let t = std::time::Instant::now();
    let mut b = 0;
    for _ in 0..reps {
        b = cells(!0, std::hint::black_box(&v));
    }
    let hw = t.elapsed().as_secs_f64() / reps as f64;
    assert_eq!(a, b);
    let mb = (n * 8) as f64 / 1e6;
    println!(
        "bench crc_cells_16MiB sw_ms={:.2} ({:.0} MB/s) hw_ms={:.2} ({:.0} MB/s) path={} x{:.1}",
        sw * 1e3,
        mb / sw,
        hw * 1e3,
        mb / hw,
        path(),
        sw / hw
    );
}
