//! T-LV-PCD-01: the published root, its path, the certificate. Every refusal has a positive twin;
//! the corrupted input is ONE NAMED BYTE, never a fuzzer (no-fuzzers rule).

use super::*;
use crate::nodekey::{hex, sha256};

/// The catalogue the gate fixtures are cut from (`crates/bebop-wasm/oracle_verify.py --make`):
/// 165 dishes, `p_000`..`p_164`, price `100 + 7 i` lek minor units.
pub(crate) fn catalog(n: usize) -> Vec<(Vec<u8>, i64, Hash)> {
    let rows: Vec<(Vec<u8>, i64)> = (0..n).map(|i| (format!("p_{i:03}").into_bytes(), 100 + 7 * i as i64)).collect();
    price_leaves(&rows).expect("ids are distinct")
}

fn hashes(l: &[(Vec<u8>, i64, Hash)]) -> Vec<Hash> {
    l.iter().map(|x| x.2).collect()
}

#[test]
fn rfc6962_vectors_empty_tree_and_empty_leaf() {
    // RFC 6962 / CT test vectors: MTH({}) = sha256(""), and the leaf hash of the empty string.
    assert_eq!(hex(&root(&[])), "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855");
    assert_eq!(hex(&leaf_hash(b"")), "6e340b9cffb37a989ca544e6bb780a2c78901d3fb33738768511a30617afa01d");
}

#[test]
fn domain_separation_leaf_is_not_node_is_not_plain() {
    let a = leaf_hash(b"x");
    let b = leaf_hash(b"y");
    let mut cat = a.to_vec();
    cat.extend_from_slice(&b);
    assert_ne!(node_hash(&a, &b), leaf_hash(&cat), "0x01 vs 0x00: a node is never a leaf");
    assert_ne!(node_hash(&a, &b), sha256(&cat), "a node is never the bare pair hash");
    assert_ne!(node_hash(&a, &b), node_hash(&b, &a), "the pair hash is ordered");
}

#[test]
fn every_leaf_of_every_size_proves_and_verifies() {
    for n in 1..=33usize {
        let hs = hashes(&catalog(n));
        let r = root(&hs);
        for i in 0..n {
            let p = prove(&hs, i).expect("in range");
            assert!(verify(&hs[i], i as u64, n as u64, &p, &r), "n={n} i={i}");
            // its twin: the neighbour's leaf under this path does not verify
            if n > 1 {
                assert!(!verify(&hs[(i + 1) % n], i as u64, n as u64, &p, &r), "n={n} i={i} swapped leaf");
            }
        }
        assert!(prove(&hs, n).is_none(), "index n is out of range");
    }
}

/// `Merkle.lean`'s path is TOP-DOWN with explicit sides; ours is bottom-up with sides from (i, n).
/// The model's `climb` over `zip(sides, path).reverse()` must land on the root, and `verify` must be
/// exactly "root = climb" (`verify_path`), on a 5-leaf tree -- the odd case the model leaves to Rust.
#[test]
fn lean_twin_verify_path_on_five_leaves() {
    let hs = hashes(&catalog(5));
    let r = root(&hs);
    let other = leaf_hash(b"not the root");
    for i in 0..5u64 {
        let path = prove(&hs, i as usize).unwrap();
        let sd = sides(i, 5, path.len()).unwrap();
        let mut top_down: Vec<(Side, Hash)> = sd.into_iter().zip(path.iter().copied()).collect();
        top_down.reverse();
        // Lean: climb [] x = x; climb ((s, sib) :: p) x = combine s sib (climb p x)
        fn lean_climb(p: &[(Side, Hash)], x: Hash) -> Hash {
            match p.split_first() {
                None => x,
                Some(((Side::Left, sib), rest)) => node_hash(sib, &lean_climb(rest, x)),
                Some(((Side::Right, sib), rest)) => node_hash(&lean_climb(rest, x), sib),
            }
        }
        let c = lean_climb(&top_down, hs[i as usize]);
        assert_eq!(c, r, "leaf {i}: the model's climb is the root");
        for cand in [r, other] {
            assert_eq!(verify(&hs[i as usize], i, 5, &path, &cand), cand == c, "verify_path at leaf {i}");
        }
    }
    // leaf 4 of 5 sits alone under the root's right: one sibling, on the left
    assert_eq!(prove(&hs, 4).unwrap().len(), 1);
    assert_eq!(sides(4, 5, 1), Some(vec![Side::Left]));
}

#[test]
fn a_path_one_level_short_or_long_is_malformed() {
    let hs = hashes(&catalog(8));
    let mut p = prove(&hs, 3).unwrap();
    assert!(climb(&hs[3], 3, 8, &p).is_some(), "positive twin: the full path has a shape");
    let last = p.pop().unwrap();
    // an interior node presented as the leaf one level up: the depth check refuses it
    assert!(climb(&hs[3], 3, 8, &p).is_none(), "short path");
    p.push(last);
    p.push(last);
    assert!(climb(&hs[3], 3, 8, &p).is_none(), "long path");
}

