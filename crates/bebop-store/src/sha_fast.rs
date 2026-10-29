//! SHA-256 without allocation, a block at a time, on the CPU's SHA instructions when it has them
//! (R-EMBED-STORE §12 P16, W-SPEED-A, 2026-10-29).
//!
//! `nodekey::sha256` copied its input into a `Vec` and padded it a byte per push; the Merkle root
//! over 165 dishes cost 191 us against a 20 us target. Here the full blocks are compressed straight
//! from the caller's slice, and the padding is one or two blocks on the stack.
//!
//! Three compressions of ONE function (FIPS 180-4 §6.2.2):
//!   * AArch64 SHA2 (`sha256h/sha256h2/sha256su0/sha256su1`, feature `sha2`),
//!   * x86_64 SHA-NI (`sha256rnds2/sha256msg1/sha256msg2`, features `sha` + SSSE3 + SSE4.1),
//!   * the scalar rounds (`scalar.rs`) -- the fallback, and the ONLY one compiled for wasm32, so
//!     the reader module `bytes.baseline` ratchets carries no intrinsics.
//! A path is taken when its feature is on at compile time, else when std detects it at run time.
//! `unsafe` lives only in `arm.rs` / `x86.rs`, each block under a SAFETY comment; this file is safe.
//!
//! Proof of sameness: `sha_fast/tests.rs` -- the NIST FIPS 180-4 / CAVP vectors through the
//! dispatching function AND through the scalar function by name, every length 0..300 against the
//! old allocating loop (kept there as the oracle), and the level-wise root against RFC 9162's
//! recursive one.

mod scalar;
#[cfg(target_arch = "aarch64")]
mod arm;
#[cfg(target_arch = "x86_64")]
mod x86;

use scalar::{compress_sw, H0};

/// SHA-256 of `data`.
#[cfg(not(target_arch = "wasm32"))]
#[inline]
pub fn sha256(data: &[u8]) -> [u8; 32] {
    let mut h = H0;
    let whole = data.len() & !63;
    compress(&mut h, &data[..whole]);
    finish(h, &data[whole..], data.len() as u64, compress)
}

/// SHA-256 of `data` on wasm32: the pre-P16 loop, kept there because it is the smallest code and
/// the reader module's bytes are ratcheted (`crates/bebop-wasm/bytes.baseline`); the streaming
/// form cost that module +112 bytes. Speed there is not what P16 measures.
#[cfg(target_arch = "wasm32")]
pub fn sha256(data: &[u8]) -> [u8; 32] {
    let mut h = H0;
    let mut msg = data.to_vec();
    msg.push(0x80);
    while msg.len() % 64 != 56 {
        msg.push(0);
    }
    msg.extend_from_slice(&((data.len() as u64).wrapping_mul(8)).to_be_bytes());
    compress_sw(&mut h, &msg);
    digest(&h)
}

/// The same, on the scalar rounds only -- the control for the bench and the tests.
#[allow(dead_code)]
pub(crate) fn sha256_sw(data: &[u8]) -> [u8; 32] {
    let whole = data.len() & !63;
    let mut h = H0;
    compress_sw(&mut h, &data[..whole]);
    finish(h, &data[whole..], data.len() as u64, compress_sw)
}

/// Which compression `sha256` takes on this CPU.
#[allow(dead_code)]
pub(crate) fn path() -> &'static str {
    #[cfg(target_arch = "aarch64")]
    {
        if arm::active() {
            return "aarch64-sha2";
        }
    }
    #[cfg(target_arch = "x86_64")]
    {
        if x86::active() {
            return "x86_64-shani";
        }
    }
    "sw"
}

/// The last `rest` (< 64) bytes and the padding: 0x80, zeros, the bit length big-endian in the
/// last 8 bytes -- one block, or two when `rest` leaves fewer than 9 bytes free.
#[inline(always)]
fn finish(mut h: [u32; 8], rest: &[u8], total: u64, f: fn(&mut [u32; 8], &[u8])) -> [u8; 32] {
    let fill = rest.len();
    let mut tail = [0u8; 128];
    tail[..fill].copy_from_slice(rest);
    tail[fill] = 0x80;
    let n = if fill < 56 { 64 } else { 128 };
    tail[n - 8..n].copy_from_slice(&total.wrapping_mul(8).to_be_bytes());
    f(&mut h, &tail[..n]);
    digest(&h)
}

