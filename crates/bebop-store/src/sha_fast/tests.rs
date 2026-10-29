//! P16: the fast sha256 is FIPS 180-4's, on every path. NIST vectors through the dispatching
//! function and the scalar one by name; every length 0..300 and the part splits against the old
//! allocating loop, kept here verbatim as the oracle.

use super::*;

fn hex(d: &[u8; 32]) -> String {
    d.iter().map(|b| format!("{b:02x}")).collect()
}

/// `nodekey::sha256` as it was before P16 (allocating, byte-at-a-time padding): the oracle.
fn sha256_old(data: &[u8]) -> [u8; 32] {
    let mut h = H0;
    let mut msg = data.to_vec();
    msg.push(0x80);
    while msg.len() % 64 != 56 {
        msg.push(0);
    }
    msg.extend_from_slice(&((data.len() as u64).wrapping_mul(8)).to_be_bytes());
    let mut w = [0u32; 64];
    for block in msg.chunks_exact(64) {
        for (i, c) in block.chunks_exact(4).enumerate() {
            w[i] = u32::from_be_bytes([c[0], c[1], c[2], c[3]]);
        }
        for i in 16..64 {
            let s0 = w[i - 15].rotate_right(7) ^ w[i - 15].rotate_right(18) ^ (w[i - 15] >> 3);
            let s1 = w[i - 2].rotate_right(17) ^ w[i - 2].rotate_right(19) ^ (w[i - 2] >> 10);
            w[i] = w[i - 16].wrapping_add(s0).wrapping_add(w[i - 7]).wrapping_add(s1);
        }
        let [mut a, mut b, mut c, mut d, mut e, mut f, mut g, mut hh] = h;
        for i in 0..64 {
            let s1 = e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25);
            let ch = (e & f) ^ (!e & g);
            let t1 = hh.wrapping_add(s1).wrapping_add(ch).wrapping_add(ORACLE_K[i]).wrapping_add(w[i]);
            let s0 = a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22);
            let maj = (a & b) ^ (a & c) ^ (b & c);
            let t2 = s0.wrapping_add(maj);
            hh = g;
            g = f;
            f = e;
            e = d.wrapping_add(t1);
            d = c;
            c = b;
            b = a;
            a = t1.wrapping_add(t2);
        }
        for (x, y) in h.iter_mut().zip([a, b, c, d, e, f, g, hh]) {
            *x = x.wrapping_add(y);
        }
    }
    let mut out = [0u8; 32];
    for (i, x) in h.iter().enumerate() {
        out[4 * i..4 * i + 4].copy_from_slice(&x.to_be_bytes());
    }
    out
}

/// The oracle's OWN copy of the round constants, so a mutant of `K` cannot hide in both sides.
const ORACLE_K: [u32; 64] = [
    0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4, 0xab1c5ed5,
    0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe, 0x9bdc06a7, 0xc19bf174,
    0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f, 0x4a7484aa, 0x5cb0a9dc, 0x76f988da,
    0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7, 0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967,
    0x27b70a85, 0x2e1b2138, 0x4d2c6dfc, 0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85,
    0xa2bfe8a1, 0xa81a664b, 0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070,
    0x19a4c116, 0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
    0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7, 0xc67178f2,
];

/// FIPS 180-4 / NIST CAVP SHA-256 examples.
const NIST: [(&[u8], &str); 4] = [
    (b"", "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"),
    (b"abc", "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"),
    (
        b"abcdbcdecdefdefgefghfghighijhijkijkljklmklmnlmnomnopnopq",
        "248d6a61d20638b8e5c026930c3e6039a33ce45964ff2167f6ecedd419db06c1",
    ),
    (
        b"abcdefghbcdefghicdefghijdefghijkefghijklfghijklmghijklmnhijklmnoijklmnopjklmnopqklmnopqrlmnopqrsmnopqrstnopqrstu",
        "cf5b16a778af8380036ce59e7b0492370b249b11e8f07a51afac45037afee9d1",
    ),
];

#[test]
fn nist_vectors_on_the_dispatching_and_the_scalar_path() {
    for (m, want) in NIST {
        assert_eq!(hex(&sha256(m)), want, "{} len={}", path(), m.len());
        assert_eq!(hex(&sha256_sw(m)), want, "sw len={}", m.len());
        assert_eq!(hex(&sha256_old(m)), want, "oracle len={}", m.len());
    }
    let million = vec![b'a'; 1_000_000];
    let want = "cdc76e5c9914fb9281a1c7e284d73e67f1809a48a497200e046d39ccc7112cd0";
    assert_eq!(hex(&sha256(&million)), want, "{} 1e6 x 'a'", path());
    assert_eq!(hex(&sha256_sw(&million)), want, "sw 1e6 x 'a'");
}