#[test]
fn a_repeated_id_is_refused() {
    let ok = vec![(b"a".to_vec(), 1), (b"b".to_vec(), 2)];
    assert!(price_leaves(&ok).is_some(), "positive twin");
    let dup = vec![(b"a".to_vec(), 1), (b"a".to_vec(), 2)];
    assert!(price_leaves(&dup).is_none(), "one id, one price");
}

#[test]
fn certificate_round_trips_and_checks() {
    let l = catalog(165);
    for i in [0usize, 7, 128, 164] {
        let c = Cert::of(&l, i).unwrap();
        let b = c.encode();
        let d = Cert::decode(&b).expect("decodes");
        assert_eq!(d, c);
        assert_eq!(d.check(), Ok(root(&hashes(&l))), "line {i}");
    }
}

/// T-LV-PCD-01's named byte: byte 0 of the FIRST sibling of `p_007`'s path. The path then climbs to
/// another root and the certificate is a Mismatch (status 51 in every reader).
#[test]
fn one_named_path_byte_flipped_is_a_mismatch() {
    let l = catalog(165);
    let c = Cert::of(&l, 7).unwrap();
    let mut b = c.encode();
    let at = b.len() - 32 * c.path.len(); // the path field's first byte
    assert_eq!(Cert::decode(&b).unwrap().check(), Ok(c.root), "positive twin");
    b[at] ^= 0x01;
    match Cert::decode(&b).unwrap().check() {
        Err(CertError::Mismatch { climbed }) => assert_ne!(climbed, c.root),
        other => panic!("expected a mismatch, got {other:?}"),
    }
    // and a moved price (the receipt said 1 lek more) is refused the same way
    let mut m = c.clone();
    m.price += 1;
    assert!(matches!(m.check(), Err(CertError::Mismatch { .. })));
}

#[test]
fn decode_is_strict() {
    let c = Cert::of(&catalog(3), 1).unwrap();
    let b = c.encode();
    assert!(Cert::decode(&b).is_ok(), "positive twin");
    let mut trailing = b.clone();
    trailing.push(0);
    assert_eq!(Cert::decode(&trailing), Err(CertError::Malformed), "a trailing byte");
    let mut tag = b.clone();
    tag[0] = b'W';
    assert_eq!(Cert::decode(&tag), Err(CertError::Malformed), "the wrong tag");
    assert_eq!(Cert::decode(&b[..b.len() - 1]), Err(CertError::Malformed), "one byte short");
}

/// AC-LV-PCD-01 size: the path is 32 B per level. 165 dishes = 8 levels = 256 B (the AC's bound);
/// 10^4 leaves = 14 levels = 448 B -- the AC's "<= 256 B at n <= 10^4" cannot hold with full
/// SHA-256 siblings (it would need 18-byte hashes); the bound that holds is n <= 256.
#[test]
fn proof_size_by_catalogue_size() {
    let small = hashes(&catalog(165));
    assert_eq!(prove(&small, 100).unwrap().len() * 32, 256);
    let big: Vec<Hash> = (0..10_000u32).map(|i| leaf_hash(&i.to_le_bytes())).collect();
    let longest = (0..big.len()).step_by(997).map(|i| prove(&big, i).unwrap().len()).max().unwrap();
    assert_eq!(longest * 32, 448);
}

#[test]
fn the_signed_message_names_every_input() {
    let r = root(&hashes(&catalog(3)));
    let m = signed_message(b"dubin", 4, 3, &r);
    assert_ne!(m, signed_message(b"dubin", 5, 3, &r), "generation");
    assert_ne!(m, signed_message(b"dubim", 4, 3, &r), "slug");
    assert_ne!(m, signed_message(b"dubin", 4, 4, &r), "count");
}

/// Benchmarks (AC-LV-PCD-01: root over 165 <= 20 us, verify <= 10 us). Printed, not asserted: a
/// debug build on a loaded phone is not the number; run with `--release -- --nocapture bench`.
#[test]
fn bench_root_165_and_verify() {
    let l = catalog(165);
    let hs = hashes(&l);
    let reps = 200u32;
    let t = std::time::Instant::now();
    let mut r = [0u8; 32];
    for _ in 0..reps {
        r = root(std::hint::black_box(&hs));
    }
    let root_us = t.elapsed().as_secs_f64() * 1e6 / reps as f64;
    let c = Cert::of(&l, 77).unwrap();
    let t = std::time::Instant::now();
    for _ in 0..reps {
        assert!(std::hint::black_box(&c).check().is_ok());
    }
    let verify_us = t.elapsed().as_secs_f64() * 1e6 / reps as f64;
    println!("bench merkle_root_165_us={root_us:.1} verify_cert_us={verify_us:.2} root={}", hex(&r));
}
