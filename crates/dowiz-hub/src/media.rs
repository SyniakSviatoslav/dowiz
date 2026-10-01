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
pub const MAX_BYTES: usize = crate::CEILING_BYTES;

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
mod tests;
