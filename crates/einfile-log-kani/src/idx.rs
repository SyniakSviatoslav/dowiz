//! `einfile_image::{idx, idx_end, idx_u}`, the one door from a cell to an index
//! (`tools/gates/cell-usize.sh`). A cell is any i64; `as usize` keeps 32 bits on wasm32, so the
//! reader there followed a ref every other reader called outside the image.
//!
//! BOUNDS: NONE on the values. Each harness covers every i64 / u64, no loop, so no unwind. The limit
//! is a different one, and it is the point of this file:
//!
//! KANI DOES NOT CHECK ON A 32-BIT TARGET. Its documented targets are 64-bit hosts, so `usize` in a
//! harness is 64 bits and the real `idx` can never see the failure it exists for: no cell above
//! 2^32 is "too wide". The wasm32 half is therefore checked on a MODEL, in three steps:
//!   1. `idx_on::<U>` is the body of `idx` with `usize` as a parameter.
//!   2. `model_agrees_with_real_at_host_width` ties it to the real function at `U = usize`.
//!   3. the 32-bit harnesses run `U = u32`.
//! So what they would establish is "this body, at 32 bits", not "the wasm32 build". If the body of
//! `idx` changes, step 2 fails, which is what keeps the copy honest. The shipped wasm32 code itself
//! stays covered only by whatever runs on wasm32.

use einfile_image::{idx, idx_end, idx_u};

// The harnesses below read `usize` as 64 bits. A 32-bit Kani target must break the build, not pass
// these with a different meaning.
const _: () = assert!(usize::BITS == 64);

const TWO_32: i64 = 1 << 32;

/// `idx`, the machine width a parameter.
fn idx_on<U: TryFrom<i64>>(cell: i64) -> Option<U> {
    U::try_from(cell).ok()
}

fn idx_u_on<U: TryFrom<u64>>(field: u64) -> Option<U> {
    U::try_from(field).ok()
}

/// `idx_end` on a 32-bit machine.
fn idx_end_32(cell: i64) -> u32 {
    idx_on::<u32>(cell).unwrap_or(u32::MAX)
}

/// PROPERTY (host width): `idx` is `None` exactly on negatives and otherwise returns the same
/// number, truncating nothing. Every i64.
#[kani::proof]
fn idx_is_none_on_negatives_and_exact_otherwise() {
    let cell: i64 = kani::any();
    match idx(cell) {
        None => assert!(cell < 0),
        Some(v) => {
            assert!(cell >= 0);
            assert_eq!(v as u64, cell as u64);
        }
    }
}

/// PROPERTY (host width): `idx_end` is `idx` with outside mapped past every bound: a negative cell
/// is `usize::MAX`, so `idx_end(c) < len` is false for it whatever `len` is. Every i64.
#[kani::proof]
fn idx_end_reads_outside_as_past_every_bound() {
    let cell: i64 = kani::any();
    let len: usize = kani::any();
    assert_eq!(idx_end(cell), idx(cell).unwrap_or(usize::MAX));
    if cell < 0 {
        assert!(idx_end(cell) >= len);
    }
}

/// PROPERTY (host width): `idx_u` keeps the value, and is total on 64 bits. Every u64.
#[kani::proof]
fn idx_u_is_exact_at_host_width() {
    let field: u64 = kani::any();
    assert_eq!(idx_u(field).map(|v| v as u64), Some(field));
}

/// PROPERTY: the model IS the real body at host width, for every i64 and u64. Everything below
/// rests on this one: without it they would be about a function nobody calls.
#[kani::proof]
fn model_agrees_with_real_at_host_width() {
    let (cell, field): (i64, u64) = (kani::any(), kani::any());
    assert_eq!(idx_on::<usize>(cell), idx(cell));
    assert_eq!(idx_on::<usize>(cell).unwrap_or(usize::MAX), idx_end(cell));
    assert_eq!(idx_u_on::<usize>(field), idx_u(field));
}

/// PROPERTY (32-bit model): `Some` exactly on `0..2^32`, value kept. A cell at or above 2^32 does
/// not wrap to a small index. Every i64. The cover shows the defect is really in the input space:
/// such a cell whose `as u32` would have landed inside a small image.
#[kani::proof]
fn idx_32_never_wraps() {
    let cell: i64 = kani::any();
    kani::cover!(cell >= TWO_32 && (cell as u32) < 1024, "a cell that `as u32` would wrap into the image");
    match idx_on::<u32>(cell) {
        Some(v) => {
            assert!((0..TWO_32).contains(&cell));
            assert_eq!(i64::from(v), cell);
        }
        None => assert!(!(0..TWO_32).contains(&cell)),
    }
}

/// PROPERTY (32-bit model): the named edges. The solver is not needed for these; they are here
/// because a boundary stated by value is what a reader looks for first.
#[kani::proof]
fn idx_32_at_the_named_boundaries() {
    assert_eq!(idx_on::<u32>(i64::from(i32::MAX)), Some(0x7FFF_FFFF));
    assert_eq!(idx_on::<u32>(1 << 31), Some(0x8000_0000));
    assert_eq!(idx_on::<u32>(TWO_32 - 1), Some(u32::MAX));
    assert_eq!(idx_on::<u32>(TWO_32), None);
    assert_eq!(idx_on::<u32>(TWO_32 + 1), None);
    assert_eq!(idx_on::<u32>(i64::MAX), None);
    assert_eq!(idx_on::<u32>(-1), None);
    assert_eq!(idx_on::<u32>(i64::MIN), None);
    assert_eq!(idx_u_on::<u32>(u64::from(u32::MAX)), Some(u32::MAX));
    assert_eq!(idx_u_on::<u32>(1u64 << 32), None);
    assert_eq!(idx_u_on::<u32>(u64::MAX), None);
}

/// PROPERTY (32-bit model against the real function at 64): the wasm32 reader and a 64-bit reader
/// agree on "is this cell inside `len`", for every i64 and every `len`, under both forms of the
/// test. That agreement is the actual requirement behind `idx`.
///
/// `<` needs nothing. `<=` needs `len != u32::MAX`: the sentinel of `idx_end` is a real value at 32
/// bits, so a cell outside `0..2^32` reads as equal to a `len` of exactly `u32::MAX`. That `len` is
/// 2^32 - 1 cells, above anything wasm32 can address, so it is excluded, and the cover below keeps
/// the exclusion visible: the exception is real and is this one value.
#[kani::proof]
fn idx_32_and_64_agree_on_inside() {
    let cell: i64 = kani::any();
    let len: u32 = kani::any();
    let (at32, at64) = (idx_end_32(cell), idx_end(cell));
    assert_eq!(at32 < len, at64 < len as usize);
    kani::cover!(len == u32::MAX && (at32 <= len) != (at64 <= len as usize), "the `<=` exception is real");
    if len != u32::MAX {
        assert_eq!(at32 <= len, at64 <= len as usize);
    }
}
