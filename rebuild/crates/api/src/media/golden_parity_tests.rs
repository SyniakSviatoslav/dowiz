//! REV-S4-3 golden-fixture parity suite — sharp is the INDEPENDENT oracle (fixtures generated
//! ONCE via `tests/fixtures/media/generate-goldens.mjs` against this repo's pinned sharp 0.34.5,
//! committed as bytes below). Asserts EXACT dimension parity + a quantified, documented
//! perceptual tolerance (mean per-channel absolute delta) — NOT the E2E net's `\.webp$` regex
//! (breaker H2's "null oracle" finding). Covers all six input classes the council resolution
//! names (phone JPEG w/ EXIF, PNG, odd aspect ratio, 1×N edge, CMYK JPEG, 16-bit PNG) plus the
//! JPEG decode path the Q3 spike itself never exercised.
//!
//! ## Empirical finding folded into this suite (see `phone_jpeg_with_exif_orientation` below)
//! Verified against the ACTUAL pinned sharp version: `sharp(buf).resize(...).webp(...)` does
//! **NOT** auto-apply EXIF orientation without an explicit `.rotate()` call in the pipeline. The
//! old `spa-proxy.ts:222-226` product-image route (and `themes.ts:127-130` theme-logo) never
//! call `.rotate()` — so TODAY they do not correct orientation either; only entry-photo
//! (`spa-proxy.ts:279`) does. REV-S4-4 mandates this Rust port apply orientation correction on
//! ALL THREE profiles regardless — a deliberate IMPROVEMENT over current Node behavior for
//! product/logo, not a parity preservation. The phone-JPEG golden is therefore generated WITH an
//! explicit `.rotate()` (see the generator script's own comment) — it is the oracle for "does the
//! resize+encode match sharp given correctly-oriented pixels", which is what REV-S4-4 asks this
//! port to produce.
//!
//! ## REV-S4-4 — all 8 EXIF orientation values (`orientation_matrix` below)
//! Each `orient-N.jpg` fixture stores the SAME four quadrant colors permuted so that applying
//! EXIF orientation value N recovers the identical canonical arrangement (red/green/blue/yellow,
//! top-left/top-right/bottom-left/bottom-right) — the permutations were hand-derived from the
//! EXIF orientation standard's transform table (the same one `image::metadata::Orientation`
//! documents), independent of both sharp's and this crate's own code, so this is a real
//! (non-circular) check that `decode_and_orient` applies each of the 8 values correctly,
//! including the mirrored 2/4/5/7 variants breaker H3 named as the exact undertested class.

use super::processor::{self, PRODUCT_PROFILE};

/// Chosen, not derived from a formal perceptual model (documented per the Task-Exit Rule): sharp/
/// libvips and this port's `webp` crate both ultimately encode through the SAME libwebp library,
/// but the resize kernel (Lanczos3 here vs libvips' default lanczos3 — expected close, not
/// guaranteed identical) and JPEG decode (`zune-jpeg` vs libjpeg-turbo/mozjpeg) differ, so some
/// pixel drift is expected. 10/255 (~4%) mean absolute per-channel difference absorbs that drift
/// while still catching a genuinely wrong decode (wrong colorspace, a mis-scaled channel, a
/// resize bug), which would push the mean delta far higher than a few percent.
const MEAN_CHANNEL_DELTA_TOLERANCE: f64 = 10.0;

fn decode_webp_rgba(bytes: &[u8]) -> image::RgbaImage {
    image::load_from_memory_with_format(bytes, image::ImageFormat::WebP)
        .expect("golden/produced webp must decode")
        .to_rgba8()
}

/// Runs `transcode` on `input` (product profile) and asserts it matches `golden_webp` (sharp's
/// own output) on BOTH exact dimensions and the documented perceptual tolerance.
fn assert_parity(input: &[u8], golden_webp: &[u8], label: &str) {
    let produced = processor::transcode(input, PRODUCT_PROFILE)
        .unwrap_or_else(|e| panic!("{label}: transcode failed: {e}"));
    let produced_img = decode_webp_rgba(&produced);
    let golden_img = decode_webp_rgba(golden_webp);

    assert_eq!(
        (produced_img.width(), produced_img.height()),
        (golden_img.width(), golden_img.height()),
        "{label}: dimension parity against sharp's golden output"
    );

    let mut total_delta = 0f64;
    let mut count: u32 = 0;
    for (p, g) in produced_img.pixels().zip(golden_img.pixels()) {
        for c in 0..4 {
            total_delta += (f64::from(p[c]) - f64::from(g[c])).abs();
            count += 1;
        }
    }
    let mean_delta = total_delta / f64::from(count);
    assert!(
        mean_delta <= MEAN_CHANNEL_DELTA_TOLERANCE,
        "{label}: mean per-channel delta {mean_delta:.2} exceeds the documented tolerance \
         {MEAN_CHANNEL_DELTA_TOLERANCE} (a quantified perceptual bound, not a null oracle)"
    );
}

#[test]
fn phone_jpeg_with_exif_orientation() {
    assert_parity(
        include_bytes!("../../tests/fixtures/media/phone-jpeg-exif6.jpg"),
        include_bytes!("../../tests/fixtures/media/phone-jpeg-exif6-golden.webp"),
        "phone-jpeg-exif6",
    );
}

