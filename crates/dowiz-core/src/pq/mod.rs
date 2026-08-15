//! Post-quantum crypto primitives (FIPS 203/204), zero external crates.
//!
//! This no_std core ships the pure primitives:
//! - `keccak`: inlined Keccak-f[1600] + SHAKE128/256 (FIPS 202) — the only digest primitive.
//! - `kem`: ML-KEM-768 (FIPS 203) keygen / encaps / decaps.
//! - `dsa`: ML-DSA-65 (FIPS 204) keygen / sign / verify.
//! - `entropy`: pure seed mixing/derivation (the std OS/QRNG provider is in the kernel shim).
//! - `fractal`, `root_delegation`: supporting primitives.
//!
//! The serde/external-crate wire types (`envelope`, `hybrid`, `hybrid_signing`,
//! `volume`, `codesign` — serde; `x25519` — curve25519-dalek) live in the kernel
//! held-handle shim: they need (de)serialization / external crypto crates that the
//! zero-dependency core cannot hold.
//!
//! All randomness must be supplied by the caller (`rng` fill closures) — no `rand`
//! dependency. Bit-exactness vs the NIST reference is verified by KAT tests that
//! decode the vendored mldsa-native / ACVP vectors.

pub mod dsa;
pub mod entropy;
pub mod fractal;
pub mod keccak;
pub mod kem;
pub mod root_delegation;
