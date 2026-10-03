//! RFC 8291 §5 and Appendix A, byte for byte, then a round trip through a
//! decryption (`ece::open`, test-only) written from the RFC: the browser's side.
use super::*;
use base64::engine::general_purpose::URL_SAFE_NO_PAD as B64;
use base64::Engine;

fn b(s: &str) -> Vec<u8> {
    B64.decode(s.split_whitespace().collect::<String>()).expect("test vector is base64url")
}

// RFC 8291 §5 / Appendix A, copied from rfc-editor.org/rfc/rfc8291.txt.
const PLAIN: &str = "When I grow up, I want to be a watermelon";
const AS_PRIVATE: &str = "yfWPiYE-n46HLnH0KqZOF1fJJU3MYrct3AELtAQ-oRw";
const AS_PUBLIC: &str = "BP4z9KsN6nGRTbVYI_c7VJSPQTBtkgcy27mlmlMoZIIgDll6e3vCYLocInmYWAmS6TlzAC8wEqKK6PBru3jl7A8";
const UA_PRIVATE: &str = "q1dXpw3UpT5VOmu_cf_v6ih07Aems3njxI-JWgLcM94";
const UA_PUBLIC: &str = "BCVxsr7N_eNgVRqvHtD0zTZsEc6-VV-JvLexhqUzORcxaOzi6-AYWXvTBHm4bjyPjs7Vd8pZGH6SRpkNtoIAiw4";
const SALT: &str = "DGv6ra1nlYgDCS1FRnbzlw";
const AUTH: &str = "BTBZMqHH6r4Tts7J_aSIgg";
const ECDH_SECRET: &str = "kyrL1jIIOHEzg3sM2ZWRHDRB62YACZhhSlknJ672kSs";
const CEK: &str = "oIhVW04MRdy2XN9CiKLxTg";
const NONCE: &str = "4h_95klXJ5E_qnoN";
const HEADER: &str = "DGv6ra1nlYgDCS1FRnbzlwAAEABBBP4z 9KsN6nGRTbVYI_c7VJSPQTBtkgcy27ml mlMoZIIgDll6e3vCYLocInmYWAmS6Tlz AC8wEqKK6PBru3jl7A8";
const CIPHERTEXT: &str = "8pfeW0KbunFT06SuDKoJH9Ql87S1QUrd irN6GcG7sFz1y1sqLgVi1VhjVkHsUoEs bI_0LpXMuGvnzQ";
const BODY: &str = "DGv6ra1nlYgDCS1FRnbzlwAAEABBBP4z9KsN6nGRTbVYI_c7VJSPQTBtkgcy27ml
   mlMoZIIgDll6e3vCYLocInmYWAmS6TlzAC8wEqKK6PBru3jl7A_yl95bQpu6cVPT
   pK4Mqgkf1CXztLVBSt2Ks3oZwbuwXPXLWyouBWLVWGNWQexSgSxsj_Qulcy4a-fN";

fn arr<const N: usize>(v: Vec<u8>) -> [u8; N] {
    v.try_into().expect("length")
}

#[test]
fn the_rfc_8291_example_is_reproduced_byte_for_byte() {
    let out = encrypt_with(PLAIN.as_bytes(), &b(UA_PUBLIC), &b(AUTH), &arr(b(AS_PRIVATE)), &arr(b(SALT))).unwrap();
    assert_eq!(out.len(), 144, "§5 decodes to 144 octets (its Content-Length line says 145); Appendix A: 86 header + 58 ciphertext");
    assert_eq!(&out[..HEADER_LEN], b(HEADER).as_slice(), "the 86-octet header");
    assert_eq!(&out[HEADER_LEN..], b(CIPHERTEXT).as_slice(), "the ciphertext");
    assert_eq!(out, b(BODY), "the §5 body");
}

#[test]
fn the_intermediate_values_are_the_appendix_a_values() {
    let (cek, nonce) = derive(&b(ECDH_SECRET), &b(UA_PUBLIC), &b(AS_PUBLIC), &b(AUTH), &b(SALT));
    assert_eq!(cek.to_vec(), b(CEK));
    assert_eq!(nonce.to_vec(), b(NONCE));
    // and the ECDH secret is what the two private keys agree on
    let sk = SecretKey::from_slice(&b(AS_PRIVATE)).unwrap();
    let ua = PublicKey::from_sec1_bytes(&b(UA_PUBLIC)).unwrap();
    let shared = p256::ecdh::diffie_hellman(sk.to_nonzero_scalar(), ua.as_affine());
    assert_eq!(shared.raw_secret_bytes().to_vec(), b(ECDH_SECRET));
}

use super::open as decrypt;

#[test]
fn a_fresh_message_opens_with_the_browsers_key_and_never_repeats() {
    let msg = br#"{"title":"Order #1a2b3c4d","body":"Ready"}"#;
    let one = encrypt(msg, &b(UA_PUBLIC), &b(AUTH)).unwrap();
    let two = encrypt(msg, &b(UA_PUBLIC), &b(AUTH)).unwrap();
    assert_ne!(one[..16], two[..16], "a fresh salt each time");
    assert_ne!(one[21..HEADER_LEN], two[21..HEADER_LEN], "a fresh ephemeral key each time");
    assert_eq!(decrypt(&one, &b(UA_PRIVATE), &b(AUTH)), msg.to_vec());
    assert_eq!(decrypt(&two, &b(UA_PRIVATE), &b(AUTH)), msg.to_vec());
}

#[test]
fn a_key_that_is_not_a_p256_point_is_refused_and_a_real_one_is_not() {
    let mut bad = b(UA_PUBLIC);
    bad[40] ^= 0x01; // off the curve
    assert_eq!(encrypt(b"x", &bad, &b(AUTH)), Err(EceError::BadPublicKey));
    assert_eq!(encrypt(b"x", &bad[..33], &b(AUTH)), Err(EceError::BadPublicKey), "a compressed point is not what the browser sends");
    assert!(encrypt(b"x", &b(UA_PUBLIC), &b(AUTH)).is_ok());
}

#[test]
fn an_auth_secret_of_the_wrong_length_is_refused_and_sixteen_is_not() {
    assert_eq!(encrypt(b"x", &b(UA_PUBLIC), &[0u8; 15]), Err(EceError::BadAuth));
    assert!(encrypt(b"x", &b(UA_PUBLIC), &[7u8; 16]).is_ok());
}

#[test]
fn a_message_past_one_record_is_refused_and_one_at_the_bound_fits_in_4096() {
    let big = vec![b'a'; PLAINTEXT_MAX + 1];
    assert_eq!(encrypt(&big, &b(UA_PUBLIC), &b(AUTH)), Err(EceError::TooLong(PLAINTEXT_MAX + 1)));
    let at = vec![b'a'; PLAINTEXT_MAX];
    assert_eq!(encrypt(&at, &b(UA_PUBLIC), &b(AUTH)).unwrap().len(), 4096);
}