#[test]
fn plain_png() {
    assert_parity(
        include_bytes!("../../tests/fixtures/media/plain.png"),
        include_bytes!("../../tests/fixtures/media/plain-png-golden.webp"),
        "plain-png",
    );
}

#[test]
fn odd_aspect_ratio() {
    assert_parity(
        include_bytes!("../../tests/fixtures/media/odd-aspect.png"),
        include_bytes!("../../tests/fixtures/media/odd-aspect-golden.webp"),
        "odd-aspect",
    );
}

#[test]
fn one_by_n_edge_case() {
    assert_parity(
        include_bytes!("../../tests/fixtures/media/one-by-n.png"),
        include_bytes!("../../tests/fixtures/media/one-by-n-golden.webp"),
        "one-by-n",
    );
}

#[test]
fn cmyk_jpeg() {
    // Breaker H2 named CMYK/progressive/ICC handling as UNVERIFIED against the spike (which only
    // ever decoded a self-generated RGB PNG). This proves `zune-jpeg` (via `image`) can decode a
    // real Adobe-style CMYK JPEG at all AND that the produced colors are within tolerance of
    // sharp's own CMYK-aware decode — closing that specific evidence gap, not just asserting it
    // doesn't panic.
    assert_parity(
        include_bytes!("../../tests/fixtures/media/cmyk.jpg"),
        include_bytes!("../../tests/fixtures/media/cmyk-golden.webp"),
        "cmyk-jpeg",
    );
}

#[test]
fn sixteen_bit_png() {
    assert_parity(
        include_bytes!("../../tests/fixtures/media/sixteen-bit.png"),
        include_bytes!("../../tests/fixtures/media/sixteen-bit-golden.webp"),
        "sixteen-bit-png",
    );
}

// ── REV-S4-4: all 8 EXIF orientation values ─────────────────────────────────────────────────

const RED: [u8; 3] = [237, 28, 36];
const GREEN: [u8; 3] = [34, 177, 76];
const BLUE: [u8; 3] = [63, 72, 204];
const YELLOW: [u8; 3] = [255, 242, 0];

/// Samples the center pixel of each 24px quadrant in the 48x48 fixture (well away from the JPEG
/// block-boundary seam at the exact midline) and asserts it matches the CANONICAL arrangement —
/// identical across all 8 fixtures if `decode_and_orient` applies each value correctly.
fn assert_canonical_quadrants(image: &image::DynamicImage, label: &str) {
    let rgba = image.to_rgba8();
    let sample = |x: u32, y: u32| -> [u8; 3] {
        let p = rgba.get_pixel(x, y);
        [p[0], p[1], p[2]]
    };
    let tolerance = 25i32; // JPEG q95 quantization noise at a flat-block center; generous but real.
    let close = |a: [u8; 3], b: [u8; 3]| {
        a.iter()
            .zip(b.iter())
            .all(|(x, y)| (i32::from(*x) - i32::from(*y)).abs() <= tolerance)
    };
    let (tl, tr, bl, br) = (
        sample(12, 12),
        sample(36, 12),
        sample(12, 36),
        sample(36, 36),
    );
    assert!(close(tl, RED), "{label}: top-left must be RED, got {tl:?}");
    assert!(
        close(tr, GREEN),
        "{label}: top-right must be GREEN, got {tr:?}"
    );
    assert!(
        close(bl, BLUE),
        "{label}: bottom-left must be BLUE, got {bl:?}"
    );
    assert!(
        close(br, YELLOW),
        "{label}: bottom-right must be YELLOW, got {br:?}"
    );
}

macro_rules! orientation_test {
    ($name:ident, $file:literal, $label:literal) => {
        #[test]
        fn $name() {
            let bytes = include_bytes!($file);
            let image = processor::decode_and_orient(bytes)
                .unwrap_or_else(|e| panic!("{}: decode_and_orient failed: {e}", $label));
            assert_canonical_quadrants(&image, $label);
        }
    };
}

orientation_test!(
    orientation_1_no_transform,
    "../../tests/fixtures/media/orient-1.jpg",
    "orientation-1"
);
orientation_test!(
    orientation_2_flip_horizontal,
    "../../tests/fixtures/media/orient-2.jpg",
    "orientation-2"
);
orientation_test!(
    orientation_3_rotate_180,
    "../../tests/fixtures/media/orient-3.jpg",
    "orientation-3"
);
orientation_test!(
    orientation_4_flip_vertical,
    "../../tests/fixtures/media/orient-4.jpg",
    "orientation-4"
);
orientation_test!(
    orientation_5_rotate90_flip_h,
    "../../tests/fixtures/media/orient-5.jpg",
    "orientation-5"
);
orientation_test!(
    orientation_6_rotate90,
    "../../tests/fixtures/media/orient-6.jpg",
    "orientation-6"
);
orientation_test!(
    orientation_7_rotate270_flip_h,
    "../../tests/fixtures/media/orient-7.jpg",
    "orientation-7"
);
orientation_test!(
    orientation_8_rotate270,
    "../../tests/fixtures/media/orient-8.jpg",
    "orientation-8"
);
