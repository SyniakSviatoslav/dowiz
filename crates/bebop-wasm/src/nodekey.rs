//! The wasm32 reader of the node key (SPEC-BEBOP-DAG-RUNTIME-2026-09-28 §2).
//!
//! A FRAME BUILDER OF ITS OWN, deliberately not `bebop_store::nodekey::Frame`:
//! gate.sh asks four readers to build the three frames of `fixtures/key.expected`
//! from their field values, and a reader that only re-exported the native one
//! would be the native reader counted twice. Lists here are built inside out (the
//! elements into a buffer, then the buffer as one field) where the native builder
//! reserves and back-patches, so a wrong field order or list encoding in either
//! shows up as a disagreement between them.
//!
//! What IS shared is the two hash primitives: `bebop_store::crc32` (zlib CRC-32,
//! already the KV reader's) and `bebop_store::nodekey::sha256` (FIPS 180-4, KAT
//! tested there). They are standard functions with known answers; the frame is
//! the thing DG2 defines, and the frame is built here.
//!
//! `bw_key(which, out)` writes six i64 cells: frame length, K64, and K256 as four
//! big-endian 64-bit words. `harness.mjs --key` prints them as the gate's line.
//! It is a wasm export only under the `key` feature (Cargo.toml): the Worker's
//! module does not carry fixture frames (`bytes.baseline` is unchanged by DG2).

use bebop_store::nodekey::sha256;

/// `len(8, u64 LE) ‖ bytes`.
fn field(out: &mut Vec<u8>, b: &[u8]) {
    out.extend_from_slice(&(b.len() as u64).to_le_bytes());
    out.extend_from_slice(b);
}

/// A number: `len = 8`, then its little-endian bytes.
fn num(out: &mut Vec<u8>, v: i64) {
    field(out, &v.to_le_bytes());
}

const CD: i64 = -7046029254386353131;
const SRC: &[u8] = b"fn add(a: i64, b: i64) -> i64 { a + b }";

fn compile() -> Vec<u8> {
    let mut f = vec![b'C'];
    num(&mut f, CD);
    field(&mut f, SRC);
    num(&mut f, 81985529216486895);
    num(&mut f, -2);
    num(&mut f, 1311768467294899695);
    f
}

/// `inputs` and `params` as lists; `two` = the two-input fixture, else both empty.
fn projection(two: bool) -> Vec<u8> {
    let mut inputs = Vec::new();
    let mut params = Vec::new();
    if two {
        num(&mut inputs, 1234605616436508552);
        num(&mut inputs, 7);
        let digest: [u8; 32] = core::array::from_fn(|i| i as u8);
        field(&mut inputs, &digest);
        num(&mut inputs, 1);
        num(&mut params, 1790000000000);
        num(&mut params, 3);
    }
    let mut f = vec![b'P'];
    num(&mut f, CD);
    num(&mut f, k64(&compile()));
    field(&mut f, &inputs);
    field(&mut f, &params);
    f
}

/// `(crc32 << 32) | (len & 0xffffffff)`.
pub fn k64(frame: &[u8]) -> i64 {
    (((bebop_store::crc32(frame) as u64) << 32) | (frame.len() as u64 & 0xffff_ffff)) as i64
}

/// 0 = compile, 1 = proj, 2 = empty.
pub fn frame(which: i32) -> Option<Vec<u8>> {
    match which {
        0 => Some(compile()),
        1 => Some(projection(true)),
        2 => Some(projection(false)),
        _ => None,
    }
}

/// `[len, K64, K256 words 0..4]` for fixture `which`.
pub fn cells(which: i32) -> Option<[i64; 6]> {
    let f = frame(which)?;
    let d = sha256(&f);
    let mut c = [f.len() as i64, k64(&f), 0, 0, 0, 0];
    for (i, w) in d.chunks_exact(8).enumerate() {
        c[2 + i] = i64::from_be_bytes(w.try_into().expect("8 bytes"));
    }
    Some(c)
}

/// Status for a fixture number that names no frame (after `abi.rs`'s 0..=5).
pub const NO_SUCH_KEY: i32 = 6;

/// Six cells into `out`; status 0, `abi::NULL_ARG` for a null `out`,
/// `NO_SUCH_KEY` for an unknown fixture.
///
/// # Safety
/// `out` must point at six writable i64 cells.
#[cfg_attr(feature = "key", no_mangle)]
pub unsafe extern "C" fn bw_key(which: i32, out: *mut i64) -> i32 {
    if out.is_null() {
        return crate::abi::NULL_ARG;
    }
    match cells(which) {
        Some(c) => {
            core::ptr::copy_nonoverlapping(c.as_ptr(), out, 6);
            crate::abi::OK
        }
        None => NO_SUCH_KEY,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// RT K-3 in this reader: `1 + 5*8 + 8 + len(fn_source) + 8 + 8 + 8`.
    #[test]
    fn compile_frame_length_is_k3() {
        assert_eq!(compile().len(), 1 + 5 * 8 + 8 + SRC.len() + 8 + 8 + 8);
        assert_eq!(cells(0).unwrap()[0] as usize, compile().len());
    }

    #[test]
    fn unknown_fixture_and_null_out_are_statuses_not_traps() {
        let mut out = [0i64; 6];
        assert_eq!(unsafe { bw_key(3, out.as_mut_ptr()) }, NO_SUCH_KEY);
        assert_eq!(unsafe { bw_key(0, core::ptr::null_mut()) }, crate::abi::NULL_ARG);
        assert_eq!(unsafe { bw_key(2, out.as_mut_ptr()) }, crate::abi::OK);
        assert_eq!(out, cells(2).unwrap());
    }
}
