use super::*;

/// A WHOLE jpeg, not just one that starts like one.
///
/// These fixtures used to stop after the JFIF header, which made every test
/// in this module assert against a file no browser can draw -- the same
/// shape as the truncated photograph that reached production. A fixture
/// that could not survive the thing being tested is not a fixture.
fn jpeg() -> Vec<u8> {
    let mut v = vec![0xFF, 0xD8, 0xFF, 0xE0];
    v.extend_from_slice(b"\x00\x10JFIF\x00\x01");
    v.extend(std::iter::repeat_n(0u8, 64));
    v.extend_from_slice(&[0xFF, 0xC0]); // SOF0: a frame
    v.extend(std::iter::repeat_n(0u8, 15));
    v.extend_from_slice(&[0xFF, 0xDA]); // SOS: the scan
    v.extend(std::iter::repeat_n(3u8, 32));
    v.extend_from_slice(&[0xFF, 0xD9]); // EOI
    v
}
/// Truncated at the header, exactly like the one that shipped.
fn jpeg_cut_short() -> Vec<u8> {
    let mut v = vec![0xFF, 0xD8, 0xFF, 0xE0];
    v.extend_from_slice(b"\x00\x10JFIF\x00\x01");
    v.extend(std::iter::repeat_n(0u8, 64));
    v
}
fn png() -> Vec<u8> {
    let mut v = b"\x89PNG\r\n\x1a\n".to_vec();
    v.extend(std::iter::repeat_n(7u8, 64));
    v.extend_from_slice(b"IEND\xae\x42\x60\x82");
    v
}

/// RED before `complete()` existed: `prepare` accepted this and the hub
/// served 4012 bytes of header as a photograph, with a 200 and an
/// `image/jpeg` content type, which every browser drew as nothing.
#[test]
fn a_file_that_starts_like_an_image_and_stops_is_refused() {
    assert_eq!(sniff(&jpeg_cut_short()), Some(Kind::Jpeg), "it still sniffs as a jpeg");
    assert!(matches!(
        prepare(&jpeg_cut_short()),
        Err(MediaError::Truncated(Kind::Jpeg))
    ));
    // And the whole one is still accepted, so the check is not just strict.
    assert!(prepare(&jpeg()).is_ok());

    let mut cut_png = png();
    cut_png.truncate(cut_png.len() - 8);
    assert!(matches!(prepare(&cut_png), Err(MediaError::Truncated(Kind::Png))));
}

#[test]
fn the_four_real_formats_are_recognised() {
    assert_eq!(sniff(&jpeg()), Some(Kind::Jpeg));
    assert_eq!(sniff(&png()), Some(Kind::Png));
    let mut webp = b"RIFF".to_vec();
    webp.extend_from_slice(&4u32.to_le_bytes());
    webp.extend_from_slice(b"WEBPVP8 ");
    assert_eq!(sniff(&webp), Some(Kind::Webp));
    let mut gif = b"GIF89a".to_vec();
    gif.extend(std::iter::repeat_n(0u8, 16));
    gif.push(0x3B);
    assert_eq!(sniff(&gif), Some(Kind::Gif));
}

/// The one that matters. An SVG is a document that can carry script, and
/// serving it from the venue's own origin under a name the uploader chose
/// is a cross-site scripting hole in a menu.
#[test]
fn svg_and_other_documents_are_not_images() {
    for bad in [
        &b"<svg xmlns=\"http://www.w3.org/2000/svg\"><script>alert(1)</script></svg>"[..],
        b"<!DOCTYPE html><html><body>hello there now",
        b"<?xml version=\"1.0\"?><svg></svg>xxxxxxx",
        b"%PDF-1.4 something something",
        b"\x7fELF\x02\x01\x01\x00\x00\x00\x00\x00",
        b"",
        b"short",
    ] {
        assert_eq!(sniff(bad), None, "accepted {:?}", String::from_utf8_lossy(&bad[..bad.len().min(20)]));
        assert!(matches!(prepare(bad), Err(MediaError::NotAnImage)));
    }
}

/// A client-declared type is worthless; the bytes decide. A PNG announced
/// as a JPEG is stored, served and named as a PNG.
#[test]
fn the_bytes_decide_the_type_not_the_caller() {
    let s = prepare(&png()).expect("png");
    assert_eq!(s.kind, Kind::Png);
    assert!(s.url().ends_with(".png"), "{}", s.url());
    assert_eq!(s.kind.mime(), "image/png");
}

/// Content addressing: the same photo twice is one file.
#[test]
fn identical_bytes_produce_one_name() {
    let a = prepare(&jpeg()).expect("a");
    let b = prepare(&jpeg()).expect("b");
    assert_eq!(a.digest, b.digest);
    assert_eq!(a.url(), b.url());
    // And one changed byte is a different file. The byte changed is one in
    // the SCAN, not the last one: the last byte is now half of the EOI
    // marker, and flipping it makes a TRUNCATED file rather than a
    // different image -- which is a different test, two lines up.
    let mut other = jpeg();
    let mid = other.len() - 8;
    other[mid] ^= 0x01;
    assert_ne!(prepare(&other).expect("c").digest, a.digest);
}

#[test]
fn a_digest_is_sixty_four_lowercase_hex() {
    let s = prepare(&jpeg()).expect("jpeg");
    assert_eq!(s.digest.len(), 64);
    assert!(s.digest.bytes().all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase()));
}

#[test]
fn something_enormous_is_refused_by_size_first() {
    let huge = vec![0xFFu8; MAX_BYTES + 1];
    assert!(matches!(prepare(&huge), Err(MediaError::TooLarge(_))));
}

/// The serving gate. Every one of these is a way to name a file that is not
/// in the media directory, and every one must be refused by SHAPE rather
/// than by sanitising -- sanitising a path is a game you eventually lose.
#[test]
fn nothing_but_a_digest_can_be_served() {
    let good = format!("{}.jpg", "a".repeat(64));
    assert!(parse_name(&good).is_some());

    for bad in [
        "../../../etc/passwd",
        "../../signing.key",
        "..%2f..%2fsigning.key",
        "orders.store",
        "a.jpg",
        &format!("{}.jpg", "a".repeat(63)),
        &format!("{}.jpg", "a".repeat(65)),
        &format!("{}.svg", "a".repeat(64)),
        &format!("{}.jpg", "A".repeat(64)),
        &format!("{}.exe", "a".repeat(64)),
        &format!("{}.jpg/../x", "a".repeat(64)),
        &format!("{}", "a".repeat(64)),
        "",
        ".",
        "..",
    ] {
        assert!(parse_name(bad).is_none(), "accepted {bad:?}");
    }
}

#[test]
fn extensions_round_trip_through_their_kinds() {
    for k in [Kind::Jpeg, Kind::Png, Kind::Webp, Kind::Gif] {
        assert_eq!(Kind::from_extension(k.extension()), Some(k));
    }
    assert_eq!(Kind::from_extension("jpeg"), Some(Kind::Jpeg), "both spellings");
    assert_eq!(Kind::from_extension("svg"), None);
    assert_eq!(Kind::from_extension(""), None);
}
