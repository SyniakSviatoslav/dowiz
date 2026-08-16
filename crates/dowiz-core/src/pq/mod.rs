//! Post-quantum crypto primitives (FIPS 203/204), zero external crates.
//!
//! This no_std core ships the pure primitives:
//! - `keccak`: inlined Keccak-f[1600] + SHAKE128/256 (FIPS 202) — the only digest primitive.
//! - `kem`: ML-KEM-768 (FIPS 203) keygen / encaps / decaps.
//! - `dsa`: ML-DSA-65 (FIPS 204) keygen / sign / verify.
//! - `entropy`: pure seed mixing/derivation (the std OS/QRNG provider is in the kernel shim).
//! - `fractal`, `root_delegation`: supporting primitives.
//!
//! The wire types that need serde derive impls (`codesign`, `envelope`, `hybrid`,
//! `hybrid_signing`) live here too, with their `Serialize`/`Deserialize` derives
//! gated behind the `json-api` feature (enabled transitively by the kernel's `pq`
//! feature). The one remaining kernel-side wire type is `volume` (AES-256-GCM
//! at-rest crypto) — it stays in the held-handle shim until the hand-rolled AES-GCM
//! lands, because the zero-dependency core cannot hold the `aes-gcm` crate.
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
pub mod codesign;
pub mod aes_gcm;
pub mod envelope;
pub mod hybrid;
pub mod hybrid_signing;
pub mod x25519;
