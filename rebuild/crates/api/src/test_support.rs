//! Test-only support shared across the S2 auth test modules. NOT compiled into any release/normal
//! build (`#[cfg(test)]` at the declaration site in `main.rs`).
//!
//! ## Why runtime keygen instead of committed `.pem` files
//! The repo has an OPEN secrets-in-history incident and a guardrail literally named
//! "no .env/.pem/.key in git history", plus an open-source goal gated on secret scrubbing. So NO
//! key material — not even throwaway test keys — may enter the tree. This module GENERATES two
//! distinct throwaway RSA-2048 keypairs at test runtime (via the `rsa` crate, already fully
//! resolved in `Cargo.lock`) and encodes them to PKCS#8 / SPKI PEM, which `jsonwebtoken`'s
//! `from_rsa_pem` accepts. Nothing is written to disk; the PEM strings live only in test memory.
//!
//! Keygen is ~hundreds of ms per 2048-bit key, so both keypairs are generated ONCE per test
//! binary via `LazyLock` and leaked to `&'static str` (a test process runs once and exits, so the
//! one-time leak is intentional and bounded — never a per-test cost).

use std::sync::LazyLock;

use rsa::pkcs8::{EncodePrivateKey, EncodePublicKey, LineEnding};
use rsa::{RsaPrivateKey, RsaPublicKey};

/// One PEM keypair: `(pkcs8_private_pem, spki_public_pem)`, both `&'static str`.
struct KeyPairPem {
    private: &'static str,
    public: &'static str,
}

fn generate_keypair() -> KeyPairPem {
    let mut rng = rand::thread_rng();
    // 2048-bit RSA — matches the size the committed test keys used; large enough that RS256 signs
    // and verifies exactly as the prod keys would, small enough to keep test keygen quick.
    let private = RsaPrivateKey::new(&mut rng, 2048).expect("test RSA keygen must succeed");
    let public = RsaPublicKey::from(&private);
    let private_pem = private
        .to_pkcs8_pem(LineEnding::LF)
        .expect("PKCS#8 PEM encode must succeed")
        .to_string();
    let public_pem = public
        .to_public_key_pem(LineEnding::LF)
        .expect("SPKI PEM encode must succeed");
    KeyPairPem {
        // Leak to 'static: a test binary generates these once and runs to process exit, so this
        // is a bounded one-time allocation, not a per-test leak (see module doc).
        private: Box::leak(private_pem.into_boxed_str()),
        public: Box::leak(public_pem.into_boxed_str()),
    }
}

/// The "prod" test keypair — the analog of the old `prod_{private,public}.pem`.
static PROD: LazyLock<KeyPairPem> = LazyLock::new(generate_keypair);
/// The "dev" test keypair — a SECOND, distinct keypair (dev-kid segregation tests need it to
/// differ from prod). The analog of the old `dev_{private,public}.pem`. Only the `dev-routes`
/// test build references the dev keys, so gate it there to stay warning-clean in default builds.
#[cfg(feature = "dev-routes")]
static DEV: LazyLock<KeyPairPem> = LazyLock::new(generate_keypair);

pub(crate) mod keys {
    #[cfg(feature = "dev-routes")]
    use super::DEV;
    use super::PROD;

    /// PKCS#8 PEM of the throwaway "prod" private key (RS256 signing).
    pub(crate) fn prod_private() -> &'static str {
        PROD.private
    }
    /// SPKI PEM of the throwaway "prod" public key (RS256 verification).
    pub(crate) fn prod_public() -> &'static str {
        PROD.public
    }
    /// PKCS#8 PEM of the throwaway "dev" private key (distinct from prod — dev-kid segregation).
    /// `dev-routes`-only: every dev-key test site is itself feature-gated.
    #[cfg(feature = "dev-routes")]
    pub(crate) fn dev_private() -> &'static str {
        DEV.private
    }
    /// SPKI PEM of the throwaway "dev" public key.
    #[cfg(feature = "dev-routes")]
    pub(crate) fn dev_public() -> &'static str {
        DEV.public
    }
}

#[cfg(test)]
mod tests {
    use super::keys;

    #[test]
    fn prod_keypair_is_pem_shaped() {
        assert!(keys::prod_private().starts_with("-----BEGIN PRIVATE KEY-----"));
        assert!(keys::prod_public().starts_with("-----BEGIN PUBLIC KEY-----"));
    }

    #[cfg(feature = "dev-routes")]
    #[test]
    fn dev_keypair_is_pem_shaped_and_distinct_from_prod() {
        assert!(keys::dev_private().starts_with("-----BEGIN PRIVATE KEY-----"));
        assert!(keys::dev_public().starts_with("-----BEGIN PUBLIC KEY-----"));
        // The two keypairs MUST differ (dev-kid segregation relies on it).
        assert_ne!(keys::prod_private(), keys::dev_private());
        assert_ne!(keys::prod_public(), keys::dev_public());
    }

    #[test]
    fn lazylock_returns_the_same_static_across_calls() {
        // Proves keygen happens ONCE (the pointers are stable), not per call.
        assert_eq!(keys::prod_private().as_ptr(), keys::prod_private().as_ptr());
    }
}
