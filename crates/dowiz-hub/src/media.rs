//! Photographs, content-addressed on disk.
//!
//! NOT IN A BEBOP STORE, and that is a considered exception rather than an
//! oversight. The KV layout this crate uses everywhere else is rewritten WHOLE
//! on every write -- which is exactly right for a menu of fifty products and
//! exactly wrong for a hundred JPEGs: adding the fiftieth photo would rewrite
//! the other forty-nine. Blobs go to the filesystem; only the reference to them
//! lives in the catalogue.
//!
//! CONTENT-ADDRESSED, so the same photo uploaded twice is stored once, a
//! re-upload of an unchanged image costs nothing, and the URL can be cached by
//! a browser forever -- the bytes behind a hash cannot change. That last point
//! is why the served path carries the digest and not the product id: a product
//! id's image changes, so its URL could never be immutable.
//!
//! THE TYPE IS DECIDED BY THE BYTES, never by the client. A caller that says
//! "image/png" over a file starting with `<script` is either confused or
//! hostile, and serving it back under the name they chose is how a menu becomes
//! a cross-site scripting vector on the venue's own domain.

use crate::crypto::hex;
use sha2::{Digest, Sha256};

/// What the magic bytes say this is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Jpeg,
    Png,
    Webp,
    Gif,
}

impl Kind {
    pub fn extension(self) -> &'static str {
        match self {
            Kind::Jpeg => "jpg",
            Kind::Png => "png",
            Kind::Webp => "webp",
            Kind::Gif => "gif",
        }
    }
    pub fn mime(self) -> &'static str {
        match self {
            Kind::Jpeg => "image/jpeg",
            Kind::Png => "image/png",
            Kind::Webp => "image/webp",
            Kind::Gif => "image/gif",
        }
    }
    pub fn from_extension(ext: &str) -> Option<Kind> {
        match ext {
            "jpg" | "jpeg" => Some(Kind::Jpeg),
            "png" => Some(Kind::Png),
            "webp" => Some(Kind::Webp),
            "gif" => Some(Kind::Gif),
            _ => None,
        }
    }
}

/// Identify an image by its leading bytes.
///
/// The four formats a phone or a laptop actually produces. SVG is deliberately
/// ABSENT: it is a document that can carry script, and an "image" upload that
/// can execute in the venue's origin is not an image upload.
pub fn sniff(bytes: &[u8]) -> Option<Kind> {
    if bytes.len() < 12 {
        return None;
    }
    if bytes.starts_with(&[0xFF, 0xD8, 0xFF]) {
        return Some(Kind::Jpeg);
    }
    if bytes.starts_with(b"\x89PNG\r\n\x1a\n") {
        return Some(Kind::Png);
    }
    if bytes.starts_with(b"RIFF") && bytes[8..12] == *b"WEBP" {
        return Some(Kind::Webp);
    }
    if bytes.starts_with(b"GIF87a") || bytes.starts_with(b"GIF89a") {
        return Some(Kind::Gif);
    }
    None
}

/// Does the container close the way its format says it must?
///
/// One rule per format, and each is the format's own end-of-file marker:
///   * JPEG must carry a frame header (SOF), a scan (SOS) and end with EOI;
///   * PNG must end with the `IEND` chunk;
///   * GIF must end with the trailer byte `0x3B`;
///   * WEBP's RIFF header states its own length, so the file must be at least
///     that long.
/// Trailing NUL padding is tolerated on the marker formats: some tools pad, and
/// a padded file still decodes.
pub fn complete(bytes: &[u8], kind: Kind) -> bool {
    let trimmed = {
        let mut end = bytes.len();
        while end > 0 && bytes[end - 1] == 0 {
            end -= 1;
        }
        &bytes[..end]
    };
    match kind {
        Kind::Jpeg => {
            trimmed.ends_with(&[0xFF, 0xD9])
                && trimmed.windows(2).any(|w| w == [0xFF, 0xDA])
                && trimmed
                    .windows(2)
                    .any(|w| w[0] == 0xFF && matches!(w[1], 0xC0 | 0xC1 | 0xC2))
        }
        Kind::Png => trimmed.ends_with(b"IEND\xae\x42\x60\x82"),
        Kind::Gif => trimmed.last() == Some(&0x3B),
        Kind::Webp => {
            if trimmed.len() < 12 {
                return false;
            }
            let stated = u32::from_le_bytes([trimmed[4], trimmed[5], trimmed[6], trimmed[7]]);
            trimmed.len() as u64 >= stated as u64 + 8
        }
    }
}