#[test]
fn every_length_equals_the_old_loop() {
    let buf: Vec<u8> = (0..300usize).map(|i| (i.wrapping_mul(31).wrapping_add(7)) as u8).collect();
    for n in 0..=300 {
        let o = sha256_old(&buf[..n]);
        assert_eq!(sha256(&buf[..n]), o, "{} len={n}", path());
        assert_eq!(sha256_sw(&buf[..n]), o, "sw len={n}");
    }
}

/// Bench (P16): one 65-byte node hash (a Merkle node), scalar vs this path.
/// `cargo test --release sha_fast_bench -- --ignored --nocapture`.
#[test]
#[ignore]
fn sha_fast_bench() {
    let node = [7u8; 65];
    let reps = 20_000u32;
    let time = |f: &dyn Fn(&[u8]) -> [u8; 32]| {
        let t = std::time::Instant::now();
        let mut r = [0u8; 32];
        for _ in 0..reps {
            r = f(std::hint::black_box(&node));
        }
        (t.elapsed().as_secs_f64() * 1e9 / reps as f64, r)
    };
    let (old, a) = time(&sha256_old);
    let (sw, b) = time(&sha256_sw);
    let (fast, c) = time(&sha256);
    assert!(a == b && b == c);
    println!("bench sha256_65B old_ns={old:.0} sw_ns={sw:.0} fast_ns={fast:.0} path={}", path());
}

/// RFC 9162's tree over `leaves` with the node hash `h` -- `merkle::root`'s shape, so the old and
/// the new sha256 time the same tree in one process.
fn root_with(leaves: &[[u8; 32]], h: fn(&[u8]) -> [u8; 32]) -> [u8; 32] {
    match leaves.len() {
        0 => h(&[]),
        1 => leaves[0],
        n => {
            let mut k = 1;
            while k << 1 < n {
                k <<= 1;
            }
            let mut b = [0u8; 65];
            b[0] = 0x01;
            b[1..33].copy_from_slice(&root_with(&leaves[..k], h));
            b[33..].copy_from_slice(&root_with(&leaves[k..], h));
            h(&b)
        }
    }
}

/// Bench (P16 AC: root over 165 leaves <= 20 us): old loop (control) vs scalar vs dispatched, one
/// process, best of 7 batches. `cargo test --release root165_bench -- --ignored --nocapture`.
#[test]
#[ignore]
fn root165_bench() {
    let leaves: Vec<[u8; 32]> = (0..165u32).map(|i| sha256_old(&i.to_le_bytes())).collect();
    let want = root_with(&leaves, sha256_old);
    let best = |h: fn(&[u8]) -> [u8; 32]| {
        let mut m = f64::MAX;
        for _ in 0..7 {
            let t = std::time::Instant::now();
            for _ in 0..200 {
                assert_eq!(root_with(std::hint::black_box(&leaves), h), want);
            }
            m = m.min(t.elapsed().as_secs_f64() * 1e6 / 200.0);
        }
        m
    };
    let (old, sw, fast) = (best(sha256_old), best(sha256_sw), best(sha256));
    let mut lv = f64::MAX;
    for _ in 0..7 {
        let t = std::time::Instant::now();
        for _ in 0..200 {
            assert_eq!(merkle_root(std::hint::black_box(&leaves)), want);
        }
        lv = lv.min(t.elapsed().as_secs_f64() * 1e6 / 200.0);
    }
    println!(
        "bench root165 old_us={old:.1} sw_us={sw:.1} fast_us={fast:.1} levels2x_us={lv:.1} path={}",
        path()
    );
}

/// The level-wise, two-at-a-time root is RFC 9162's recursive MTH on every size 0..=300.
#[test]
fn merkle_root_equals_the_recursive_tree() {
    let leaves: Vec<[u8; 32]> = (0..300u32).map(|i| sha256_old(&i.to_le_bytes())).collect();
    for n in 0..=300 {
        assert_eq!(merkle_root(&leaves[..n]), root_with(&leaves[..n], sha256_old), "{} n={n}", path());
    }
}
