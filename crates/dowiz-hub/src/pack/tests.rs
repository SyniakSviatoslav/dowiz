//! The at-rest envelope (W-SPACE): every shape round-trips byte-equal, an image written before
//! the envelope loads as it is, and a damaged copy is refused by name, never decoded wrong.
use super::*;

/// Deterministic noise (xorshift): bytes LZ4 cannot shrink.
fn noise(n: usize) -> Vec<u8> {
    let mut x: u64 = 0x9E37_79B9_7F4A_7C15;
    (0..n).map(|_| { x ^= x << 13; x ^= x >> 7; x ^= x << 17; x as u8 }).collect()
}

/// An image-like buffer: a header, JSON-ish records that repeat with small changes, a zero tail.
fn image_like() -> Vec<u8> {
    let mut b = b"BEBOPST1\x02\0\0\0\0\0\0\0".to_vec();
    for i in 0..400 {
        b.extend_from_slice(format!(r#"{{"v":1,"at":{},"by":"staff-{}","key":"product:d{}","price":{}}}"#, 1_700_000_000_000u64 + i * 977, i % 3, i % 40, 500 + i).as_bytes());
        b.extend_from_slice(&(i as u32).to_le_bytes());
    }
    b.resize(b.len() + 40_000, 0);
    b
}

fn shapes() -> Vec<(&'static str, Vec<u8>)> {
    vec![
        ("empty", vec![]),
        ("one", vec![7]),
        ("twelve", b"abcdefghijkl".to_vec()),
        ("thirteen-same", vec![b'a'; 13]),
        ("zeros", vec![0; 100_000]),
        ("period-3", b"xyz".repeat(5_000)),
        ("noise", noise(70_000)),
        ("image-like", image_like()),
        // A match longer than 65535 back must not be taken: the same block twice, 70 KB apart.
        ("far-repeat", [noise(70_000), noise(70_000)].concat()),
    ]
}

/// RED-able: make `pack` write the raw length one short and every LZ4 shape refuses.
#[test]
fn every_shape_round_trips_byte_equal() {
    for (name, raw) in shapes() {
        let p = pack(&raw);
        assert!(is_packed(&p), "{name}");
        assert_eq!(unpack(&p).as_deref(), Ok(&raw[..]), "{name}: envelope");
        assert!(p.len() <= raw.len() + HEADER, "{name}: an envelope never costs more than its header");
    }
}

/// RED-able: make `pack` always store (codec 0) and the ratios fall to 1.
#[test]
fn compressible_images_shrink_and_noise_is_stored() {
    let img = image_like();
    let p = pack(&img);
    assert_eq!(p[8], LZ4);
    assert!(p.len() * 4 < img.len(), "image-like: {} -> {}", img.len(), p.len());
    let n = noise(4096);
    assert_eq!(pack(&n)[8], STORED, "noise does not compress, so it is stored");
}

/// RED-able: make `unpack` require the magic (refuse what lacks it) and an old bundle's image
/// stops loading.
#[test]
fn an_image_written_before_the_envelope_loads_as_it_is() {
    let old = image_like();
    assert!(!is_packed(&old));
    let got = unpack(&old).unwrap();
    assert!(matches!(got, Cow::Borrowed(_)), "no copy for an old image");
    assert_eq!(&got[..], &old[..]);
}

/// The packed image with header field bytes `at..at+n` replaced.
fn with(p: &[u8], at: usize, v: &[u8]) -> Vec<u8> {
    let mut c = p.to_vec();
    c[at..at + v.len()].copy_from_slice(v);
    c
}

/// RED-able: skip the crc compare in `unpack` and the changed literal is served as the image.
#[test]
fn a_changed_payload_byte_is_refused_by_the_crc() {
    let raw = image_like();
    let p = pack(&raw);
    assert_eq!(p[8], LZ4);
    // One named byte: the `E` of the image's own `BEBOPST1`, a literal in the first sequence.
    let at = HEADER + p[HEADER..].windows(5).position(|w| w == b"BEBOP").unwrap() + 1;
    assert_eq!(unpack(&with(&p, at, b"e")).err(), Some(PackError::Crc));
    // And the crc field itself changed: the payload is intact, the claim about it is not.
    assert_eq!(unpack(&with(&p, 12, &(crc32fast::hash(&raw) ^ 1).to_le_bytes())).err(), Some(PackError::Crc));
}

/// RED-able: drop the `MAX_RAW` filter and the claim past it on a large payload is allocated and
/// decoded instead of refused (`Body`, not `Length`).
#[test]
fn a_lying_raw_length_is_refused_before_anything_is_allocated() {
    let p = pack(&image_like());
    let lie = |n: u64| unpack(&with(&p, 16, &n.to_le_bytes())).err();
    assert_eq!(lie(1 << 62), Some(PackError::Length(1 << 62)), "past MAX_RAW");
    // The cap on its own: a payload big enough that 255x of it passes MAX_RAW (~263 KB), so only
    // the cap stands between the claim and a 64 MiB allocation.
    let big = pack(&[noise(400_000), vec![0; 20_000]].concat());
    assert_eq!(big[8], LZ4);
    assert!((big.len() - HEADER) * 255 > MAX_RAW, "the payload must not be refused by expansion alone");
    let over = MAX_RAW as u64 + 1;
    assert_eq!(unpack(&with(&big, 16, &over.to_le_bytes())).err(), Some(PackError::Length(over)), "past MAX_RAW");
    // Under the cap but more than this payload could ever expand to.
    let past = ((p.len() - HEADER) * 255 + 1) as u64;
    assert_eq!(lie(past), Some(PackError::Length(past)), "past the payload's expansion");
    // A plausible lie, one byte long or short: the decoder's exact-length check names it.
    let real = image_like().len() as u64;
    assert!(matches!(lie(real + 1), Some(PackError::Body(_))), "{:?}", lie(real + 1));
    assert!(matches!(lie(real - 1), Some(PackError::Body(_))), "{:?}", lie(real - 1));
    // A stored image whose length does not match its payload.
    let s = pack(&noise(4096));
    assert_eq!(s[8], STORED);
    assert_eq!(unpack(&with(&s, 16, &4095u64.to_le_bytes())).err(), Some(PackError::Length(4095)));
}

/// RED-able: map a decoder error to `Ok` (serve what decoded) and the half block is served.
#[test]
fn a_truncated_copy_is_refused_by_name() {
    let p = pack(&image_like());
    assert!(matches!(unpack(&p[..p.len() / 2]), Err(PackError::Body(_))), "half the payload");
    assert!(matches!(unpack(&p[..p.len() - 1]), Err(PackError::Body(_))), "one byte short");
    assert_eq!(unpack(&p[..HEADER - 1]).err(), Some(PackError::Header), "half a header");
    assert_eq!(unpack(&p[..HEADER]).err(), Some(PackError::Length(image_like().len() as u64)), "a header and no payload");
}

/// RED-able: drop the padding check and a foreign envelope (a later version's field there) opens.
#[test]
fn an_unknown_codec_or_padding_is_refused() {
    let p = pack(&image_like());
    assert_eq!(unpack(&with(&p, 8, &[9])).err(), Some(PackError::Codec(9)));
    assert_eq!(unpack(&with(&p, 10, &[1])).err(), Some(PackError::Header));
}

/// THE REAL RATIO, on the images a live venue holds (W-SPACE). Not in the default run: the
/// images are a copy of a venue's data and are never committed. Run with
/// `DOWIZ_SPACE_DIR=<dir of *.img> cargo test --release --lib pack::tests::real -- --ignored --nocapture`.
#[test]
#[ignore]
fn real_images_ratio() {
    let Ok(dir) = std::env::var("DOWIZ_SPACE_DIR") else { panic!("set DOWIZ_SPACE_DIR") };
    let (mut raw_total, mut packed_total) = (0, 0);
    let mut names: Vec<_> = std::fs::read_dir(&dir).unwrap().flatten().map(|e| e.path()).filter(|p| p.extension().is_some_and(|x| x == "img")).collect();
    names.sort();
    for p in names {
        let raw = std::fs::read(&p).unwrap();
        let t = std::time::Instant::now();
        let z = pack(&raw);
        let tc = t.elapsed();
        let t = std::time::Instant::now();
        assert_eq!(unpack(&z).unwrap().as_ref(), &raw[..]);
        let td = t.elapsed();
        println!("{:<22} {:>9} -> {:>8} ({:.2}x)  pack {:?} unpack {:?}", p.file_name().unwrap().to_string_lossy(), raw.len(), z.len(), raw.len() as f64 / z.len() as f64, tc, td);
        raw_total += raw.len();
        packed_total += z.len();
    }
    println!("TOTAL {raw_total} -> {packed_total} ({:.2}x)", raw_total as f64 / packed_total.max(1) as f64);
}

/// The envelope's crc is the zlib CRC-32 `bebop_store::crc32` computes: swapping the crate in
/// changes no packed byte. RED-able: a different polynomial (crc32c) and this fails.
#[test]
fn the_envelope_crc_is_the_zlib_crc32() {
    for (name, raw) in shapes() {
        assert_eq!(crc32fast::hash(&raw), bebop_store::crc32(&raw), "{name}");
    }
}

/// RED-able: drop the `spent + raw.len() <= budget` test (pack everything) and the log-sized
/// image is packed past the budget.
#[test]
fn an_export_packs_smallest_first_and_never_past_its_budget() {
    let (small, mid, big) = (image_like(), vec![7u8; 150_000], [image_like(), vec![0; 300_000]].concat());
    let raws: Vec<&[u8]> = vec![&big, &small, &mid];
    let out = pack_within(&raws, PACK_BUDGET);
    assert_eq!(out.len(), 3);
    let packed: Vec<bool> = out.iter().map(|o| o.1).collect();
    assert_eq!(packed, [false, true, true], "big ({} B) is past the budget once small + mid are spent", big.len());
    let spent: usize = raws.iter().zip(&out).filter(|(_, o)| o.1).map(|(r, _)| r.len()).sum();
    assert!(spent <= PACK_BUDGET, "{spent} raw bytes packed");
    for (raw, (stored, p)) in raws.iter().zip(&out) {
        assert_eq!(is_packed(stored), *p);
        assert_eq!(unpack(stored).unwrap().as_ref(), *raw, "every entry opens to its raw image");
    }
    // Order is the input's, whatever the sizes: a zero budget packs nothing, a huge one all.
    assert!(pack_within(&raws, 0).iter().all(|o| !o.1 && !is_packed(&o.0)));
    assert!(pack_within(&raws, usize::MAX / 2).iter().all(|o| o.1));
}
