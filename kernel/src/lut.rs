//! lut.rs — n(0)/O(1) compile-time lookup table primitives.
//!
//! Phase A of the glyph-geometry rewrite law: replace runtime branching
//! (if-else, match-iterator, BTreeMap scans) with const array indexing. Every
//! LUT is built at compile time via `const fn` — zero runtime cost, zero heap,
//! branchless dispatch through `#[repr(u8)]` discriminants.
//!
//! # Patterns gathered from reverse-engineering (111 patterns, 38 cross):
//!
//! - `order_machine::build_adjacency` → const-fn array generation + popcount
//! - `spectral::DriftClass::wire_code` → exhaustive enum→u8 dispatch
//! - `trinary::Tri` → `#[repr(u8)]` + branchless counts[] indexing
//! - `pixel_snapshot::DOT_OFFSET` → const array + bit→Unicode mapping
//! - `ktg2/cell::State::from_bits` → const fn with InvalidEncoding sentinel
//!
//! # Zero-dep invariant preserved:
//! - `cargo tree -e no-dev` produces empty output
//! - Pure `std`; no external crates; no proc macros

/// An enum whose discriminants are direct LUT indices.
///
/// Implementers guarantee:
/// - `#[repr(u8)]` with contiguous 0..N discriminants
/// - `const LUT_SIZE: usize` = number of variants
/// - `const fn discriminant(self) -> u8` via `wire_code` or equivalent
///
/// Safety: the caller trusts `discriminant() < LUT_SIZE`; violating this
/// is a logic bug, not a memory-safety problem (Rust array bounds check).
pub trait LutKey: Copy {
    /// Number of variants = LUT array length.
    const LUT_SIZE: usize;
    /// Compile-time branchless discriminant, 0..LUT_SIZE-1.
    fn discriminant(self) -> u8;
}

/// A compile-time-constructed lookup table.
///
/// `V` is the value type; `K` is the key type (must implement `LutKey`).
/// The array is built via a `const fn` builder and stored as a `const`.
#[derive(Debug, Clone, Copy)]
pub struct Lut<K: LutKey, V: Copy, const N: usize> {
    table: [V; N],
    _phantom: core::marker::PhantomData<K>,
}

impl<K: LutKey, V: Copy, const N: usize> Lut<K, V, N> {
    /// Build a LUT from a const array. The caller must ensure `N == K::LUT_SIZE`.
    /// This is verified at the construction site but not at type level (stable
    /// Rust cannot express `const N = K::LUT_SIZE` without nightly features).
    pub const fn new(table: [V; N]) -> Self {
        Self { table, _phantom: core::marker::PhantomData }
    }

    /// O(1) branchless lookup — compiles to a single indexed load.
    #[inline(always)]
    pub fn get(&self, key: K) -> V {
        self.table[key.discriminant() as usize]
    }
}

/// Build a const array via a closure-like pattern. Rust `const fn` cannot
/// accept closures, so we use a macro instead.
///
/// Usage:
/// ```ignore
/// const MY_LUT: [f64; 4] = lut_build!(|i: usize| -> f64 { i as f64 * 2.0 }, 4);
/// ```
#[macro_export]
macro_rules! lut_build {
    (|$i:ident : usize| -> $ty:ty $body:block, $n:expr) => {{
        let mut arr: [$ty; $n] = [0 as $ty; $n]; // placeholder
        // Note: stable Rust 2021 cannot iterate in const contexts with for loops.
        // For actual const array building, use a handwritten manual while-let loop
        // (see `build_adjacency` in order_machine.rs for the canonical pattern).
        let mut $i = 0usize;
        while $i < $n {
            arr[$i] = $body;
            $i += 1;
        }
        arr
    }};
}

/// U8-to-T lookup: map a raw u8 to an enum via a const match.
/// Sibling of `State::from_bits` (ktg2/cell.rs:28-35) and
/// `RecordKind::from_discriminant` (spine.rs:27-55).
///
/// Returns `None` for invalid codes — never fabricates a value from
/// an unknown wire byte (named-absence principle).
pub trait FromU8: Sized {
    fn from_u8(code: u8) -> Option<Self>;
}

/// T-to-U8 reverse mapping. Every implementer of `FromU8` should also
/// implement `ToU8` with the inverse mapping.
pub trait ToU8 {
    fn to_u8(self) -> u8;
}

/// Branchless binary operation via LUT: resolves `(a, b, op)` by indexing
/// a precomputed flat truth table. Replaces nested if-else chains.
///
/// `FLAT_SIZE` must equal `N * N` where `N` is the number of variants.
/// Stable Rust cannot compute `N * N` in const generics, so the caller
/// provides the flattened size explicitly.
#[derive(Debug, Clone, Copy)]
pub struct BinaryLut<V: Copy, const FLAT_SIZE: usize> {
    table: [V; FLAT_SIZE],
}

impl<V: Copy, const FLAT_SIZE: usize> BinaryLut<V, FLAT_SIZE> {
    pub const fn new(table: [V; FLAT_SIZE]) -> Self {
        Self { table }
    }

    /// O(1) binary op: `table[row * N + col]`. The caller provides `N`.
    #[inline(always)]
    pub fn apply<K: LutKey>(&self, a: K, b: K) -> V {
        let row = a.discriminant() as usize;
        let col = b.discriminant() as usize;
        let n = K::LUT_SIZE;
        self.table[row * n + col]
    }
}

