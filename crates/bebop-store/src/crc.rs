//! zlib CRC-32, TABLE-DRIVEN (W-CRC, 2026-10-05).
//!
//! The format's checksum was a table-free bitwise loop (eight shift/xor rounds per
//! byte, ~190 MB/s measured on the phone, R-BEBOPDB X4: 0.9-2.3 ms for every object of
//! a catalogue). That was affordable while only the WRITER computed it. Now every
//! loader checks the crc of every object it reads (`verify.rs`), so the same loop runs
//! on the read path of every request that loads an image.
//!
//! A cell is exactly eight little-endian bytes, which is exactly one step of
//! slice-by-8: `step_cell` folds a whole cell with eight table lookups and no inner
//! loop. `step_bytes` is the classic one-table, one-byte-at-a-time form for byte
//! slices (superblocks go through cells too; only `crc32(&[u8])` uses it).
//!
//! BYTE-IDENTICAL TO zlib, and that is the whole contract: every image already in
//! storage was sealed with the bitwise loop, and bebop's `crc32x` and the python
//! oracle compute `zlib.crc32`. `tests::table_matches_the_bitwise_loop` pins both
//! steps against the old loop and against zlib's check value for "123456789".

/// The reflected zlib polynomial.
const POLY: u32 = 0xEDB8_8320;

const fn tables() -> [[u32; 256]; 8] {
    let mut t = [[0u32; 256]; 8];
    let mut i = 0;
    while i < 256 {
        let mut c = i as u32;
        let mut k = 0;
        while k < 8 {
            c = if c & 1 != 0 { (c >> 1) ^ POLY } else { c >> 1 };
            k += 1;
        }
        t[0][i] = c;
        i += 1;
    }
    let mut s = 1;
    while s < 8 {
        let mut i = 0;
        while i < 256 {
            let p = t[s - 1][i];
            t[s][i] = (p >> 8) ^ t[0][(p & 0xFF) as usize];
            i += 1;
        }
        s += 1;
    }
    t
}

/// `T[0]` is the one-byte table; `T[k]` advances a byte through k more zero bytes.
static T: [[u32; 256]; 8] = tables();

/// The running register over more bytes, one byte per lookup.
#[inline]
pub(crate) fn step_bytes(mut c: u32, bytes: &[u8]) -> u32 {
    for &b in bytes {
        c = T[0][((c ^ b as u32) & 0xFF) as usize] ^ (c >> 8);
    }
    c
}

/// The running register over ONE CELL: its eight little-endian bytes, slice-by-8.
#[inline]
pub(crate) fn step_cell(c: u32, v: i64) -> u32 {
    let x = v as u64;
    let lo = (x as u32) ^ c;
    let hi = (x >> 32) as u32;
    T[7][(lo & 0xFF) as usize]
        ^ T[6][((lo >> 8) & 0xFF) as usize]
        ^ T[5][((lo >> 16) & 0xFF) as usize]
        ^ T[4][(lo >> 24) as usize]
        ^ T[3][(hi & 0xFF) as usize]
        ^ T[2][((hi >> 8) & 0xFF) as usize]
        ^ T[1][((hi >> 16) & 0xFF) as usize]
        ^ T[0][(hi >> 24) as usize]
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The loop every image in storage was sealed with, kept here as the oracle.
    fn bitwise(mut c: u32, bytes: &[u8]) -> u32 {
        for &b in bytes {
            c ^= b as u32;
            for _ in 0..8 {
                let m = (c & 1).wrapping_neg();
                c = (c >> 1) ^ (POLY & m);
            }
        }
        c
    }

    #[test]
    fn table_matches_the_bitwise_loop() {
        // zlib's own check value.
        assert_eq!(!step_bytes(!0, b"123456789"), 0xCBF4_3926);
        assert_eq!(!bitwise(!0, b"123456789"), 0xCBF4_3926);
        assert_eq!(crate::crc32(b""), 0);
        // Every cell shape that matters: zero, all ones, each byte lane, negatives,
        // and a deterministic spread -- through both steps, against the old loop.
        let mut cells: Vec<i64> = vec![0, -1, 1, i64::MIN, i64::MAX, 0x0102_0304_0506_0708];
        for k in 0..64 {
            cells.push(1i64 << k);
        }
        let mut x: u64 = 0x9E37_79B9_7F4A_7C15;
        for _ in 0..512 {
            x ^= x << 13;
            x ^= x >> 7;
            x ^= x << 17;
            cells.push(x as i64);
        }
        let (mut a, mut b, mut c) = (!0u32, !0u32, !0u32);
        for &v in &cells {
            a = step_cell(a, v);
            b = step_bytes(b, &v.to_le_bytes());
            c = bitwise(c, &v.to_le_bytes());
            assert_eq!(a, c, "slice-by-8 diverged at {v:#x}");
            assert_eq!(b, c, "slice-by-1 diverged at {v:#x}");
        }
    }
}