/// RFC 9162 node hash `sha256(0x01 ‖ l ‖ r)`: 65 bytes, so exactly two padded blocks.
#[cfg_attr(target_arch = "wasm32", allow(dead_code))]
fn node_msg(l: &[u8; 32], r: &[u8; 32]) -> [u8; 128] {
    let mut m = [0u8; 128];
    m[0] = 0x01;
    m[1..33].copy_from_slice(l);
    m[33..65].copy_from_slice(r);
    m[65] = 0x80;
    m[120..].copy_from_slice(&(65u64 * 8).to_be_bytes());
    m
}

fn digest(h: &[u32; 8]) -> [u8; 32] {
    let mut out = [0u8; 32];
    for (i, x) in h.iter().enumerate() {
        out[4 * i..4 * i + 4].copy_from_slice(&x.to_be_bytes());
    }
    out
}

/// Two node hashes at once (interleaved on AArch64, one after the other elsewhere).
#[cfg_attr(target_arch = "wasm32", allow(dead_code))]
fn node2(a: (&[u8; 32], &[u8; 32]), b: (&[u8; 32], &[u8; 32])) -> ([u8; 32], [u8; 32]) {
    let (ma, mb) = (node_msg(a.0, a.1), node_msg(b.0, b.1));
    let (mut ha, mut hb) = (H0, H0);
    #[cfg(target_arch = "aarch64")]
    {
        if arm::compress2(&mut ha, &ma, &mut hb, &mb) {
            return (digest(&ha), digest(&hb));
        }
    }
    compress(&mut ha, &ma);
    compress(&mut hb, &mb);
    (digest(&ha), digest(&hb))
}

/// The RFC 9162 Merkle tree hash over leaf hashes -- `merkle::root`, computed level by level
/// bottom-up (pair neighbours, carry an odd last node up unchanged), which is the same tree as
/// splitting at the largest power of two below n: every left subtree of MTH is complete. Two
/// nodes of a level are hashed side by side. `merkle_root_equals_the_recursive_tree` pins it.
#[cfg_attr(target_arch = "wasm32", allow(dead_code))]
pub(crate) fn merkle_root(leaves: &[[u8; 32]]) -> [u8; 32] {
    match leaves.len() {
        0 => return sha256(&[]),
        1 => return leaves[0],
        _ => {}
    }
    let mut cur = leaves.to_vec();
    while cur.len() > 1 {
        let n = cur.len();
        let pairs = n / 2;
        let mut i = 0;
        // Node i reads 2i, 2i+1 (>= i), so overwriting in place never clobbers an unread input.
        while i + 1 < pairs {
            let (x, y) = node2((&cur[2 * i], &cur[2 * i + 1]), (&cur[2 * i + 2], &cur[2 * i + 3]));
            cur[i] = x;
            cur[i + 1] = y;
            i += 2;
        }
        if i < pairs {
            let mut h = H0;
            compress(&mut h, &node_msg(&cur[2 * i], &cur[2 * i + 1]));
            cur[i] = digest(&h);
        }
        if n % 2 == 1 {
            cur[pairs] = cur[n - 1];
            cur.truncate(pairs + 1);
        } else {
            cur.truncate(pairs);
        }
    }
    cur[0]
}

/// Compress `blocks` (a multiple of 64 bytes) into `h` on the best path this CPU has.
fn compress(h: &mut [u32; 8], blocks: &[u8]) {
    #[cfg(target_arch = "aarch64")]
    {
        if arm::compress(h, blocks) {
            return;
        }
    }
    #[cfg(target_arch = "x86_64")]
    {
        if x86::compress(h, blocks) {
            return;
        }
    }
    compress_sw(h, blocks)
}

#[cfg(test)]
mod tests;

