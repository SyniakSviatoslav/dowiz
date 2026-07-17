//! Post-quantum crypto primitives (FIPS 203/204), zero external crates.
//!
//! Modules:
//! - `keccak`: inlined Keccak-f[1600] + SHAKE128/256 (FIPS 202) — the only digest primitive.
//! - `kem`: ML-KEM-768 (FIPS 203) keygen / encaps / decaps.
//! - `dsa`: ML-DSA-65 (FIPS 204) keygen / sign / verify.
//!
//! All randomness must be supplied by the caller (`rng` fill closures) — no `rand`
//! dependency. Bit-exactness vs the NIST reference is verified by KAT tests that
//! decode the vendored mldsa-native / ACVP vectors.

pub mod codesign;
pub mod dsa;
pub mod entropy;
pub mod envelope;
pub mod fractal;
pub mod hybrid;
pub mod keccak;
pub mod kem;
pub mod volume;
pub mod x25519;