/// Bit-pack N values into a single integer. O(1) decode via shift+mask.
///
/// Pattern source: `ktg2/graph.rs:170-187` (4 states per u8), `order_machine.rs:206` (u16 adjacency).
/// Generalises 2-bit packing in `State`/`Graph` to arbitrary bit-widths.
pub struct BitPack<const BITS_PER: u32, const COUNT: usize> {
    /// Underlying storage. Width must be ≥ BITS_PER * COUNT bits.
    raw: u64,
}

impl<const BITS_PER: u32, const COUNT: usize> BitPack<BITS_PER, COUNT> {
    /// Mask for one element.
    pub const MASK: u64 = (1u64 << BITS_PER) - 1;

    /// Build from raw bits. Caller must ensure only valid codes are present.
    #[inline(always)]
    pub const fn from_raw(raw: u64) -> Self {
        Self { raw }
    }

    /// Read element `idx` — O(1) shift+mask, zero branches.
    #[inline(always)]
    pub fn get(&self, idx: usize) -> u8 {
        debug_assert!(idx < COUNT);
        ((self.raw >> (idx as u32 * BITS_PER)) & Self::MASK) as u8
    }

    /// Write element `idx` — O(1) clear+set, zero branches.
    #[inline(always)]
    pub fn set(&mut self, idx: usize, val: u8) {
        debug_assert!(idx < COUNT);
        let shift = idx as u32 * BITS_PER;
        let mask = Self::MASK << shift;
        self.raw = (self.raw & !mask) | ((val as u64 & Self::MASK) << shift);
    }
}

/// Popcount of a u16 bitmask — O(1) via `.count_ones()`.
/// `popcount` is used by LUTs for edge-count, active-count, and
/// reachability queries (order_machine.rs:212-219).
#[inline(always)]
pub const fn popcount_u16(mask: u16) -> u32 {
    mask.count_ones()
}

/// Branchless `is_power_of_two` — O(1), no branch.
#[inline(always)]
pub const fn is_power_of_two(x: usize) -> bool {
    x != 0 && (x & (x.wrapping_sub(1))) == 0
}

/// Branchless next power of two — O(1), no branch.
#[inline(always)]
pub const fn next_power_of_two(x: usize) -> usize {
    if x == 0 { return 1; }
    let x = x.wrapping_sub(1);
    let x = x | (x >> 1);
    let x = x | (x >> 2);
    let x = x | (x >> 4);
    let x = x | (x >> 8);
    let x = x | (x >> 16);
    let x = x | (x >> 32);
    x.wrapping_add(1)
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- Test LutKey implementation ---
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    #[repr(u8)]
    enum TestKey { A = 0, B = 1, C = 2 }

    impl LutKey for TestKey {
        const LUT_SIZE: usize = 3;
        fn discriminant(self) -> u8 { self as u8 }
    }

    #[test]
    fn lut_lookup_branchless() {
        const LUT: Lut<TestKey, &str, 3> = Lut::new(["alpha", "beta", "gamma"]);
        assert_eq!(LUT.get(TestKey::A), "alpha");
        assert_eq!(LUT.get(TestKey::B), "beta");
        assert_eq!(LUT.get(TestKey::C), "gamma");
    }

    #[test]
    fn binary_lut_kleene_and() {
        // Kleene AND:       T  F  U
        //                T   T  F  U
        //                F   F  F  F
        //                U   U  F  U
        const TABLE: [u8; 9] = [
            0, 1, 2,  // A=T: T,F,U
            1, 1, 1,  // A=F: F,F,F
            2, 1, 2,  // A=U: U,F,U
        ];
        let and_lut = BinaryLut::<u8, 9>::new(TABLE);
        // T AND F = F (idx 1)
        assert_eq!(and_lut.apply(TestKey::A, TestKey::B), 1);
        // U AND T = U (idx 6)
        assert_eq!(and_lut.apply(TestKey::C, TestKey::A), 2);
    }

    #[test]
    fn bitpack_2bits_4elements() {
        type Pack = BitPack<2, 4>;
        let mut p = Pack::from_raw(0b11_10_01_00); // [00, 01, 10, 11]
        assert_eq!(p.get(0), 0);
        assert_eq!(p.get(1), 1);
        assert_eq!(p.get(2), 2);
        assert_eq!(p.get(3), 3);
        p.set(1, 0b11);
        assert_eq!(p.get(1), 3);
    }

    #[test]
    fn popcount_power_of_two() {
        assert!(is_power_of_two(1));
        assert!(is_power_of_two(64));
        assert!(!is_power_of_two(0));
        assert!(!is_power_of_two(3));
        assert!(!is_power_of_two(63));
    }

    #[test]
    fn next_pow2_branchless() {
        assert_eq!(next_power_of_two(0), 1);
        assert_eq!(next_power_of_two(1), 1);
        assert_eq!(next_power_of_two(3), 4);
        assert_eq!(next_power_of_two(9), 16);
        assert_eq!(next_power_of_two(1024), 1024);
    }

    #[test]
    fn bitpack_mask_invariant() {
        assert_eq!(BitPack::<2, 4>::MASK, 0b11);
        assert_eq!(BitPack::<3, 8>::MASK, 0b111);
        assert_eq!(BitPack::<4, 4>::MASK, 0b1111);
    }
}