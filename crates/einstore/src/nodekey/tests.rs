use super::fixtures::{self, FN_SOURCE};
use super::*;

/// RT K-3: the compile-node frame is `1 + 5*8 + 8 + len(fn_source) + 8 + 8 + 8`
/// bytes -- derived here from the fields, never a constant.
#[test]
fn dagc_key_fields_compile_frame_length_is_k3() {
    let f = fixtures::compile();
    assert_eq!(f.len(), 1 + 5 * 8 + 8 + FN_SOURCE.len() + 8 + 8 + 8);
    // and K64's low half IS that length (RT §2.2)
    assert_eq!(f.k64() as u64 & 0xffff_ffff, f.len() as u64);
}

/// Its positive twin at the other edge: an empty fn_source still obeys K-3.
#[test]
fn k3_holds_for_an_empty_source() {
    let mut f = Frame::new(TAG_COMPILE);
    f.i64(1).bytes(b"").i64(2).i64(3).i64(4);
    assert_eq!(f.len(), 1 + 5 * 8 + 8 + 0 + 8 + 8 + 8);
}

/// A list is ONE field whose bytes are its elements' fields.
#[test]
fn a_list_nests_lengths_inside_one_field() {
    let mut f = Frame::new(TAG_PROJECTION);
    let at = f.open();
    f.i64(-1).bytes(&[9, 9, 9]).close(at);
    let b = f.as_bytes();
    assert_eq!(b[0], b'P');
    assert_eq!(u64::from_le_bytes(b[1..9].try_into().unwrap()), 16 + 11);
    assert_eq!(u64::from_le_bytes(b[9..17].try_into().unwrap()), 8);
    assert_eq!(&b[17..25], &[0xff; 8]);
    assert_eq!(u64::from_le_bytes(b[25..33].try_into().unwrap()), 3);
    assert_eq!(&b[33..], &[9, 9, 9]);
}

/// An empty list is a len-0 field: 8 zero bytes and nothing after them.
#[test]
fn an_empty_list_is_eight_zero_bytes() {
    let f = fixtures::empty();
    assert_eq!(f.len(), 1 + 16 + 16 + 8 + 8);
    assert_eq!(&f.as_bytes()[33..], &[0u8; 16]);
}

/// The field order is part of the key: two fields swapped move both hashes.
#[test]
fn swapping_two_fields_moves_both_keys() {
    let mut a = Frame::new(TAG_COMPILE);
    a.i64(1).i64(2);
    let mut b = Frame::new(TAG_COMPILE);
    b.i64(2).i64(1);
    assert_eq!(a.len(), b.len());
    assert_ne!(a.k64(), b.k64());
    assert_ne!(a.k256(), b.k256());
}

/// FIPS 180-4 known answers: empty, "abc", the two-block 448-bit message.
#[test]
fn sha256_known_answers() {
    assert_eq!(hex(&sha256(b"")), "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855");
    assert_eq!(hex(&sha256(b"abc")), "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad");
    assert_eq!(
        hex(&sha256(b"abcdbcdecdefdefgefghfghighijhijkijkljklmklmnlmnomnopnopq")),
        "248d6a61d20638b8e5c026930c3e6039a33ce45964ff2167f6ecedd419db06c1"
    );
}

/// K64 is the same function `dowiz_hub::block::schema::k64` is: zlib crc32 in
/// the high half, the length in the low half. "123456789" is zlib's check value.
#[test]
fn k64_is_zlib_crc_over_length() {
    assert_eq!(crate::crc32(b"123456789"), 0xcbf4_3926);
    assert_eq!(k64(b"123456789") as u64, (0xcbf4_3926u64 << 32) | 9);
}

#[test]
fn named_knows_the_three_fixtures_and_nothing_else() {
    for n in ["compile", "proj", "empty"] {
        assert!(fixtures::named(n).is_some(), "{n}");
    }
    assert!(fixtures::named("object").is_none());
}
