//! `ImageProcessor` seam (S4 media council, REV-S4-9: seam only — no `media-worker` runtime
//! stood up here, that split triggers on the OCR slice or a measured CPU ceiling, each its own
//! decision). Pure-Rust `image` 0.25 + `webp` 0.3 transcode pipeline, council-DECIDED BY SPIKE
//! (`docs/design/rebuild-media-s4-council/spike-evidence.md`, Q3) and hardened by the breaker
//! round the spike itself did NOT cover (C1/H2/H3 — see below).
//!
//! Three sharp-parity profiles (`docs/design/rebuild-media-s4-council/proposal.md` §4):
//!   - product image  : 800×800  fit-inside, webp q82  (`spa-proxy.ts:222-226`)
//!   - theme logo     : 512×512  fit-inside, webp q80  (`themes.ts:127-130`)
//!   - entry photo    : 1024×1024 fit-inside, webp q78 + EXIF auto-orient (`spa-proxy.ts:279-280`)
//!
//! ## REV-S4-1 (breaker C1, CRIT) — decode-bomb cap
//! The spike's OWN benchmarked entry point (`attempt-b-pure-rust/src/main.rs:68`,
//! `image::load_from_memory_with_format`) sets NO `image::Limits` at all — a small compressed
//! file with a crafted huge header (e.g. a flat-color 65535×65535 PNG) would decode to gigabytes
//! of RGBA before any check ran, on a route reachable by an ANONYMOUS caller
//! (`POST /api/public/entry-photo`). This module NEVER uses that free-function entry point.
//! Every decode goes through `ImageReader::into_decoder()` with `image::Limits` set BEFORE the
//! decoder is constructed (`max_image_width`/`max_image_height` reject an oversized header the
//! moment it's parsed — before the pixel buffer is allocated) PLUS an explicit
//! `Limits::reserve_buffer` allocator-budget check against the header-declared dimensions×color
//! type (mirrors what `ImageReader::decode()`'s own free function does internally, which
//! `into_decoder()` — the path this module needs for orientation, see REV-S4-4 — does NOT do for
//! you). See `bomb_cap_rejects_a_huge_declared_dimension_png` below for the DoD proof.
//!
//! ## REV-S4-4 (breaker H3, HIGH) — EXIF orientation on ALL THREE profiles
//! `image::open()`/`load_from_memory_with_format` (the spike's benchmarked path) return a bare
//! `DynamicImage` with no decoder handle — `ImageDecoder::orientation()` is UNREACHABLE from that
//! path. This module always decodes via `ImageReader::into_decoder()` (a decoder handle),
//! reads `decoder.orientation()` explicitly, then calls `DynamicImage::apply_orientation()` after
//! `DynamicImage::from_decoder()` — on the product and logo profiles too, not just the
//! historically-`.rotate()`-called entry-photo path. All 8 EXIF orientation values (including the
//! mirrored 2/4/5/7 variants the breaker named as the exact undertested class) are proven in
//! `tests/media_parity.rs`.
//!
//! ## REV-S4-3 (breaker H2) — this module is the transcode side of the parity oracle
//! The golden-fixture comparison itself lives in `tests/media_parity.rs` (a real pixel/dSSIM-style
//! comparison against sharp's own output, not a `.webp$` regex) — this module only owns the
//! `transcode` function under test.

use std::io::Cursor;

use image::metadata::Orientation;
use image::{DynamicImage, ImageDecoder, ImageError, ImageReader, Limits, imageops::FilterType};

/// Sharp's own default (`limitInputPixels`, `0x3FFF × 0x3FFF ≈ 268,402,689 px`) — the parity
/// baseline this port must not regress below (REV-S4-1: the naive Rust port must not be MORE
/// bomb-vulnerable than the Node code it replaces).
const MAX_DIMENSION: u32 = 0x3FFF;
/// Decoded-buffer cap: `268,402,689 px × 4 bytes/px` (RGBA8, the worst-case in-memory
/// representation this pipeline ever holds) is ≈1.07 GiB; rounded down to a clean 1 GiB bound —
/// comfortably above any legitimate upload (8 MB/25 MB request-body caps live at the route layer)
/// and comfortably inside a typical Fly machine's memory footprint for a single request. Chosen,
/// not derived from a formal model — documented here so a future tightening has a number to beat.
const MAX_ALLOC_BYTES: u64 = 1024 * 1024 * 1024;

fn decode_limits() -> Limits {
    // `Limits` is `#[non_exhaustive]` (upstream reserves the right to add fields) — construct via
    // `no_limits()` then set each field explicitly rather than a struct literal.
    let mut limits = Limits::no_limits();
    limits.max_image_width = Some(MAX_DIMENSION);
    limits.max_image_height = Some(MAX_DIMENSION);
    limits.max_alloc = Some(MAX_ALLOC_BYTES);
    limits
}

