//! Hypervector (VSA / hyperdimensional computing) — fixed-width bind/bundle.
//!
//! A hypervector is a fixed-width binary vector over `D` bits. Computation is
//! **symbolic-in-superposition**: every value/symbol/feature is a pseudo-random
//! `D`-bit code; structure is built with three operations that keep the width
//! constant, so there is **no rank blow-up** as graphs get folded into vectors:
//!
//! - **bind** (`⊗`): XOR — binds two vectors into one (associative, invertible:
//!   `a ⊗ b ⊗ b == a`). Encodes role–filler pairs and edge (subject, predicate,
//!   object) triples.
//! - **bundle** (`⊕`): element-wise majority — superposes many vectors into one
//!   "average" that is still similar to each constituent. Encodes sets.
//! - **similarity**: normalized Hamming overlap in `[0,1]` (1.0 = identical,
//!   0.5 = orthogonal/random).
//!
//! The design doc (`docs/design/internal-retrieval-living-memory-blueprint.md`)
//! calls this the pragmatic "tensor-like" win: fold N-ary graphs into
//! fixed-width hypervectors at O(N log N), built on the same `csr`/`spectral`
//! vocabulary. This module is the zero-dep primitive; `csr`-backed accumulation
//! can consume it without pulling anything external.
//!
//! Zero external crates. Pure `std`. Deterministic (seeded via [`splitmix64`]).

/// Default hypervector width in bits. Power of two, word-aligned.
pub const D: usize = 1024;
/// Number of `u64` words per hypervector.
pub const WORDS: usize = D / 64;

/// A fixed-width `D`-bit binary hypervector, packed into `u64` words.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Hypervector {
    words: [u64; WORDS],
}

impl Hypervector {
    /// The all-zero vector.
    pub const fn zero() -> Self {
        Self { words: [0u64; WORDS] }
    }

    /// Build from raw words (caller guarantees `words.len() == WORDS`).
    pub fn from_words(words: [u64; WORDS]) -> Self {
        Self { words }
    }

    pub const fn as_words(&self) -> &[u64; WORDS] {
        &self.words
    }

    /// A deterministic pseudo-random code for a symbol/seed. The same seed
    /// always yields the same code; distinct seeds are (with overwhelming
    /// probability) near-orthogonal.
    pub fn code(seed: u64) -> Self {
        let mut s = seed;
        let mut words = [0u64; WORDS];
        for w in words.iter_mut() {
            *w = crate::rng::splitmix64(&mut s);
        }
        Self { words }
    }

    /// Bind (`⊗`): XOR. `a.bind(b).bind(b) == a` (self-inverse).
    pub fn bind(self, other: &Self) -> Self {
        let mut words = [0u64; WORDS];
        for i in 0..WORDS {
            words[i] = self.words[i] ^ other.words[i];
        }
        Self { words }
    }

    /// Bundle (`⊕`): element-wise majority over a set of vectors. Ties (equal
    /// 0/1 counts) resolve to 0 deterministically, so bundling is stable.
    pub fn bundle<'a, I: IntoIterator<Item = &'a Self>>(items: I) -> Self {
        let mut counts = [0i32; WORDS * 64];
        let mut n = 0i32;
        for item in items {
            n += 1;
            for (w, word) in item.words.iter().enumerate() {
                for b in 0..64 {
                    if (word >> b) & 1 == 1 {
                        counts[w * 64 + b] += 1;
                    }
                }
            }
        }
        let mut words = [0u64; WORDS];
        if n > 0 {
            for (i, &c) in counts.iter().enumerate() {
                // majority: bit is 1 if more than half the items set it
                if c * 2 > n {
                    words[i / 64] |= 1u64 << (i % 64);
                }
            }
        }
        Self { words }
    }

    /// Normalized similarity in `[0,1]`: fraction of agreeing bits.
    /// 1.0 = identical, 0.5 = random/orthogonal (expected for unrelated codes).
    pub fn similarity(&self, other: &Self) -> f64 {
        let mut same = 0u32;
        for i in 0..WORDS {
            same += (!(self.words[i] ^ other.words[i])).count_ones();
        }
        same as f64 / D as f64
    }

