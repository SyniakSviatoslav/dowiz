// TEMPORARY reference-vector dumper for MESH-13 bit-exact KAT. Deleted after use.
use dowiz_kernel::pq::kem::{decaps_internal, encaps_internal, keygen_internal};

fn hex(b: &[u8]) -> String {
    b.iter().map(|x| format!("{:02x}", x)).collect()
}

#[test]
fn dump_reference_vectors() {
    // Fixed seeds so the vectors are reproducible.
    let d: [u8; 32] = [0x11u8; 32];
    let m: [u8; 32] = [0x22u8; 32];
    let (pk, sk) = keygen_internal(&d);
    let (ct, ss) = encaps_internal(&pk, &m);
    let ss_dec = decaps_internal(&sk, &ct);
    assert_eq!(ss, ss_dec, "self-consistency");
    println!("KEM_REF_PK={}", hex(&pk));
    println!("KEM_REF_SK={}", hex(&sk));
    println!("KEM_REF_CT={}", hex(&ct));
    println!("KEM_REF_SS={}", hex(&ss));

    // Second seed for a second vector.
    let d2: [u8; 32] = [0x33u8; 32];
    let m2: [u8; 32] = [0x44u8; 32];
    let (pk2, sk2) = keygen_internal(&d2);
    let (ct2, ss2) = encaps_internal(&pk2, &m2);
    assert_eq!(ss2, decaps_internal(&sk2, &ct2));
    println!("KEM_REF2_PK={}", hex(&pk2));
    println!("KEM_REF2_SK={}", hex(&sk2));
    println!("KEM_REF2_CT={}", hex(&ct2));
    println!("KEM_REF2_SS={}", hex(&ss2));
}