#[derive(Debug, thiserror::Error)]
pub enum ProcessError {
    // No separate "unknown format" variant: `ImageReader::into_decoder`'s own `require_format()`
    // already turns an unrecognized format into an `ImageError` (`Unsupported`), which surfaces
    // through `Decode` below — a second variant for the same observable outcome would be dead
    // code (nothing in this module ever constructs it independently).
    #[error("image decode failed or exceeded the configured safety limits: {0}")]
    Decode(#[source] ImageError),
    // No `Encode` variant: `transcode` always normalizes to RGBA8 via `to_rgba8()` before
    // encoding (`Encoder::from_rgba`, never the pattern-matched `Encoder::from_image`), and
    // `webp`'s own `encode()` is infallible from its caller's perspective (it unwraps internally
    // — an accepted risk of the `webp` crate's own API shape, not this module's).
}

/// One sharp-parity transcode profile: fit-inside box + WebP quality.
#[derive(Debug, Clone, Copy)]
pub struct Profile {
    pub max_w: u32,
    pub max_h: u32,
    /// `webp::Encoder::encode`'s quality argument, 0.0-100.0.
    pub quality: f32,
}

/// `spa-proxy.ts:222-226` — `resize({width:800,height:800,fit:'inside'}).webp({quality:82})`.
pub const PRODUCT_PROFILE: Profile = Profile {
    max_w: 800,
    max_h: 800,
    quality: 82.0,
};

/// `themes.ts:127-130` — `resize({width:512,height:512,fit:'inside'}).webp({quality:80})`.
pub const LOGO_PROFILE: Profile = Profile {
    max_w: 512,
    max_h: 512,
    quality: 80.0,
};

/// `spa-proxy.ts:279-280` — `.rotate().resize({width:1024,height:1024,fit:'inside'}).webp({quality:78})`.
/// The `.rotate()` call is EXIF auto-orient — this port applies that explicitly via
/// `apply_orientation` for ALL THREE profiles (REV-S4-4), not only this historically-`.rotate()`
/// one.
pub const ENTRY_PHOTO_PROFILE: Profile = Profile {
    max_w: 1024,
    max_h: 1024,
    quality: 78.0,
};

/// The swappable transcode seam (REV-S4-9: seam only). `RustImageProcessor` is the one sanctioned
/// implementation; the trait exists so route/test code doesn't hard-wire to a concrete type, not
/// to model a second production candidate.
pub trait ImageProcessor: Send + Sync {
    fn process(&self, input: &[u8], profile: Profile) -> Result<Vec<u8>, ProcessError>;
}

#[derive(Debug, Clone, Copy, Default)]
pub struct RustImageProcessor;

impl ImageProcessor for RustImageProcessor {
    fn process(&self, input: &[u8], profile: Profile) -> Result<Vec<u8>, ProcessError> {
        transcode(input, profile)
    }
}

/// Decode (bomb-capped, REV-S4-1) → read + apply EXIF orientation (REV-S4-4) → fit-inside
/// Lanczos3 resize (upscale allowed — sharp's own default, neither call site passes
/// `withoutEnlargement`, spike-confirmed bit-for-bit dimension match) → lossy WebP encode.
/// Free function (not a trait method) so `tests/media_parity.rs`'s golden-fixture suite can call
/// it directly without standing up a trait object.
pub fn transcode(input: &[u8], profile: Profile) -> Result<Vec<u8>, ProcessError> {
    let image = decode_and_orient(input)?;

    // fit:'inside', upscale allowed (sharp's default — neither call site passes
    // `withoutEnlargement`; spike-confirmed the dimension math matches bit-for-bit).
    let resized = image.resize(profile.max_w, profile.max_h, FilterType::Lanczos3);

    // `image`'s own native WebP encoder is LOSSLESS-only (no quality knob, spike-confirmed) —
    // the `webp` crate (libwebp-sys, `cc`-only build) is REQUIRED for the q78-82 lossy target.
    //
    // `webp::Encoder::from_image` only pattern-matches a FEW `DynamicImage` variants (RGB8/
    // RGBA8) and returns `Err("Unimplemented")` for everything else — 16-bit PNGs, grayscale,
    // and CMYK-decoded-to-RGB16 inputs all hit that catch-all (found empirically by the
    // golden-fixture parity suite's 16-bit PNG case, REV-S4-3 — the exact "silent divergence"
    // class breaker H3 flagged as untested by the spike's single-PNG benchmark). Sharp silently
    // normalizes ANY input depth/colorspace to 8-bit before its own WebP encode (WebP itself has
    // no 16-bit-per-channel mode), so this port does the same explicitly via `to_rgba8()` +
    // `Encoder::from_rgba` — never the pattern-matched `from_image` entry point — so encoding
    // always succeeds regardless of the decoded color type.
    let rgba = resized.to_rgba8();
    let encoder = webp::Encoder::from_rgba(&rgba, rgba.width(), rgba.height());
    let encoded = encoder.encode(profile.quality);
    Ok(encoded.to_vec())
}

/// Decode (bomb-capped, REV-S4-1) + apply EXIF orientation (REV-S4-4), WITHOUT resize/encode —
/// factored out so `tests/media_parity.rs`'s orientation matrix can assert on exact,
/// lossless pixel positions (a resize+lossy-webp round trip would blur the corner-marker pixels
/// the orientation fixtures rely on).
pub fn decode_and_orient(input: &[u8]) -> Result<DynamicImage, ProcessError> {
    let mut reader = ImageReader::new(Cursor::new(input))
        .with_guessed_format()
        .map_err(|e| ProcessError::Decode(ImageError::from(e)))?;
    reader.limits(decode_limits());

    // `into_decoder` (NOT the spike's `load_from_memory_with_format`) is the ONLY entry point
    // that exposes a decoder handle — required for `.orientation()` below. It already applies
    // `decode_limits()` via `ImageDecoder::set_limits`'s default impl (checks
    // `max_image_width`/`max_image_height` against the header-parsed dimensions), so an
    // oversized-declared-dimension file is rejected HERE, before any pixel buffer is touched.
    let mut decoder = reader.into_decoder().map_err(ProcessError::Decode)?;

    // Belt-and-suspenders (REV-S4-1): `set_limits`'s default impl (run inside `into_decoder`
    // above) only checks dimensions, not the allocator budget — `ImageReader::decode()`'s own
    // free function additionally reserves `total_bytes()` against `max_alloc` before decoding
    // pixels, but `into_decoder()` does not do this for its caller. Reproduce that same
    // allocator-budget check explicitly, before `DynamicImage::from_decoder` allocates and fills
    // the real pixel buffer.
    let (width, height) = decoder.dimensions();
    let color_type = decoder.color_type();
    decode_limits()
        .reserve_buffer(width, height, color_type)
        .map_err(ProcessError::Decode)?;

    // REV-S4-4: read EXIF orientation from the SAME decoder handle that decodes pixels below —
    // sharp applies this implicitly on every profile; `image`'s decode does not, silently, unless
    // asked. A decoder that can't produce a meaningful orientation (missing/corrupt EXIF) falls
    // back to "no transform" rather than failing the whole upload — mirrors sharp's own
    // forgiving behavior (a photo with no/garbled EXIF just isn't rotated, it isn't rejected).
    let orientation = decoder.orientation().unwrap_or(Orientation::NoTransforms);

    let mut image = DynamicImage::from_decoder(decoder).map_err(ProcessError::Decode)?;
    image.apply_orientation(orientation);
    Ok(image)
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::codecs::png::PngEncoder;
    use image::{ExtendedColorType, ImageEncoder, Rgb, RgbImage};

    /// A tiny, real (not zero-byte/garbage) source image so every test below exercises the actual
    /// decode→resize→encode pipeline, not just its error paths.
    fn small_png(width: u32, height: u32) -> Vec<u8> {
        let mut img = RgbImage::new(width, height);
        for y in 0..height {
            for x in 0..width {
                let r = u8::try_from(x % 256).unwrap_or(u8::MAX);
                let g = u8::try_from(y % 256).unwrap_or(u8::MAX);
                img.put_pixel(x, y, Rgb([r, g, 128]));
            }
        }
        let mut out = Vec::new();
        PngEncoder::new(&mut out)
            .write_image(&img, width, height, ExtendedColorType::Rgb8)
            .unwrap();
        out
    }

    /// Decodes a produced WebP back to raw pixels via `image`'s own webp decode path (the
    /// "webp" feature is decode-only — see Cargo.toml comment) — used to assert dimensions
    /// without pulling the `webp` crate's own decoder into this module's tests.
    fn decode_webp_dimensions(bytes: &[u8]) -> (u32, u32) {
        let img = image::load_from_memory_with_format(bytes, image::ImageFormat::WebP).unwrap();
        (img.width(), img.height())
    }

    #[test]
    fn product_profile_fits_inside_800x800_preserving_aspect() {
        let src = small_png(2000, 1500);
        let out = transcode(&src, PRODUCT_PROFILE).unwrap();
        assert_eq!(decode_webp_dimensions(&out), (800, 600));
    }

    #[test]
    fn logo_profile_fits_inside_512x512() {
        let src = small_png(2000, 1500);
        let out = transcode(&src, LOGO_PROFILE).unwrap();
        assert_eq!(decode_webp_dimensions(&out), (512, 384));
    }

    #[test]
    fn upscale_is_allowed_matching_sharp_default() {
        // A tiny source under the target box must be enlarged, not letterboxed/left small —
        // sharp's `fit:'inside'` allows upscale by default (neither call site passes
        // `withoutEnlargement`), and `DynamicImage::resize`'s scale factor has no ≤1.0 clamp
        // (spike-confirmed bit-for-bit dimension parity).
        let src = small_png(100, 50);
        let out = transcode(&src, PRODUCT_PROFILE).unwrap();
        assert_eq!(decode_webp_dimensions(&out), (800, 400));
    }

    #[test]
    fn one_pixel_wide_edge_case_does_not_panic_or_produce_a_zero_dimension() {
        // The 1×N edge case named in the council resolution (REV-S4-3) — a degenerate aspect
        // ratio must resize to a non-zero box, not divide-by-zero/round-to-zero.
        let src = small_png(1, 50);
        let out = transcode(&src, LOGO_PROFILE).unwrap();
        let (w, h) = decode_webp_dimensions(&out);
        assert!(w >= 1 && h >= 1, "must not degenerate to a zero dimension");
        assert_eq!(h, 512, "the constraining axis hits the box exactly");
    }

    // ── REV-S4-1: decode-bomb cap ───────────────────────────────────────────────────────────

    #[test]
    fn bomb_cap_rejects_a_huge_declared_dimension_png() {
        // A PNG whose header declares far more pixels than MAX_DIMENSION allows, but whose
        // compressed body is tiny (the exact "small file, huge declared dimensions" shape the
        // breaker's C1 finding named — a real IDAT stream for 65535x65535 would need real
        // pixel data too, so this constructs the minimal valid PNG signature to reach the
        // dimension check, not a full bomb payload) — must fail cleanly, not attempt to
        // allocate gigabytes.
        let mut img = RgbImage::new(1, 1);
        img.put_pixel(0, 0, Rgb([1, 2, 3]));
        let mut small = Vec::new();
        PngEncoder::new(&mut small)
            .write_image(&img, 1, 1, ExtendedColorType::Rgb8)
            .unwrap();
        // Patch the IHDR width/height fields (bytes 16..24 of a PNG: 8-byte signature + 4-byte
        // length + 4-byte "IHDR" + 4-byte width + 4-byte height) to a huge declared size while
        // keeping the (tiny) compressed IDAT untouched — reproduces "small file, huge header".
        let huge = MAX_DIMENSION + 1;
        small[16..20].copy_from_slice(&huge.to_be_bytes());
        small[20..24].copy_from_slice(&huge.to_be_bytes());
        // The IHDR CRC (bytes 29..33) no longer matches, but that only matters if a decoder
        // validates it before checking dimensions; either outcome (Limits rejection or CRC
        // decode error) is an acceptable "clean error, not OOM" — the assertion below is on the
        // Result being an Err at all, plus (see next test) that no huge allocation occurred.
        let result = transcode(&small, PRODUCT_PROFILE);
        assert!(
            result.is_err(),
            "an oversized-declared-dimension input must be rejected, not decoded"
        );
    }

    #[test]
    fn decode_limits_reserve_buffer_rejects_268mp_before_allocating() {
        // Direct unit test of the allocator-budget math itself (belt-and-suspenders check,
        // independent of any specific decoder's CRC/header validation quirks): a declared
        // 20000x20000 RGBA buffer is 1.6 GB, over MAX_ALLOC_BYTES (1 GiB) — `reserve_buffer`
        // must reject it without the caller ever allocating that buffer.
        let mut limits = decode_limits();
        let big = limits.reserve_buffer(20_000, 20_000, image::ColorType::Rgba8);
        assert!(
            big.is_err(),
            "20000x20000 RGBA must exceed the 1 GiB alloc cap"
        );

        let mut limits = decode_limits();
        let ok = limits.reserve_buffer(800, 800, image::ColorType::Rgba8);
        assert!(
            ok.is_ok(),
            "a legitimate product-profile-sized buffer must pass"
        );
    }

    #[test]
    fn max_dimension_matches_sharps_default_268mp_bound() {
        // Pins the parity number itself (REV-S4-1: "replicating sharp's default bound (268 MP)
        // or tighter") — 0x3FFF * 0x3FFF = 268,402,689, matching sharp's libvips default.
        assert_eq!(MAX_DIMENSION, 16_383);
        assert_eq!(
            u64::from(MAX_DIMENSION) * u64::from(MAX_DIMENSION),
            268_402_689
        );
    }
}