/// The largest file accepted.
///
/// A dish photo that has been through a browser canvas at 1600px is well under
/// 400 KB. Two megabytes leaves room for a camera original that skipped the
/// resize, and refuses a video somebody renamed.
pub const MAX_BYTES: usize = 2 * 1024 * 1024;

#[derive(Debug)]
pub enum MediaError {
    TooLarge(usize),
    /// The bytes are not one of the four image formats.
    NotAnImage,
    /// It begins as an image and does not finish as one.
    Truncated(Kind),
    Io(String),
}

impl std::fmt::Display for MediaError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MediaError::TooLarge(n) => write!(f, "{n} bytes is larger than the {MAX_BYTES} limit"),
            MediaError::Truncated(k) => write!(
                f,
                "that {k:?} file is incomplete -- it starts like an image and has no end marker, \
                 so a browser will show nothing where the photograph should be"
            ),
            MediaError::NotAnImage => {
                write!(f, "that file is not a jpeg, png, webp or gif")
            }
            MediaError::Io(e) => write!(f, "{e}"),
        }
    }
}

/// A stored image.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Stored {
    /// sha256 of the bytes, hex. The name on disk and in the URL.
    pub digest: String,
    pub kind: Kind,
    pub bytes: usize,
}

impl Stored {
    /// The path the storefront references.
    pub fn url(&self) -> String {
        format!("/media/{}.{}", self.digest, self.kind.extension())
    }
    pub fn filename(&self) -> String {
        format!("{}.{}", self.digest, self.kind.extension())
    }
}

/// Validate and name an image, without touching the filesystem.
///
/// Separated from writing so the whole decision -- is this an image, is it
/// small enough, what is it called -- is testable without a directory.
pub fn prepare(bytes: &[u8]) -> Result<Stored, MediaError> {
    if bytes.len() > MAX_BYTES {
        return Err(MediaError::TooLarge(bytes.len()));
    }
    let kind = sniff(bytes).ok_or(MediaError::NotAnImage)?;
    // ── THE FILE MUST END AS WELL AS BEGIN ──
    //
    // `sniff` reads the first twelve bytes, which is what a FORMAT check needs
    // and not what an INTEGRITY check needs. A truncated upload -- a dropped
    // connection, a half-written file -- still starts with the right magic, so
    // it was accepted, stored, served with a 200 and an `image/jpeg` header, and
    // drawn by the browser as nothing at all. That is exactly what happened to
    // this product's only dish photograph: 4012 bytes, a JFIF header, and no end
    // marker (found 2026-09-17; Chromium reports `naturalWidth 0`).
    //
    // Nothing here decodes the image -- that is a decoder's job and a Worker has
    // no room for one. It checks that the container is closed, which is cheap,
    // has no false positives on a whole file, and catches every truncation.
    if !complete(bytes, kind) {
        return Err(MediaError::Truncated(kind));
    }
    let digest = hex(&Sha256::digest(bytes));
    Ok(Stored { digest, kind, bytes: bytes.len() })
}

/// Is this a name this module could have produced?
///
/// THE GATE ON THE SERVING PATH. A request for `/media/<name>` must never be
/// able to name a file outside the media directory, so the name is checked
/// against the shape `prepare` produces -- 64 hex characters, a dot, a known
/// extension -- rather than being sanitised. Sanitising a path is a game you
/// eventually lose; refusing anything that is not exactly right is not.
pub fn parse_name(name: &str) -> Option<(String, Kind)> {
    let (digest, ext) = name.rsplit_once('.')?;
    if digest.len() != 64 || !digest.bytes().all(|b| b.is_ascii_hexdigit()) {
        return None;
    }
    // Lowercase only: two spellings of one digest would be two cache entries
    // for one image, and two names for one file.
    if digest.bytes().any(|b| b.is_ascii_uppercase()) {
        return None;
    }
    Some((digest.to_string(), Kind::from_extension(ext)?))
}

#[cfg(test)]
mod tests {
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
}
