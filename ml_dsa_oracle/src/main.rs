use hybrid_array::{Array, typenum::U32};
use ml_dsa::{
    common::KeyExport,
    signature::Verifier,
    Keypair, MlDsa65, SigningKey,
};

fn main() {
    let seed: [u8; 32] = [
        0x93, 0x4d, 0x60, 0xb3, 0x56, 0x24, 0xd7, 0x40,
        0xb3, 0x0a, 0x7f, 0x22, 0x7a, 0xf2, 0xae, 0x7c,
        0x67, 0x8e, 0x4e, 0x04, 0xe1, 0x3c, 0x5f, 0x50,
        0x9e, 0xad, 0xe2, 0xb7, 0x9a, 0xea, 0x77, 0xe2,
    ];
    let rnd = [0u8; 32];
    let msg: &[u8] = b"This is a test message for ML-DSA digital signature algorithm!";

    let seed_arr: &Array<u8, U32> = (&seed).into();
    let rnd_arr: &Array<u8, U32> = (&rnd).into();

    let sk = SigningKey::<MlDsa65>::from_seed(seed_arr);
    let exp = sk.expanded_key();
    let vk = sk.verifying_key();
    let pk_bytes = vk.to_bytes();
    let sk_bytes = exp.to_expanded();
    let sig = exp.sign_internal(&[msg], rnd_arr);
    let sig_bytes = sig.encode();

    // Self-consistency: the emitted signature must verify under the SAME message
    // using the internal (raw tr||M) mu builder as sign_internal.
    let ok = vk.verify_internal(msg, &sig);

    println!("SEED={}", hex::encode(seed));
    println!("RND={}", hex::encode(rnd));
    println!("MSG={}", hex::encode(msg));
    println!("PK_LEN={}", pk_bytes.len());
    println!("PK={}", hex::encode(&pk_bytes[..]));
    println!("SK_LEN={}", sk_bytes.len());
    println!("SK={}", hex::encode(&sk_bytes[..]));
    println!("SIG_LEN={}", sig_bytes.len());
    println!("SIG={}", hex::encode(&sig_bytes[..]));
    println!("VERIFY_OK={}", ok);
}