    /// Hamming distance (count of disagreeing bits).
    pub fn hamming(&self, other: &Self) -> u32 {
        let mut diff = 0u32;
        for i in 0..WORDS {
            diff += (self.words[i] ^ other.words[i]).count_ones();
        }
        diff
    }

    /// Permute (fixed rotation by `shift` bits) — encodes sequence/order.
    pub fn permute(&self, shift: usize) -> Self {
        let shift = shift % D;
        if shift == 0 {
            return *self;
        }
        // Bit-level rotation across the whole D-bit word array.
        let mut out = [0u64; WORDS];
        let word_shift = shift / 64;
        let bit_shift = shift % 64;
        for i in 0..WORDS {
            let src = (i + WORDS - word_shift) % WORDS;
            let hi = self.words[src].wrapping_shl(bit_shift as u32);
            let lo = if bit_shift == 0 {
                0
            } else {
                let prev = (src + WORDS - 1) % WORDS;
                self.words[prev].wrapping_shr((64 - bit_shift) as u32)
            };
            out[i] = hi | lo;
        }
        Self { words: out }
    }

    /// Count of set bits (population count over the whole vector).
    pub fn popcount(&self) -> u32 {
        self.words.iter().map(|w| w.count_ones()).sum()
    }
}

impl core::ops::BitXor for Hypervector {
    type Output = Self;
    fn bitxor(self, rhs: Self) -> Self {
        self.bind(&rhs)
    }
}

impl core::ops::BitXorAssign for Hypervector {
    fn bitxor_assign(&mut self, rhs: Self) {
        for i in 0..WORDS {
            self.words[i] ^= rhs.words[i];
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn width_is_word_aligned() {
        assert_eq!(D % 64, 0);
        assert_eq!(WORDS * 64, D);
    }

    #[test]
    fn bind_is_self_inverse() {
        let a = Hypervector::code(1);
        let b = Hypervector::code(2);
        assert_eq!(a.bind(&b).bind(&b), a);
        // XOR is commutative
        assert_eq!(a.bind(&b), b.bind(&a));
    }

    #[test]
    fn distinct_codes_are_near_orthogonal() {
        let a = Hypervector::code(7);
        let b = Hypervector::code(8);
        let sim = a.similarity(&b);
        assert!(sim > 0.45 && sim < 0.55, "unrelated codes ~0.5, got {sim}");
    }

    #[test]
    fn same_seed_is_identical() {
        assert_eq!(Hypervector::code(42), Hypervector::code(42));
        assert_eq!(Hypervector::code(42).similarity(&Hypervector::code(42)), 1.0);
    }

    #[test]
    fn bundle_is_similar_to_its_constituents() {
        let a = Hypervector::code(1);
        let b = Hypervector::code(2);
        let c = Hypervector::code(3);
        let bundled = Hypervector::bundle([&a, &b, &c]);
        assert!(bundled.similarity(&a) > 0.55);
        assert!(bundled.similarity(&b) > 0.55);
        assert!(bundled.similarity(&c) > 0.55);
    }

    #[test]
    fn bound_pair_recovers_cleanly() {
        // role-filler: name ⊗ subject, then bundle; similarity to name ⊗ subject
        // recovers the "subject" binding even after superposition with noise.
        let name = Hypervector::code(100);
        let subject = Hypervector::code(200);
        let noise = Hypervector::code(300);
        let record = Hypervector::bundle([&name.bind(&subject), &noise]);
        let probe = name.bind(&subject);
        assert!(record.similarity(&probe) > 0.7);
    }

    #[test]
    fn permute_is_invertible_over_full_period() {
        let v = Hypervector::code(99);
        assert_eq!(v.permute(0), v);
        assert_eq!(v.permute(D), v);
        // permute by half-width twice = identity (for even D)
        assert_eq!(v.permute(D / 2).permute(D / 2), v);
    }

    #[test]
    fn hamming_complements_similarity() {
        let a = Hypervector::code(5);
        let b = Hypervector::code(6);
        assert_eq!(a.hamming(&b), (D as f64 * (1.0 - a.similarity(&b))).round() as u32);
    }
}
